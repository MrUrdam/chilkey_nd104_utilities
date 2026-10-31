//! Chilkey ND104 – the first keyboard model. Verified in M0 (docs/protocol.md).

use anyhow::Context;
use jiff::civil::DateTime;
use nd104_protocol::{
    checksum8, crc16, keyboard_version_query, normalize_version, parse_version_response,
    screen_version_query, time_packet, weather_packet, ClockTime, Target, Weather, WeatherIcon,
    KEYBOARD, SCREEN, VENDOR_USAGE, VENDOR_USAGE_PAGE,
};

use super::{
    Capabilities, FirmwareInfo, InterfaceSpec, KeyboardModel, Packet, ReportClass, Role,
    ScreenVariant, VerifiedFirmware,
};
use crate::weather::{Condition, WeatherReport};

const TIME_COMMAND: u8 = 0x38;
const WEATHER_COMMAND: u8 = 0xFE;
const STATUS_OK: u8 = 0xFE;

static INTERFACES: [InterfaceSpec; 2] = [
    InterfaceSpec {
        role: Role::Keyboard,
        vid: KEYBOARD.vid,
        pid: KEYBOARD.pid,
        usage_page: VENDOR_USAGE_PAGE,
        usage: VENDOR_USAGE,
        report_len: 32,
    },
    InterfaceSpec {
        role: Role::Screen,
        vid: SCREEN.vid,
        pid: SCREEN.pid,
        usage_page: VENDOR_USAGE_PAGE,
        usage: VENDOR_USAGE,
        report_len: 64,
    },
];

static VERIFIED: [VerifiedFirmware; 2] = [
    VerifiedFirmware {
        role: Role::Keyboard,
        version: "3.0.14",
    },
    VerifiedFirmware {
        role: Role::Screen,
        version: "1.2.4",
    },
];

/// Chilkey ND104 (keyboard `6D67:016C`, screen `5542:0001`).
pub struct Nd104;

impl Nd104 {
    /// Time and weather go through the keyboard, like the web driver does.
    const DATA_ROUTE: Role = Role::Keyboard;

    fn icon(weather: &WeatherReport) -> WeatherIcon {
        match (weather.condition, weather.is_day) {
            (Condition::Clear, true) => WeatherIcon::SUNNY,
            (Condition::Clear, false) => WeatherIcon::CLEAR_NIGHT,
            (Condition::PartlyCloudy, true) => WeatherIcon::PARTLY_CLOUDY,
            (Condition::PartlyCloudy, false) => WeatherIcon::PARTLY_CLOUDY_NIGHT,
            (Condition::Cloudy, _) => WeatherIcon::CLOUDY,
            (Condition::Overcast, _) => WeatherIcon::OVERCAST,
            (Condition::Fog, _) => WeatherIcon::FOG,
            (Condition::Drizzle | Condition::Rain, _) => WeatherIcon::RAIN,
            (Condition::Snow, _) => WeatherIcon::SNOW,
            (Condition::Thunderstorm, _) => WeatherIcon::THUNDERSTORM,
        }
    }

    fn firmware(role: Role, report: &[u8]) -> ReportClass {
        let target = match role {
            Role::Keyboard => Target::Keyboard,
            Role::Screen => Target::Screen,
        };
        match parse_version_response(target, report) {
            Some(v) => ReportClass::Firmware(FirmwareInfo {
                role,
                version: normalize_version(&v.version).unwrap_or_else(|| v.version.clone()),
                raw: v.raw,
                build: v.build,
            }),
            None => ReportClass::Malformed(format!("invalid {role} version answer")),
        }
    }

    /// `1C` frame from the keyboard: receipt or a relayed `A5` answer.
    fn classify_keyboard_frame(report: &[u8]) -> ReportClass {
        let mut zeroed = report.to_vec();
        zeroed[6] = 0;
        zeroed[7] = 0;
        let crc = crc16(&zeroed);
        if report[6] != (crc & 0xFF) as u8 || report[7] != (crc >> 8) as u8 {
            return ReportClass::Malformed("keyboard frame with bad CRC".into());
        }
        if report[1] == 0x01 && report[2..6].iter().all(|&b| b == 0) {
            return ReportClass::Receipt;
        }
        if report[8] != 0xA5 {
            return ReportClass::Malformed("keyboard frame without A5 magic".into());
        }
        let command = report[9];
        let len = report[11] as usize;
        let end = 12 + len;
        if report[10] != 0 || report[5] as usize != len + 5 || end >= report.len() {
            return ReportClass::Malformed("keyboard frame with inconsistent length".into());
        }
        let mut payload_zeroed = report.to_vec();
        payload_zeroed[end] = 0;
        if checksum8(&payload_zeroed[9..]) != report[end] {
            return ReportClass::Malformed("keyboard frame with bad checksum".into());
        }
        Self::answer(command, &report[12..end])
    }

