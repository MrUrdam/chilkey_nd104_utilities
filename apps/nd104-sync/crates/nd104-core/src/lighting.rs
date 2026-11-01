//! Keyboard lighting: zones, what can be changed, and the state read back.
//!
//! Lighting is an optional extra. A firmware that does not support a command
//! answers "unsupported" and the feature reports that instead of locking the
//! keyboard; damaged answers still lock like every other protocol violation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightZone {
    /// Per-key lighting.
    PerKey,
    /// Bottom lighting (underglow).
    Bottom,
}

/// What is read or changed in a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightField {
    /// 0–255.
    Brightness,
    /// Effect index; 0 = off, 1 = solid colour.
    Effect,
    /// Speed of animated effects, 0–255.
    Speed,
    /// Hue and saturation, 0–255 each.
    Color,
}

/// One change sent to the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case")]
pub enum LightChange {
    Brightness { value: u8 },
    Effect { value: u8 },
    Speed { value: u8 },
    Color { hue: u8, sat: u8 },
}

impl LightChange {
    pub fn field(&self) -> LightField {
        match self {
            LightChange::Brightness { .. } => LightField::Brightness,
            LightChange::Effect { .. } => LightField::Effect,
            LightChange::Speed { .. } => LightField::Speed,
            LightChange::Color { .. } => LightField::Color,
        }
    }
}

/// Current values of a zone; `None` where the firmware does not report them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ZoneState {
    pub effect: Option<u8>,
    pub brightness: Option<u8>,
    pub speed: Option<u8>,
    pub hue: Option<u8>,
    pub sat: Option<u8>,
}

/// Effect index that switches a zone off.
pub const EFFECT_OFF: u8 = 0;
/// Effect used to switch a zone on when no earlier effect is known.
pub const EFFECT_SOLID: u8 = 1;

/// Lighting operation for the device thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightOp {
    Read(LightZone),
    /// Applies the changes; `save` stores them in the keyboard afterwards.
    Apply {
        zone: LightZone,
        changes: Vec<LightChange>,
        save: bool,
    },
}
