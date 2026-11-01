//! TOML configuration in `%APPDATA%\nd104-sync\config.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::clock::ClockSettings;
use crate::weather::WeatherSettings;

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema: u32,
    pub general: GeneralSettings,
    pub device: DeviceSettings,
    pub clock: ClockSettings,
    pub weather: WeatherSettings,
    pub updates: UpdateSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            general: GeneralSettings::default(),
            device: DeviceSettings::default(),
            clock: ClockSettings::default(),
            weather: WeatherSettings::default(),
            updates: UpdateSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// `auto`, `de` or `en`.
    pub language: String,
    /// `auto`, `dark` or `light`.
    pub theme: String,
    /// Look of the windows: `calm` (default) or `gamer`.
    pub style: String,
    pub autostart: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            theme: "auto".into(),
            style: "calm".into(),
            autostart: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceSettings {
    /// `auto` = detect by USB IDs.
    pub model: String,
    pub poll_interval_s: u32,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            model: "auto".into(),
            poll_interval_s: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSettings {
    /// Check for firmware updates (manifest in the repository).
    pub firmware_check: bool,
    /// Check GitHub releases for a newer nd104-sync.
    pub app_check: bool,
    pub check_interval_h: u32,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            firmware_check: true,
            app_check: true,
            check_interval_h: 24,
        }
    }
}

/// `%APPDATA%\nd104-sync` (Windows) or the platform equivalent.
pub fn config_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().ok_or_else(|| anyhow!("no home directory"))?;
    Ok(base.config_dir().join("nd104-sync"))
}

/// `%LOCALAPPDATA%\nd104-sync\logs` (Windows) or the platform equivalent.
pub fn log_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().ok_or_else(|| anyhow!("no home directory"))?;
    Ok(base.data_local_dir().join("nd104-sync").join("logs"))
}

pub fn default_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

impl Config {
    /// Loads the file; a missing file gives the defaults.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        Self::from_toml(&text).with_context(|| format!("invalid config {}", path.display()))
    }

    pub fn from_toml(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text)?;
        if config.schema > SCHEMA {
            return Err(anyhow!(
                "config schema {} is newer than this version supports ({SCHEMA})",
                config.schema
            ));
        }
        Ok(config)
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Writes atomically (temp file + rename).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, self.to_toml()?)?;
        fs::rename(&tmp, path).with_context(|| format!("cannot write {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{ClockSource, DstMode};

    #[test]
    fn defaults_round_trip() {
        let c = Config::default();
        assert_eq!(Config::from_toml(&c.to_toml().unwrap()).unwrap(), c);
    }

    #[test]
    fn partial_file_uses_defaults() {
        let c = Config::from_toml(
            r#"
            [clock]
            source = "timezone"
            timezone = "Europe/Berlin"
            dst_mode = "force_winter"

            [weather.location]
            name = "Berlin"
            lat = 52.52
            lon = 13.405
            "#,
        )
        .unwrap();
        assert_eq!(c.clock.source, ClockSource::Timezone);
        assert_eq!(c.clock.dst_mode, DstMode::ForceWinter);
        assert_eq!(c.clock.sync_interval_min, 60);
        assert_eq!(c.weather.provider, "open-meteo");
        assert_eq!(c.weather.location.location().unwrap().lat, 52.52);
    }

    #[test]
    fn newer_schema_is_rejected() {
        assert!(Config::from_toml("schema = 99").is_err());
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join(format!("nd104-config-test-{}", std::process::id()));
        let path = dir.join("config.toml");
        let mut c = Config::default();
        c.weather.interval_min = 30;
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), c);
        fs::remove_dir_all(dir).unwrap();
    }
}
