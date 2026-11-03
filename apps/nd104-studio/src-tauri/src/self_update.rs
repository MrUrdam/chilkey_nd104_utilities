//! "Install update": downloads the newest release's artifact, checks it against
//! the release's SHA256SUMS.txt and swaps it in.
//!
//! - **Windows setup install** (1.1.0+): runs the new setup in update mode
//!   (`--update`: progress window only, restarts the app afterwards).
//! - **Linux AppImage** (`$APPIMAGE` set): replaces the AppImage file in place
//!   and restarts.
//!
//! Other install types (MSI, portable, `.deb`/`.rpm`) do not update themselves;
//! the app keeps the "open release page" link for them.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use nd104_core::app_update::{self, Release};
use sha2::{Digest, Sha256};

/// `%LOCALAPPDATA%\Programs\nd104-studio`, where the setup installs.
#[cfg(windows)]
fn setup_install_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(
        std::path::PathBuf::from(base)
            .join("Programs")
            .join("nd104-studio"),
    )
}

/// Path of the running AppImage (`$APPIMAGE`), if this copy was started as one.
#[cfg(target_os = "linux")]
fn appimage_path() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("APPIMAGE")?;
    (!path.is_empty()).then(|| std::path::PathBuf::from(path))
}

/// This copy can replace itself (Windows setup install or Linux AppImage).
#[cfg(windows)]
pub fn supported() -> bool {
    let (Some(dir), Ok(exe)) = (setup_install_dir(), std::env::current_exe()) else {
        return false;
    };
    let same = |a: &std::path::Path, b: &std::path::Path| {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    };
    exe.parent().is_some_and(|p| same(p, &dir)) && dir.join("nd104-studio-setup.exe").exists()
}

#[cfg(target_os = "linux")]
pub fn supported() -> bool {
    appimage_path().is_some()
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn supported() -> bool {
    false
}

/// Downloads `url`, checks its SHA-256 against the `name` entry in the release's
/// SHA256SUMS.txt (from `sums_url`), and returns the verified bytes.
async fn fetch_verified(url: &str, sums_url: &str, name: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent(nd104_core::user_agent())
        .timeout(Duration::from_secs(300))
        .build()?;
    let get = |url: String| {
        let request = client.get(url);
        async move {
            let response = request.send().await?.error_for_status()?;
            anyhow::Ok(response.bytes().await?)
        }
    };

    let sums = String::from_utf8_lossy(
        &get(sums_url.to_string())
            .await
            .context("cannot download SHA256SUMS.txt")?,
    )
    .into_owned();
    let expected = expected_hash(&sums, name)
        .ok_or_else(|| anyhow!("{name} is not listed in SHA256SUMS.txt"))?;
    let bytes = get(url.to_string())
        .await
        .context("cannot download the update")?;
    let actual = hex(&Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(&expected) {
        bail!("checksum of {name} does not match – download discarded");
    }
    Ok(bytes.to_vec())
}

/// Downloads and verifies the new artifact, then starts the swap. The caller
/// quits the app.
#[cfg(windows)]
pub async fn download_and_start(release: &Release) -> Result<()> {
    let setup_url = release
        .setup_url
        .as_deref()
        .ok_or_else(|| anyhow!("release {} has no setup", release.version))?;
    let sums_url = release
        .checksums_url
        .as_deref()
        .ok_or_else(|| anyhow!("release {} has no SHA256SUMS.txt", release.version))?;
    let name = app_update::setup_name(&release.version);
    let setup = fetch_verified(setup_url, sums_url, &name).await?;

    let dir = std::env::temp_dir().join("nd104-studio-update");
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

#[cfg(target_os = "linux")]
pub async fn download_and_start(release: &Release) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let current = appimage_path().ok_or_else(|| anyhow!("not running as an AppImage"))?;
    let url = release
        .appimage_url
        .as_deref()
        .ok_or_else(|| anyhow!("release {} has no AppImage", release.version))?;
    let sums_url = release
        .checksums_url
        .as_deref()
        .ok_or_else(|| anyhow!("release {} has no SHA256SUMS.txt", release.version))?;
    let name = app_update::appimage_name(&release.version);
    let bytes = fetch_verified(url, sums_url, &name).await?;

    // Write next to the current AppImage, then rename over it (atomic on the
    // same filesystem). The running process keeps the old inode until it exits,
    // so the restart below launches the replacement.
    let dir = current
        .parent()
        .ok_or_else(|| anyhow!("the AppImage has no parent directory"))?;
    let file_name = current
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("nd104-studio.AppImage");
    let tmp = dir.join(format!(".{file_name}.new"));
    std::fs::write(&tmp, &bytes).with_context(|| format!("cannot write {}", tmp.display()))?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
        .context("cannot make the new AppImage executable")?;
    std::fs::rename(&tmp, &current)
        .with_context(|| format!("cannot replace {}", current.display()))?;

    std::process::Command::new(&current)
        .spawn()
        .context("cannot restart the app")?;
    tracing::info!("update to {} started", release.version);
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
pub async fn download_and_start(_release: &Release) -> Result<()> {
    bail!("this installation cannot update itself")
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
            "{h}  nd104-studio_1.1.0_x64-setup.exe\n{}  other.zip\n",
            "b".repeat(64)
        );
        assert_eq!(
            expected_hash(&sums, "nd104-studio_1.1.0_x64-setup.exe"),
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
