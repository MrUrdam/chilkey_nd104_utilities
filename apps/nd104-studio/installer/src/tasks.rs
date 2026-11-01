//! Install, update and uninstall.
//!
//! An update is an install over the existing one: the new files are unpacked
//! next to the old ones and swapped in at the end, so a failure leaves the
//! old version working (rollback). Settings, API keys and logs are not
//! touched by install or update.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use nd104_core::config::{self, Config};
use nd104_core::weather;
use serde::Serialize;

use crate::payload;
use crate::system::{self, APP_EXE, SETUP_EXE};

/// What the progress line says (keys in the locale files).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Preparing,
    Closing,
    Copying,
    Finishing,
    Removing,
    Done,
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub step: Step,
    /// 0.0 … 1.0
    pub value: f32,
}

pub type Report<'a> = &'a (dyn Fn(Progress) + Sync);

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InstallOptions {
    pub autostart: bool,
    pub desktop_shortcut: bool,
    /// Look of the app windows (`calm` / `gamer`); `None` keeps the setting.
    #[serde(default)]
    pub style: Option<String>,
}

/// Webview data of the app (cache, local storage).
fn webview_data_dir() -> Result<PathBuf> {
    Ok(system::data_dir()?.with_file_name(system::APP_USER_MODEL_ID))
}

/// Files the old NSIS setup put into `%LOCALAPPDATA%\nd104-studio` (next to the logs).
const LEGACY_FILES: &[&str] = &[
    "nd104-studio.exe",
    "nd104-cli.exe",
    "nd104-probe.exe",
    "uninstall.exe",
    "README.md",
    "CHANGELOG.md",
    "LICENSE",
];

pub fn install(options: InstallOptions, report: Report) -> Result<()> {
    let say = |step, value| report(Progress { step, value });
    say(Step::Preparing, 0.0);
    anyhow::ensure!(
        payload::available(),
        "this setup does not contain nd104-studio"
    );

    let dir = system::install_dir()?;
    let previous = system::installed();
    let legacy_dir = previous
        .as_ref()
        .filter(|p| p.legacy && p.location.as_os_str().len() > 3)
        .map(|p| p.location.clone());
    // installed by the setup under the former name nd104-sync (1.1–1.5)
    let old_name_dir = system::old_installed()
        .filter(|p| !p.legacy && p.location.as_os_str().len() > 3 && p.location != dir)
        .map(|p| p.location);

    // 1. close the running app (current and old install locations)
    say(Step::Closing, 0.03);
    if let Some(old) = &old_name_dir {
        system::stop_app(
            std::slice::from_ref(old),
            Some(&old.join(system::OLD_APP_EXE)),
        )?;
    }
    let mut dirs = vec![dir.clone()];
    dirs.extend(legacy_dir.clone());
    // only 1.1.0+ understands --quit; older versions are ended directly
    let graceful = previous
        .as_ref()
        .filter(|p| !p.legacy && !p.old_name)
        .map(|_| dir.join(APP_EXE));
    system::stop_app(&dirs, graceful.as_deref())?;

    // 2. unpack next to the installation
    say(Step::Copying, 0.08);
    let staging = sibling(&dir, "new");
    remove_dir(&staging)?;
    std::fs::create_dir_all(&staging)
        .with_context(|| format!("cannot create {}", staging.display()))?;
    let unpacked = payload::extract(&staging, |done, total| {
        say(Step::Copying, 0.08 + 0.72 * done as f32 / total as f32)
    })
    .and_then(|()| {
        // the setup stays in the folder as uninstaller (and for repairs)
        std::fs::copy(std::env::current_exe()?, staging.join(SETUP_EXE))?;
        Ok(())
    });
    if let Err(e) = unpacked {
        let _ = remove_dir(&staging);
        return Err(e);
    }

    // 3. swap in the new version, file by file
    say(Step::Finishing, 0.84);
    // left over by setups up to 1.4.0, which swapped the whole folder
    let _ = remove_dir(&sibling(&dir, "old"));
    let swap = match swap_in(&staging, &dir) {
        Ok(swap) => swap,
        Err(e) => {
            let _ = remove_dir(&staging);
            return Err(e);
        }
    };

    // 4. Windows integration; on failure the old files come back
    let exe = dir.join(APP_EXE);
    let integrated = (|| -> Result<()> {
        if let Some(legacy) = &legacy_dir {
            migrate_legacy(legacy);
        }
        // settings, keymap backups and API keys of the former name; before
        // store_settings, which would otherwise start a fresh config
        for line in nd104_core::rebrand::migrate() {
            eprintln!("taken over from nd104-sync – {line}");
        }
        say(Step::Finishing, 0.9);
        system::create_shortcuts(&exe, options.desktop_shortcut)?;
        system::set_autostart(options.autostart.then_some(exe.as_path()))?;
        store_settings(options.autostart, options.style.as_deref())?;
        system::register(&dir, nd104_core::APP_VERSION)
    })();
    if let Err(e) = integrated {
        swap.roll_back();
        let _ = remove_dir(&staging);
        return Err(e);
    }
    swap.finish();
    let _ = remove_dir(&staging);
    // the former name: its folder and Windows entries go once the new
    // installation is complete
    if let Some(old) = &old_name_dir {
        let _ = remove_dir(old);
    }
    let _ = system::remove_old_integration();
    say(Step::Done, 1.0);
    Ok(())
}

