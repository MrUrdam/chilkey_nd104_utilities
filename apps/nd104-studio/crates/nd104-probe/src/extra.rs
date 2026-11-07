//! Lighting, keyboard settings, keymap and picture commands of the probe.
//! Reads are the default; anything that changes the keyboard needs an
//! explicit value, and lighting/settings changes are only stored with
//! `--store`. Works over USB and – except for the picture, which needs the
//! screen controller – over Bluetooth.

use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use nd104_core::keyboard_settings::KeyboardSetting;
use nd104_core::model;
use nd104_core::picture::{self, Fit};
use nd104_protocol::{parse_picture_ack, PictureAck, Target};

use crate::hid::{hex_dump, Device, Probe};

/// A hidapi context for opening paths. Windows: without hidapi's slow scan of
/// every HID device; Linux: hidapi reads sysfs (cheap), and the app's device
/// list has already created a normal context, which "without scan" may not
/// follow (hidapi panics then).
fn hid_api() -> hidapi::HidResult<hidapi::HidApi> {
    #[cfg(windows)]
    #[allow(deprecated)]
    return hidapi::HidApi::new_without_enumerate();
    #[cfg(not(windows))]
    return hidapi::HidApi::new();
}

const ANSWER: Duration = Duration::from_millis(1000);

/// Sends one command and returns the first answer starting with the same
/// command byte (or `FF`: not supported).
fn ask(device: &Device, packet: &[u8]) -> Result<Vec<u8>> {
    device.send(packet, 32)?;
    let deadline = Instant::now() + ANSWER;
    while Instant::now() < deadline {
        if let Some(answer) = device.read_one(Duration::from_millis(100)) {
            if answer.first() == packet.first() || answer.first() == Some(&0xFF) {
                if answer.first() == Some(&0xFF) {
                    bail!("the keyboard does not support command 0x{:02X}", packet[0]);
                }
                return Ok(answer);
            }
        }
    }
    bail!("no answer to command 0x{:02X}", packet[0])
}

fn keyboard() -> Result<Device> {
    Probe::new()?.open(Target::Keyboard)
}

// --- lighting ---------------------------------------------------------------

pub struct LightChange {
    pub effect: Option<u8>,
    pub brightness: Option<u8>,
    pub speed: Option<u8>,
    /// Hue and saturation, 0–255 each.
    pub color: Option<(u8, u8)>,
    pub store: bool,
}

pub fn light(channel: u8, change: LightChange) -> Result<()> {
    let device = keyboard()?;
    let mut writes: Vec<Vec<u8>> = Vec::new();
    if let Some(v) = change.effect {
        writes.push(vec![0x07, channel, 0x02, v]);
    }
    if let Some(v) = change.brightness {
        writes.push(vec![0x07, channel, 0x01, v]);
    }
    if let Some(v) = change.speed {
        writes.push(vec![0x07, channel, 0x03, v]);
    }
    if let Some((h, s)) = change.color {
        writes.push(vec![0x07, channel, 0x04, h, s]);
    }
    for packet in &writes {
        let answer = ask(&device, packet)?;
        let echoed = answer.get(..packet.len()) == Some(packet.as_slice());
        println!(
            "set [{}] → {}",
            hex(packet),
            if echoed {
                "echoed"
            } else {
                "UNEXPECTED answer"
            }
        );
        if !echoed {
            print!("{}", hex_dump(&answer));
            bail!("stopped: the answer does not match");
        }
    }
    if change.store && !writes.is_empty() {
        let answer = ask(&device, &[0x09, channel])?;
        println!(
            "store [09 {channel:02x}] → {}",
            hex(&answer[..2.min(answer.len())])
        );
    } else if !writes.is_empty() {
        println!("(not stored – the old values return after a restart; add --store to keep)");
    }
    // read everything back
    let read = |id: u8, len: usize| -> Result<Vec<u8>> {
        let a = ask(&device, &[0x08, channel, id])?;
        Ok(a.get(3..3 + len).unwrap_or_default().to_vec())
    };
    println!("effect     {}", read(0x02, 1)?[0]);
    println!("brightness {}", read(0x01, 1)?[0]);
    println!("speed      {}", read(0x03, 1)?[0]);
    let c = read(0x04, 2)?;
    println!("color      hue {} sat {}", c[0], c[1]);
    Ok(())
}

