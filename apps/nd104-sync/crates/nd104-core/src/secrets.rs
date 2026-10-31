//! API keys in the operating system's credential store (PLAN.md § 5.8).
//!
//! Windows: Credential Manager, entry `<provider>.nd104-sync`. Keys are never
//! written to the config file or the log.

use anyhow::{bail, Context, Result};
use keyring::Entry;

const SERVICE: &str = "nd104-sync";

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
