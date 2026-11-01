//! "Install update": downloads the setup of the newest release, checks it
//! against the release's SHA256SUMS.txt and runs it in update mode
//! (`--update`: progress window only, restarts the app afterwards).
//!
//! Offered only when this copy was installed by nd104-sync-setup (1.1.0+);
//! MSI and portable installs keep the "open release page" link.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use nd104_core::app_update::{self, Release};
use sha2::{Digest, Sha256};

/// `%LOCALAPPDATA%\Programs\nd104-sync`, where the setup installs.
fn setup_install_dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("Programs").join("nd104-sync"))
}

/// This executable belongs to an installation made by the setup.
pub fn supported() -> bool {
    let (Some(dir), Ok(exe)) = (setup_install_dir(), std::env::current_exe()) else {
        return false;
    };
    let same = |a: &std::path::Path, b: &std::path::Path| {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    };
    exe.parent().is_some_and(|p| same(p, &dir)) && dir.join("nd104-sync-setup.exe").exists()
}

/// Downloads and verifies the setup, then starts it. The caller quits the app.
pub async fn download_and_start(release: &Release) -> Result<()> {
    let setup_url = release
        .setup_url
        .as_deref()
        .ok_or_else(|| anyhow!("release {} has no setup", release.version))?;
    let sums_url = release
        .checksums_url
        .as_deref()
        .ok_or_else(|| anyhow!("release {} has no SHA256SUMS.txt", release.version))?;
    let client = reqwest::Client::builder()
        .user_agent(nd104_core::user_agent())
        .timeout(Duration::from_secs(300))
        .build()?;
    let get = |url: &str| {
        let request = client.get(url);
        async move {
            let response = request.send().await?.error_for_status()?;
            anyhow::Ok(response.bytes().await?)
        }
    };

    let name = app_update::setup_name(&release.version);
    let sums = String::from_utf8_lossy(
        &get(sums_url)
            .await
            .context("cannot download SHA256SUMS.txt")?,
    )
    .into_owned();
    let expected = expected_hash(&sums, &name)
        .ok_or_else(|| anyhow!("{name} is not listed in SHA256SUMS.txt"))?;
    let setup = get(setup_url).await.context("cannot download the setup")?;
    let actual = hex(&Sha256::digest(&setup));
    if !actual.eq_ignore_ascii_case(&expected) {
        bail!("checksum of {name} does not match – download discarded");
    }

    let dir = std::env::temp_dir().join("nd104-sync-update");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&name);
    std::fs::write(&path, &setup).with_context(|| format!("cannot write {}", path.display()))?;
    std::process::Command::new(&path)
        .arg("--update")
        .spawn()
        .context("cannot start the setup")?;
    tracing::info!("update to {} started", release.version);
    Ok(())
}

/// Hash for `file` from `sha256sum` output (`<hash>  <name>` or `<hash> *<name>`).
fn expected_hash(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.trim().split_once(char::is_whitespace)?;
        let name = name.trim_start().trim_start_matches('*');
        (name == file && hash.len() == 64).then(|| hash.to_string())
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_hash_in_sha256sums() {
        let h = "a".repeat(64);
        let sums = format!(
            "{h}  nd104-sync_1.1.0_x64-setup.exe\n{}  other.zip\n",
            "b".repeat(64)
        );
        assert_eq!(
            expected_hash(&sums, "nd104-sync_1.1.0_x64-setup.exe"),
            Some(h.clone())
        );
        assert_eq!(expected_hash(&format!("{h} *x.exe"), "x.exe"), Some(h));
        assert_eq!(expected_hash(&sums, "missing.exe"), None);
    }

    #[test]
    fn hex_encodes() {
        assert_eq!(hex(&[0x00, 0xab, 0x10]), "00ab10");
    }
}