// --- keyboard settings -------------------------------------------------------

pub fn setting(setting: KeyboardSetting, value: Option<u8>, store: bool) -> Result<()> {
    let device = keyboard()?;
    if let Some(v) = value {
        if !setting.allows(v) {
            bail!("{v} is not a value Chilkey's driver offers for {setting:?}");
        }
        let packet = [0x07, 0x00, 0x04, setting.write_id(), v];
        let answer = ask(&device, &packet)?;
        let echoed = answer.get(..5) == Some(&packet[..]);
        println!(
            "set [{}] → {}",
            hex(&packet),
            if echoed { "echoed" } else { "UNEXPECTED" }
        );
        if !echoed {
            bail!("stopped: the answer does not match");
        }
        if store {
            ask(&device, &[0x09, 0x00])?;
            println!("stored");
        } else {
            println!("(not stored – add --store to keep)");
        }
    }
    let answer = ask(&device, &[0x08, 0x00, 0x04, setting.read_id()])?;
    println!("{setting:?} = {}", answer.get(4).copied().unwrap_or(0));
    Ok(())
}

// --- keymap -----------------------------------------------------------------

/// Reads `len` bytes of the keymap buffer (`12 off_hi off_lo len`).
fn keymap_bytes(device: &Device, offset: u16, len: u8) -> Result<Vec<u8>> {
    let [hi, lo] = offset.to_be_bytes();
    let answer = ask(device, &[0x12, hi, lo, len])?;
    answer
        .get(4..4 + len as usize)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| anyhow!("keymap answer too short"))
}

fn key_offset(layer: u8, row: u8, col: u8) -> u16 {
    (u16::from(layer) * 128 + u16::from(row) * 16 + u16::from(col)) * 2
}

