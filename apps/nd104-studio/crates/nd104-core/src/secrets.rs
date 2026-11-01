//! API keys in the operating system's credential store.
//!
//! Windows: Credential Manager, entry `<provider>.nd104-studio`. Keys are never
//! written to the config file or the log.

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
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("cannot read API key from the credential store"),
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
    entry(provider)?
        .set_password(key)
        .context("cannot store API key in the credential store")
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
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("cannot delete API key from the credential store"),
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