/// Files moved into the installation, for rollback and clean-up.
struct Swap {
    /// (installed file, its previous version renamed to `*.old`)
    moved: Vec<(PathBuf, Option<PathBuf>)>,
}

impl Swap {
    /// Puts the previous files back.
    fn roll_back(self) {
        for (target, old) in self.moved.into_iter().rev() {
            let _ = std::fs::remove_file(&target);
            if let Some(old) = old {
                let _ = std::fs::rename(&old, &target);
            }
        }
    }

    /// Deletes the previous files. One still locked (e.g. by a virus
    /// scanner) stays as `*.old` and is removed by the next update.
    fn finish(self) {
        for (_, old) in self.moved {
            if let Some(old) = old {
                let _ = std::fs::remove_file(old);
            }
        }
    }
}

/// Moves every file from `staging` into `dir`. An existing file is renamed
/// to `<name>.old` first: Windows allows renaming a file that another
/// process still has open (a just-ended app being scanned, Explorer reading
/// its icon), but not renaming the folder that contains it (os error 32).
fn swap_in(staging: &Path, dir: &Path) -> Result<Swap> {
    let mut swap = Swap { moved: Vec::new() };
    let result = (|| -> Result<()> {
        for source in files_in(staging)? {
            let relative = source.strip_prefix(staging)?;
            let target = dir.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("cannot create {}", parent.display()))?;
            }
            let old = if target.exists() {
                let old = with_suffix(&target, "old");
                // a leftover from an earlier update
                let _ = std::fs::remove_file(&old);
                retry(|| std::fs::rename(&target, &old))
                    .with_context(|| format!("{} is in use", target.display()))?;
                Some(old)
            } else {
                None
            };
            if let Err(e) = retry(|| std::fs::rename(&source, &target)) {
                if let Some(old) = &old {
                    let _ = std::fs::rename(old, &target);
                }
                return Err(e).with_context(|| format!("cannot replace {}", target.display()));
            }
            swap.moved.push((target, old));
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(swap),
        Err(e) => {
            swap.roll_back();
            Err(e)
        }
    }
}