    /// `A5` answer straight from the screen.
    fn classify_screen_answer(report: &[u8]) -> ReportClass {
        let command = report[1];
        let len = report[3] as usize;
        let end = 4 + len;
        if report[2] != 0 || end >= report.len() {
            return ReportClass::Malformed("screen answer with inconsistent length".into());
        }
        if checksum8(&report[1..end]) != report[end] {
            return ReportClass::Malformed("screen answer with bad checksum".into());
        }
        Self::answer(command, &report[4..end])
    }

    fn answer(command: u8, payload: &[u8]) -> ReportClass {
        match payload {
            [status, 0x00] => ReportClass::Ack {
                command,
                ok: *status == STATUS_OK,
            },
            _ => ReportClass::Ignored(format!(
                "unknown answer for command 0x{command:02X} ({} bytes)",
                payload.len()
            )),
        }
    }
}

impl KeyboardModel for Nd104 {
    fn id(&self) -> &'static str {
        "chilkey-nd104"
    }

    fn display_name(&self) -> &'static str {
        "Chilkey ND104"
    }

    fn interfaces(&self) -> &'static [InterfaceSpec] {
        &INTERFACES
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            time: true,
            weather: true,
            weather_icons: WeatherIcon::MAX_INDEX + 1,
            screen_width: 320,
            screen_height: 172,
        }
    }

    fn firmware_queries(&self) -> Vec<Packet> {
        vec![
            Packet {
                role: Role::Keyboard,
                command: 0x11,
                bytes: keyboard_version_query(),
            },
            Packet {
                role: Role::Screen,
                command: 0x11,
                bytes: screen_version_query(),
            },
        ]
    }

    fn verified_firmware(&self) -> &'static [VerifiedFirmware] {
        &VERIFIED
    }

    fn screen_variant(&self, firmware: &[FirmwareInfo]) -> ScreenVariant {
        // Only builds we have seen are recognised (docs/firmware.md).
        let screen = firmware.iter().find(|f| f.role == Role::Screen);
        match screen.map(|f| f.build.as_str()) {
            Some(build) if build.starts_with("Mar 17 2026") => ScreenVariant::Celsius24h,
            _ => ScreenVariant::Unknown,
        }
    }

    fn encode_time(&self, time: &DateTime) -> anyhow::Result<Packet> {
        let clock = ClockTime {
            year: u16::try_from(time.year()).context("year out of range")?,
            month: time.month() as u8,
            day: time.day() as u8,
            hour: time.hour() as u8,
            minute: time.minute() as u8,
            second: time.second() as u8,
            weekday: time.weekday().to_sunday_zero_offset() as u8,
        };
        Ok(Packet {
            role: Self::DATA_ROUTE,
            command: TIME_COMMAND,
            bytes: time_packet(&clock)?.to_vec(),
        })
    }

    fn encode_weather(&self, weather: &WeatherReport) -> anyhow::Result<Packet> {
        let packet = weather_packet(&Weather {
            icon: Self::icon(weather),
            current_c: weather.current_c,
            max_c: weather.max_c,
            min_c: weather.min_c,
        })?;
        Ok(Packet {
            role: Self::DATA_ROUTE,
            command: WEATHER_COMMAND,
            bytes: packet.to_vec(),
        })
    }

    fn classify_report(&self, role: Role, report: &[u8]) -> ReportClass {
        let expected = match role {
            Role::Keyboard => 32,
            Role::Screen => 64,
        };
        if report.len() != expected {
            return ReportClass::Malformed(format!(
                "{role} report has {} bytes, expected {expected}",
                report.len()
            ));
        }
        match (role, report[0]) {
            (_, 0x11) => Self::firmware(role, report),
            (Role::Keyboard, 0x1C) => Self::classify_keyboard_frame(report),
            (Role::Screen, 0xA5) => Self::classify_screen_answer(report),
            (Role::Screen, 0x1C) if report[9] == 0xFB => {
                ReportClass::Ignored("screen info report (1C … FB)".into())
            }
            (_, first) => {
                ReportClass::Malformed(format!("unknown {role} report starting with 0x{first:02X}"))
            }
        }
    }

    fn manifest_key(&self) -> &'static str {
        "nd104"
    }

    fn firmware_build_date(&self, firmware: &FirmwareInfo) -> Option<jiff::civil::Date> {
        build_date(&firmware.build)
    }

    fn manifest_part(&self, role: Role, variant: ScreenVariant) -> Option<&'static str> {
        match (role, variant) {
            (Role::Keyboard, _) => Some("keyboard"),
            (Role::Screen, ScreenVariant::Celsius24h) => Some("screen_24h_c"),
            (Role::Screen, ScreenVariant::Fahrenheit12h) => Some("screen_12h_f"),
            (Role::Screen, ScreenVariant::Unknown) => None,
        }
    }
}

