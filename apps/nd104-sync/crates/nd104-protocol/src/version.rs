use crate::checksum::{apply_crc, crc16};
use crate::device::Target;

const VERSION_COMMAND: u8 = 0x11;
const TEXT_OFFSET: usize = 8;

/// 32-byte firmware version query for the keyboard: `11 02 00 00 00 00 BC 60 …`.
pub fn keyboard_version_query() -> Vec<u8> {
    version_query(Target::Keyboard.report_len())
}

/// 64-byte firmware version query for the screen: `11 02 00 00 00 00 FD 1B …`.
pub fn screen_version_query() -> Vec<u8> {
    version_query(Target::Screen.report_len())
}

fn version_query(len: usize) -> Vec<u8> {
    let mut p = vec![0u8; len];
    p[0] = VERSION_COMMAND;
    p[1] = 0x02;
    apply_crc(&mut p);
    p
}

/// Firmware version as reported by a device, e.g. `"V3.0.14 20260710_123331"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareVersion {
    /// Full text as sent by the device.
    pub raw: String,
    /// First token, e.g. `"V3.0.14"`.
    pub version: String,
    /// Rest of the text (build date/time), e.g. `"20260710_123331"`.
    pub build: String,
}

/// Parses and validates a version answer.
///
/// Answer layout (verified on hardware, keyboard V3.0.14 / screen V1.2.4):
///
/// | Offset | Value                                   |
/// |--------|-----------------------------------------|
/// | 0–1    | `11 02`                                 |
/// | 5      | length of the version text              |
/// | 6–7    | CRC-16 of the whole report, low byte first |
/// | 8…     | ASCII version text                      |
///
/// Returns `None` if anything deviates (wrong command, wrong report length,
/// bad CRC, text not printable ASCII) – the caller treats that as a protocol
/// mismatch. Other answers, e.g. the screen's `1C … FB` report with a numeric
/// byte that confuses the web driver, are ignored this way as well.
pub fn parse_version_response(target: Target, report: &[u8]) -> Option<FirmwareVersion> {
    if report.len() != target.report_len() || report[0] != VERSION_COMMAND || report[1] != 0x02 {
        return None;
    }

    let mut zeroed = report.to_vec();
    zeroed[6] = 0;
    zeroed[7] = 0;
    let crc = crc16(&zeroed);
    if report[6] != (crc & 0xFF) as u8 || report[7] != (crc >> 8) as u8 {
        return None;
    }

    let len = report[5] as usize;
    let text = report.get(TEXT_OFFSET..TEXT_OFFSET + len)?;
    if len == 0 || !text.iter().all(|&b| b.is_ascii_graphic() || b == b' ') {
        return None;
    }
    let raw = String::from_utf8(text.to_vec()).ok()?.trim().to_string();
    let (version, build) = match raw.split_once(' ') {
        Some((v, b)) => (v.to_string(), b.trim().to_string()),
        None => (raw.clone(), String::new()),
    };
    Some(FirmwareVersion {
        raw,
        version,
        build,
    })
}

/// Reduces a version string to its numeric part for comparisons,
/// e.g. `"V3.0.14"` → `"3.0.14"`. Returns `None` if no digits are found.
pub fn normalize_version(version: &str) -> Option<String> {
    let start = version.find(|c: char| c.is_ascii_digit())?;
    let numeric: String = version[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let numeric = numeric.trim_end_matches('.');
    (!numeric.is_empty()).then(|| numeric.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect();
        v.resize(len, 0);
        v
    }

    /// Real answer of the keyboard (firmware V3.0.14, 2026-10-31).
    fn keyboard_answer() -> Vec<u8> {
        hex(
            "11020000001791f656332e302e31342032303236303731305f31323333333100",
            32,
        )
    }

    /// Real answer of the screen (firmware V1.2.4, 2026-10-31).
    fn screen_answer() -> Vec<u8> {
        hex(
            "11020000001be0d956312e322e34204d61722031372032303236203231\
             3a33373a3030",
            64,
        )
    }

    #[test]
    fn keyboard_query_bytes() {
        let q = keyboard_version_query();
        assert_eq!(q.len(), 32);
        assert_eq!(&q[..8], &[0x11, 0x02, 0, 0, 0, 0, 0xBC, 0x60]);
        assert!(q[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn screen_query_matches_web_driver_constant() {
        let q = screen_version_query();
        assert_eq!(q.len(), 64);
        assert_eq!(&q[..8], &[0x11, 0x02, 0, 0, 0, 0, 0xFD, 0x1B]);
        assert!(q[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn parses_real_keyboard_answer() {
        let v = parse_version_response(Target::Keyboard, &keyboard_answer()).unwrap();
        assert_eq!(v.raw, "V3.0.14 20260710_123331");
        assert_eq!(v.version, "V3.0.14");
        assert_eq!(v.build, "20260710_123331");
    }

    #[test]
    fn parses_real_screen_answer() {
        let v = parse_version_response(Target::Screen, &screen_answer()).unwrap();
        assert_eq!(v.raw, "V1.2.4 Mar 17 2026 21:37:00");
        assert_eq!(v.version, "V1.2.4");
        assert_eq!(v.build, "Mar 17 2026 21:37:00");
    }

    #[test]
    fn rejects_bad_crc() {
        let mut r = keyboard_answer();
        r[6] ^= 0x01;
        assert_eq!(parse_version_response(Target::Keyboard, &r), None);
    }

    #[test]
    fn rejects_wrong_report_length() {
        // a keyboard answer on the screen (64-byte) interface is a mismatch
        assert_eq!(
            parse_version_response(Target::Screen, &keyboard_answer()),
            None
        );
    }

    #[test]
    fn rejects_other_reports() {
        let mut r = vec![0u8; 64];
        r[0] = 0x1C;
        r[9] = 0xFB;
        r[21] = 3;
        assert_eq!(parse_version_response(Target::Screen, &r), None);
        assert_eq!(parse_version_response(Target::Screen, &[0x11, 0x02]), None);
    }

    #[test]
    fn normalizes_versions() {
        assert_eq!(normalize_version("V3.0.14").as_deref(), Some("3.0.14"));
        assert_eq!(normalize_version(" v1.2.4 ").as_deref(), Some("1.2.4"));
        assert_eq!(normalize_version("1.2.1.").as_deref(), Some("1.2.1"));
        assert_eq!(normalize_version("abc"), None);
    }
}
