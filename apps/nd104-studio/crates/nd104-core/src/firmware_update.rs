//! Firmware update check.
//!
//! 1. **Official source first:** the vendor's public Google Drive folder per
//!    part ([`OfficialSource`]); the newest file is the one with the latest
//!    `_YYYYMMDD.bin` date in its name.
//! 2. **Fallback:** the reviewed version list `firmware/versions.json` in this
//!    repository, for every part whose official folder cannot be read.
//!
//! The build date reported by the keyboard is compared with that date.
//! nd104-studio never downloads or installs firmware – it only reports and links.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use crate::model::{FirmwareInfo, KeyboardModel, OfficialSource, Role, ScreenVariant};

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/MrUrdam/chilkey_nd104_utilities/main/firmware/versions.json";
/// Overrides the list source (file path or URL), e.g. for testing a new list
/// before it is pushed.
pub const MANIFEST_ENV: &str = "ND104_FIRMWARE_MANIFEST";
const SCHEMA: u32 = 1;
const DRIVE_FOLDER: &str = "https://drive.google.com/drive/folders/";

// ---------------------------------------------------------------------------
// Version list (fallback)

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

pub fn parse_manifest(text: &str) -> Result<Manifest> {
    // Files edited on Windows often start with a byte order mark.
    let text = text.trim_start_matches('\u{feff}');
    let manifest: Manifest = serde_json::from_str(text).context("invalid firmware version list")?;
    if manifest.schema > SCHEMA {
        bail!(
            "firmware version list uses schema {} – please update ND104 Studio",
            manifest.schema
        );
    }
    Ok(manifest)
}

/// Loads the version list from the repository (or from [`MANIFEST_ENV`]).
pub async fn fetch_manifest() -> Result<Manifest> {
    let source = std::env::var(MANIFEST_ENV).unwrap_or_else(|_| MANIFEST_URL.to_string());
    if !source.starts_with("https://") {
        let text = std::fs::read_to_string(&source)
            .with_context(|| format!("cannot read firmware version list {source}"))?;
        return parse_manifest(&text);
    }
    let body = client()?
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

// ---------------------------------------------------------------------------
// Official source (vendor's Google Drive folders)

pub fn drive_folder_url(id: &str) -> String {
    format!("{DRIVE_FOLDER}{id}")
}

/// `.bin` file names in a public Drive folder page. They appear JSON-escaped
/// in the page data: `\x22NAME.bin\x22`.
pub fn drive_files(html: &str) -> Vec<String> {
    let mut files: Vec<String> = html
        .split("\\x22")
        .filter(|s| {
            s.ends_with(".bin")
                && s.len() <= 200
                && s.trim() == *s
                && !s.contains(['<', '>', '"', '\\', '/', '[', ']', '{', '}', ',', ';'])
                && !s.starts_with('.')
        })
        .map(str::to_string)
        .collect();
    files.sort();
    files.dedup();
    files
}

/// `…_YYYYMMDD.bin` → date.
fn file_date(name: &str) -> Option<Date> {
    let stem = name.strip_suffix(".bin")?;
    let digits = stem.rsplit('_').next()?;
    if digits.len() != 8 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Date::new(
        digits[..4].parse().ok()?,
        digits[4..6].parse().ok()?,
        digits[6..].parse().ok()?,
    )
    .ok()
}

/// Newest matching firmware file of one official source.
pub fn latest_from_files(source: &OfficialSource, files: &[String]) -> Option<Latest> {
    files
        .iter()
        .filter(|f| source.include.iter().all(|s| f.contains(s)))
        .filter(|f| !source.exclude.iter().any(|s| f.contains(s)))
        .filter_map(|f| Some((file_date(f)?, f)))
        .max_by_key(|(date, _)| *date)
        .map(|(build_date, file)| Latest {
            version: None,
            build_date,
            file: file.clone(),
            ui_file: None,
        })
}

async fn fetch_official(source: &OfficialSource) -> Result<Latest> {
    let url = drive_folder_url(source.drive_folder_id);
    let html = client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| e.without_url())
        .context("cannot reach Google Drive")?
        .error_for_status()
        .map_err(|e| e.without_url())
        .context("Google Drive folder not available")?
        .text()
        .await?;
    let files = drive_files(&html);
    if files.is_empty() {
        bail!("no firmware files found in the folder (page layout changed?)");
    }
    latest_from_files(source, &files).ok_or_else(|| anyhow!("no matching firmware file found"))
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        // Drive serves the folder listing to browsers; identify honestly anyway.
        .user_agent(format!("Mozilla/5.0 (compatible; {})", crate::user_agent()))
        .timeout(Duration::from_secs(20))
        .build()?)
}

