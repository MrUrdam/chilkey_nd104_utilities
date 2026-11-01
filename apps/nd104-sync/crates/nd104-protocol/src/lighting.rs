//! Keyboard lighting (docs/protocol.md § 5.1).
//!
//! The keyboard firmware is QMK based and speaks the VIA lighting commands on
//! its vendor interface (32-byte reports, no `1C` frame):
//!
//! | Bytes | Meaning |
//! |---|---|
//! | `07 zone value data…` | set a value (not stored yet) |
//! | `08 zone value` | read a value; the answer repeats the bytes and adds the data |
//! | `09 zone` | store the zone's values in the keyboard's EEPROM |
//!
//! Every command is answered with an echo of the request (with the data for
//! reads). `FF` as the first byte means "command not supported".

/// Lighting zone (VIA channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightZone {
    /// Per-key lighting (QMK `rgb_matrix`; web driver: "Background lighting").
    PerKey,
    /// Bottom lighting (QMK `rgblight` underglow; web driver: "Bottom lighting").
    Bottom,
}

impl LightZone {
    pub const fn channel(self) -> u8 {
        match self {
            LightZone::PerKey => 3,
            LightZone::Bottom => 2,
        }
    }

    pub fn from_channel(channel: u8) -> Option<Self> {
        match channel {
            3 => Some(LightZone::PerKey),
            2 => Some(LightZone::Bottom),
            _ => None,
        }
    }
}

/// Value id within a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightValue {
    /// 0–255.
    Brightness,
    /// Effect index; 0 = off, 1 = solid colour (both zones).
    Effect,
    /// 0–255.
    Speed,
    /// Hue and saturation, 0–255 each.
    Color,
}

impl LightValue {
    pub const fn id(self) -> u8 {
        match self {
            LightValue::Brightness => 1,
            LightValue::Effect => 2,
            LightValue::Speed => 3,
            LightValue::Color => 4,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(LightValue::Brightness),
            2 => Some(LightValue::Effect),
            3 => Some(LightValue::Speed),
            4 => Some(LightValue::Color),
            _ => None,
        }
    }

    /// Data bytes of the value.
    pub const fn data_len(self) -> usize {
        match self {
            LightValue::Color => 2,
            _ => 1,
        }
    }
}

pub const LIGHT_SET: u8 = 0x07;
pub const LIGHT_GET: u8 = 0x08;
pub const LIGHT_SAVE: u8 = 0x09;
/// First byte of the answer to a command the firmware does not know.
pub const LIGHT_UNHANDLED: u8 = 0xFF;
/// Effect index that switches a zone off.
pub const EFFECT_OFF: u8 = 0;
/// Effect index for a solid colour.
pub const EFFECT_SOLID: u8 = 1;

/// `07 zone value data…` (unpadded; the caller pads to the report length).
pub fn light_set(zone: LightZone, value: LightValue, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![LIGHT_SET, zone.channel(), value.id()];
    bytes.extend_from_slice(data);
    bytes
}

/// `08 zone value`.
pub fn light_get(zone: LightZone, value: LightValue) -> Vec<u8> {
    vec![LIGHT_GET, zone.channel(), value.id()]
}

/// `09 zone`.
pub fn light_save(zone: LightZone) -> Vec<u8> {
    vec![LIGHT_SAVE, zone.channel()]
}

/// An answer on the keyboard's vendor interface to a lighting command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightAnswer {
    /// Echo of a set/save, or a read with its data (`data` = bytes after the value id).
    Echo {
        command: u8,
        zone: LightZone,
        value: Option<LightValue>,
        data: Vec<u8>,
    },
    /// `FF …`: the firmware does not support the command.
    Unhandled,
}