pub fn key(layer: u8, row: u8, col: u8, code: Option<u16>) -> Result<()> {
    if layer > 5 || row > 7 || col > 15 {
        bail!("layer 0–5, row 0–7, column 0–15");
    }
    let device = keyboard()?;
    let read = |device: &Device| -> Result<u16> {
        let b = keymap_bytes(device, key_offset(layer, row, col), 2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    };
    let old = read(&device)?;
    println!("layer {layer} row {row} col {col}: 0x{old:04X}");
    if let Some(code) = code {
        let [hi, lo] = code.to_be_bytes();
        let packet = [0x05, layer, row, col, hi, lo];
        let answer = ask(&device, &packet)?;
        if answer.get(..6) != Some(&packet[..]) {
            bail!("stopped: the answer does not match");
        }
        let new = read(&device)?;
        println!("written and read back: 0x{new:04X} (the keyboard stores keys immediately)");
        println!("undo: nd104-probe key --layer {layer} --row {row} --col {col} --code {old:04X}");
    }
    Ok(())
}

pub fn knob(layer: u8, right: bool, code: Option<u16>) -> Result<()> {
    if layer > 3 {
        bail!("the knob has layers 0–3");
    }
    let device = keyboard()?;
    let dir = u8::from(right);
    let read = |device: &Device| -> Result<u16> {
        let a = ask(device, &[0x14, layer, 0x00, dir])?;
        Ok(u16::from_be_bytes([a[4], a[5]]))
    };
    let old = read(&device)?;
    let side = if right { "right" } else { "left" };
    println!("knob layer {layer} {side}: 0x{old:04X}");
    if let Some(code) = code {
        let [hi, lo] = code.to_be_bytes();
        let packet = [0x15, layer, 0x00, dir, hi, lo];
        let answer = ask(&device, &packet)?;
        if answer.get(..6) != Some(&packet[..]) {
            bail!("stopped: the answer does not match");
        }
        println!("written and read back: 0x{:04X}", read(&device)?);
        println!("undo: nd104-probe knob --layer {layer} --dir {side} --code {old:04X}");
    }
    Ok(())
}

pub fn keymap(layer: u8) -> Result<()> {
    if layer > 5 {
        bail!("layer 0–5");
    }
    let device = keyboard()?;
    let base = u16::from(layer) * 256;
    let mut bytes = Vec::new();
    // 28-byte chunks, like the web driver
    while bytes.len() < 256 {
        let len = (256 - bytes.len()).min(28) as u8;
        bytes.extend(keymap_bytes(&device, base + bytes.len() as u16, len)?);
    }
    println!("layer {layer} (row × column, 0001 = same as layer below):");
    print!("      ");
    for col in 0..16 {
        print!("{col:>5}");
    }
    println!();
    for row in 0..8 {
        print!("row {row} ");
        for col in 0..16 {
            let i = (row * 16 + col) * 2;
            print!(" {:04x}", u16::from_be_bytes([bytes[i], bytes[i + 1]]));
        }
        println!();
    }
    Ok(())
}

// --- picture ------------------------------------------------------------------

pub fn picture(file: &str, fit: Fit, dry_run: bool) -> Result<()> {
    let models = model::registry();
    let model = models.first().ok_or_else(|| anyhow!("no keyboard model"))?;
    let spec = model
        .picture_spec()
        .ok_or_else(|| anyhow!("this keyboard does not support own pictures"))?;
    let bytes = std::fs::read(file).with_context(|| format!("cannot read {file}"))?;
    let prepared =
        picture::prepare(&bytes, &spec, fit).map_err(|p| anyhow!("picture rejected: {p:?}"))?;
    let transfer = model.picture_transfer(&prepared.frames)?;
    println!(
        "{} {}×{} → {} frame(s), {} bytes, {} page(s)",
        prepared.format,
        prepared.source_width,
        prepared.source_height,
        prepared.frames.len(),
        prepared.payload_bytes,
        transfer.pages.len()
    );
    for note in &prepared.notes {
        println!("note: {note:?}");
    }
    if dry_run {
        println!("(dry run – nothing sent)");
        return Ok(());
    }

    // the picture goes straight to the screen controller (USB only)
    let device = Probe::new()?.open(Target::Screen)?;
    let len = Target::Screen.report_len();
    let wait = |timeout: Duration| -> Option<PictureAck> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(r) = device.read_one(Duration::from_millis(100)) {
                if let Some(ack) = parse_picture_ack(&r) {
                    return Some(ack);
                }
            }
        }
        None
    };
    device.send(&transfer.start, len)?;
    let mut answer = wait(Duration::from_secs(5));
    let mut next = 0usize;
    let mut retries = 0;
    loop {
        let page = match answer {
            Some(PictureAck::Next) if next == transfer.pages.len() => break,
            Some(PictureAck::Next) => {
                retries = 0;
                next += 1;
                next - 1
            }
            Some(PictureAck::Resend) if next > 0 && retries < 3 => {
                retries += 1;
                println!("screen asked for page {next} again");
                next - 1
            }
            Some(PictureAck::Resend) => bail!("the screen rejected the picture data"),
            None => bail!("no answer from the screen (page {next})"),
        };
        let reports = &transfer.pages[page];
        for (i, report) in reports.iter().enumerate() {
            device.send(report, len)?;
            if i + 1 < reports.len() {
                thread::sleep(Duration::from_millis(2));
            }
        }
        let last = page + 1 == transfer.pages.len();
        answer = match wait(if last {
            Duration::from_millis(1500)
        } else {
            Duration::from_secs(5)
        }) {
            // no answer after the last page is normal
            None if last => Some(PictureAck::Next),
            other => other,
        };
        print!("\rpage {}/{}", page + 1, transfer.pages.len());
    }
    println!("\npicture stored on the screen");
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// --- all collections --------------------------------------------------------

