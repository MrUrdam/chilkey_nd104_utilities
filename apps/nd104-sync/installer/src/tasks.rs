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

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct InstallOptions {
    pub autostart: bool,
    pub desktop_shortcut: bool,
}

/// Webview data of the app (cache, local storage).
fn webview_data_dir() -> Result<PathBuf> {
    Ok(system::data_dir()?.with_file_name(system::APP_USER_MODEL_ID))
}

/// Files the old NSIS setup put into `%LOCALAPPDATA%\nd104-sync` (next to the logs).
const LEGACY_FILES: &[&str] = &[
    "nd104-sync.exe",
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
        "this setup does not contain nd104-sync"
    );

    let dir = system::install_dir()?;
    let previous = system::installed();
    let legacy_dir = previous
        .as_ref()
        .filter(|p| p.legacy && p.location.as_os_str().len() > 3)
        .map(|p| p.location.clone());

    // 1. close the running app (current and old install location)
    say(Step::Closing, 0.03);
    let mut dirs = vec![dir.clone()];
    dirs.extend(legacy_dir.clone());
    // only 1.1.0+ understands --quit; older versions are ended directly
    let graceful = previous
        .as_ref()
        .filter(|p| !p.legacy)
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

    // 3. swap in the new version
    say(Step::Finishing, 0.84);
    let backup = sibling(&dir, "old");
    remove_dir(&backup)?;
    if dir.exists() {
        retry(|| std::fs::rename(&dir, &backup)).inspect_err(|_| {
            let _ = remove_dir(&staging);
        })?;
    }
    if let Err(e) = retry(|| std::fs::rename(&staging, &dir)) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, &dir);
        }
        let _ = remove_dir(&staging);
        return Err(e);
    }

    // 4. Windows integration; on failure the old files come back
    let exe = dir.join(APP_EXE);
    let integrated = (|| -> Result<()> {
        if let Some(legacy) = &legacy_dir {
            migrate_legacy(legacy);
        }
        say(Step::Finishing, 0.9);
        system::create_shortcuts(&exe, options.desktop_shortcut)?;
        system::set_autostart(options.autostart.then_some(exe.as_path()))?;
        store_autostart(options.autostart)?;
        system::register(&dir, nd104_core::APP_VERSION)
    })();
    if let Err(e) = integrated {
        if backup.exists() {
            let _ = remove_dir(&dir);
            let _ = std::fs::rename(&backup, &dir);
        }
        return Err(e);
    }
    let _ = remove_dir(&backup);
    say(Step::Done, 1.0);
    Ok(())
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

/// The app re-applies `general.autostart` on every start, so it must agree.
fn store_autostart(enabled: bool) -> Result<()> {
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
    if config.general.autostart != enabled || !path.exists() {
        config.general.autostart = enabled;
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

/// Files of a just-ended process can stay locked for a moment
/// (virus scanners, Explorer); try a few times.
fn retry(mut op: impl FnMut() -> std::io::Result<()>) -> Result<()> {
    let mut last = None;
    for _ in 0..10 {
        match op() {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err(last
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow::anyhow!("failed")))
}