/// Parses a lighting answer; `None` if the report is not one (or names an
/// unknown zone/value – the caller treats that as unexpected).
pub fn parse_light_answer(report: &[u8]) -> Option<LightAnswer> {
    match *report.first()? {
        LIGHT_UNHANDLED => Some(LightAnswer::Unhandled),
        command @ (LIGHT_SET | LIGHT_GET) => {
            let zone = LightZone::from_channel(*report.get(1)?)?;
            let value = LightValue::from_id(*report.get(2)?)?;
            let data = report.get(3..3 + value.data_len())?.to_vec();
            Some(LightAnswer::Echo {
                command,
                zone,
                value: Some(value),
                data,
            })
        }
        LIGHT_SAVE => Some(LightAnswer::Echo {
            command: LIGHT_SAVE,
            zone: LightZone::from_channel(*report.get(1)?)?,
            value: None,
            data: Vec::new(),
        }),
        _ => None,
    }
}

/// RGB (0–255 each) → hue and saturation on the 0–255 scale, like the web
/// driver. Brightness is set separately.
pub fn rgb_to_hue_sat(r: u8, g: u8, b: u8) -> (u8, u8) {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let sat = if max == 0.0 { 0.0 } else { delta / max };
    (
        (hue / 360.0 * 255.0).round().min(255.0) as u8,
        (sat * 255.0).round() as u8,
    )
}

/// Hue and saturation (0–255) at full value → RGB, for showing the colour.
pub fn hue_sat_to_rgb(hue: u8, sat: u8) -> (u8, u8, u8) {
    let h = f32::from(hue) * 360.0 / 255.0;
    let s = f32::from(sat) / 255.0;
    let x = s * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match h {
        h if h < 60.0 => (s, x, 0.0),
        h if h < 120.0 => (x, s, 0.0),
        h if h < 180.0 => (0.0, s, x),
        h if h < 240.0 => (0.0, x, s),
        h if h < 300.0 => (x, 0.0, s),
        _ => (s, 0.0, x),
    };
    let m = 1.0 - s;
    let c = |v: f32| ((v + m) * 255.0).round() as u8;
    (c(r), c(g), c(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_commands_like_the_web_driver() {
        assert_eq!(
            light_set(LightZone::PerKey, LightValue::Brightness, &[200]),
            [7, 3, 1, 200]
        );
        assert_eq!(
            light_set(LightZone::Bottom, LightValue::Color, &[85, 255]),
            [7, 2, 4, 85, 255]
        );
        assert_eq!(light_get(LightZone::PerKey, LightValue::Effect), [8, 3, 2]);
        assert_eq!(light_save(LightZone::Bottom), [9, 2]);
    }

    #[test]
    fn parses_answers() {
        let mut report = [0u8; 32];
        report[..5].copy_from_slice(&[8, 3, 4, 170, 128]);
        assert_eq!(
            parse_light_answer(&report),
            Some(LightAnswer::Echo {
                command: 8,
                zone: LightZone::PerKey,
                value: Some(LightValue::Color),
                data: vec![170, 128],
            })
        );
        report[..3].copy_from_slice(&[9, 2, 0]);
        assert!(matches!(
            parse_light_answer(&report),
            Some(LightAnswer::Echo {
                command: 9,
                zone: LightZone::Bottom,
                value: None,
                ..
            })
        ));
        report[0] = 0xFF;
        assert_eq!(parse_light_answer(&report), Some(LightAnswer::Unhandled));
        report[..3].copy_from_slice(&[8, 7, 1]);
        assert_eq!(parse_light_answer(&report), None);
        report[..3].copy_from_slice(&[0x1C, 3, 1]);
        assert_eq!(parse_light_answer(&report), None);
    }

    #[test]
    fn colour_conversion_matches_the_web_driver() {
        assert_eq!(rgb_to_hue_sat(255, 0, 0), (0, 255));
        assert_eq!(rgb_to_hue_sat(0, 255, 0), (85, 255));
        assert_eq!(rgb_to_hue_sat(0, 0, 255), (170, 255));
        assert_eq!(rgb_to_hue_sat(255, 255, 255), (0, 0));
        assert_eq!(hue_sat_to_rgb(0, 255), (255, 0, 0));
        assert_eq!(hue_sat_to_rgb(0, 0), (255, 255, 255));
        let (h, s) = rgb_to_hue_sat(34, 211, 238);
        let (r, g, b) = hue_sat_to_rgb(h, s);
        assert!(r.abs_diff(0) < 40 && g > 200 && b > 240);
    }
}
