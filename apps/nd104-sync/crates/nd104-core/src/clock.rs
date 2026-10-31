//! Clock sources, time zones and DST handling (PLAN.md § 5.5).

use std::net::UdpSocket;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClockSource {
    /// PC local time.
    #[default]
    System,
    /// PC time converted to `timezone`.
    Timezone,
    /// Time from an NTP server, converted to `timezone` (or the PC zone).
    Ntp,
    /// PC local time plus `manual_offset_min`.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DstMode {
    /// From the time-zone rules.
    #[default]
    Auto,
    ForceSummer,
    ForceWinter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockSettings {
    pub source: ClockSource,
    /// IANA zone, e.g. `Europe/Berlin`. Empty = PC time zone.
    pub timezone: String,
    pub dst_mode: DstMode,
    pub ntp_server: String,
    pub manual_offset_min: i32,
    pub sync_interval_min: u32,
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            source: ClockSource::System,
            timezone: String::new(),
            dst_mode: DstMode::Auto,
            ntp_server: "pool.ntp.org".into(),
            manual_offset_min: 0,
            sync_interval_min: 60,
        }
    }
}

impl ClockSettings {
    fn zone(&self) -> Result<TimeZone> {
        match self.source {
            ClockSource::Timezone | ClockSource::Ntp if !self.timezone.trim().is_empty() => {
                TimeZone::get(self.timezone.trim())
                    .with_context(|| format!("unknown time zone {:?}", self.timezone))
            }
            _ => Ok(TimeZone::system()),
        }
    }

    /// Wall-clock time to show for the UTC instant `now`.
    ///
    /// For NTP the caller passes the NTP-corrected instant.
    pub fn local_time(&self, now: Timestamp) -> Result<DateTime> {
        let zone = self.zone()?;
        let mut now = now;
        if self.source == ClockSource::Manual {
            now = now
                .checked_add(SignedDuration::from_mins(self.manual_offset_min.into()))
                .context("manual offset out of range")?;
        }
        let info = zone.to_offset_info(now);
        let is_dst = info.dst().is_dst();
        let hour = SignedDuration::from_hours(1);
        let offset = match (self.dst_mode, is_dst) {
            (DstMode::ForceSummer, false) => shift(info.offset(), hour)?,
            (DstMode::ForceWinter, true) => shift(info.offset(), -hour)?,
            _ => info.offset(),
        };
        Ok(offset.to_datetime(now))
    }

    /// Next DST/offset change of the zone after `now` (only in `Auto` mode).
    pub fn next_transition(&self, now: Timestamp) -> Option<Timestamp> {
        if self.dst_mode != DstMode::Auto {
            return None;
        }
        let zone = self.zone().ok()?;
        zone.following(now).next().map(|t| t.timestamp())
    }
}

fn shift(offset: Offset, by: SignedDuration) -> Result<Offset> {
    let seconds = offset.seconds() + i32::try_from(by.as_secs()).unwrap_or(0);
    Offset::from_seconds(seconds).map_err(|e| anyhow!("offset out of range: {e}"))
}

/// Seconds between 1900-01-01 (NTP epoch) and 1970-01-01.
const NTP_UNIX_DELTA: i64 = 2_208_988_800;

/// Asks an NTP server (SNTP, RFC 4330) for the offset between its clock and
/// the PC clock. Blocking; call from a blocking context.
pub fn ntp_offset(server: &str, timeout: Duration) -> Result<SignedDuration> {
    let socket = UdpSocket::bind("0.0.0.0:0").context("cannot open UDP socket")?;
    socket.set_read_timeout(Some(timeout))?;
    let address = if server.contains(':') {
        server.to_string()
    } else {
        format!("{server}:123")
    };
    socket
        .connect(&address)
        .with_context(|| format!("cannot reach NTP server {server}"))?;

    let mut request = [0u8; 48];
    request[0] = 0x1B; // LI 0, version 3, mode 3 (client)
    let sent_at = Timestamp::now();
    let start = Instant::now();
    socket.send(&request)?;
    let mut answer = [0u8; 48];
    let n = socket
        .recv(&mut answer)
        .with_context(|| format!("no answer from NTP server {server}"))?;
    let round_trip = start.elapsed();

    let server_time = parse_ntp_answer(&answer[..n])?;
    let local_mid = sent_at
        .checked_add(SignedDuration::try_from(round_trip / 2).unwrap_or(SignedDuration::ZERO))?;
    Ok(server_time.duration_since(local_mid))
}

