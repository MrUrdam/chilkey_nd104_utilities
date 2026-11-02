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
mod keymap;
mod lighting;
mod picture;
mod settings;
mod theme;
mod time;
mod version;
mod weather;

pub use checksum::{checksum8, crc16};
pub use device::{
    Target, UsbId, KEYBOARD, RECEIVER, SCREEN, SCREEN_DFU, VENDOR_USAGE, VENDOR_USAGE_PAGE,
};
pub use error::ProtocolError;
pub use frame::{frame, FrameKind, FRAME_LEN};
pub use keymap::{
    keymap_read, keymap_read_plan, keymap_set_key, knob_get, knob_set, parse_keymap_answer,
    KeymapAnswer, KEYMAP_COLS, KEYMAP_GET_BUFFER, KEYMAP_LAYERS, KEYMAP_ROWS, KEYMAP_SET_KEY,
    KEYS_PER_LAYER, KNOB_DIRECTIONS, KNOB_GET, KNOB_LAYERS, KNOB_SET,
};
pub use lighting::{
    hue_sat_to_rgb, light_get, light_save, light_set, parse_light_answer, rgb_to_hue_sat,
    LightAnswer, LightValue, LightZone, EFFECT_OFF, EFFECT_SOLID, LIGHT_GET, LIGHT_SAVE, LIGHT_SET,
    LIGHT_UNHANDLED,
};
pub use picture::{
    parse_picture_ack, picture_pages, picture_payload, picture_start, PictureAck, PictureError,
    PictureFrame, PICTURE_HEIGHT, PICTURE_MAX_BYTES, PICTURE_MAX_FRAMES, PICTURE_REPORT_LEN,
    PICTURE_WIDTH,
};
pub use settings::{
    parse_setting_answer, setting_get, setting_set, settings_save, SettingAnswer, SETTINGS_CHANNEL,
    SETTINGS_GROUP,
};
pub use theme::{theme_packet, Rgb, Theme};
pub use time::{time_packet, ClockTime};
pub use version::{
    keyboard_version_query, normalize_version, parse_version_response, screen_version_query,
    FirmwareVersion,
};
pub use weather::{encode_temperature, weather_packet, Weather, WeatherIcon};