fn files_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(files_in(&path)?);
        } else {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

/// Removes the installation of the old NSIS setup (≤ 1.0.1). Logs stay.
fn migrate_legacy(dir: &Path) {
    // registers again right after, under the same name
    let _ = system::run_legacy_uninstaller(dir);
    for name in LEGACY_FILES {
        let _ = std::fs::remove_file(dir.join(name));
    }
    // empty folder only (the logs live here too)
    let _ = std::fs::remove_dir(dir);
}

/// The app re-applies `general.autostart` on every start, so it must agree;
/// the style chosen in the setup is stored too.
fn store_settings(enabled: bool, style: Option<&str>) -> Result<()> {
    let path = config::default_path()?;
    let mut config = if path.exists() {
        match Config::load(&path) {
            Ok(c) => c,
            // never overwrite a config the user has to repair
            Err(_) => return Ok(()),
        }
    } else {
        Config::default()
    };
    let style = style.filter(|s| *s == "calm" || *s == "gamer");
    let style_changed = style.is_some_and(|s| config.general.style != s);
    if config.general.autostart != enabled || style_changed || !path.exists() {
        config.general.autostart = enabled;
        if let Some(style) = style {
            config.general.style = style.to_string();
        }
        config.save(&path)?;
    }
    Ok(())
}

pub fn uninstall(keep_settings: bool, report: Report) -> Result<()> {
    let say = |step, value| report(Progress { step, value });
    say(Step::Closing, 0.05);
    let dir = system::installed()
        .map(|i| i.location)
        .filter(|l| l.as_os_str().len() > 3)
        .unwrap_or(system::install_dir()?);
    system::stop_app(std::slice::from_ref(&dir), Some(&dir.join(APP_EXE)))?;

    say(Step::Removing, 0.3);
    system::remove_shortcuts()?;
    system::set_autostart(None)?;
    let _ = system::remove_old_integration();
    remove_dir(&dir)?;
    say(Step::Removing, 0.7);

    if !keep_settings {
        remove_dir(&config::config_dir()?)?;
        remove_dir(&system::data_dir()?)?;
        let _ = remove_dir(&webview_data_dir()?);
        for provider in weather::PROVIDERS.iter().filter(|p| p.requires_api_key) {
            let _ = nd104_core::secrets::delete(provider.id);
        }
    }
    system::unregister()?;
    say(Step::Done, 1.0);
    Ok(())
}

fn sibling(dir: &Path, suffix: &str) -> PathBuf {
    let mut name = dir.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    dir.with_file_name(name)
}

fn remove_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    retry(|| std::fs::remove_dir_all(dir))
        .with_context(|| format!("cannot remove {}", dir.display()))
}

/// Files of a just-ended process can stay locked for a while (virus
/// scanners, Explorer); keep trying for about 10 seconds.
fn retry(mut op: impl FnMut() -> std::io::Result<()>) -> Result<()> {
    let mut last = None;
    for _ in 0..40 {
        match op() {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(last
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow::anyhow!("failed")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nd104-setup-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn swaps_files_while_one_is_open_and_cleans_up() {
        let root = temp("swap");
        let (dir, staging) = (root.join("app"), root.join("app.new"));
        write(&dir.join("nd104-studio.exe"), "old exe");
        write(&dir.join("README.md"), "old readme");
        write(&dir.join("only-old.txt"), "stays");
        write(&staging.join("nd104-studio.exe"), "new exe");
        write(&staging.join("README.md"), "new readme");
        write(&staging.join("sub/extra.txt"), "new extra");
        write(&dir.join("README.md.old"), "leftover of an earlier update");

        // like a virus scanner: the file stays open during the swap
        let held = std::fs::File::open(dir.join("nd104-studio.exe")).unwrap();
        let swap = swap_in(&staging, &dir).unwrap();
        drop(held);
        swap.finish();

        assert_eq!(read(&dir.join("nd104-studio.exe")), "new exe");
        assert_eq!(read(&dir.join("README.md")), "new readme");
        assert_eq!(read(&dir.join("sub/extra.txt")), "new extra");
        assert_eq!(read(&dir.join("only-old.txt")), "stays");
        assert!(!dir.join("nd104-studio.exe.old").exists());
        assert!(!dir.join("README.md.old").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn roll_back_restores_the_previous_files() {
        let root = temp("rollback");
        let (dir, staging) = (root.join("app"), root.join("app.new"));
        write(&dir.join("nd104-studio.exe"), "old exe");
        write(&staging.join("nd104-studio.exe"), "new exe");
        write(&staging.join("new-file.txt"), "new");

        swap_in(&staging, &dir).unwrap().roll_back();

        assert_eq!(read(&dir.join("nd104-studio.exe")), "old exe");
        assert!(!dir.join("new-file.txt").exists());
        assert!(!dir.join("nd104-studio.exe.old").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
