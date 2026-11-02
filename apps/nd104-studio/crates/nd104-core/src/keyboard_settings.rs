//! Keyboard settings: debounce and sleep times (docs/protocol.md § 5.2).
//!
//! What Chilkey's web driver shows, with exactly its values, plus whether PC
//! lighting software may take over the LEDs (the driver's "online RGB",
//! docs/protocol.md § 5.2.1). The driver hides the USB polling rate, and
//! "keyboard colour" has no effect, so those are left out.

use serde::{Deserialize, Serialize};

/// One setting the app can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardSetting {
    /// 0 none, 1 "send, then debounce", 2 "debounce, then send".
    DebounceMode,
    /// Debounce time in ms (0 while the mode is "none").
    DebounceTime,
    /// Minutes without a key press until the keyboard sleeps.
    SleepTime,
    /// Minutes without a key press until the keyboard sleeps deeply.
    DeepSleepTime,
    /// May PC lighting software (OpenRGB, SignalRGB) take over the LEDs?
    /// In the app 1 = allowed, 0 = blocked; see [`Self::to_raw`].
    PcLighting,
}

/// The keyboard's value for "PC lighting software allowed". V3.0.14 stores
/// it inverted: the driver's "online RGB" 1 ("enable") *blocks* PC software
/// (docs/protocol.md § 5.2.1). Should a firmware swap it, change it here.
const PC_LIGHTING_ALLOWED_RAW: u8 = 0;

impl KeyboardSetting {
    pub const ALL: [KeyboardSetting; 5] = [
        KeyboardSetting::DebounceMode,
        KeyboardSetting::DebounceTime,
        KeyboardSetting::SleepTime,
        KeyboardSetting::DeepSleepTime,
        KeyboardSetting::PcLighting,
    ];

    /// Id the value is read with. The sleep time is read with `0B`, the id
    /// it is written with: the web driver reads `06`, but the keyboard
    /// always answers 0 there (V3.0.14, checked 2026-11-02).
    pub const fn read_id(self) -> u8 {
        match self {
            KeyboardSetting::DebounceMode => 0x02,
            KeyboardSetting::DebounceTime => 0x01,
            KeyboardSetting::SleepTime => 0x0B,
            KeyboardSetting::DeepSleepTime => 0x07,
            KeyboardSetting::PcLighting => 0x05,
        }
    }

    /// Id the value is written with (the web driver's ids).
    pub const fn write_id(self) -> u8 {
        self.read_id()
    }

    /// The app's value as the keyboard stores it.
    pub const fn to_raw(self, value: u8) -> u8 {
        match self {
            KeyboardSetting::PcLighting => {
                if value == 1 {
                    PC_LIGHTING_ALLOWED_RAW
                } else {
                    1 - PC_LIGHTING_ALLOWED_RAW
                }
            }
            _ => value,
        }
    }

    /// The keyboard's stored value as the app shows it.
    pub const fn from_raw(self, raw: u8) -> u8 {
        match self {
            KeyboardSetting::PcLighting => (raw == PC_LIGHTING_ALLOWED_RAW) as u8,
            _ => raw,
        }
    }

    /// Values the web driver offers.
    pub fn values(self) -> Vec<u8> {
        match self {
            KeyboardSetting::DebounceMode => vec![0, 1, 2],
            KeyboardSetting::DebounceTime => {
                let mut v: Vec<u8> = (1..=20).collect();
                v.extend((25..=50).step_by(5));
                v.extend((60..=100).step_by(10));
                v.push(200);
                v
            }
            KeyboardSetting::SleepTime | KeyboardSetting::DeepSleepTime => {
                let mut v: Vec<u8> = (1..=20).collect();
                v.extend((25..=50).step_by(5));
                v.push(60);
                v
            }
            KeyboardSetting::PcLighting => vec![0, 1],
        }
    }

    /// Is `value` one the web driver would send? (Debounce time 0 is what
    /// the driver sends together with mode "none".)
    pub fn allows(self, value: u8) -> bool {
        (self == KeyboardSetting::DebounceTime && value == 0) || self.values().contains(&value)
    }
}

/// Values read from the keyboard; `None`: not reported by this firmware.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardSettings {
    pub debounce_mode: Option<u8>,
    pub debounce_time: Option<u8>,
    pub sleep_time: Option<u8>,
    pub deep_sleep_time: Option<u8>,
    /// 1: PC lighting software may take over the LEDs, 0: blocked.
    pub pc_lighting: Option<u8>,
}

impl KeyboardSettings {
    pub fn set(&mut self, setting: KeyboardSetting, value: Option<u8>) {
        match setting {
            KeyboardSetting::DebounceMode => self.debounce_mode = value,
            KeyboardSetting::DebounceTime => self.debounce_time = value,
            KeyboardSetting::SleepTime => self.sleep_time = value,
            KeyboardSetting::DeepSleepTime => self.deep_sleep_time = value,
            KeyboardSetting::PcLighting => self.pc_lighting = value,
        }
    }
}

/// Settings operation for the device thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsOp {
    Read,
    /// Writes one value, stores it in the keyboard, then reads all again.
    Write(KeyboardSetting, u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_web_driver() {
        let debounce = KeyboardSetting::DebounceTime.values();
        assert_eq!(debounce.len(), 20 + 6 + 5 + 1);
        assert_eq!(debounce.last(), Some(&200));
        let sleep = KeyboardSetting::SleepTime.values();
        assert_eq!(sleep.len(), 20 + 6 + 1);
        assert_eq!(sleep.last(), Some(&60));
        assert!(KeyboardSetting::DebounceTime.allows(0));
        assert!(!KeyboardSetting::SleepTime.allows(0));
        assert!(!KeyboardSetting::SleepTime.allows(61));
        assert!(!KeyboardSetting::DebounceMode.allows(3));
    }

    #[test]
    fn pc_lighting_is_stored_inverted() {
        let s = KeyboardSetting::PcLighting;
        assert_eq!(s.write_id(), 0x05);
        // allowed (1 in the app) is 0 in the keyboard, and back
        assert_eq!(s.to_raw(1), 0);
        assert_eq!(s.to_raw(0), 1);
        assert_eq!(s.from_raw(0), 1);
        assert_eq!(s.from_raw(1), 0);
        assert_eq!(KeyboardSetting::SleepTime.to_raw(15), 15);
    }

    #[test]
    fn sleep_is_written_and_read_with_its_own_id() {
        assert_eq!(KeyboardSetting::SleepTime.read_id(), 0x0B);
        assert_eq!(KeyboardSetting::SleepTime.write_id(), 0x0B);
        assert_eq!(KeyboardSetting::DeepSleepTime.write_id(), 0x07);
    }
}
