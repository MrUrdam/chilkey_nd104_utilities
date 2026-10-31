//! Packet building for the Chilkey ND104 keyboard display.
//!
//! Pure functions only: no USB access, no clocks, no network. Everything in
//! here is derived from the official web driver and the community notes, see
//! `docs/protocol.md` in the repository.
//!
//! Scope: wired USB connection only.

mod checksum;
mod device;
mod error;
mod frame;
mod theme;
mod time;
mod version;
mod weather;

pub use checksum::{checksum8, crc16};
pub use device::{Target, UsbId, KEYBOARD, SCREEN, SCREEN_DFU, VENDOR_USAGE, VENDOR_USAGE_PAGE};
pub use error::ProtocolError;
pub use frame::{frame, FrameKind, FRAME_LEN};
pub use theme::{theme_packet, Rgb, Theme};
pub use time::{time_packet, ClockTime};
pub use version::{
    keyboard_version_query, normalize_version, parse_version_response, screen_version_query,
    FirmwareVersion,
};
pub use weather::{encode_temperature, weather_packet, Weather, WeatherIcon};
