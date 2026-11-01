//! Takes over the data of the app's former name, nd104-sync (up to 1.5.x):
//!
//! | Old | New |
//! |---|---|
//! | `%APPDATA%\nd104-sync` (config, keymap page, profiles) | `%APPDATA%\nd104-studio` |
//! | `%LOCALAPPDATA%\nd104-sync\keymap-backups` | `%LOCALAPPDATA%\nd104-studio\keymap-backups` |
//! | `%LOCALAPPDATA%\nd104-sync\logs` | removed (diagnostics only, new logs start fresh) |
//! | `%LOCALAPPDATA%\io.github.mrurdam.nd104-sync` (window storage) | `…\io.github.mrurdam.nd104-studio` |
//! | Credential Manager `<provider>.nd104-sync` | `<provider>.nd104-studio` |
//!
//! Runs at every start of the app and in the setup; once everything has
//! moved it finds nothing to do. Data that already exists under the new name
//! always wins – nothing new is ever overwritten. Old folders are removed
//! only once they are empty.

use std::path::Path;

use anyhow::Result;

use crate::{config, secrets, weather};

/// Technical name before the rename.
pub const OLD_ID: &str = "nd104-sync";
/// Window storage (WebView2 data) is kept per app identifier.
pub const OLD_IDENTIFIER: &str = "io.github.mrurdam.nd104-sync";
pub const IDENTIFIER: &str = "io.github.mrurdam.nd104-studio";

/// Moves everything over; returns what was done (for the log).
pub fn migrate() -> Vec<String> {
    let mut done = Vec::new();
    let mut note = |what: &str, result: Result<usize>| match result {
        Ok(0) => {}
        Ok(n) => done.push(format!("{what}: {n} moved")),
        Err(e) => done.push(format!("{what}: {e:#}")),
    };

    if let Ok(new) = config::config_dir() {
        let old = new.with_file_name(OLD_ID);
        note("settings", move_entries(&old, &new, |_| true));
    }
    if let Ok(new) = config::local_data_dir() {
        let old = new.with_file_name(OLD_ID);
        note(
            "keymap backups",
            move_entries(&old, &new, |name| name == "keymap-backups"),
        );
        // old logs are not worth keeping
        let _ = std::fs::remove_dir_all(old.join("logs"));
        remove_if_empty(&old);
        let old_web = new.with_file_name(OLD_IDENTIFIER);
        let new_web = new.with_file_name(IDENTIFIER);
        if old_web.is_dir() && !new_web.exists() {
            note(
                "window storage",
                std::fs::rename(&old_web, &new_web)
                    .map(|()| 1)
                    .map_err(Into::into),
            );
        } else if old_web.is_dir() {
            let _ = std::fs::remove_dir_all(&old_web);
        }
    }
    let providers: Vec<&str> = weather::PROVIDERS
        .iter()
        .filter(|p| p.requires_api_key)
        .map(|p| p.id)
        .collect();
    note("API keys", secrets::take_over(OLD_ID, &providers));
    done
}

/// Moves the entries of `old` that `wanted` accepts into `new`, unless
/// `new` already has an entry of that name. Removes `old` once it is empty.
fn move_entries(old: &Path, new: &Path, wanted: impl Fn(&str) -> bool) -> Result<usize> {
    if !old.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(new)?;
    let mut moved = 0;
    for entry in std::fs::read_dir(old)? {
        let entry = entry?;
        let name = entry.file_name();
        if !wanted(&name.to_string_lossy()) {
            continue;
        }
        let target = new.join(&name);
        if target.exists() {
            continue;
        }
        std::fs::rename(entry.path(), &target)?;
        moved += 1;
    }
    remove_if_empty(old);
    Ok(moved)
}

fn remove_if_empty(dir: &Path) {
    // fails (and keeps the folder) when something is left in it
    let _ = std::fs::remove_dir(dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nd104-rebrand-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn moves_missing_entries_and_keeps_new_ones() {
        let root = temp("move");
        let (old, new) = (root.join("nd104-sync"), root.join("nd104-studio"));
        std::fs::create_dir_all(old.join("keymap-profiles")).unwrap();
        std::fs::write(old.join("keymap-profiles").join("p1.json"), "profile").unwrap();
        std::fs::write(old.join("config.toml"), "old").unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("config.toml"), "new").unwrap();

        assert_eq!(move_entries(&old, &new, |_| true).unwrap(), 1);
        assert_eq!(
            std::fs::read_to_string(new.join("config.toml")).unwrap(),
            "new"
        );
        assert!(new.join("keymap-profiles").join("p1.json").exists());
        // the old config stays, so the old folder stays too
        assert!(old.join("config.toml").exists());

        std::fs::remove_file(old.join("config.toml")).unwrap();
        assert_eq!(move_entries(&old, &new, |_| true).unwrap(), 0);
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn filter_and_missing_old_folder() {
        let root = temp("filter");
        let (old, new) = (root.join("old"), root.join("new"));
        assert_eq!(move_entries(&old, &new, |_| true).unwrap(), 0);
        std::fs::create_dir_all(old.join("keymap-backups")).unwrap();
        std::fs::create_dir_all(old.join("logs")).unwrap();
        assert_eq!(
            move_entries(&old, &new, |n| n == "keymap-backups").unwrap(),
            1
        );
        assert!(new.join("keymap-backups").is_dir());
        assert!(old.join("logs").is_dir());
        let _ = std::fs::remove_dir_all(root);
    }
}
