use crate::checksum::{apply_crc, checksum8};
use crate::error::ProtocolError;

/// Length of a `1C` command frame.
pub const FRAME_LEN: usize = 32;

/// Largest payload that still leaves room for the checksum byte.
const MAX_PAYLOAD: usize = FRAME_LEN - 13;

/// Byte 1 of a `1C` frame. The web driver uses `02` for generic commands
/// (e.g. weather) and `03` for the time packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    Command = 0x02,
    Time = 0x03,
}

/// Builds a 32-byte `1C` frame (web driver function `J`).
///
/// | Offset       | Value                      |
/// |--------------|----------------------------|
/// | 0            | `1C`                       |
/// | 1            | kind (`02` / `03`)         |
/// | 5            | `payload.len() + 5`        |
/// | 6–7          | CRC-16, low byte first     |
/// | 8            | `A5`                       |
/// | 9            | command                    |
/// | 10           | `00`                       |
/// | 11           | `payload.len()`            |
/// | 12…          | payload                    |
/// | 12 + len     | 8-bit checksum of 9..end   |
pub fn frame(
    kind: FrameKind,
    command: u8,
    payload: &[u8],
) -> Result<[u8; FRAME_LEN], ProtocolError> {
    let len = payload.len();
    if len > MAX_PAYLOAD {
        return Err(ProtocolError::PayloadTooLong(len));
    }

    let mut p = [0u8; FRAME_LEN];
    p[0] = 0x1C;
    p[1] = kind as u8;
    p[5] = (len + 5) as u8;
    p[8] = 0xA5;
    p[9] = command;
    p[10] = 0x00;
    p[11] = len as u8;
    p[12..12 + len].copy_from_slice(payload);
    p[12 + len] = checksum8(&p[9..]);
    apply_crc(&mut p);
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::crc16;

    #[test]
    fn frame_layout() {
        let p = frame(FrameKind::Command, 0x39, &[0x00, 0x02]).unwrap();
        assert_eq!(&p[..2], &[0x1C, 0x02]);
        assert_eq!(p[5], 7);
        assert_eq!(&p[8..14], &[0xA5, 0x39, 0x00, 0x02, 0x00, 0x02]);
        // checksum: 0x39 + 0x02 + 0x02 = 0x3D -> 0x3D ^ 0xFF = 0xC2
        assert_eq!(p[14], 0xC2);
        assert!(p[15..].iter().all(|&b| b == 0));
    }

    #[test]
    fn frame_crc_is_low_byte_first() {
        let p = frame(FrameKind::Command, 0x39, &[0x00, 0x02]).unwrap();
        let mut zeroed = p;
        zeroed[6] = 0;
        zeroed[7] = 0;
        let crc = crc16(&zeroed);
        assert_eq!(p[6], (crc & 0xFF) as u8);
        assert_eq!(p[7], (crc >> 8) as u8);
    }

    #[test]
    fn frame_rejects_long_payload() {
        assert!(frame(FrameKind::Command, 0x01, &[0u8; 19]).is_ok());
        assert_eq!(
            frame(FrameKind::Command, 0x01, &[0u8; 20]),
            Err(ProtocolError::PayloadTooLong(20))
        );
    }
}
