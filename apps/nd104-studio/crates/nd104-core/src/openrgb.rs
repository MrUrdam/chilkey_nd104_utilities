//! The ND104 for programs that speak the OpenRGB SDK – OpenRGB (as a client
//! of another server), Artemis, Aurora …
//!
//! The keyboard itself speaks OpenRGB's QMK protocol only on its lighting
//! channel (USB interface 4, `FF59:0060`, report ID 05), where stock OpenRGB
//! does not look (docs/protocol.md § 5.4). ND104 Studio therefore offers the
//! keyboard through its own small SDK server and passes the colours on to
//! that channel. This module is the protocol part: packet header, the device
//! description and the lighting-channel reports. The network part is in the
//! app (`src-tauri/src/openrgb.rs`).
//!
//! Protocol: OpenRGB's `Documentation/OpenRGBSDK.md`, enums from
//! `RGBController.h` (release 0.9). All numbers little-endian.

use serde::Deserialize;

/// Default port. OpenRGB's own server uses 6742; this one must be different
/// so that OpenRGB can run its server and still connect to ND104 Studio.
pub const DEFAULT_PORT: u16 = 6743;
/// Highest SDK protocol version this server speaks (OpenRGB 0.9: segments,
/// brightness, vendor). Clients use the lower of theirs and this.
pub const SERVER_PROTOCOL: u32 = 4;
pub const MAGIC: [u8; 4] = *b"ORGB";
pub const HEADER_LEN: usize = 16;

/// Packet IDs used here.
pub mod packet {
    pub const REQUEST_CONTROLLER_COUNT: u32 = 0;
    pub const REQUEST_CONTROLLER_DATA: u32 = 1;
    pub const REQUEST_PROTOCOL_VERSION: u32 = 40;
    pub const SET_CLIENT_NAME: u32 = 50;
    pub const DEVICE_LIST_UPDATED: u32 = 100;
    pub const PROFILE_LIST: u32 = 150;
    pub const RESIZE_ZONE: u32 = 1000;
    pub const UPDATE_LEDS: u32 = 1050;
    pub const UPDATE_ZONE_LEDS: u32 = 1051;
    pub const UPDATE_SINGLE_LED: u32 = 1052;
    pub const SET_CUSTOM_MODE: u32 = 1100;
    pub const UPDATE_MODE: u32 = 1101;
    pub const SAVE_MODE: u32 = 1102;
}

const DEVICE_TYPE_KEYBOARD: i32 = 5;
const ZONE_TYPE_LINEAR: i32 = 1;
const ZONE_TYPE_MATRIX: i32 = 2;
const MODE_FLAG_HAS_PER_LED_COLOR: u32 = 1 << 5;
const MODE_COLORS_NONE: u32 = 0;
const MODE_COLORS_PER_LED: u32 = 1;
/// Matrix cell without an LED.
const NO_LED: u32 = 0xFFFF_FFFF;
/// The key matrix: every key LED in its own cell (checked by a test).
const MATRIX_WIDTH: u32 = 24;
const MATRIX_HEIGHT: u32 = 6;

/// Mode 0 sends the client's colours to every LED; mode 1 gives the
/// lighting back to the keyboard's own (stored) effects.
pub const MODE_DIRECT: i32 = 0;
pub const MODE_KEYBOARD: i32 = 1;

/// One LED as the keyboard reports it (`signalrgb/layout.json`, made by
/// `signalrgb/make_plugin.py` from what the keyboard answered).
#[derive(Debug, Clone, Deserialize)]
pub struct Led {
    pub i: u16,
    pub x: u8,
    pub y: u8,
    pub name: String,
    /// `keys`, `bar` (light bar) or `edge` (edge glow).
    pub zone: String,
}

