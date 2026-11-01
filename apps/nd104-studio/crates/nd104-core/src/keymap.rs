//! Keymap backup and restore: all layers and the knob, as a JSON file.
//!
//! Restore writes only the keys that differ from the keyboard, then reads
//! everything back and compares (the keyboard stores each key at once).
//! Macros are not included: the official tool never reads them back, so
//! there is no verified way to back them up yet.

use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};

/// Size of a model's keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct KeymapSpec {
    pub layers: usize,
    pub rows: usize,
    pub cols: usize,
    pub knob_layers: usize,
    pub knob_directions: usize,
}

/// Key codes (16 bit) per layer (`rows × cols`, row by row) and knob
/// actions per layer (left, right).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keymap {
    pub layers: Vec<Vec<u16>>,
    pub knob: Vec<Vec<u16>>,
}

/// What a restore changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeymapChanges {
    /// (layer, row, col, code)
    pub keys: Vec<(u8, u8, u8, u16)>,
    /// (layer, direction, code)
    pub knob: Vec<(u8, u8, u16)>,
}

impl KeymapChanges {
    pub fn len(&self) -> usize {
        self.keys.len() + self.knob.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Keymap {
    /// Checks the dimensions against the model.
    pub fn validate(&self, spec: &KeymapSpec) -> Result<()> {
        ensure!(
            self.layers.len() == spec.layers,
            "keymap has {} layers, expected {}",
            self.layers.len(),
            spec.layers
        );
        for (i, layer) in self.layers.iter().enumerate() {
            ensure!(
                layer.len() == spec.rows * spec.cols,
                "layer {i} has {} keys, expected {}",
                layer.len(),
                spec.rows * spec.cols
            );
        }
        ensure!(
            self.knob.len() == spec.knob_layers,
            "knob has {} layers, expected {}",
            self.knob.len(),
            spec.knob_layers
        );
        for (i, knob) in self.knob.iter().enumerate() {
            ensure!(
                knob.len() == spec.knob_directions,
                "knob layer {i} has {} actions, expected {}",
                knob.len(),
                spec.knob_directions
            );
        }
        Ok(())
    }

    /// Changes that turn `current` into `self`.
    pub fn changes_from(&self, current: &Keymap, spec: &KeymapSpec) -> KeymapChanges {
        let mut changes = KeymapChanges::default();
        for (layer, (want, have)) in self.layers.iter().zip(&current.layers).enumerate() {
            for (index, (w, h)) in want.iter().zip(have).enumerate() {
                if w != h {
                    changes.keys.push((
                        layer as u8,
                        (index / spec.cols) as u8,
                        (index % spec.cols) as u8,
                        *w,
                    ));
                }
            }
        }
        for (layer, (want, have)) in self.knob.iter().zip(&current.knob).enumerate() {
            for (direction, (w, h)) in want.iter().zip(have).enumerate() {
                if w != h {
                    changes.knob.push((layer as u8, direction as u8, *w));
                }
            }
        }
        changes
    }

    /// Keys that are not empty (code ≠ 0), for the summary.
    pub fn assigned_keys(&self) -> usize {
        self.layers.iter().flatten().filter(|&&c| c != 0).count()
    }
}

pub const BACKUP_FORMAT: &str = "nd104-studio keymap backup";
/// Backups written before the rename (nd104-sync 1.5).
pub const OLD_BACKUP_FORMAT: &str = "nd104-sync keymap backup";
pub const BACKUP_VERSION: u32 = 1;

/// The backup file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapBackup {
    pub format: String,
    pub version: u32,
    /// RFC 3339 time of the backup.
    pub created: String,
    pub app_version: String,
    /// Model id, e.g. `chilkey-nd104`.
    pub model: String,
    /// Keyboard firmware at the time of the backup (for the restore hint).
    pub keyboard_firmware: String,
    pub rows: usize,
    pub cols: usize,
    #[serde(flatten)]
    pub keymap: Keymap,
}

impl KeymapBackup {
    pub fn new(keymap: Keymap, spec: &KeymapSpec, model: &str, firmware: &str) -> Self {
        Self {
            format: BACKUP_FORMAT.into(),
            version: BACKUP_VERSION,
            created: jiff::Timestamp::now().to_string(),
            app_version: crate::APP_VERSION.into(),
            model: model.into(),
            keyboard_firmware: firmware.into(),
            rows: spec.rows,
            cols: spec.cols,
            keymap,
        }
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Reads and checks a backup file for `model` / `spec`.
    pub fn from_json(text: &str, model: &str, spec: &KeymapSpec) -> Result<Self> {
        let backup: KeymapBackup =
            serde_json::from_str(text).context("not an nd104-studio keymap backup")?;
        if backup.format != BACKUP_FORMAT && backup.format != OLD_BACKUP_FORMAT {
            bail!("not an nd104-studio keymap backup");
        }
        if backup.version > BACKUP_VERSION {
            bail!(
                "backup from a newer nd104-studio (format {}); update first",
                backup.version
            );
        }
        if backup.model != model {
            bail!(
                "backup is for keyboard model {:?}, not {model:?}",
                backup.model
            );
        }
        ensure!(
            backup.rows == spec.rows && backup.cols == spec.cols,
            "backup has a {}×{} key matrix, expected {}×{}",
            backup.rows,
            backup.cols,
            spec.rows,
            spec.cols
        );
        backup.keymap.validate(spec)?;
        Ok(backup)
    }
}

/// Keymap operation for the device thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapOp {
    Read,
    /// Write the differences to this keymap, then verify.
    Restore(Keymap),
}

/// Result of a [`KeymapOp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapResult {
    Read(Keymap),
    /// Number of keys/knob actions written (0: nothing differed).
    Restored(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: KeymapSpec = KeymapSpec {
        layers: 2,
        rows: 2,
        cols: 3,
        knob_layers: 1,
        knob_directions: 2,
    };

    fn keymap() -> Keymap {
        Keymap {
            layers: vec![vec![4, 5, 6, 0, 0, 7], vec![0; 6]],
            knob: vec![vec![0xE9, 0xEA]],
        }
    }

    #[test]
    fn changes_list_only_differences() {
        let current = keymap();
        let mut target = keymap();
        target.layers[0][4] = 0x29; // row 1, col 1
        target.layers[1][2] = 0x2A; // row 0, col 2
        target.knob[0][1] = 0x80;
        let changes = target.changes_from(&current, &SPEC);
        assert_eq!(changes.keys, vec![(0, 1, 1, 0x29), (1, 0, 2, 0x2A)]);
        assert_eq!(changes.knob, vec![(0, 1, 0x80)]);
        assert_eq!(changes.len(), 3);
        assert!(keymap().changes_from(&current, &SPEC).is_empty());
        assert_eq!(current.assigned_keys(), 4);
    }

    #[test]
    fn backup_round_trip_and_checks() {
        let backup = KeymapBackup::new(keymap(), &SPEC, "chilkey-nd104", "V3.0.14 20260710_123331");
        let json = backup.to_json().unwrap();
        assert!(json.contains("\"layers\""));
        let back = KeymapBackup::from_json(&json, "chilkey-nd104", &SPEC).unwrap();
        assert_eq!(back.keymap, keymap());

        assert!(KeymapBackup::from_json(&json, "other-model", &SPEC).is_err());
        assert!(KeymapBackup::from_json("{}", "chilkey-nd104", &SPEC).is_err());
        let wrong = json.replace(BACKUP_FORMAT, "something else");
        assert!(KeymapBackup::from_json(&wrong, "chilkey-nd104", &SPEC).is_err());
        let newer = json.replace("\"version\": 1", "\"version\": 9");
        assert!(KeymapBackup::from_json(&newer, "chilkey-nd104", &SPEC).is_err());
        let mut short = backup.clone();
        short.keymap.layers[1].pop();
        assert!(
            KeymapBackup::from_json(&short.to_json().unwrap(), "chilkey-nd104", &SPEC).is_err()
        );
    }
}