/// Build date from the version answer: keyboard `20260710_123331`,
/// screen `Mar 17 2026 21:37:00` (C `__DATE__` format).
fn build_date(build: &str) -> Option<jiff::civil::Date> {
    let digits: String = build.chars().take(8).collect();
    if digits.len() == 8 && digits.chars().all(|c| c.is_ascii_digit()) {
        let (y, m, d) = (&digits[..4], &digits[4..6], &digits[6..]);
        return jiff::civil::Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok();
    }
    let mut parts = build.split_whitespace();
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let day: i8 = parts.next()?.parse().ok()?;
    let year: i16 = parts.next()?.parse().ok()?;
    jiff::civil::Date::new(year, month, day).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str, len: usize) -> Vec<u8> {
        let s: String = s.split_whitespace().collect();
        let mut v: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect();
        v.resize(len, 0);
        v
    }

    fn report(condition: Condition, is_day: bool) -> WeatherReport {
        WeatherReport {
            condition,
            is_day,
            current_c: 1.0,
            max_c: 2.0,
            min_c: -1.0,
            observed_at: jiff::Timestamp::UNIX_EPOCH,
            provider: "test".into(),
        }
    }

    // Answers recorded on real hardware in M0 (keyboard V3.0.14, screen V1.2.4).

    #[test]
    fn keyboard_receipt() {
        let r = hex("1c 01 00 00 00 00 eb 24", 32);
        assert_eq!(
            Nd104.classify_report(Role::Keyboard, &r),
            ReportClass::Receipt
        );
    }

    #[test]
    fn relayed_acks() {
        let time = hex("1c 03 00 00 00 07 90 7e a5 38 00 02 fe 00 c7", 32);
        let weather = hex("1c 02 00 00 00 07 8f f0 a5 fe 00 02 fe 00 01", 32);
        assert_eq!(
            Nd104.classify_report(Role::Keyboard, &time),
            ReportClass::Ack {
                command: 0x38,
                ok: true
            }
        );
        assert_eq!(
            Nd104.classify_report(Role::Keyboard, &weather),
            ReportClass::Ack {
                command: 0xFE,
                ok: true
            }
        );
    }

    #[test]
    fn unsolicited_0x35_is_a_wellformed_ack() {
        let r = hex("1c 03 00 00 00 07 54 76 a5 35 00 02 fe 00 ca", 32);
        assert_eq!(
            Nd104.classify_report(Role::Keyboard, &r),
            ReportClass::Ack {
                command: 0x35,
                ok: true
            }
        );
    }

    #[test]
    fn direct_screen_ack() {
        let r = hex("a5 38 00 02 fe 00 c7", 64);
        assert_eq!(
            Nd104.classify_report(Role::Screen, &r),
            ReportClass::Ack {
                command: 0x38,
                ok: true
            }
        );
    }

    #[test]
    fn real_firmware_answers() {
        let kb = hex(
            "11020000001791f656332e302e31342032303236303731305f31323333333100",
            32,
        );
        match Nd104.classify_report(Role::Keyboard, &kb) {
            ReportClass::Firmware(f) => {
                assert_eq!(f.version, "3.0.14");
                assert_eq!(f.build, "20260710_123331");
            }
            other => panic!("unexpected {other:?}"),
        }
        let sc = hex(
            "11020000001be0d956312e322e34204d61722031372032303236203231 3a33373a3030",
            64,
        );
        let ReportClass::Firmware(screen) = Nd104.classify_report(Role::Screen, &sc) else {
            panic!("screen answer not recognised");
        };
        assert_eq!(screen.version, "1.2.4");
        assert_eq!(
            Nd104.screen_variant(std::slice::from_ref(&screen)),
            ScreenVariant::Celsius24h
        );
    }

    #[test]
    fn damaged_reports_are_malformed() {
        let mut r = hex("1c 03 00 00 00 07 90 7e a5 38 00 02 fe 00 c7", 32);
        r[14] ^= 1; // checksum
        assert!(matches!(
            Nd104.classify_report(Role::Keyboard, &r),
            ReportClass::Malformed(_)
        ));
        let mut r = hex("1c 03 00 00 00 07 90 7e a5 38 00 02 fe 00 c7", 32);
        r[6] ^= 1; // CRC
        assert!(matches!(
            Nd104.classify_report(Role::Keyboard, &r),
            ReportClass::Malformed(_)
        ));
        let r = hex("42", 32);
        assert!(matches!(
            Nd104.classify_report(Role::Keyboard, &r),
            ReportClass::Malformed(_)
        ));
        let r = hex("a5 38 00 02 fe 00 c7", 32); // wrong length for screen
        assert!(matches!(
            Nd104.classify_report(Role::Screen, &r),
            ReportClass::Malformed(_)
        ));
    }

    #[test]
    fn non_ok_status() {
        // same frame with status 0x01 and recalculated checksums
        let mut r = vec![0u8; 32];
        r[..6].copy_from_slice(&[0x1C, 0x03, 0, 0, 0, 0x07]);
        r[8..14].copy_from_slice(&[0xA5, 0x38, 0x00, 0x02, 0x01, 0x00]);
        r[14] = checksum8(&r[9..]);
        let crc = crc16(&r);
        r[6] = (crc & 0xFF) as u8;
        r[7] = (crc >> 8) as u8;
        assert_eq!(
            Nd104.classify_report(Role::Keyboard, &r),
            ReportClass::Ack {
                command: 0x38,
                ok: false
            }
        );
    }

    #[test]
    fn build_dates() {
        let d = |s: &str| build_date(s).map(|d| d.to_string());
        assert_eq!(d("20260710_123331").as_deref(), Some("2026-07-10"));
        assert_eq!(d("Mar 17 2026 21:37:00").as_deref(), Some("2026-03-17"));
        assert_eq!(d("Mar  5 2026 10:00:00").as_deref(), Some("2026-03-05"));
        assert_eq!(d("garbage"), None);
        assert_eq!(d(""), None);
    }

    #[test]
    fn icon_mapping_uses_all_ten_icons() {
        let cases = [
            (Condition::Clear, true, 0),
            (Condition::Overcast, true, 1),
            (Condition::PartlyCloudy, true, 2),
            (Condition::Cloudy, false, 3),
            (Condition::Rain, true, 4),
            (Condition::Drizzle, false, 4),
            (Condition::Snow, true, 5),
            (Condition::Clear, false, 6),
            (Condition::PartlyCloudy, false, 7),
            (Condition::Fog, true, 8),
            (Condition::Thunderstorm, true, 9),
        ];
        for (condition, is_day, icon) in cases {
            let p = Nd104.encode_weather(&report(condition, is_day)).unwrap();
            assert_eq!(p.bytes[13], icon, "{condition:?} day={is_day}");
            assert_eq!(p.role, Role::Keyboard);
            assert_eq!(p.command, 0xFE);
        }
    }

    #[test]
    fn encode_time_uses_sunday_zero_weekday() {
        let t: DateTime = "2026-10-31T14:05:09".parse().unwrap(); // Saturday
        let p = Nd104.encode_time(&t).unwrap();
        assert_eq!(&p.bytes[14..22], &[0x07, 0xEA, 10, 31, 14, 5, 9, 6]);
        assert_eq!(p.command, 0x38);
    }
}