/// The 130 LEDs of the ND104 (firmware V3.0.14), in the keyboard's order.
pub fn layout() -> Vec<Led> {
    serde_json::from_str(include_str!("../../../signalrgb/layout.json"))
        .expect("signalrgb/layout.json is valid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub device: u32,
    pub id: u32,
    pub size: u32,
}

/// `None` if the magic is wrong (not an OpenRGB client).
pub fn parse_header(b: &[u8; HEADER_LEN]) -> Option<Header> {
    if b[..4] != MAGIC {
        return None;
    }
    let u = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    Some(Header {
        device: u(4),
        id: u(8),
        size: u(12),
    })
}

/// A complete packet: header + data.
pub fn packet(device: u32, id: u32, data: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(HEADER_LEN + data.len());
    p.extend(MAGIC);
    p.extend(device.to_le_bytes());
    p.extend(id.to_le_bytes());
    p.extend((data.len() as u32).to_le_bytes());
    p.extend(data);
    p
}

/// Builds the little-endian blocks of the SDK.
struct Writer(Vec<u8>);

impl Writer {
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend(v.to_le_bytes());
    }
    /// Length (with the terminating 0) and the text with a 0 at the end.
    fn text(&mut self, s: &str) {
        self.u16(s.len() as u16 + 1);
        self.0.extend(s.as_bytes());
        self.0.push(0);
    }
}

/// What the client sees of the keyboard besides the LEDs.
pub struct DeviceInfo<'a> {
    pub name: &'a str,
    pub vendor: &'a str,
    pub description: &'a str,
    pub version: &'a str,
    pub location: &'a str,
    pub active_mode: i32,
}

/// The "controller data" block for `protocol` (≤ [`SERVER_PROTOCOL`]),
/// including its leading `data_size`.
pub fn controller_data(protocol: u32, info: &DeviceInfo, leds: &[Led], colors: &[u32]) -> Vec<u8> {
    let mut w = Writer(Vec::new());
    w.u32(0); // data_size, filled in at the end
    w.i32(DEVICE_TYPE_KEYBOARD);
    w.text(info.name);
    if protocol >= 1 {
        w.text(info.vendor);
    }
    w.text(info.description);
    w.text(info.version);
    w.text(""); // serial
    w.text(info.location);

    // modes
    w.u16(2);
    w.i32(info.active_mode);
    for (name, value, flags, color_mode) in [
        (
            "Direct",
            MODE_DIRECT,
            MODE_FLAG_HAS_PER_LED_COLOR,
            MODE_COLORS_PER_LED,
        ),
        ("Keyboard effects", MODE_KEYBOARD, 0, MODE_COLORS_NONE),
    ] {
        w.text(name);
        w.i32(value);
        w.u32(flags);
        w.u32(0); // speed min
        w.u32(0); // speed max
        if protocol >= 3 {
            w.u32(0); // brightness min
            w.u32(0); // brightness max
        }
        w.u32(0); // colors min
        w.u32(0); // colors max
        w.u32(0); // speed
        if protocol >= 3 {
            w.u32(0); // brightness
        }
        w.u32(0); // direction
        w.u32(color_mode);
        w.u16(0); // mode colours
    }

    // zones: the keys as a matrix, light bar and edge glow as strips
    let zones = zones(leds);
    w.u16(zones.len() as u16);
    for zone in &zones {
        w.text(zone.name);
        w.i32(if zone.matrix.is_some() {
            ZONE_TYPE_MATRIX
        } else {
            ZONE_TYPE_LINEAR
        });
        let count = zone.leds.len() as u32;
        w.u32(count);
        w.u32(count);
        w.u32(count);
        match &zone.matrix {
            Some(map) => {
                w.u16((8 + 4 * map.len()) as u16);
                w.u32(MATRIX_HEIGHT);
                w.u32(MATRIX_WIDTH);
                for &cell in map {
                    w.u32(cell);
                }
            }
            None => w.u16(0),
        }
        if protocol >= 4 {
            w.u16(0); // segments
        }
    }

    w.u16(leds.len() as u16);
    for led in leds {
        w.text(&led.name);
        w.u32(u32::from(led.i));
    }
    w.u16(colors.len() as u16);
    for &c in colors {
        w.u32(c);
    }
    let size = w.0.len() as u32;
    w.0[..4].copy_from_slice(&size.to_le_bytes());
    w.0
}

struct Zone<'a> {
    name: &'static str,
    leds: Vec<&'a Led>,
    /// For the keys: LED index within the zone per cell, row by row.
    matrix: Option<Vec<u32>>,
}

