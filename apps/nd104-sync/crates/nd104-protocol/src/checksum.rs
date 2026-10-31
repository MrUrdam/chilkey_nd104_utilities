/// CRC-16/CCITT-FALSE (poly `0x1021`, init `0xFFFF`, no reflection, no final XOR).
///
/// Calculated over the whole packet while the CRC bytes (offset 6–7) are still 0.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// 8-bit payload checksum: `(sum of bytes) XOR 0xFF`, truncated to 8 bit.
///
/// Callers pass `packet[9..]` with the checksum byte itself still 0.
pub fn checksum8(data: &[u8]) -> u8 {
    let sum = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    sum ^ 0xFF
}

/// Writes the CRC of `packet` into offset 6 (low byte) and 7 (high byte).
pub(crate) fn apply_crc(packet: &mut [u8]) {
    packet[6] = 0;
    packet[7] = 0;
    let crc = crc16(packet);
    packet[6] = (crc & 0xFF) as u8;
    packet[7] = (crc >> 8) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_standard_check_value() {
        // CRC-16/CCITT-FALSE check value for "123456789".
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn crc16_matches_web_driver_screen_version_query() {
        // The web driver hard-codes FD 1B for `11 02` + 62 zero bytes.
        let mut packet = [0u8; 64];
        packet[0] = 0x11;
        packet[1] = 0x02;
        assert_eq!(crc16(&packet), 0x1BFD);
    }

    #[test]
    fn checksum8_wraps() {
        assert_eq!(checksum8(&[0x00]), 0xFF);
        assert_eq!(checksum8(&[0xFF, 0x01]), 0xFF);
        assert_eq!(checksum8(&[0x01]), 0xFE);
    }
}
