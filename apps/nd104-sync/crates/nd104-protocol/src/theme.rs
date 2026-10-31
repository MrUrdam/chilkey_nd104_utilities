use crate::error::ProtocolError;
use crate::frame::{frame, FrameKind, FRAME_LEN};

const THEME_COMMAND: u8 = 0xFD;

/// Display theme as offered by the official web driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme(u8);

impl Theme {
    /// "Neon Grid" – default.
    pub const NEON_GRID: Self = Self(1);
    /// "Mechanical Age".
    pub const MECHANICAL_AGE: Self = Self(2);
    /// "Emotional Moment".
    pub const EMOTIONAL_MOMENT: Self = Self(3);

    pub fn new(id: u8) -> Result<Self, ProtocolError> {
        if !(1..=3).contains(&id) {
            return Err(ProtocolError::InvalidTheme(id));
        }
        Ok(Self(id))
    }

    pub fn id(self) -> u8 {
        self.0
    }
}

/// 24-bit colour, e.g. from `#AA7ECC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Parses `#RRGGBB` or `RRGGBB`.
    pub fn from_hex(text: &str) -> Option<Self> {
        let hex = text.trim().trim_start_matches('#');
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
        })
    }

    /// RGB565 as used by the display (5 bit red, 6 bit green, 5 bit blue).
    pub fn to_rgb565(self) -> u16 {
        (u16::from(self.r >> 3) << 11) | (u16::from(self.g >> 2) << 5) | u16::from(self.b >> 3)
    }
}

/// Builds the 32-byte theme packet (command `FD`, 6 payload bytes):
/// `00, background (RGB565 BE), text colour (RGB565 BE), theme id`.
/// Layout from the official web driver (docs/protocol.md § 4.4).
pub fn theme_packet(
    theme: Theme,
    background: Rgb,
    text: Rgb,
) -> Result<[u8; FRAME_LEN], ProtocolError> {
    let bg = background.to_rgb565().to_be_bytes();
    let fg = text.to_rgb565().to_be_bytes();
    frame(
        FrameKind::Command,
        THEME_COMMAND,
        &[0x00, bg[0], bg[1], fg[0], fg[1], theme.id()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_like_the_web_driver() {
        // web driver default background #AA7ECC → "abf9"
        assert_eq!(Rgb::from_hex("#AA7ECC").unwrap().to_rgb565(), 0xABF9);
        assert_eq!(Rgb::from_hex("000000").unwrap().to_rgb565(), 0x0000);
        assert_eq!(Rgb::from_hex("#FFFFFF").unwrap().to_rgb565(), 0xFFFF);
        assert_eq!(Rgb::from_hex("#22D3EE").unwrap().to_rgb565(), 0x269D);
    }

    #[test]
    fn hex_parsing() {
        assert!(Rgb::from_hex("#12345").is_none());
        assert!(Rgb::from_hex("#GG0000").is_none());
        assert_eq!(
            Rgb::from_hex(" #0a0B0c ").unwrap(),
            Rgb {
                r: 10,
                g: 11,
                b: 12
            }
        );
    }

    #[test]
    fn theme_range() {
        assert!(Theme::new(0).is_err());
        assert!(Theme::new(4).is_err());
        assert_eq!(Theme::new(3).unwrap(), Theme::EMOTIONAL_MOMENT);
    }

    #[test]
    fn packet_layout() {
        let p = theme_packet(
            Theme::NEON_GRID,
            Rgb::from_hex("#AA7ECC").unwrap(),
            Rgb::from_hex("#000000").unwrap(),
        )
        .unwrap();
        assert_eq!(&p[..6], &[0x1C, 0x02, 0x00, 0x00, 0x00, 0x0B]);
        assert_eq!(
            &p[8..18],
            &[0xA5, 0xFD, 0x00, 0x06, 0x00, 0xAB, 0xF9, 0x00, 0x00, 0x01]
        );
        let sum: u32 = p[9..18].iter().map(|&b| u32::from(b)).sum();
        assert_eq!(p[18], (sum as u8) ^ 0xFF);
    }
}
