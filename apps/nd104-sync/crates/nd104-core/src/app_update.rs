//! Checks the GitHub repository for a newer nd104-sync release.
//!
//! The repository hosts several tools, so nd104-sync releases use tags of the
//! form `nd104-sync-v<major>.<minor>.<patch>`. Drafts and pre-releases are
//! ignored. Nothing is downloaded or installed automatically.

use std::cmp::Ordering;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const RELEASES_API: &str =
    "https://api.github.com/repos/MrUrdam/chilkey_nd104_utilities/releases?per_page=50";
pub const RELEASES_PAGE: &str = "https://github.com/MrUrdam/chilkey_nd104_utilities/releases";
pub const TAG_PREFIX: &str = "nd104-sync-v";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub url: String,
    pub published_at: Option<String>,
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
    let latest = releases
        .into_iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| {
            let version = r.tag_name.strip_prefix(TAG_PREFIX)?.to_string();
            parse_version(&version)?;
            Some(Release {
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

fn parse_version(v: &str) -> Option<[u64; 3]> {
    let mut parts = v.split('.').map(|p| p.parse::<u64>().ok());
    let version = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(version)
}

fn compare(a: &str, b: &str) -> Ordering {
    parse_version(a).cmp(&parse_version(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"[
        {"tag_name": "nd104-sync-v0.3.0-rc1", "html_url": "u0", "prerelease": true},
        {"tag_name": "nd104-sync-v0.2.0", "html_url": "u1", "published_at": "2026-11-20T10:00:00Z"},
        {"tag_name": "nd104-sync-v0.10.0", "html_url": "u2", "draft": true},
        {"tag_name": "other-tool-v9.0.0", "html_url": "u3"},
        {"tag_name": "nd104-sync-v0.1.1", "html_url": "u4"}
    ]"#;

    #[test]
    fn newer_release_available() {
        match evaluate("0.1.0", BODY).unwrap() {
            UpdateStatus::Available(r) => {
                assert_eq!(r.version, "0.2.0");
                assert_eq!(r.url, "u1");
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
        assert!(parse_version("1.0.0-rc1").is_none());
    }
}
