//! Keymap read/write (docs/protocol.md § 5.4), VIA "dynamic keymap"
//! commands on the keyboard's vendor interface (32-byte reports, no `1C`
//! frame). Every command is answered with an echo of the request; reads
//! carry the data in the echo.
//!
//! | Bytes | Meaning |
//! |---|---|
//! | `12 off_hi off_lo size` | read `size` bytes of the keymap buffer (answer: data from byte 4) |
//! | `05 layer row col kc_hi kc_lo` | set one key |
//! | `14 layer 00 dir` | read the knob (answer: `kc_hi kc_lo` at bytes 4–5) |
//! | `15 layer 00 dir kc_hi kc_lo` | set the knob |
//!
//! The buffer holds [`KEYMAP_LAYERS`] layers of [`KEYMAP_ROWS`] ×
//! [`KEYMAP_COLS`] keys, two bytes (big-endian) per key, so key `index =
//! row * 16 + col` of `layer` sits at offset `(layer * 128 + index) * 2`.

pub const KEYMAP_GET_BUFFER: u8 = 0x12;
pub const KEYMAP_SET_KEY: u8 = 0x05;
pub const KNOB_GET: u8 = 0x14;
pub const KNOB_SET: u8 = 0x15;

pub const KEYMAP_LAYERS: usize = 6;
pub const KEYMAP_ROWS: usize = 8;
pub const KEYMAP_COLS: usize = 16;
pub const KEYS_PER_LAYER: usize = KEYMAP_ROWS * KEYMAP_COLS;
/// Layers whose knob actions the web driver reads and sets.
pub const KNOB_LAYERS: usize = 4;
/// Turning left (0) and right (1).
pub const KNOB_DIRECTIONS: usize = 2;

const LAYER_BYTES: usize = KEYS_PER_LAYER * 2;
/// Bytes per read, like the web driver (fits a 32-byte report).
const CHUNK: usize = 28;

/// `12 off_hi off_lo size`.
pub fn keymap_read(offset: u16, size: u8) -> Vec<u8> {
    let [hi, lo] = offset.to_be_bytes();
    vec![KEYMAP_GET_BUFFER, hi, lo, size]
}

/// All reads for the whole keymap, never crossing a layer boundary (like
/// the web driver): `(offset, size)`.
pub fn keymap_read_plan() -> Vec<(u16, u8)> {
    let mut plan = Vec::new();
    for layer in 0..KEYMAP_LAYERS {
        let start = layer * LAYER_BYTES;
        let mut at = 0;
        while at < LAYER_BYTES {
            let size = CHUNK.min(LAYER_BYTES - at);
            plan.push(((start + at) as u16, size as u8));
            at += size;
        }
    }
    plan
}

/// `05 layer row col kc_hi kc_lo`.
pub fn keymap_set_key(layer: u8, row: u8, col: u8, code: u16) -> Vec<u8> {
    let [hi, lo] = code.to_be_bytes();
    vec![KEYMAP_SET_KEY, layer, row, col, hi, lo]
}

/// `14 layer 00 dir`.
pub fn knob_get(layer: u8, direction: u8) -> Vec<u8> {
    vec![KNOB_GET, layer, 0, direction]
}

/// `15 layer 00 dir kc_hi kc_lo`.
pub fn knob_set(layer: u8, direction: u8, code: u16) -> Vec<u8> {
    let [hi, lo] = code.to_be_bytes();
    vec![KNOB_SET, layer, 0, direction, hi, lo]
}

/// An answer to one of the keymap commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapAnswer {
    /// Part of the keymap buffer.
    Buffer { offset: u16, data: Vec<u8> },
    /// Echo of a key write.
    KeySet {
        layer: u8,
        row: u8,
        col: u8,
        code: u16,
    },
    /// A knob action (read answer or write echo).
    Knob {
        set: bool,
        layer: u8,
        direction: u8,
        code: u16,
    },
}

