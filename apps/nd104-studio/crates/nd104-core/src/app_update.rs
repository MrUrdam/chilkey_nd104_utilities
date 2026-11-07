//! Checks the GitHub repository for a newer nd104-studio release.
//!
//! The repository hosts several tools, so nd104-studio releases use tags of the
//! form `nd104-studio-v<major>.<minor>.<patch>[-<pre-release>]`. Drafts are
//! ignored. Pre-releases (`-beta.N`, `-rc.N`) form a beta channel: a stable
//! install never sees them, an install of a pre-release also sees newer
//! pre-releases (and every newer stable release). Versions compare by SemVer
//! precedence. Nothing is downloaded or installed automatically; the app offers
//! "Install update" when the release carries the setup (since 1.1.0) or, on
//! Linux, the AppImage.

use std::cmp::Ordering;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const RELEASES_API: &str =
    "https://api.github.com/repos/MrUrdam/chilkey_nd104_utilities/releases?per_page=50";
pub const RELEASES_PAGE: &str = "https://github.com/MrUrdam/chilkey_nd104_utilities/releases";
pub const TAG_PREFIX: &str = "nd104-studio-v";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub url: String,
    pub published_at: Option<String>,
    /// Download URL of `nd104-studio_<version>_x64-setup.exe`, if attached.
    pub setup_url: Option<String>,
    /// Download URL of `nd104-studio_<version>_amd64.AppImage`, if attached.
    pub appimage_url: Option<String>,
    /// Download URL of `SHA256SUMS.txt`, if attached.
    pub checksums_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateStatus {
    UpToDate { latest: Option<Release> },
    Available(Release),
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// File name of the Windows setup attached to the release of `version`.
pub fn setup_name(version: &str) -> String {
    format!("nd104-studio_{version}_x64-setup.exe")
}

/// File name of the Linux AppImage attached to the release of `version`.
pub fn appimage_name(version: &str) -> String {
    format!("nd104-studio_{version}_amd64.AppImage")
}

/// Asks GitHub for the newest release and compares it with `current`.
pub async fn check(current: &str) -> Result<UpdateStatus> {
    let body = reqwest::Client::builder()
        .user_agent(crate::user_agent())
        .timeout(std::time::Duration::from_secs(15))
        .build()?
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("GitHub request failed")?
        .error_for_status()
        .context("GitHub returned an error")?
        .text()
        .await?;
    evaluate(current, &body)
}

fn evaluate(current: &str, body: &str) -> Result<UpdateStatus> {
    let releases: Vec<GhRelease> =
        serde_json::from_str(body).context("unexpected GitHub answer")?;
    // beta channel: only an install of a pre-release looks at pre-releases
    let beta_channel = parse_version(current).is_some_and(|v| v.is_pre_release());
    let latest = releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let version = r.tag_name.strip_prefix(TAG_PREFIX)?.to_string();
            let parsed = parse_version(&version)?;
            // a stable install skips anything flagged or named as a pre-release
            if !beta_channel && (r.prerelease || parsed.is_pre_release()) {
                return None;
            }
            let asset = |name: &str| {
                r.assets
                    .iter()
                    .find(|a| a.name == name)
                    .map(|a| a.browser_download_url.clone())
            };
            let setup_url = asset(&setup_name(&version));
            let appimage_url = asset(&appimage_name(&version));
            let checksums_url = asset("SHA256SUMS.txt");
            Some(Release {
                setup_url,
                appimage_url,
                checksums_url,
                version,
                tag: r.tag_name,
                url: r.html_url,
                published_at: r.published_at,
            })
        })
        .max_by(|a, b| compare(&a.version, &b.version));

    Ok(match latest {
        Some(r) if compare(&r.version, current) == Ordering::Greater => UpdateStatus::Available(r),
        latest => UpdateStatus::UpToDate { latest },
    })
}

/// `major.minor.patch` with an optional SemVer pre-release (`-beta.2`,
/// `-rc.1`); build metadata (`+...`) is ignored.
#[derive(Debug, PartialEq, Eq)]
struct Version {
    release: [u64; 3],
    pre: Vec<PreId>,
}

/// One dot-separated pre-release identifier. Declaration order gives SemVer's
/// rule that a numeric identifier sorts before an alphanumeric one.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PreId {
    Num(u64),
    Alpha(String),
}

