//! API keys in the operating system's credential store.
//!
//! Windows: Credential Manager, entry `<provider>.nd104-studio`. Linux: the
//! Secret Service (GNOME Keyring, KWallet) via `keyring`. Keys are never
//! written to the config file or the log.
//!
//! Linux fallback: on a desktop with no running Secret Service (minimal window
//! managers such as Sway/i3/Hyprland, or headless), the keyring calls fail. The
//! key is then stored in `<config_dir>/secrets` with mode 0600 and the app
//! warns that this is less protected than the system keyring
//! ([`secure_store_available`]).

use anyhow::{bail, Context, Result};
use keyring::Entry;

const SERVICE: &str = "nd104-studio";

fn entry(provider: &str) -> Result<Entry> {
    Entry::new(SERVICE, provider).context("credential store not available")
}

/// Stored key for `provider`, if any.
pub fn get(provider: &str) -> Result<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(Some(key)),
        // not in the keyring: a fallback copy may still exist (Linux)
        Err(keyring::Error::NoEntry) => Ok(fallback_get(provider)),
        Err(e) => {
            #[cfg(target_os = "linux")]
            if let Some(key) = fallback::get(provider) {
                tracing::warn!("system keyring unavailable ({e}); read the API key from the 0600 fallback file");
                return Ok(Some(key));
            }
            Err(e).context("cannot read API key from the credential store")
        }
    }
}

pub fn has(provider: &str) -> bool {
    matches!(get(provider), Ok(Some(_)))
}

/// Stores (or replaces) the key for `provider`.
pub fn set(provider: &str, key: &str) -> Result<()> {
    let key = key.trim();
    if key.is_empty() {
        bail!("API key is empty");
    }
    if key.chars().any(char::is_whitespace) {
        bail!("API key must not contain spaces");
    }
    match entry(provider)?.set_password(key) {
        Ok(()) => {
            // keyring took it: drop any stale fallback copy so the two agree
            let _ = fallback_delete(provider);
            Ok(())
        }
        Err(e) => store_failed(provider, key, e),
    }
}

/// Keyring store failed: on Linux fall back to the 0600 file; elsewhere the
/// error stands.
#[cfg(target_os = "linux")]
fn store_failed(provider: &str, key: &str, e: keyring::Error) -> Result<()> {
    tracing::warn!(
        "system keyring unavailable ({e}); storing the API key in the 0600 fallback file"
    );
    fallback::set(provider, key)
}

#[cfg(not(target_os = "linux"))]
fn store_failed(_provider: &str, _key: &str, e: keyring::Error) -> Result<()> {
    Err(e).context("cannot store API key in the credential store")
}

/// Copies the keys stored under the app's former service name `old` and
/// deletes the old entries once the copy is in place. A key that already
/// exists under the current name is kept. Returns how many keys moved.
pub fn take_over(old: &str, providers: &[&str]) -> Result<usize> {
    let mut moved = 0;
    for provider in providers {
        let Ok(old_entry) = Entry::new(old, provider) else {
            continue;
        };
        let key = match old_entry.get_password() {
            Ok(key) => key,
            Err(keyring::Error::NoEntry) => continue,
            Err(e) => return Err(e).context("cannot read the old API key"),
        };
        if get(provider)?.is_none() {
            entry(provider)?
                .set_password(&key)
                .context("cannot store API key in the credential store")?;
            moved += 1;
        }
        // only once the key is safe under the new name
        if get(provider)?.is_some() {
            let _ = old_entry.delete_credential();
        }
    }
    Ok(moved)
}

/// Removes the key for `provider`; a missing key is not an error.
pub fn delete(provider: &str) -> Result<()> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(e) => {
            #[cfg(not(target_os = "linux"))]
            return Err(e).context("cannot delete API key from the credential store");
            #[cfg(target_os = "linux")]
            tracing::warn!("system keyring unavailable ({e}); deleting only the fallback file");
        }
    }
    let _ = fallback_delete(provider);
    Ok(())
}

/// Whether the OS credential store is usable. On Linux, `false` means the
/// Secret Service is not reachable, so keys go to the 0600 fallback file and
/// the UI should warn. Always `true` on Windows.
pub fn secure_store_available() -> bool {
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
    #[cfg(target_os = "linux")]
    {
        // a sentinel read: NoEntry means the store works and is just empty
        match Entry::new(SERVICE, "__probe__").and_then(|e| e.get_password()) {
            Ok(_) | Err(keyring::Error::NoEntry) => true,
            Err(_) => false,
        }
    }
}

#[cfg(target_os = "linux")]
fn fallback_get(provider: &str) -> Option<String> {
    fallback::get(provider)
}

#[cfg(not(target_os = "linux"))]
fn fallback_get(_provider: &str) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn fallback_delete(provider: &str) -> Result<()> {
    fallback::delete(provider)
}

#[cfg(not(target_os = "linux"))]
fn fallback_delete(_provider: &str) -> Result<()> {
    Ok(())
}

/// Plain-file store used only when the Secret Service is unavailable. One
/// `provider key` line per entry in `<config_dir>/secrets`, mode 0600. Keys
/// never contain whitespace (checked in [`set`]), so the format is unambiguous.
#[cfg(target_os = "linux")]
mod fallback {
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::path::PathBuf;

    use anyhow::{Context, Result};

    fn path() -> Result<PathBuf> {
        Ok(crate::config::config_dir()?.join("secrets"))
    }

    fn read_all() -> BTreeMap<String, String> {
        let Ok(p) = path() else {
            return BTreeMap::new();
        };
        let Ok(text) = std::fs::read_to_string(p) else {
            return BTreeMap::new();
        };
        text.lines()
            .filter_map(|l| l.split_once(' '))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn write_all(map: &BTreeMap<String, String>) -> Result<()> {
        let p = path()?;
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let mut body = String::new();
        for (k, v) in map {
            body.push_str(k);
            body.push(' ');
            body.push_str(v);
            body.push('\n');
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&p)
            .with_context(|| format!("cannot write {}", p.display()))?;
        f.write_all(body.as_bytes())?;
        // enforce 0600 even if the file already existed with other permissions
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        Ok(())
    }

    pub fn get(provider: &str) -> Option<String> {
        read_all().get(provider).cloned()
    }

    pub fn set(provider: &str, key: &str) -> Result<()> {
        let mut map = read_all();
        map.insert(provider.to_string(), key.to_string());
        write_all(&map)
    }

    pub fn delete(provider: &str) -> Result<()> {
        let mut map = read_all();
        if map.remove(provider).is_some() {
            write_all(&map)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_or_spaced_keys() {
        assert!(set("test-provider", "   ").is_err());
        assert!(set("test-provider", "abc def").is_err());
    }
}
