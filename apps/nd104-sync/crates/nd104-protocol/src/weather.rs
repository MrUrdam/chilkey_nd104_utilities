use crate::error::ProtocolError;
use crate::frame::{frame, FrameKind, FRAME_LEN};

const WEATHER_COMMAND: u8 = 0xFE;

/// Weather icon index understood by the display firmware.
///
/// Meanings of 0, 2, 4–8 come from the web driver's mapping; 1, 3 and 9 are
/// accepted by the firmware according to the community notes but their look
/// is still unknown (to be documented in M0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherIcon(u8);

impl WeatherIcon {
    pub const SUNNY: Self = Self(0);
    pub const PARTLY_CLOUDY: Self = Self(2);
    pub const RAIN: Self = Self(4);
    pub const SNOW: Self = Self(5);
    pub const CLEAR_NIGHT: Self = Self(6);
    pub const PARTLY_CLOUDY_NIGHT: Self = Self(7);
    pub const CLOUDY: Self = Self(8);

    pub const MAX_INDEX: u8 = 9;

    pub fn new(index: u8) -> Result<Self, ProtocolError> {
        if index > Self::MAX_INDEX {
            return Err(ProtocolError::InvalidIcon(index));
        }
        Ok(Self(index))
    }

    pub fn index(self) -> u8 {
        self.0
    }
}

/// Weather values as shown on the display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    pub icon: WeatherIcon,
    pub current_c: f32,
    pub max_c: f32,
    pub min_c: f32,
}

/// Encodes °C as the display expects it: tenths of a degree in the lower
/// 15 bits, bit 15 set for negative values (sign + magnitude, *not* two's
/// complement).
///
/// Values are rounded to the nearest tenth. Unlike the web driver, 0 °C is
/// sent as `0x0000` and not as "negative zero" (`0x8000`).
pub fn encode_temperature(celsius: f32) -> Result<u16, ProtocolError> {
    if !celsius.is_finite() {
        return Err(ProtocolError::TemperatureOutOfRange(celsius));
    }
    let tenths = (celsius * 10.0).round() as i32;
    if tenths.abs() > 999 {
        return Err(ProtocolError::TemperatureOutOfRange(celsius));
    }
    let magnitude = tenths.unsigned_abs() as u16;
    Ok(if tenths < 0 { magnitude | 0x8000 } else { magnitude })
}

/// Builds the 32-byte weather packet (command `FE`, 8 payload bytes):
/// `00, icon, current (BE), max (BE), min (BE)`.
///
/// The icon really is sent at offset 13 – the web driver computes the icon
/// but always sends a constant `02` there.
pub fn weather_packet(weather: &Weather) -> Result<[u8; FRAME_LEN], ProtocolError> {
    let current = encode_temperature(weather.current_c)?.to_be_bytes();
    let max = encode_temperature(weather.max_c)?.to_be_bytes();
    let min = encode_temperature(weather.min_c)?.to_be_bytes();
    let payload = [
        0x00,
        weather.icon.index(),
        current[0],
        current[1],
        max[0],
        max[1],
        min[0],
        min[1],
    ];
    frame(FrameKind::Command, WEATHER_COMMAND, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_temperatures() {
        assert_eq!(encode_temperature(12.3).unwrap(), 123);
        assert_eq!(encode_temperature(0.04).unwrap(), 0);
        assert_eq!(encode_temperature(99.9).unwrap(), 999);
    }

    #[test]
    fn negative_temperatures_use_sign_bit() {
        assert_eq!(encode_temperature(-3.5).unwrap(), 0x8023);
        assert_eq!(encode_temperature(-0.1).unwrap(), 0x8001);
        assert_eq!(encode_temperature(-99.9).unwrap(), 0x83E7);
    }

    #[test]
    fn zero_is_not_negative_zero() {
        assert_eq!(encode_temperature(0.0).unwrap(), 0);
        assert_eq!(encode_temperature(-0.0).unwrap(), 0);
        assert_eq!(encode_temperature(-0.04).unwrap(), 0);
    }

    #[test]
    fn out_of_range_temperatures() {
        assert!(encode_temperature(100.0).is_err());
        assert!(encode_temperature(-100.0).is_err());
        assert!(encode_temperature(f32::NAN).is_err());
    }

    #[test]
    fn icon_range() {
        assert!(WeatherIcon::new(9).is_ok());
        assert_eq!(WeatherIcon::new(10), Err(ProtocolError::InvalidIcon(10)));
    }

    #[test]
    fn weather_packet_layout() {
        let p = weather_packet(&Weather {
            icon: WeatherIcon::SNOW,
            current_c: -3.5,
            max_c: 1.0,
            min_c: -7.2,
        })
        .unwrap();

        // header, as documented in the community notes: 1c 02 00 00 00 0d
        assert_eq!(&p[..6], &[0x1C, 0x02, 0x00, 0x00, 0x00, 0x0D]);
        assert_eq!(&p[8..12], &[0xA5, 0xFE, 0x00, 0x08]);
        assert_eq!(p[12], 0x00);
        assert_eq!(p[13], 5, "real icon index, not the web driver's constant 2");
        assert_eq!(&p[14..16], &[0x80, 0x23]);
        assert_eq!(&p[16..18], &[0x00, 0x0A]);
        assert_eq!(&p[18..20], &[0x80, 0x48]);

        let sum: u32 = p[9..20].iter().map(|&b| b as u32).sum();
        assert_eq!(p[20], ((sum as u8) ^ 0xFF));
        assert!(p[21..].iter().all(|&b| b == 0));
    }
}