// ---------------------------------------------------------------------------
// Report

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Vendor's own download folder.
    Official,
    /// Reviewed list in this repository (fallback).
    Repository,
}

/// Newest version of one part and where that information came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestInfo {
    pub label: String,
    pub latest: Latest,
    pub download_url: String,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartState {
    UpToDate,
    UpdateAvailable,
    /// Installed build is newer than the newest known file.
    Newer,
    /// Build date or display variant not recognisable, or no source available.
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
    pub source: Option<Source>,
    pub state: PartState,
    /// Why the official source was not used for this part.
    pub official_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FirmwareReport {
    pub model_name: String,
    /// Date of the repository list, if it was used.
    pub list_updated: Option<Date>,
    pub instructions_url: String,
    pub parts: Vec<PartReport>,
}

impl FirmwareReport {
    pub fn update_available(&self) -> bool {
        self.parts
            .iter()
            .any(|p| p.state == PartState::UpdateAvailable)
    }
}

/// Part key per installed firmware (keyboard, display variant).
fn part_keys(model: &dyn KeyboardModel, firmware: &[FirmwareInfo]) -> Vec<Option<&'static str>> {
    let variant: ScreenVariant = model.screen_variant(firmware);
    firmware
        .iter()
        .map(|f| model.manifest_part(f.role, variant))
        .collect()
}

fn from_manifest(manifest: &Manifest, model: &dyn KeyboardModel, part: &str) -> Option<LatestInfo> {
    let p = manifest.models.get(model.manifest_key())?.parts.get(part)?;
    Some(LatestInfo {
        label: p.label.clone(),
        latest: p.latest.clone(),
        download_url: p.download_url.clone(),
        source: Source::Repository,
    })
}

/// Builds the report from the newest version per part (pure, testable).
pub fn report(
    model: &dyn KeyboardModel,
    firmware: &[FirmwareInfo],
    latest: &HashMap<&'static str, LatestInfo>,
    official_errors: &HashMap<&'static str, String>,
    manifest: Option<&Manifest>,
) -> FirmwareReport {
    let keys = part_keys(model, firmware);
    let parts = firmware
        .iter()
        .zip(keys)
        .map(|(f, key)| {
            let installed_build = model.firmware_build_date(f);
            let info = key.and_then(|k| latest.get(k));
            let state = match (installed_build, info) {
                (Some(i), Some(l)) if i < l.latest.build_date => PartState::UpdateAvailable,
                (Some(i), Some(l)) if i > l.latest.build_date => PartState::Newer,
                (Some(_), Some(_)) => PartState::UpToDate,
                _ => PartState::Unknown,
            };
            PartReport {
                role: f.role,
                label: info.map(|l| l.label.clone()),
                installed: f.raw.clone(),
                installed_build,
                latest: info.map(|l| l.latest.clone()),
                download_url: info.map(|l| l.download_url.clone()),
                source: info.map(|l| l.source),
                state,
                official_error: key.and_then(|k| official_errors.get(k).cloned()),
            }
        })
        .collect::<Vec<_>>();
    let used_list = parts.iter().any(|p| p.source == Some(Source::Repository));
    let entry = manifest.and_then(|m| m.models.get(model.manifest_key()));
    FirmwareReport {
        model_name: entry.map_or_else(|| model.display_name().to_string(), |e| e.name.clone()),
        list_updated: manifest.filter(|_| used_list).map(|m| m.updated),
        instructions_url: entry.map_or_else(
            || model.firmware_instructions_url().to_string(),
            |e| e.instructions_url.clone(),
        ),
        parts,
    }
}

/// Repository list only (e.g. `nd104-cli firmware-check --list`).
pub fn evaluate(
    manifest: &Manifest,
    model: &dyn KeyboardModel,
    firmware: &[FirmwareInfo],
) -> FirmwareReport {
    let latest = part_keys(model, firmware)
        .into_iter()
        .flatten()
        .filter_map(|k| Some((k, from_manifest(manifest, model, k)?)))
        .collect();
    report(model, firmware, &latest, &HashMap::new(), Some(manifest))
}

