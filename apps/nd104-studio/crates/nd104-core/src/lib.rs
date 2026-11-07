//! Core of nd104-studio: everything except the UI.
//!
//! - [`model`]: keyboard model interface and the Chilkey ND104 implementation
//! - [`hid`]: HID access (real backend via hidapi, mockable for tests)
//! - [`keymap`]: keymap backup and restore
//! - [`keyboard_settings`]: debounce and sleep times
//! - [`lighting`]: keyboard lighting zones and changes
//! - [`picture`]: own picture/GIF for the screen (checks and conversion)
//! - [`device`]: device manager with the firmware/protocol guard (P1–P6)
//! - [`clock`]: clock sources, time zones, DST handling
//! - [`weather`]: neutral weather data, providers, geocoding
//! - [`config`]: TOML configuration
//! - [`engine`]: scheduler that ties everything together
//! - [`app_update`]: check the GitHub repository for new app releases
//! - [`secrets`]: API keys in the OS credential store
//! - [`rebrand`]: takes over the data of the former name nd104-sync
//! - [`firmware_update`]: compare keyboard firmware with `firmware/versions.json`
//!
//! Scope: Windows and Linux; USB cable, Bluetooth and 2.4 GHz.

pub mod app_update;
pub mod clock;
pub mod config;
pub mod device;
pub mod engine;
pub mod firmware_update;
pub mod hid;
pub mod keyboard_settings;
pub mod keymap;
pub mod lighting;
pub mod model;
pub mod openrgb;
pub mod picture;
pub mod rebrand;
pub mod secrets;
pub mod weather;

/// Version of this build, e.g. `0.1.0`.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// User agent for all HTTP requests.
pub fn user_agent() -> String {
    format!("nd104-studio/{APP_VERSION} (+https://github.com/MrUrdam/chilkey_nd104_utilities)")
}
