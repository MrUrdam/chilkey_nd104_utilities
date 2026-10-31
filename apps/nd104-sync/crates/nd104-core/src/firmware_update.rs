//! Firmware update check (PLAN.md § 5.4).
//!
//! Compares the build date reported by the keyboard with the reviewed version
//! list `firmware/versions.json` in this repository. nd104-sync never
//! downloads or installs firmware – it only reports and links to Chilkey.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use crate::model::{FirmwareInfo, KeyboardModel, Role, ScreenVariant};

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/MrUrdam/chilkey_nd104_utilities/main/firmware/versions.json";
const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub updated: Date,
    pub models: HashMap<String, ManifestModel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestModel {
    pub name: String,
    pub instructions_url: String,
    pub parts: HashMap<String, ManifestPart>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestPart {
    pub label: String,
    pub latest: Latest,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Latest {
    pub version: Option<String>,
    pub build_date: Date,
    pub file: String,
    #[serde(default)]
    pub ui_file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartState {
    UpToDate,
    UpdateAvailable,
    /// Installed build is newer than the list (list not updated yet).
    Newer,
    /// Build date or display variant not recognisable, or part not listed.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartReport {
    pub role: Role,
    pub label: Option<String>,
    pub installed: String,
    pub installed_build: Option<Date>,
    pub latest: Option<Latest>,
    pub download_url: Option<String>,
    pub state: PartState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FirmwareReport {
    pub model_name: String,
    pub manifest_updated: Date,
    pub instructions_url: Option<String>,
    pub parts: Vec<PartReport>,
}

impl FirmwareReport {
    pub fn update_available(&self) -> bool {
        self.parts
            .iter()
            .any(|p| p.state == PartState::UpdateAvailable)
    }
}

pub fn parse_manifest(text: &str) -> Result<Manifest> {
    // Files edited on Windows often start with a byte order mark.
    let text = text.trim_start_matches('\u{feff}');
    let manifest: Manifest = serde_json::from_str(text).context("invalid firmware version list")?;
    if manifest.schema > SCHEMA {
        bail!(
            "firmware version list uses schema {} – please update nd104-sync",
            manifest.schema
        );
    }
    Ok(manifest)
}

/// Overrides the list source (file path or URL), e.g. for testing a new list
/// before it is pushed.
pub const MANIFEST_ENV: &str = "ND104_FIRMWARE_MANIFEST";

/// Loads the version list from the repository (or from [`MANIFEST_ENV`]).
pub async fn fetch_manifest() -> Result<Manifest> {
    let source = std::env::var(MANIFEST_ENV).unwrap_or_else(|_| MANIFEST_URL.to_string());
    if !source.starts_with("https://") {
        let text = std::fs::read_to_string(&source)
            .with_context(|| format!("cannot read firmware version list {source}"))?;
        return parse_manifest(&text);
    }
    let body = reqwest::Client::builder()
        .user_agent(crate::user_agent())
        .timeout(std::time::Duration::from_secs(15))
        .build()?
        .get(&source)
        .send()
        .await
        .context("cannot load the firmware version list")?
        .error_for_status()
        .context("cannot load the firmware version list")?
        .text()
        .await?;
    parse_manifest(&body)
}

/// Compares the installed firmware with the list.
pub fn evaluate(
    manifest: &Manifest,
    model: &dyn KeyboardModel,
    firmware: &[FirmwareInfo],
) -> FirmwareReport {
    let entry = manifest.models.get(model.manifest_key());
    let variant: ScreenVariant = model.screen_variant(firmware);
    let parts = firmware
        .iter()
        .map(|f| {
            let installed_build = model.firmware_build_date(f);
            let part = model
                .manifest_part(f.role, variant)
                .and_then(|key| entry.and_then(|e| e.parts.get(key)));
            let state = match (installed_build, part) {
                (Some(installed), Some(p)) if installed < p.latest.build_date => {
                    PartState::UpdateAvailable
                }
                (Some(installed), Some(p)) if installed > p.latest.build_date => PartState::Newer,
                (Some(_), Some(_)) => PartState::UpToDate,
                _ => PartState::Unknown,
            };
            PartReport {
                role: f.role,
                label: part.map(|p| p.label.clone()),
                installed: f.raw.clone(),
                installed_build,
                latest: part.map(|p| p.latest.clone()),
                download_url: part.map(|p| p.download_url.clone()),
                state,
            }
        })
        .collect();
    FirmwareReport {
        model_name: entry.map_or_else(|| model.display_name().to_string(), |e| e.name.clone()),
        manifest_updated: manifest.updated,
        instructions_url: entry.map(|e| e.instructions_url.clone()),
        parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Nd104;

    /// The real list from this repository.
    const REPO_MANIFEST: &str = include_str!("../../../../../firmware/versions.json");

    fn fw(role: Role, raw: &str) -> FirmwareInfo {
        let (version, build) = raw.split_once(' ').unwrap();
        FirmwareInfo {
            role,
            raw: raw.into(),
            version: version.trim_start_matches('V').into(),
            build: build.into(),
        }
    }

    fn verified_device() -> Vec<FirmwareInfo> {
        vec![
            fw(Role::Keyboard, "V3.0.14 20260710_123331"),
            fw(Role::Screen, "V1.2.4 Mar 17 2026 21:37:00"),
        ]
    }

    #[test]
    fn repository_manifest_parses() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        let nd104 = &m.models["nd104"];
        assert_eq!(nd104.parts.len(), 3);
        for key in ["keyboard", "screen_24h_c", "screen_12h_f"] {
            assert!(nd104.parts.contains_key(key), "{key}");
        }
    }

    #[test]
    fn verified_device_is_up_to_date_with_repository_manifest() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        let r = evaluate(&m, &Nd104, &verified_device());
        assert!(!r.update_available());
        assert!(
            r.parts.iter().all(|p| p.state == PartState::UpToDate),
            "{r:?}"
        );
        assert_eq!(r.parts[1].label.as_deref(), Some("Display 24 h / °C"));
    }

    #[test]
    fn older_keyboard_build_reports_update() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        let mut fw = verified_device();
        fw[0] = self::fw(Role::Keyboard, "V3.0.4 20260201_101010");
        let r = evaluate(&m, &Nd104, &fw);
        assert!(r.update_available());
        assert_eq!(r.parts[0].state, PartState::UpdateAvailable);
        assert_eq!(r.parts[1].state, PartState::UpToDate);
    }

    #[test]
    fn newer_build_and_unknown_variant() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        let fw = vec![
            fw(Role::Keyboard, "V3.0.20 20261201_000000"),
            // unknown screen build → variant unknown → cannot compare
            fw(Role::Screen, "V1.3.0 Dec 24 2026 10:00:00"),
        ];
        let r = evaluate(&m, &Nd104, &fw);
        assert_eq!(r.parts[0].state, PartState::Newer);
        assert_eq!(r.parts[1].state, PartState::Unknown);
        assert!(!r.update_available());
    }

    #[test]
    fn newer_schema_is_rejected() {
        let text = REPO_MANIFEST.replacen("\"schema\": 1", "\"schema\": 99", 1);
        assert!(parse_manifest(&text).is_err());
    }
}