/// Official sources first, repository list for every part that fails.
pub async fn check(model: &dyn KeyboardModel, firmware: &[FirmwareInfo]) -> Result<FirmwareReport> {
    let wanted: Vec<&'static str> = part_keys(model, firmware).into_iter().flatten().collect();
    let mut latest = HashMap::new();
    let mut official_errors = HashMap::new();
    for key in &wanted {
        let Some(source) = model
            .official_firmware_sources()
            .iter()
            .find(|s| s.part == *key)
        else {
            continue;
        };
        match fetch_official(source).await {
            Ok(l) => {
                latest.insert(
                    *key,
                    LatestInfo {
                        label: source.label.to_string(),
                        latest: l,
                        download_url: drive_folder_url(source.drive_folder_id),
                        source: Source::Official,
                    },
                );
            }
            Err(e) => {
                tracing::warn!("official firmware source for {key} failed: {e:#}");
                official_errors.insert(*key, format!("{e:#}"));
            }
        }
    }

    let missing: Vec<&'static str> = wanted
        .iter()
        .copied()
        .filter(|k| !latest.contains_key(k))
        .collect();
    let mut manifest = None;
    if !missing.is_empty() {
        match fetch_manifest().await {
            Ok(m) => {
                for key in missing {
                    if let Some(info) = from_manifest(&m, model, key) {
                        latest.insert(key, info);
                    }
                }
                manifest = Some(m);
            }
            Err(e) if latest.is_empty() => {
                let official: Vec<String> = official_errors.values().cloned().collect();
                bail!(
                    "official sources: {}; version list: {e:#}",
                    if official.is_empty() {
                        "none".to_string()
                    } else {
                        official.join("; ")
                    }
                );
            }
            Err(e) => tracing::warn!("firmware version list failed: {e:#}"),
        }
    }
    Ok(report(
        model,
        firmware,
        &latest,
        &official_errors,
        manifest.as_ref(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Nd104;

    /// The real list from this repository.
    const REPO_MANIFEST: &str = include_str!("../../../../../firmware/versions.json");

    /// Excerpts of the real Drive folder pages (2026-10-31).
    const KEYBOARD_FOLDER: &str = r#"[[\x22Flashing Keyboard Firmware.mp4\x22],[\x22ND104_Firmware_Flashing_Guide.docx\x22],
        [\x22VN001_314A108_1V0_RGB_TYPE1_20260710.bin\x22],[\x22Wuque_toolbox.exe\x22] Function.prototype.bin a.bin"#;
    const SCREEN_FOLDER: &str = r#"\x22ND104_SCREEN_MCU_320_172_V1.0.0_20260317.bin\x22,
        \x22ND104_SCREEN_UI_320_172_V1.0.0_20251104.bin\x22,\x22Wuque_toolbox.exe\x22"#;
    const SCREEN_CF_FOLDER: &str = r#"\x22ND104_SCREEN_MCU_CF_320_172_V1.0.0_20260305.bin\x22
        \x22ND104_SCREEN_UI_320_172_V1.0.0_20251230.bin\x22"#;

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

    fn source(part: &str) -> &'static OfficialSource {
        Nd104
            .official_firmware_sources()
            .iter()
            .find(|s| s.part == part)
            .unwrap()
    }

    fn official(part: &'static str, html: &str) -> (&'static str, LatestInfo) {
        let s = source(part);
        let latest = latest_from_files(s, &drive_files(html)).unwrap();
        (
            part,
            LatestInfo {
                label: s.label.into(),
                latest,
                download_url: drive_folder_url(s.drive_folder_id),
                source: Source::Official,
            },
        )
    }

    #[test]
    fn drive_file_names_are_extracted() {
        assert_eq!(
            drive_files(KEYBOARD_FOLDER),
            vec!["VN001_314A108_1V0_RGB_TYPE1_20260710.bin".to_string()]
        );
        assert_eq!(drive_files(SCREEN_FOLDER).len(), 2);
    }

    #[test]
    fn newest_matching_file_per_part() {
        let kb = latest_from_files(source("keyboard"), &drive_files(KEYBOARD_FOLDER)).unwrap();
        assert_eq!(kb.build_date.to_string(), "2026-07-10");
        // 24 h/°C: MCU only (not the UI image, not the CF build)
        let both = format!("{SCREEN_FOLDER}{SCREEN_CF_FOLDER}");
        let c = latest_from_files(source("screen_24h_c"), &drive_files(&both)).unwrap();
        assert_eq!(c.file, "ND104_SCREEN_MCU_320_172_V1.0.0_20260317.bin");
        let f = latest_from_files(source("screen_12h_f"), &drive_files(&both)).unwrap();
        assert_eq!(f.file, "ND104_SCREEN_MCU_CF_320_172_V1.0.0_20260305.bin");
        // a newer file wins
        let newer = format!("{KEYBOARD_FOLDER} \\x22VN001_314A108_1V0_RGB_TYPE1_20261120.bin\\x22");
        let kb = latest_from_files(source("keyboard"), &drive_files(&newer)).unwrap();
        assert_eq!(kb.build_date.to_string(), "2026-11-20");
    }

    #[test]
    fn verified_device_is_up_to_date_with_official_sources() {
        let latest = HashMap::from([
            official("keyboard", KEYBOARD_FOLDER),
            official("screen_24h_c", SCREEN_FOLDER),
        ]);
        let r = report(&Nd104, &verified_device(), &latest, &HashMap::new(), None);
        assert!(!r.update_available());
        assert!(
            r.parts.iter().all(|p| p.state == PartState::UpToDate),
            "{r:?}"
        );
        assert!(r.parts.iter().all(|p| p.source == Some(Source::Official)));
        assert_eq!(r.list_updated, None);
        assert_eq!(r.instructions_url, Nd104.firmware_instructions_url());
    }

    #[test]
    fn newer_official_file_reports_update() {
        let newer = format!("{KEYBOARD_FOLDER} \\x22VN001_314A108_1V0_RGB_TYPE1_20261120.bin\\x22");
        let latest = HashMap::from([
            official("keyboard", &newer),
            official("screen_24h_c", SCREEN_FOLDER),
        ]);
        let r = report(&Nd104, &verified_device(), &latest, &HashMap::new(), None);
        assert!(r.update_available());
        assert_eq!(r.parts[0].state, PartState::UpdateAvailable);
    }

    #[test]
    fn repository_list_as_fallback() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        // keyboard official, screen from the list
        let mut latest = HashMap::from([official("keyboard", KEYBOARD_FOLDER)]);
        latest.insert(
            "screen_24h_c",
            from_manifest(&m, &Nd104, "screen_24h_c").unwrap(),
        );
        let errors = HashMap::from([(
            "screen_24h_c",
            "Google Drive folder not available".to_string(),
        )]);
        let r = report(&Nd104, &verified_device(), &latest, &errors, Some(&m));
        assert_eq!(r.parts[0].source, Some(Source::Official));
        assert_eq!(r.parts[1].source, Some(Source::Repository));
        assert!(r.parts[1].official_error.is_some());
        assert_eq!(r.list_updated, Some(m.updated));
        assert!(r.parts.iter().all(|p| p.state == PartState::UpToDate));
    }

    #[test]
    fn repository_manifest_alone() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        let r = evaluate(&m, &Nd104, &verified_device());
        assert!(r.parts.iter().all(|p| p.state == PartState::UpToDate));
        let mut old = verified_device();
        old[0] = fw(Role::Keyboard, "V3.0.4 20260201_101010");
        assert!(evaluate(&m, &Nd104, &old).update_available());
    }

    #[test]
    fn newer_build_and_unknown_variant() {
        let m = parse_manifest(REPO_MANIFEST).unwrap();
        let fw = vec![
            fw(Role::Keyboard, "V3.0.20 20261201_000000"),
            fw(Role::Screen, "V1.3.0 Dec 24 2026 10:00:00"),
        ];
        let r = evaluate(&m, &Nd104, &fw);
        assert_eq!(r.parts[0].state, PartState::Newer);
        assert_eq!(r.parts[1].state, PartState::Unknown);
    }

    #[test]
    fn manifest_schema_and_bom() {
        assert!(
            parse_manifest(&REPO_MANIFEST.replacen("\"schema\": 1", "\"schema\": 99", 1)).is_err()
        );
        assert!(parse_manifest(&format!("\u{feff}{REPO_MANIFEST}")).is_ok());
    }
}