/// Parses a keymap answer; `None` if the report is not one or its fields
/// are out of range.
pub fn parse_keymap_answer(report: &[u8]) -> Option<KeymapAnswer> {
    let byte = |i: usize| report.get(i).copied();
    let word = |i: usize| Some(u16::from_be_bytes([byte(i)?, byte(i + 1)?]));
    match byte(0)? {
        KEYMAP_GET_BUFFER => {
            let offset = word(1)?;
            let size = byte(3)? as usize;
            if size == 0 || size > CHUNK || offset as usize + size > KEYMAP_LAYERS * LAYER_BYTES {
                return None;
            }
            Some(KeymapAnswer::Buffer {
                offset,
                data: report.get(4..4 + size)?.to_vec(),
            })
        }
        KEYMAP_SET_KEY => {
            let (layer, row, col) = (byte(1)?, byte(2)?, byte(3)?);
            if layer as usize >= KEYMAP_LAYERS
                || row as usize >= KEYMAP_ROWS
                || col as usize >= KEYMAP_COLS
            {
                return None;
            }
            Some(KeymapAnswer::KeySet {
                layer,
                row,
                col,
                code: word(4)?,
            })
        }
        command @ (KNOB_GET | KNOB_SET) => {
            let (layer, direction) = (byte(1)?, byte(3)?);
            if layer as usize >= KEYMAP_LAYERS
                || direction as usize >= KNOB_DIRECTIONS
                || byte(2)? != 0
            {
                return None;
            }
            Some(KeymapAnswer::Knob {
                set: command == KNOB_SET,
                layer,
                direction,
                code: word(4)?,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_commands_like_the_web_driver() {
        assert_eq!(keymap_read(0x0118, 28), [0x12, 0x01, 0x18, 28]);
        assert_eq!(keymap_set_key(1, 2, 3, 0x7E40), [5, 1, 2, 3, 0x7E, 0x40]);
        assert_eq!(knob_get(2, 1), [0x14, 2, 0, 1]);
        assert_eq!(knob_set(0, 0, 0x00EA), [0x15, 0, 0, 0, 0x00, 0xEA]);
    }

    #[test]
    fn read_plan_covers_every_layer_without_crossing() {
        let plan = keymap_read_plan();
        assert_eq!(plan.len(), KEYMAP_LAYERS * 10);
        let mut covered = vec![0u8; KEYMAP_LAYERS * LAYER_BYTES];
        for (offset, size) in &plan {
            let (start, end) = (*offset as usize, *offset as usize + *size as usize);
            assert_eq!(
                start / LAYER_BYTES,
                (end - 1) / LAYER_BYTES,
                "crosses a layer: {offset}"
            );
            covered[start..end].iter_mut().for_each(|c| *c += 1);
        }
        assert!(covered.iter().all(|&c| c == 1));
        assert_eq!(plan[9], (252, 4));
    }

    #[test]
    fn parses_answers() {
        let mut r = [0u8; 32];
        r[..8].copy_from_slice(&[0x12, 0x01, 0x00, 4, 0x00, 0x29, 0x00, 0x04]);
        assert_eq!(
            parse_keymap_answer(&r),
            Some(KeymapAnswer::Buffer {
                offset: 0x100,
                data: vec![0x00, 0x29, 0x00, 0x04]
            })
        );
        r[..6].copy_from_slice(&[0x14, 3, 0, 1, 0x00, 0xE9]);
        assert_eq!(
            parse_keymap_answer(&r),
            Some(KeymapAnswer::Knob {
                set: false,
                layer: 3,
                direction: 1,
                code: 0xE9
            })
        );
        r[..6].copy_from_slice(&[0x05, 0, 7, 15, 0x00, 0x04]);
        assert_eq!(
            parse_keymap_answer(&r),
            Some(KeymapAnswer::KeySet {
                layer: 0,
                row: 7,
                col: 15,
                code: 4
            })
        );
        // out of range
        r[..6].copy_from_slice(&[0x05, 6, 0, 0, 0, 0]);
        assert_eq!(parse_keymap_answer(&r), None);
        r[..4].copy_from_slice(&[0x12, 0x06, 0x00, 4]);
        assert_eq!(parse_keymap_answer(&r), None);
        r[..4].copy_from_slice(&[0x12, 0x00, 0x00, 29]);
        assert_eq!(parse_keymap_answer(&r), None);
    }
}
