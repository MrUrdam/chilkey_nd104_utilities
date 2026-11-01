//! Own picture / GIF on the screen (docs/protocol.md § 4.5).
//!
//! The web driver sends a payload straight to the screen interface (64-byte
//! reports, no `1C` frame):
//!
//! ```text
//! payload = 55 42 | width u16 BE | height u16 BE | 00 | frames u8
//!         | delay per frame (u8, 10 ms units; 1 for a still picture)
//!         | size of each frame (u32 BE)
//!         | frames (each a baseline JPEG of width × height)
//! start   = BA 06 AA B0 | payload length u32 BE            (one report)
//! data    = BA n | n payload bytes (n ≤ 62)                 (one report each)
//! ```
//!
//! The data reports are grouped in pages of 4096 payload bytes (67 reports).
//! After the start report and after each page the screen answers
//! `BA ?? A5 B0 ?? 02 FE` (send the next page) or `… 02 FD` (send the last
//! page again). After the last page the screen stays silent.

pub const PICTURE_WIDTH: u16 = 320;
pub const PICTURE_HEIGHT: u16 = 172;
/// The web driver asks for fewer than 60 frames.
pub const PICTURE_MAX_FRAMES: usize = 59;
/// The web driver accepts files up to 2 MB; the payload is limited the same.
pub const PICTURE_MAX_BYTES: usize = 2 * 1024 * 1024;

const PAGE_BYTES: usize = 4096;
const CHUNK: usize = 62;
pub const PICTURE_REPORT_LEN: usize = 64;

/// One encoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureFrame {
    pub jpeg: Vec<u8>,
    /// Display time in 10 ms units (1–255).
    pub delay: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PictureError {
    NoFrames,
    TooManyFrames(usize),
    TooLarge(usize),
}

impl std::error::Error for PictureError {}

impl std::fmt::Display for PictureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PictureError::NoFrames => write!(f, "picture has no frames"),
            PictureError::TooManyFrames(n) => {
                write!(f, "{n} frames – at most {PICTURE_MAX_FRAMES} are allowed")
            }
            PictureError::TooLarge(n) => write!(
                f,
                "{} KB after conversion – at most {} KB are allowed",
                n / 1024,
                PICTURE_MAX_BYTES / 1024
            ),
        }
    }
}

/// Builds the payload (header, frame sizes, frames).
pub fn picture_payload(frames: &[PictureFrame]) -> Result<Vec<u8>, PictureError> {
    if frames.is_empty() {
        return Err(PictureError::NoFrames);
    }
    if frames.len() > PICTURE_MAX_FRAMES {
        return Err(PictureError::TooManyFrames(frames.len()));
    }
    let mut payload = vec![0x55, 0x42];
    payload.extend_from_slice(&PICTURE_WIDTH.to_be_bytes());
    payload.extend_from_slice(&PICTURE_HEIGHT.to_be_bytes());
    payload.push(0);
    payload.push(frames.len() as u8);
    if frames.len() == 1 {
        payload.push(1);
    } else {
        payload.extend(frames.iter().map(|f| f.delay.max(1)));
    }
    for frame in frames {
        payload.extend_from_slice(&(frame.jpeg.len() as u32).to_be_bytes());
    }
    for frame in frames {
        payload.extend_from_slice(&frame.jpeg);
    }
    if payload.len() > PICTURE_MAX_BYTES {
        return Err(PictureError::TooLarge(payload.len()));
    }
    Ok(payload)
}

/// Start report `BA 06 AA B0 len`.
pub fn picture_start(payload_len: usize) -> [u8; PICTURE_REPORT_LEN] {
    let mut report = [0u8; PICTURE_REPORT_LEN];
    report[..4].copy_from_slice(&[0xBA, 0x06, 0xAA, 0xB0]);
    report[4..8].copy_from_slice(&(payload_len as u32).to_be_bytes());
    report
}

/// Data reports, grouped in pages.
pub fn picture_pages(payload: &[u8]) -> Vec<Vec<[u8; PICTURE_REPORT_LEN]>> {
    payload
        .chunks(PAGE_BYTES)
        .map(|page| {
            page.chunks(CHUNK)
                .map(|chunk| {
                    let mut report = [0u8; PICTURE_REPORT_LEN];
                    report[0] = 0xBA;
                    report[1] = chunk.len() as u8;
                    report[2..2 + chunk.len()].copy_from_slice(chunk);
                    report
                })
                .collect()
        })
        .collect()
}

/// The screen's flow-control answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureAck {
    /// Send the next page.
    Next,
    /// Send the last page again.
    Resend,
}

/// `BA ?? A5 B0 ?? 02 FE|FD`; `None` for anything else.
pub fn parse_picture_ack(report: &[u8]) -> Option<PictureAck> {
    if report.len() < 7
        || report[0] != 0xBA
        || report[2] != 0xA5
        || report[3] != 0xB0
        || report[5] != 2
    {
        return None;
    }
    match report[6] {
        0xFE => Some(PictureAck::Next),
        0xFD => Some(PictureAck::Resend),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(len: usize, delay: u8) -> PictureFrame {
        PictureFrame {
            jpeg: vec![0xAB; len],
            delay,
        }
    }

    #[test]
    fn still_picture_payload() {
        let p = picture_payload(&[frame(10, 7)]).unwrap();
        assert_eq!(&p[..9], &[0x55, 0x42, 0x01, 0x40, 0x00, 0xAC, 0x00, 1, 1]);
        assert_eq!(&p[9..13], &[0, 0, 0, 10]);
        assert_eq!(p.len(), 13 + 10);
    }

    #[test]
    fn animation_payload_has_delays_and_sizes() {
        let p = picture_payload(&[frame(3, 5), frame(4, 0)]).unwrap();
        assert_eq!(&p[7..10], &[2, 5, 1]);
        assert_eq!(&p[10..18], &[0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(p.len(), 18 + 7);
    }

    #[test]
    fn limits() {
        assert_eq!(picture_payload(&[]), Err(PictureError::NoFrames));
        let many: Vec<_> = (0..60).map(|_| frame(1, 1)).collect();
        assert_eq!(picture_payload(&many), Err(PictureError::TooManyFrames(60)));
        assert!(matches!(
            picture_payload(&[frame(PICTURE_MAX_BYTES, 1)]),
            Err(PictureError::TooLarge(_))
        ));
    }

    #[test]
    fn pages_and_reports() {
        let payload: Vec<u8> = (0..(4096 + 100)).map(|i| i as u8).collect();
        let pages = picture_pages(&payload);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].len(), 67);
        assert_eq!(pages[0][66][1], 4); // 66 × 62 + 4 = 4096
        assert_eq!(pages[1].len(), 2);
        assert_eq!(pages[1][0][..4], [0xBA, 62, 0, 1]); // byte 4096 = 0x00, 4097 = 0x01
        let start = picture_start(payload.len());
        assert_eq!(start[..8], [0xBA, 6, 0xAA, 0xB0, 0, 0, 0x10, 0x64]);
    }

    #[test]
    fn acks() {
        let mut r = [0u8; 64];
        r[..7].copy_from_slice(&[0xBA, 5, 0xA5, 0xB0, 0, 2, 0xFE]);
        assert_eq!(parse_picture_ack(&r), Some(PictureAck::Next));
        r[6] = 0xFD;
        assert_eq!(parse_picture_ack(&r), Some(PictureAck::Resend));
        r[3] = 0xFC;
        assert_eq!(parse_picture_ack(&r), None);
    }
}