/// Keys, light bar, edge glow – each a run of consecutive LEDs.
fn zones(leds: &[Led]) -> Vec<Zone<'_>> {
    let pick = |zone: &str| leds.iter().filter(|l| l.zone == zone).collect::<Vec<_>>();
    let keys = pick("keys");
    let mut map = vec![NO_LED; (MATRIX_WIDTH * MATRIX_HEIGHT) as usize];
    for (n, led) in keys.iter().enumerate() {
        let (row, col) = matrix_cell(led);
        map[(row * MATRIX_WIDTH + col) as usize] = n as u32;
    }
    vec![
        Zone {
            name: "Keyboard",
            leds: keys,
            matrix: Some(map),
        },
        Zone {
            name: "Light bar",
            leds: pick("bar"),
            matrix: None,
        },
        Zone {
            name: "Edge glow",
            leds: pick("edge"),
            matrix: None,
        },
    ]
}

/// Row and column of a key LED (QMK points: x 0–226, rows every ~13 from 12).
fn matrix_cell(led: &Led) -> (u32, u32) {
    let row = ((f32::from(led.y) - 12.0) / 12.8)
        .round()
        .clamp(0.0, (MATRIX_HEIGHT - 1) as f32);
    let col = (f32::from(led.x) * (MATRIX_WIDTH - 1) as f32 / 226.0).round();
    (row as u32, (col as u32).min(MATRIX_WIDTH - 1))
}

/// First LED of a zone (zones are consecutive in the keyboard's order).
pub fn zone_start(leds: &[Led], zone: u32) -> Option<usize> {
    let name = ["keys", "bar", "edge"].get(zone as usize)?;
    leds.iter().position(|l| l.zone == *name)
}

/// Colours from an UPDATELEDS / UPDATEZONELEDS body: `num_colors` at
/// `offset`, then that many `RGBColor`s.
pub fn parse_colors(data: &[u8], offset: usize) -> Option<Vec<u32>> {
    let n = u16::from_le_bytes([*data.get(offset)?, *data.get(offset + 1)?]) as usize;
    let start = offset + 2;
    let bytes = data.get(start..start + 4 * n)?;
    Some(
        bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    )
}

/// `RGBColor` (`r | g << 8 | b << 16`) → red, green, blue.
pub fn rgb(color: u32) -> [u8; 3] {
    let [r, g, b, _] = color.to_le_bytes();
    [r, g, b]
}

// --- the keyboard's lighting channel (SignalRGB commands, checked on the
// keyboard 2026-11-02) -------------------------------------------------------

/// Report ID of the lighting channel.
pub const REPORT_ID: u8 = 0x05;
/// Report length including the report ID.
pub const REPORT_LEN: usize = 64;
/// LEDs per stream report (the number checked on the keyboard).
pub const LEDS_PER_REPORT: usize = 9;

fn report(payload: &[u8]) -> Vec<u8> {
    let mut r = vec![REPORT_ID];
    r.extend(payload);
    r.resize(REPORT_LEN, 0);
    r
}

/// Switches the keyboard to colours from the PC.
pub fn report_direct_on() -> Vec<u8> {
    report(&[0x25])
}

/// Gives the lighting back to the keyboard (its stored effect returns).
pub fn report_direct_off() -> Vec<u8> {
    report(&[0x26])
}