/// Every collection of the keyboard, screen and receiver with its report IDs
/// and report descriptor. Uses the app's fast device list (hidapi's own scan
/// opens every HID device of the PC). Nothing is written.
pub fn descriptors() -> Result<()> {
    use nd104_core::hid::{HidBackend, HidapiBackend};
    let mut list = HidapiBackend::new()?.enumerate()?;
    list.sort_by(|a, b| {
        (a.vid, a.pid, a.interface_number, a.usage_page).cmp(&(
            b.vid,
            b.pid,
            b.interface_number,
            b.usage_page,
        ))
    });
    let api = hid_api()?;
    for info in list {
        println!(
            "{:04x}:{:04x} if {:>2} usage {:04x}:{:04x}{}",
            info.vid,
            info.pid,
            info.interface_number,
            info.usage_page,
            info.usage,
            if info.is_bluetooth() {
                " (Bluetooth)"
            } else {
                ""
            }
        );
        let Ok(path) = std::ffi::CString::new(info.path.clone()) else {
            continue;
        };
        let device = match api.open_path(&path) {
            Ok(d) => d,
            Err(e) => {
                println!("  cannot open: {e}");
                continue;
            }
        };
        let mut desc = [0u8; 4096];
        match device.get_report_descriptor(&mut desc) {
            Ok(len) => {
                let ids: Vec<String> = desc[..len]
                    .windows(2)
                    .filter(|w| w[0] == 0x85)
                    .map(|w| format!("{:02x}", w[1]))
                    .collect();
                println!(
                    "  report IDs: {}",
                    if ids.is_empty() {
                        "none".into()
                    } else {
                        ids.join(" ")
                    }
                );
                println!("  {}", hex(&desc[..len]));
            }
            Err(e) => println!("  no descriptor: {e}"),
        }
    }
    Ok(())
}

/// Reads one feature report (GET_FEATURE – nothing is changed).
pub fn feature(usage: &str, id: &str) -> Result<()> {
    use nd104_core::hid::{HidBackend, HidapiBackend};
    let hex16 = |s: &str| u16::from_str_radix(s.trim_start_matches("0x"), 16);
    let (page, use_) = usage
        .split_once(':')
        .ok_or_else(|| anyhow!("usage as page:usage, e.g. 0059:0001"))?;
    let (page, use_) = (hex16(page)?, hex16(use_)?);
    let id = u8::from_str_radix(id.trim_start_matches("0x"), 16)?;
    let info = HidapiBackend::new()?
        .enumerate()?
        .into_iter()
        .find(|i| i.vid == 0x6D67 && !i.is_bluetooth() && i.usage_page == page && i.usage == use_)
        .ok_or_else(|| anyhow!("collection {page:04x}:{use_:04x} not found (USB cable?)"))?;
    let api = hid_api()?;
    let device = api.open_path(&std::ffi::CString::new(info.path)?)?;
    let mut buf = [0u8; 256];
    buf[0] = id;
    let len = device
        .get_feature_report(&mut buf)
        .with_context(|| format!("feature report {id:02x}"))?;
    println!("feature report {id:02x} ({len} bytes):");
    print!("{}", hex_dump(&buf[..len]));
    Ok(())
}

/// Read commands of the PC lighting protocols on interface 4 (report ID 05):
/// SignalRGB 21 QMK version, 22 protocol version, 23 unique id, 27 LED
/// count, 28 firmware type; OpenRGB 01 protocol version, 02 QMK version,
/// 03 device info, 05 LED info.
const RGB_READS: [u8; 9] = [0x21, 0x22, 0x23, 0x27, 0x28, 0x01, 0x02, 0x03, 0x05];

