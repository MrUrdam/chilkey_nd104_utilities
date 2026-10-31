use crate::error::ProtocolError;
use crate::frame::{frame, FrameKind, FRAME_LEN};

const TIME_COMMAND: u8 = 0x38;

/// Local wall-clock time as shown on the display. The packet carries no time
/// zone; conversion happens before building it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockTime {
    pub year: u16,
    /// 1–12
    pub month: u8,
    /// 1–31
    pub day: u8,
    /// 0–23
    pub hour: u8,
    /// 0–59
    pub minute: u8,
    /// 0–59
    pub second: u8,
    /// 0 = Sunday … 6 = Saturday
    pub weekday: u8,
}

impl ClockTime {
    fn validate(&self) -> Result<(), ProtocolError> {
        let checks: [(bool, &'static str); 6] = [
            ((1..=12).contains(&self.month), "month"),
            ((1..=31).contains(&self.day), "day"),
            (self.hour <= 23, "hour"),
            (self.minute <= 59, "minute"),
            (self.second <= 59, "second"),
            (self.weekday <= 6, "weekday"),
        ];
        match checks.iter().find(|(ok, _)| !ok) {
            Some((_, field)) => Err(ProtocolError::InvalidTime(field)),
            None => Ok(()),
        }
    }
}

/// Builds the 32-byte time packet (command `38`, frame kind `03`):
/// `00 01, year (BE), month, day, hour, minute, second, weekday`.
pub fn time_packet(time: &ClockTime) -> Result<[u8; FRAME_LEN], ProtocolError> {
    time.validate()?;
    let year = time.year.to_be_bytes();
    let payload = [
        0x00,
        0x01,
        year[0],
        year[1],
        time.month,
        time.day,
        time.hour,
        time.minute,
        time.second,
        time.weekday,
    ];
    frame(FrameKind::Time, TIME_COMMAND, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::crc16;

    fn sample() -> ClockTime {
        ClockTime {
            year: 2026,
            month: 10,
            day: 31,
            hour: 14,
            minute: 5,
            second: 9,
            weekday: 6,
        }
    }

    #[test]
    fn time_packet_matches_web_driver_layout() {
        let p = time_packet(&sample()).unwrap();

        // header as built by the web driver: 1c 03 00 00 00 0f
        assert_eq!(&p[..6], &[0x1C, 0x03, 0x00, 0x00, 0x00, 0x0F]);
        assert_eq!(&p[8..14], &[0xA5, 0x38, 0x00, 0x0A, 0x00, 0x01]);
        assert_eq!(&p[14..16], &[0x07, 0xEA]);
        assert_eq!(&p[16..22], &[10, 31, 14, 5, 9, 6]);

        // checksum: (sum of bytes 9..=21) ^ 0xFF
        let sum: u32 = p[9..22].iter().map(|&b| b as u32).sum();
        assert_eq!(p[22], (sum as u8) ^ 0xFF);
        assert!(p[23..].iter().all(|&b| b == 0));

        // CRC low byte first, like every other packet of the web driver
        let mut zeroed = p;
        zeroed[6] = 0;
        zeroed[7] = 0;
        let crc = crc16(&zeroed);
        assert_eq!((p[6], p[7]), ((crc & 0xFF) as u8, (crc >> 8) as u8));
    }

    #[test]
    fn time_packet_rejects_invalid_fields() {
        let mut t = sample();
        t.weekday = 7;
        assert_eq!(time_packet(&t), Err(ProtocolError::InvalidTime("weekday")));
        let mut t = sample();
        t.month = 0;
        assert_eq!(time_packet(&t), Err(ProtocolError::InvalidTime("month")));
    }
}