impl Version {
    fn is_pre_release(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // a pre-release sorts before its release; identifiers compare in order,
        // and with an equal prefix the longer list is the newer one
        self.release.cmp(&other.release).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_version(v: &str) -> Option<Version> {
    let v = v.split('+').next()?;
    let (core, pre) = match v.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (v, None),
    };
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let release = [parts.next()??, parts.next()??, parts.next()??];
    if parts.next().is_some() {
        return None;
    }
    let pre = match pre {
        None => Vec::new(),
        Some(pre) => pre
            .split('.')
            .map(|id| match id {
                "" => None,
                id => Some(
                    id.parse::<u64>()
                        .map(PreId::Num)
                        .unwrap_or_else(|_| PreId::Alpha(id.to_string())),
                ),
            })
            .collect::<Option<Vec<_>>>()?,
    };
    Some(Version { release, pre })
}

/// Compares versions by SemVer precedence (unparsable ones sort first).
pub fn compare(a: &str, b: &str) -> Ordering {
    parse_version(a).cmp(&parse_version(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"[
        {"tag_name": "nd104-studio-v0.3.0-rc1", "html_url": "u0", "prerelease": true},
        {"tag_name": "nd104-studio-v0.2.0", "html_url": "u1", "published_at": "2026-11-20T10:00:00Z",
         "assets": [
            {"name": "nd104-studio_0.2.0_x64-setup.exe", "browser_download_url": "s1"},
            {"name": "nd104-studio_0.2.0_amd64.AppImage", "browser_download_url": "a1"},
            {"name": "SHA256SUMS.txt", "browser_download_url": "c1"}
         ]},
        {"tag_name": "nd104-studio-v0.10.0", "html_url": "u2", "draft": true},
        {"tag_name": "other-tool-v9.0.0", "html_url": "u3"},
        {"tag_name": "nd104-studio-v0.1.1", "html_url": "u4"}
    ]"#;

    #[test]
    fn newer_release_available() {
        match evaluate("0.1.0", BODY).unwrap() {
            UpdateStatus::Available(r) => {
                assert_eq!(r.version, "0.2.0");
                assert_eq!(r.url, "u1");
                assert_eq!(r.setup_url.as_deref(), Some("s1"));
                assert_eq!(r.appimage_url.as_deref(), Some("a1"));
                assert_eq!(r.checksums_url.as_deref(), Some("c1"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn up_to_date() {
        assert!(matches!(
            evaluate("0.2.0", BODY).unwrap(),
            UpdateStatus::UpToDate { latest: Some(r) } if r.version == "0.2.0"
        ));
        match evaluate(
            "0.0.1",
            r#"[{"tag_name": "nd104-studio-v0.1.0", "html_url": "u"}]"#,
        )
        .unwrap()
        {
            UpdateStatus::Available(r) => {
                assert!(
                    r.setup_url.is_none() && r.appimage_url.is_none() && r.checksums_url.is_none()
                )
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            evaluate("0.1.0", "[]").unwrap(),
            UpdateStatus::UpToDate { latest: None }
        );
    }

    #[test]
    fn numeric_comparison() {
        assert_eq!(compare("0.10.0", "0.9.9"), Ordering::Greater);
        assert_eq!(compare("1.0.0", "1.0.0"), Ordering::Equal);
        assert!(parse_version("1.0").is_none());
        assert!(parse_version("1.0.0-").is_none());
        assert!(parse_version("1.0.0-beta..1").is_none());
        assert!(parse_version("1.0.0-rc1").is_some());
    }

    #[test]
    fn pre_release_precedence() {
        use Ordering::*;
        assert_eq!(compare("2.4.0-beta.1", "2.4.0-beta.2"), Less);
        assert_eq!(compare("2.4.0-beta.2", "2.4.0-beta.10"), Less);
        assert_eq!(compare("2.4.0-beta.9", "2.4.0-rc.1"), Less);
        assert_eq!(compare("2.4.0-rc.1", "2.4.0"), Less);
        assert_eq!(compare("2.4.0-beta", "2.4.0-beta.1"), Less);
        assert_eq!(compare("2.4.0-1", "2.4.0-alpha"), Less);
        assert_eq!(compare("2.4.0-beta.2+build.7", "2.4.0-beta.2"), Equal);
        // the Windows setup compares itself with the installed version: a beta
        // setup must count as newer than the release before it
        assert_eq!(compare("2.4.0-beta.2", "2.3.1"), Greater);
        assert_eq!(compare("2.3.1", "2.4.0-beta.1"), Less);
    }

    const CHANNEL: &str = r#"[
        {"tag_name": "nd104-studio-v2.4.0-beta.3", "html_url": "b3", "prerelease": true},
        {"tag_name": "nd104-studio-v2.4.0-beta.2", "html_url": "b2", "prerelease": true},
        {"tag_name": "nd104-studio-v2.3.1", "html_url": "s1"}
    ]"#;

    #[test]
    fn stable_install_never_sees_pre_releases() {
        assert!(matches!(
            evaluate("2.3.1", CHANNEL).unwrap(),
            UpdateStatus::UpToDate { latest: Some(r) } if r.version == "2.3.1"
        ));
        // a suffix without GitHub's pre-release flag still counts as a pre-release
        let unflagged = r#"[{"tag_name": "nd104-studio-v2.4.0-rc.1", "html_url": "r"}]"#;
        assert!(matches!(
            evaluate("2.3.1", unflagged).unwrap(),
            UpdateStatus::UpToDate { latest: None }
        ));
    }

    #[test]
    fn beta_install_follows_the_beta_channel() {
        match evaluate("2.4.0-beta.2", CHANNEL).unwrap() {
            UpdateStatus::Available(r) => {
                assert_eq!((r.version.as_str(), r.url.as_str()), ("2.4.0-beta.3", "b3"))
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            evaluate("2.4.0-beta.3", CHANNEL).unwrap(),
            UpdateStatus::UpToDate { latest: Some(r) } if r.version == "2.4.0-beta.3"
        ));
        // ... and moves on to the stable release once it is out
        let released = r#"[
            {"tag_name": "nd104-studio-v2.4.0", "html_url": "s"},
            {"tag_name": "nd104-studio-v2.4.0-beta.3", "html_url": "b3", "prerelease": true}
        ]"#;
        match evaluate("2.4.0-beta.3", released).unwrap() {
            UpdateStatus::Available(r) => assert_eq!(r.version, "2.4.0"),
            other => panic!("{other:?}"),
        }
    }
}