/// The lighting channel: interface 4, FF59:0060, report ID 05.
fn lighting_channel() -> Result<hidapi::HidDevice> {
    use nd104_core::hid::{HidBackend, HidapiBackend};
    let info = HidapiBackend::new()?
        .enumerate()?
        .into_iter()
        .find(|i| {
            i.vid == 0x6D67
                && !i.is_bluetooth()
                && i.usage_page == 0xFF59
                && i.interface_number == 4
        })
        .ok_or_else(|| {
            anyhow!("lighting channel (interface 4, FF59:0060) not found – USB cable?")
        })?;
    let api = hid_api()?;
    Ok(api.open_path(&std::ffi::CString::new(info.path)?)?)
}

/// SignalRGB stream test (see `RgbTest`). Answers are printed as they come.
pub fn rgb_test(color: &str, seconds: u64, per_packet: u8) -> Result<()> {
    let rgb = u32::from_str_radix(color.trim_start_matches('#'), 16)
        .context("colour as hex, e.g. ff0000")?;
    let [_, r, g, b] = rgb.to_be_bytes();
    if per_packet == 0 || 4 + 3 * per_packet as usize > 64 {
        bail!("1–20 LEDs per packet");
    }
    let device = lighting_channel()?;
    let send = |payload: &[u8]| -> Result<()> {
        let mut report = vec![0x05];
        report.extend(payload);
        report.resize(64, 0);
        device.write(&report).context("write failed")?;
        let mut buf = [0u8; 64];
        if let Ok(n) = device.read_timeout(&mut buf, 20) {
            if n > 0 && buf[1] != 0x24 {
                println!("← {}", hex(&buf[..n.min(8)]));
            }
        }
        Ok(())
    };
    println!("switching to PC colours (25)");
    send(&[0x25])?;
    let leds = 130u8;
    let until = Instant::now() + Duration::from_secs(seconds);
    let mut frames = 0;
    while Instant::now() < until {
        let mut start = 0u8;
        while start < leds {
            let count = per_packet.min(leds - start);
            let mut p = vec![0x24, start, count];
            for _ in 0..count {
                p.extend([r, g, b]);
            }
            send(&p)?;
            start += count;
        }
        frames += 1;
        thread::sleep(Duration::from_millis(50));
    }
    println!("{frames} frames sent; giving the lighting back (26)");
    send(&[0x26])?;
    Ok(())
}

pub fn rgb(bytes: &[String]) -> Result<()> {
    use nd104_core::hid::{HidBackend, HidapiBackend};
    let packet = bytes
        .iter()
        .map(|b| u8::from_str_radix(b.trim_start_matches("0x"), 16))
        .collect::<Result<Vec<u8>, _>>()
        .context("bytes must be hex, e.g. 27")?;
    if !packet.first().is_some_and(|c| RGB_READS.contains(c)) {
        bail!("only read commands: 21 22 23 27 28 (SignalRGB), 01 02 03 05 (OpenRGB)");
    }
    let info = HidapiBackend::new()?
        .enumerate()?
        .into_iter()
        .find(|i| {
            i.vid == 0x6D67
                && !i.is_bluetooth()
                && i.usage_page == 0xFF59
                && i.interface_number == 4
        })
        .ok_or_else(|| {
            anyhow!("lighting channel (interface 4, FF59:0060) not found – USB cable?")
        })?;
    let api = hid_api()?;
    let device = api.open_path(&std::ffi::CString::new(info.path)?)?;
    // report ID 05 + 63 bytes
    let mut report = vec![0x05];
    report.extend(&packet);
    report.resize(64, 0);
    println!("→ lighting channel");
    print!("{}", hex_dump(&report));
    device.write(&report).context("write failed")?;
    let deadline = Instant::now() + ANSWER;
    let mut buf = [0u8; 64];
    while Instant::now() < deadline {
        let n = device.read_timeout(&mut buf, 100)?;
        if n > 0 {
            println!("← answer ({n} bytes)");
            print!("{}", hex_dump(&buf[..n]));
        }
    }
    Ok(())
}