fn parse_ntp_answer(answer: &[u8]) -> Result<Timestamp> {
    if answer.len() < 48 {
        bail!("NTP answer too short");
    }
    let leap = answer[0] >> 6;
    let mode = answer[0] & 0x07;
    let stratum = answer[1];
    if mode != 4 || stratum == 0 || leap == 3 {
        bail!("NTP server not usable (mode {mode}, stratum {stratum}, leap {leap})");
    }
    let seconds = u32::from_be_bytes(answer[40..44].try_into()?) as i64;
    let fraction = u32::from_be_bytes(answer[44..48].try_into()?) as i64;
    let nanos = (fraction * 1_000_000_000) >> 32;
    Ok(Timestamp::new(seconds - NTP_UNIX_DELTA, nanos as i32)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn berlin(dst_mode: DstMode) -> ClockSettings {
        ClockSettings {
            source: ClockSource::Timezone,
            timezone: "Europe/Berlin".into(),
            dst_mode,
            ..Default::default()
        }
    }

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn timezone_with_auto_dst() {
        let s = berlin(DstMode::Auto);
        // summer: UTC+2, winter: UTC+1
        assert_eq!(
            s.local_time(ts("2026-07-01T10:00:00Z"))
                .unwrap()
                .to_string(),
            "2026-07-01T12:00:00"
        );
        assert_eq!(
            s.local_time(ts("2026-12-01T10:00:00Z"))
                .unwrap()
                .to_string(),
            "2026-12-01T11:00:00"
        );
    }

    #[test]
    fn forced_dst_modes() {
        assert_eq!(
            berlin(DstMode::ForceSummer)
                .local_time(ts("2026-12-01T10:00:00Z"))
                .unwrap()
                .to_string(),
            "2026-12-01T12:00:00"
        );
        assert_eq!(
            berlin(DstMode::ForceWinter)
                .local_time(ts("2026-07-01T10:00:00Z"))
                .unwrap()
                .to_string(),
            "2026-07-01T11:00:00"
        );
        // forcing what is already active changes nothing
        assert_eq!(
            berlin(DstMode::ForceWinter)
                .local_time(ts("2026-12-01T10:00:00Z"))
                .unwrap()
                .to_string(),
            "2026-12-01T11:00:00"
        );
    }

    #[test]
    fn manual_offset() {
        let s = ClockSettings {
            source: ClockSource::Manual,
            manual_offset_min: 5,
            ..Default::default()
        };
        let base = ClockSettings::default()
            .local_time(ts("2026-07-01T10:00:00Z"))
            .unwrap();
        let shifted = s.local_time(ts("2026-07-01T10:00:00Z")).unwrap();
        assert_eq!(
            shifted,
            base.checked_add(SignedDuration::from_mins(5)).unwrap()
        );
    }

    #[test]
    fn next_dst_transition() {
        let s = berlin(DstMode::Auto);
        // after the change on 2026-10-25, the next one is 2027-03-28 01:00 UTC
        assert_eq!(
            s.next_transition(ts("2026-10-31T12:00:00Z")),
            Some(ts("2027-03-28T01:00:00Z"))
        );
        assert_eq!(
            berlin(DstMode::ForceSummer).next_transition(ts("2026-10-31T12:00:00Z")),
            None
        );
    }

    #[test]
    fn unknown_zone_is_an_error() {
        let s = ClockSettings {
            source: ClockSource::Timezone,
            timezone: "Mars/Olympus".into(),
            ..Default::default()
        };
        assert!(s.local_time(Timestamp::now()).is_err());
    }

    #[test]
    fn ntp_answer_parsing() {
        let mut a = [0u8; 48];
        a[0] = 0x24; // LI 0, version 4, mode 4
        a[1] = 2; // stratum
                  // 2026-10-31T12:00:00Z = 1793448000 unix
        let ntp = (1_793_448_000i64 + NTP_UNIX_DELTA) as u32;
        a[40..44].copy_from_slice(&ntp.to_be_bytes());
        a[44..48].copy_from_slice(&0x8000_0000u32.to_be_bytes()); // .5 s
        assert_eq!(parse_ntp_answer(&a).unwrap(), ts("2026-10-31T12:00:00.5Z"));
        a[1] = 0; // kiss-of-death
        assert!(parse_ntp_answer(&a).is_err());
    }
}
