//! Keyboard model interface.
//!
//! The core never talks about a specific keyboard directly. Everything
//! model-specific – USB interfaces, packets, answers, firmware – lives behind
//! [`KeyboardModel`]. The Chilkey ND104 is the only implementation for now.

mod nd104;

pub use nd104::Nd104;

use std::fmt;

use jiff::civil::DateTime;
use serde::Serialize;

use crate::keymap::KeymapSpec;
use crate::lighting::{LightChange, LightField, LightZone};
use crate::weather::WeatherReport;

/// What a USB interface is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Keyboard,
    Screen,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Role::Keyboard => "keyboard",
            Role::Screen => "screen",
        })
    }
}

/// One HID interface a model needs (check P1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceSpec {
    pub role: Role,
    pub vid: u16,
    pub pid: u16,
    pub usage_page: u16,
    pub usage: u16,
    /// Output report length without report ID (check P2/P4).
    pub report_len: usize,
}

/// Feature switches the UI uses to show or hide options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Capabilities {
    pub time: bool,
    pub weather: bool,
    pub weather_icons: u8,
    pub screen_width: u16,
    pub screen_height: u16,
}

/// Screen firmware variant (Chilkey ships 24 h/°C and 12 h/°F builds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVariant {
    Celsius24h,
    Fahrenheit12h,
    Unknown,
}

/// Firmware of one device as read with the version query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FirmwareInfo {
    pub role: Role,
    /// Full text from the device, e.g. `V3.0.14 20260710_123331`.
    pub raw: String,
    /// Normalized version, e.g. `3.0.14`.
    pub version: String,
    /// Build date/time text, e.g. `20260710_123331`.
    pub build: String,
}

/// A firmware version verified on real hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VerifiedFirmware {
    pub role: Role,
    pub version: &'static str,
}

/// An official firmware source of the vendor: a public Google Drive folder
/// and how to recognise the right file in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfficialSource {
    /// Part key, same as in `firmware/versions.json`.
    pub part: &'static str,
    pub label: &'static str,
    pub drive_folder_id: &'static str,
    /// File name must contain all of these …
    pub include: &'static [&'static str],
    /// … and none of these.
    pub exclude: &'static [&'static str],
}

/// A packet ready to be written to one interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub role: Role,
    /// Command byte the acknowledgement must echo.
    pub command: u8,
    pub bytes: Vec<u8>,
}

/// Classification of one received input report (check P5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportClass {
    /// "Packet received" from an intermediate device (e.g. keyboard relay).
    Receipt,
    /// Acknowledgement for a command.
    Ack { command: u8, ok: bool },
    /// Answer to a firmware version query.
    Firmware(FirmwareInfo),
    /// Answer to a lighting command (echo of a set/save, or a value read).
    Light {
        command: u8,
        zone: LightZone,
        field: Option<LightField>,
        data: Vec<u8>,
    },
    /// The firmware does not support the command it was sent.
    Unsupported,
    /// Flow control of a picture transfer.
    Transfer(TransferAck),
    /// Answer to a keymap command.
    Keymap(KeymapReply),
    /// Well-formed but unknown message: logged and ignored.
    Ignored(String),
    /// Damaged or structurally unknown: protocol violation.
    Malformed(String),
}

/// Flow-control answer during a picture transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAck {
    /// Send the next page (after the last page: transfer complete).
    Next,
    /// Send the last page again.
    Resend,
}

/// Answer to a keymap command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapReply {
    /// Part of the keymap buffer (`offset` in bytes).
    Buffer { offset: u16, data: Vec<u8> },
    /// Echo of a key write.
    KeySet {
        layer: u8,
        row: u8,
        col: u8,
        code: u16,
    },
    /// Knob action (read answer or write echo).
    Knob {
        set: bool,
        layer: u8,
        direction: u8,
        code: u16,
    },
}

/// Size limits of the own picture/GIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PictureSpec {
    pub width: u32,
    pub height: u32,
    pub max_frames: usize,
    /// Limit for the converted data sent to the screen.
    pub max_bytes: usize,
    /// Limit for the file the user picks (same as the vendor's tool).
    pub max_file_bytes: usize,
}

