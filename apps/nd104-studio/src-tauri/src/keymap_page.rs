//! Commands for the keymap page: read the keymap, write changes (after an
//! automatic backup of the current state), the page's own settings and
//! profiles (a keymap with a name and the application it is meant for).

use std::path::PathBuf;

use nd104_core::config;
use nd104_core::keymap::{Keymap, KeymapBackup, KeymapOp, KeymapResult};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::keymap_backup::{connected, read, run};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Automatic backups kept before every write from the keymap page.
const AUTO_BACKUPS_KEPT: usize = 20;

/// Reads all layers and the knob.
#[tauri::command]
pub async fn keymap_read(app: AppHandle) -> CmdResult<Keymap> {
    connected(&app)?;
    read(&app).await
}

/// Writes `target`: the current keymap goes to an automatic backup first,
/// then only the differences are written and everything is read back.
/// Returns the number of changes written.
#[tauri::command]
pub async fn keymap_write(app: AppHandle, target: Keymap) -> CmdResult<usize> {
    let (model, spec, firmware) = connected(&app)?;
    target.validate(&spec).map_err(|e| format!("{e:#}"))?;
    let current = read(&app).await?;
    if target.changes_from(&current, &spec).is_empty() {
        return Ok(0);
    }
    let backup = KeymapBackup::new(current, &spec, &model, &firmware);
    let path = save_auto_backup(&backup).map_err(|e| format!("automatic backup failed: {e:#}"))?;
    tracing::info!("keymap: automatic backup {} before writing", path.display());
    match run(&app, KeymapOp::Restore(target)).await? {
        KeymapResult::Restored(n) => Ok(n),
        KeymapResult::Read(_) => Err("unexpected answer".into()),
    }
}

fn save_auto_backup(backup: &KeymapBackup) -> anyhow::Result<PathBuf> {
    let dir = config::local_data_dir()?.join("keymap-backups");
    std::fs::create_dir_all(&dir)?;
    let stamp = jiff::Zoned::now().strftime("%Y-%m-%d_%H-%M-%S").to_string();
    let path = dir.join(format!("auto-{stamp}.json"));
    std::fs::write(&path, backup.to_json()?)?;
    // keep the newest ones; the names sort by time
    let mut autos: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("auto-") && n.ends_with(".json"))
        })
        .collect();
    autos.sort();
    let surplus = autos.len().saturating_sub(AUTO_BACKUPS_KEPT);
    for old in &autos[..surplus] {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

// --- page settings (layer names, shape, label presets); the page owns the content

const PAGE_SETTINGS_MAX: usize = 256 * 1024;

fn page_settings_path() -> CmdResult<PathBuf> {
    Ok(config::config_dir().map_err(err)?.join("keymap-page.json"))
}

#[tauri::command]
pub fn keymap_page_get() -> CmdResult<Option<serde_json::Value>> {
    let Ok(text) = std::fs::read_to_string(page_settings_path()?) else {
        return Ok(None);
    };
    // a broken file starts over with the defaults instead of blocking the page
    Ok(serde_json::from_str(&text).ok())
}

#[tauri::command]
pub fn keymap_page_set(settings: serde_json::Value) -> CmdResult<()> {
    let text = serde_json::to_string_pretty(&settings).map_err(err)?;
    if text.len() > PAGE_SETTINGS_MAX {
        return Err("keymap page settings are too large".into());
    }
    let path = page_settings_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

// --- profiles: a keymap backup file with a name and an application

#[derive(Serialize, Deserialize, Clone)]
struct ProfileMeta {
    name: String,
    #[serde(default)]
    app: String,
}

#[derive(Serialize, Deserialize)]
struct ProfileFile {
    profile: ProfileMeta,
    #[serde(flatten)]
    backup: KeymapBackup,
}

#[derive(Serialize)]
pub struct ProfileInfo {
    id: String,
    name: String,
    app: String,
    created: String,
}

fn profiles_dir() -> CmdResult<PathBuf> {
    Ok(config::config_dir().map_err(err)?.join("keymap-profiles"))
}

/// Profile ids are file names we made ourselves; anything else is refused.
fn profile_path(id: &str) -> CmdResult<PathBuf> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err("invalid profile id".into());
    }
    Ok(profiles_dir()?.join(format!("{id}.json")))
}

