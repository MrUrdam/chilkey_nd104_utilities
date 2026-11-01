//! Keyboard settings on VIA channel `00` (docs/protocol.md § 5.2): debounce
//! and sleep times, as Chilkey's web driver sets them.
//!
//! | Bytes | Meaning |
//! |---|---|
//! | `07 00 04 id value` | set a value (not stored yet) |
//! | `08 00 04 id` | read a value; the answer repeats the bytes and adds the value |
//! | `09 00` | store the values in the keyboard's EEPROM |
//!
//! Like lighting, every command is answered with an echo; `FF` as the first
//! byte means "command not supported".

use crate::lighting::{LIGHT_GET, LIGHT_SAVE, LIGHT_SET};

/// VIA channel of the keyboard settings.
pub const SETTINGS_CHANNEL: u8 = 0x00;
/// Value group the web driver uses for every setting.
pub const SETTINGS_GROUP: u8 = 0x04;

/// `07 00 04 id value`.
pub fn setting_set(id: u8, value: u8) -> Vec<u8> {
    vec![LIGHT_SET, SETTINGS_CHANNEL, SETTINGS_GROUP, id, value]
}

/// `08 00 04 id`.
pub fn setting_get(id: u8) -> Vec<u8> {
    vec![LIGHT_GET, SETTINGS_CHANNEL, SETTINGS_GROUP, id]
}

/// `09 00`.
pub fn settings_save() -> Vec<u8> {
    vec![LIGHT_SAVE, SETTINGS_CHANNEL]
}

/// An answer to a settings command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingAnswer {
    /// `07`, `08` or `09`.
    pub command: u8,
    /// Setting id (`None` for the save echo).
    pub id: Option<u8>,
    /// Value (set echo or read answer).
    pub value: Option<u8>,
}

/// Parses an answer on channel `00`; `None` if the report is not one.
pub fn parse_setting_answer(report: &[u8]) -> Option<SettingAnswer> {
    let command = *report.first()?;
    if *report.get(1)? != SETTINGS_CHANNEL {
        return None;
    }
    match command {
        LIGHT_SET | LIGHT_GET => {
            if *report.get(2)? != SETTINGS_GROUP {
                return None;
            }
            Some(SettingAnswer {
                command,
                id: Some(*report.get(3)?),
                value: Some(*report.get(4)?),
            })
        }
        LIGHT_SAVE => Some(SettingAnswer {
            command,
            id: None,
            value: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded(bytes: &[u8]) -> Vec<u8> {
        let mut r = bytes.to_vec();
        r.resize(32, 0);
        r
    }

    #[test]
    fn builds_like_the_web_driver() {
        assert_eq!(setting_set(0x0B, 10), [0x07, 0x00, 0x04, 0x0B, 10]);
        assert_eq!(setting_get(0x01), [0x08, 0x00, 0x04, 0x01]);
        assert_eq!(settings_save(), [0x09, 0x00]);
    }

    #[test]
    fn parses_answers() {
        assert_eq!(
            parse_setting_answer(&padded(&[0x08, 0x00, 0x04, 0x01, 5])),
            Some(SettingAnswer {
                command: 0x08,
                id: Some(1),
                value: Some(5)
            })
        );
        assert_eq!(
            parse_setting_answer(&padded(&[0x09, 0x00])),
            Some(SettingAnswer {
                command: 0x09,
                id: None,
                value: None
            })
        );
        // lighting channels and other groups are not settings
        assert_eq!(
            parse_setting_answer(&padded(&[0x08, 0x03, 0x01, 200])),
            None
        );
        assert_eq!(
            parse_setting_answer(&padded(&[0x08, 0x00, 0x05, 0x01])),
            None
        );
        assert_eq!(parse_setting_answer(&[0x08, 0x00]), None);
    }
}