/// One frame of a picture, already encoded for the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFrame {
    pub jpeg: Vec<u8>,
    /// Display time in 10 ms units.
    pub delay: u8,
}

/// A picture transfer: start report, then pages of data reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureTransfer {
    pub role: Role,
    pub start: Vec<u8>,
    pub pages: Vec<Vec<Vec<u8>>>,
}

/// Everything model-specific. Implementations must be pure: no I/O.
pub trait KeyboardModel: Send + Sync {
    /// Stable id, e.g. `chilkey-nd104`.
    fn id(&self) -> &'static str;
    /// Name for the UI, e.g. `Chilkey ND104`.
    fn display_name(&self) -> &'static str;
    /// Interfaces that must all be present (wired USB).
    fn interfaces(&self) -> &'static [InterfaceSpec];
    fn capabilities(&self) -> Capabilities;
    /// Firmware version queries, one per role.
    fn firmware_queries(&self) -> Vec<Packet>;
    /// Firmware versions verified on hardware.
    fn verified_firmware(&self) -> &'static [VerifiedFirmware];
    /// Screen variant derived from the firmware, if recognisable.
    fn screen_variant(&self, firmware: &[FirmwareInfo]) -> ScreenVariant;
    /// Encodes local wall-clock time.
    fn encode_time(&self, time: &DateTime) -> anyhow::Result<Packet>;
    /// Encodes weather; maps the neutral condition to the model's icons.
    fn encode_weather(&self, weather: &WeatherReport) -> anyhow::Result<Packet>;
    /// Classifies an input report received on `role`.
    fn classify_report(&self, role: Role, report: &[u8]) -> ReportClass;
    /// Key in `firmware/versions.json`.
    fn manifest_key(&self) -> &'static str;
    /// Build date contained in a firmware version answer, if recognisable.
    fn firmware_build_date(&self, firmware: &FirmwareInfo) -> Option<jiff::civil::Date>;
    /// Part key in `firmware/versions.json` for a role (and screen variant).
    fn manifest_part(&self, role: Role, variant: ScreenVariant) -> Option<&'static str>;
    /// Official firmware download folders of the vendor, checked first.
    fn official_firmware_sources(&self) -> &'static [OfficialSource];
    /// Vendor's firmware update instructions.
    fn firmware_instructions_url(&self) -> &'static str;
    /// Lighting zones that can be controlled (empty: no lighting support).
    fn lighting_zones(&self) -> &'static [LightZone] {
        &[]
    }
    /// Reads one lighting value.
    fn light_read(&self, _zone: LightZone, _field: LightField) -> Option<Packet> {
        None
    }
    /// Changes one lighting value (not stored yet).
    fn light_write(&self, _zone: LightZone, _change: LightChange) -> Option<Packet> {
        None
    }
    /// Stores a zone's lighting in the keyboard.
    fn light_save(&self, _zone: LightZone) -> Option<Packet> {
        None
    }
    /// Keymap size, or `None` if the keymap cannot be backed up.
    fn keymap_spec(&self) -> Option<KeymapSpec> {
        None
    }
    /// Reads of the whole keymap buffer: (packet, offset, size in bytes).
    fn keymap_reads(&self) -> Vec<(Packet, u16, usize)> {
        Vec::new()
    }
    /// Reads one knob action.
    fn knob_read(&self, _layer: u8, _direction: u8) -> Option<Packet> {
        None
    }
    /// Sets one key.
    fn key_write(&self, _layer: u8, _row: u8, _col: u8, _code: u16) -> Option<Packet> {
        None
    }
    /// Sets one knob action.
    fn knob_write(&self, _layer: u8, _direction: u8, _code: u16) -> Option<Packet> {
        None
    }
    /// Own picture/GIF on the screen: limits, or `None` if not supported.
    fn picture_spec(&self) -> Option<PictureSpec> {
        None
    }
    /// Builds the transfer for encoded frames (checks the limits).
    fn picture_transfer(&self, _frames: &[EncodedFrame]) -> anyhow::Result<PictureTransfer> {
        anyhow::bail!("this keyboard does not support own pictures")
    }
}

/// All supported models. Only the ND104 for now.
pub fn registry() -> Vec<Box<dyn KeyboardModel>> {
    vec![Box::new(Nd104)]
}
