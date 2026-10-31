use crate::checksum::apply_crc;
use crate::device::Target;

const VERSION_COMMAND: u8 = 0x11;

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

/// Extracts the ASCII firmware version from an input report.
///
/// Only answers starting with `0x11` count. Keyboard: bytes 8–14, screen:
/// bytes 8–13. Other answers (e.g. the screen's `1C … FB` report with a single
/// numeric byte that confuses the web driver) return `None`.
pub fn parse_version_response(target: Target, report: &[u8]) -> Option<String> {
    if report.first() != Some(&VERSION_COMMAND) {
        return None;
    }
    let range = match target {
        Target::Keyboard => 8..15,
        Target::Screen => 8..14,
    };
    let raw = report.get(range)?;
    let text: String = raw
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    let text = text.trim();
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return None;
    }
    Some(text.to_string())
}

/// Reduces a version string to its numeric part for comparisons,
/// e.g. `"V3.0.4"` → `"3.0.4"`. Returns `None` if no digits are found.
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

    fn report_with(text: &[u8], len: usize) -> Vec<u8> {
        let mut r = vec![0u8; len];
        r[0] = 0x11;
        r[8..8 + text.len()].copy_from_slice(text);
        r
    }

    #[test]
    fn parses_keyboard_version() {
        let r = report_with(b"V3.0.4 ", 32);
        assert_eq!(parse_version_response(Target::Keyboard, &r).as_deref(), Some("V3.0.4"));
    }

    #[test]
    fn parses_screen_version_six_chars() {
        let r = report_with(b"V1.2.1XX", 64);
        assert_eq!(parse_version_response(Target::Screen, &r).as_deref(), Some("V1.2.1"));
    }

    #[test]
    fn ignores_other_reports() {
        let mut r = vec![0u8; 64];
        r[0] = 0x1C;
        r[9] = 0xFB;
        r[21] = 3;
        assert_eq!(parse_version_response(Target::Screen, &r), None);
        assert_eq!(parse_version_response(Target::Screen, &report_with(b"", 64)), None);
        assert_eq!(parse_version_response(Target::Screen, &[0x11, 0x02]), None);
    }

    #[test]
    fn normalizes_versions() {
        assert_eq!(normalize_version("V3.0.4").as_deref(), Some("3.0.4"));
        assert_eq!(normalize_version(" v1.2.1 ").as_deref(), Some("1.2.1"));
        assert_eq!(normalize_version("1.2.1.").as_deref(), Some("1.2.1"));
        assert_eq!(normalize_version("abc"), None);
    }
}