/// Reports that set all LEDs: `24 first count r g b …`.
pub fn reports_colors(colors: &[u32]) -> Vec<Vec<u8>> {
    colors
        .chunks(LEDS_PER_REPORT)
        .enumerate()
        .map(|(n, chunk)| {
            let mut p = vec![0x24, (n * LEDS_PER_REPORT) as u8, chunk.len() as u8];
            for &c in chunk {
                p.extend(rgb(c));
            }
            report(&p)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn layout_has_the_keyboards_130_leds() {
        let leds = layout();
        assert_eq!(leds.len(), 130);
        assert!(leds.iter().enumerate().all(|(n, l)| l.i as usize == n));
        assert_eq!(zone_start(&leds, 1), Some(115));
        assert_eq!(zone_start(&leds, 2), Some(120));
    }

    #[test]
    fn every_key_has_its_own_matrix_cell() {
        let leds = layout();
        let cells: HashSet<_> = leds
            .iter()
            .filter(|l| l.zone == "keys")
            .map(matrix_cell)
            .collect();
        assert_eq!(cells.len(), 115);
    }

    #[test]
    fn header_round_trip() {
        let p = packet(0, packet::UPDATE_LEDS, &[1, 2, 3]);
        let h = parse_header(p[..16].try_into().unwrap()).unwrap();
        assert_eq!((h.device, h.id, h.size), (0, packet::UPDATE_LEDS, 3));
        assert!(parse_header(&[0; 16]).is_none());
    }

    /// Walks the block like an OpenRGB client of `protocol` would.
    fn walk(data: &[u8], protocol: u32) -> (String, usize, Vec<String>) {
        let mut at = 0usize;
        let u16_ = |at: &mut usize| {
            let v = u16::from_le_bytes([data[*at], data[*at + 1]]) as usize;
            *at += 2;
            v
        };
        let u32_ = |at: &mut usize| {
            let v = u32::from_le_bytes(data[*at..*at + 4].try_into().unwrap()) as usize;
            *at += 4;
            v
        };
        let text = |at: &mut usize| {
            let n = u16::from_le_bytes([data[*at], data[*at + 1]]) as usize;
            let s = String::from_utf8(data[*at + 2..*at + 2 + n - 1].to_vec()).unwrap();
            assert_eq!(data[*at + 2 + n - 1], 0);
            *at += 2 + n;
            s
        };
        assert_eq!(u32_(&mut at), data.len(), "data_size");
        assert_eq!(u32_(&mut at), 5, "keyboard");
        let name = text(&mut at);
        if protocol >= 1 {
            text(&mut at);
        }
        for _ in 0..4 {
            text(&mut at);
        }
        let modes = u16_(&mut at);
        u32_(&mut at); // active
        for _ in 0..modes {
            text(&mut at);
            at += 4 * if protocol >= 3 { 12 } else { 9 };
            let colors = u16_(&mut at);
            at += 4 * colors;
        }
        let zones = u16_(&mut at);
        let mut zone_names = Vec::new();
        for _ in 0..zones {
            zone_names.push(text(&mut at));
            at += 16;
            let matrix = u16_(&mut at);
            at += matrix;
            if protocol >= 4 {
                assert_eq!(u16_(&mut at), 0);
            }
        }
        let leds = u16_(&mut at);
        for _ in 0..leds {
            text(&mut at);
            u32_(&mut at);
        }
        let colors = u16_(&mut at);
        at += 4 * colors;
        assert_eq!(at, data.len(), "nothing left over");
        (name, leds, zone_names)
    }

    #[test]
    fn controller_data_parses_for_every_protocol() {
        let leds = layout();
        let info = DeviceInfo {
            name: "Chilkey ND104",
            vendor: "Chilkey",
            description: "via ND104 Studio",
            version: "V3.0.14",
            location: "USB",
            active_mode: MODE_KEYBOARD,
        };
        let colors = vec![0u32; leds.len()];
        for protocol in 0..=SERVER_PROTOCOL {
            let data = controller_data(protocol, &info, &leds, &colors);
            let (name, n, zones) = walk(&data, protocol);
            assert_eq!(name, "Chilkey ND104");
            assert_eq!(n, 130);
            assert_eq!(zones, ["Keyboard", "Light bar", "Edge glow"]);
        }
    }

    #[test]
    fn colors_become_stream_reports() {
        let colors = vec![0x0000_00FF; 130]; // red
        let reports = reports_colors(&colors);
        assert_eq!(reports.len(), 15);
        assert_eq!(&reports[0][..7], &[0x05, 0x24, 0, 9, 0xFF, 0, 0]);
        assert_eq!(&reports[14][..4], &[0x05, 0x24, 126, 4]);
        assert!(reports.iter().all(|r| r.len() == REPORT_LEN));
        assert_eq!(report_direct_on()[..2], [0x05, 0x25]);
        assert_eq!(report_direct_off()[..2], [0x05, 0x26]);
    }

    #[test]
    fn colors_are_parsed_from_update_packets() {
        let mut body = 14u32.to_le_bytes().to_vec();
        body.extend(2u16.to_le_bytes());
        body.extend(0x00_33_22_11u32.to_le_bytes());
        body.extend(0x00_66_55_44u32.to_le_bytes());
        assert_eq!(
            parse_colors(&body, 4),
            Some(vec![0x00_33_22_11, 0x00_66_55_44])
        );
        assert_eq!(rgb(0x00_33_22_11), [0x11, 0x22, 0x33]);
        assert_eq!(parse_colors(&body[..8], 4), None);
    }
}