fn read_profile(id: &str) -> CmdResult<ProfileFile> {
    let text = std::fs::read_to_string(profile_path(id)?)
        .map_err(|e| format!("cannot read profile: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("broken profile file: {e}"))
}

fn write_profile(profile: &ProfileFile) -> CmdResult<ProfileInfo> {
    std::fs::create_dir_all(profiles_dir()?).map_err(err)?;
    let id = format!("p{}", jiff::Timestamp::now().as_millisecond());
    let text = serde_json::to_string_pretty(profile).map_err(err)?;
    std::fs::write(profile_path(&id)?, text).map_err(err)?;
    Ok(info(id, profile))
}

fn info(id: String, profile: &ProfileFile) -> ProfileInfo {
    ProfileInfo {
        id,
        name: profile.profile.name.clone(),
        app: profile.profile.app.clone(),
        created: profile.backup.created.clone(),
    }
}

fn clean(text: &str, max: usize) -> String {
    text.trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}

#[tauri::command]
pub fn keymap_profiles() -> CmdResult<Vec<ProfileInfo>> {
    let Ok(entries) = std::fs::read_dir(profiles_dir()?) else {
        return Ok(Vec::new());
    };
    let mut list: Vec<ProfileInfo> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let id = p.file_stem()?.to_str()?.to_string();
            read_profile(&id).ok().map(|f| info(id, &f))
        })
        .collect();
    list.sort_by_key(|p| p.name.to_lowercase());
    Ok(list)
}

/// Saves `keymap` (the page's current state) as a new profile.
#[tauri::command]
pub fn keymap_profile_save(
    app: AppHandle,
    name: String,
    application: String,
    keymap: Keymap,
) -> CmdResult<ProfileInfo> {
    let (model, spec, firmware) = connected(&app)?;
    keymap.validate(&spec).map_err(|e| format!("{e:#}"))?;
    let name = clean(&name, 60);
    if name.is_empty() {
        return Err("Give the profile a name.".into());
    }
    write_profile(&ProfileFile {
        profile: ProfileMeta {
            name,
            app: clean(&application, 80),
        },
        backup: KeymapBackup::new(keymap, &spec, &model, &firmware),
    })
}

/// The keymap of a profile, checked against the connected keyboard.
#[tauri::command]
pub fn keymap_profile_get(app: AppHandle, id: String) -> CmdResult<Keymap> {
    let (model, spec, _) = connected(&app)?;
    let json = read_profile(&id)?
        .backup
        .to_json()
        .map_err(|e| format!("{e:#}"))?;
    let backup = KeymapBackup::from_json(&json, &model, &spec).map_err(|e| format!("{e:#}"))?;
    Ok(backup.keymap)
}

#[tauri::command]
pub fn keymap_profile_delete(id: String) -> CmdResult<()> {
    std::fs::remove_file(profile_path(&id)?).map_err(err)
}

/// Exports a profile to a file of the user's choice. `None`: cancelled.
#[tauri::command]
pub async fn keymap_profile_export(app: AppHandle, id: String) -> CmdResult<Option<String>> {
    let file = read_profile(&id)?;
    let stem: String = file
        .profile
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = format!("nd104-profile-{stem}.json");
    let dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .dialog()
            .file()
            .set_file_name(&name)
            .add_filter("nd104-studio keymap profile", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    let text = serde_json::to_string_pretty(&file).map_err(err)?;
    std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Imports an exported profile (or a plain keymap backup) as a new profile.
/// `None`: cancelled.
#[tauri::command]
pub async fn keymap_profile_import(app: AppHandle) -> CmdResult<Option<ProfileInfo>> {
    let (model, spec, _) = connected(&app)?;
    let dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .dialog()
            .file()
            .add_filter("nd104-studio keymap profile", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > 1024 * 1024 {
        return Err("This file is too large to be a keymap profile.".into());
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let backup = KeymapBackup::from_json(&text, &model, &spec).map_err(|e| format!("{e:#}"))?;
    let meta = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| serde_json::from_value::<ProfileMeta>(v.get("profile")?.clone()).ok())
        .unwrap_or_else(|| ProfileMeta {
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            app: String::new(),
        });
    let name = clean(&meta.name, 60);
    write_profile(&ProfileFile {
        profile: ProfileMeta {
            name: if name.is_empty() {
                "Profile".into()
            } else {
                name
            },
            app: clean(&meta.app, 80),
        },
        backup,
    })
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ids_cannot_leave_the_folder() {
        assert!(profile_path("p1730000000000").is_ok());
        for bad in ["", "../config", "a/b", "a\\b", "c:x", "x.json"] {
            assert!(profile_path(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn names_are_trimmed_and_cut() {
        assert_eq!(clean("  Game\n ", 60), "Game");
        assert_eq!(clean("abcdef", 3), "abc");
    }
}
