use std::fmt::Write as _;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use nd104_protocol::{
    keyboard_version_query, parse_version_response, screen_version_query, Target, KEYBOARD,
    SCREEN, SCREEN_DFU, VENDOR_USAGE, VENDOR_USAGE_PAGE,
};

/// Firmware versions as reported by the devices (raw ASCII, not normalized).
pub struct Versions {
    pub keyboard: Option<String>,
    pub screen: Option<String>,
}

pub struct Probe {
    api: HidApi,
}

pub struct Device {
    target: Target,
    handle: HidDevice,
}

impl Probe {
    pub fn new() -> Result<Self> {
        let api = HidApi::new().context("cannot initialise HID API")?;
        Ok(Self { api })
    }

    fn nd104_interfaces(&self) -> impl Iterator<Item = &DeviceInfo> {
        self.api.device_list().filter(|d| {
            let id = (d.vendor_id(), d.product_id());
            [KEYBOARD, SCREEN, SCREEN_DFU]
                .iter()
                .any(|u| (u.vid, u.pid) == id)
        })
    }

    pub fn list(&self) -> Result<()> {
        let mut found = false;
        for d in self.nd104_interfaces() {
            found = true;
            let role = match (d.vendor_id(), d.product_id()) {
                (v, p) if (v, p) == (KEYBOARD.vid, KEYBOARD.pid) => "keyboard",
                (v, p) if (v, p) == (SCREEN.vid, SCREEN.pid) => "screen",
                _ => "screen DFU",
            };
            let vendor = d.usage_page() == VENDOR_USAGE_PAGE && d.usage() == VENDOR_USAGE;
            println!(
                "{role:<10} {:04x}:{:04x}  if {:>2}  usage {:04x}:{:04x}{}  release {:x}.{:02x}  {} / {}",
                d.vendor_id(),
                d.product_id(),
                d.interface_number(),
                d.usage_page(),
                d.usage(),
                if vendor { " ← vendor" } else { "" },
                d.release_number() >> 8,
                d.release_number() & 0xFF,
                d.manufacturer_string().unwrap_or("?"),
                d.product_string().unwrap_or("?"),
            );
            println!("           path {}", d.path().to_string_lossy());
        }
        if !found {
            println!("No ND104 found. Is the keyboard connected with a USB cable?");
        }
        Ok(())
    }

    /// Opens the vendor HID collection (usage page FF60, usage 61) of a device.
    pub fn open(&self, target: Target) -> Result<Device> {
        let id = target.usb_id();
        let info = self
            .api
            .device_list()
            .find(|d| {
                d.vendor_id() == id.vid
                    && d.product_id() == id.pid
                    && d.usage_page() == VENDOR_USAGE_PAGE
                    && d.usage() == VENDOR_USAGE
            })
            .ok_or_else(|| {
                anyhow!(
                    "{} ({:04x}:{:04x}, usage {:04x}:{:02x}) not found – USB cable connected? \
                     Is the Chilkey web driver closed?",
                    target.name(),
                    id.vid,
                    id.pid,
                    VENDOR_USAGE_PAGE,
                    VENDOR_USAGE
                )
            })?;
        let handle = info
            .open_device(&self.api)
            .with_context(|| format!("cannot open {}", target.name()))?;
        Ok(Device { target, handle })
    }

    /// Queries both devices. Missing devices or silent firmware → `None`.
    pub fn read_versions(&self, timeout: Duration, verbose: bool) -> Versions {
        let query = |target: Target, packet: Vec<u8>| -> Option<String> {
            let device = match self.open(target) {
                Ok(d) => d,
                Err(e) => {
                    println!("{e:#}");
                    return None;
                }
            };
            if verbose {
                println!("\n→ {} version query", target.name());
                print!("{}", hex_dump(&packet));
            }
            if let Err(e) = device.send(&packet, target.report_len()) {
                println!("{e:#}");
                return None;
            }
            let mut version = None;
            for report in device.read_reports(timeout) {
                if verbose {
                    println!("← {} answer", target.name());
                    print!("{}", hex_dump(&report));
                }
                if version.is_none() {
                    version = parse_version_response(target, &report);
                }
            }
            version
        };
        Versions {
            keyboard: query(Target::Keyboard, keyboard_version_query()),
            screen: query(Target::Screen, screen_version_query()),
        }
    }

    pub fn listen(&self, target: Target, duration: Duration) -> Result<()> {
        let device = self.open(target)?;
        println!("Listening on {} for {} s …", target.name(), duration.as_secs());
        for report in device.read_reports(duration) {
            print!("{}", hex_dump(&report));
        }
        Ok(())
    }
}

impl Device {
    /// Writes one report: report ID 0 + packet, zero-padded to `report_len`.
    pub fn send(&self, packet: &[u8], report_len: usize) -> Result<()> {
        if packet.len() > report_len {
            bail!("packet ({} bytes) longer than report ({report_len})", packet.len());
        }
        let mut buf = vec![0u8; report_len + 1];
        buf[1..=packet.len()].copy_from_slice(packet);
        self.handle.write(&buf).with_context(|| {
            format!(
                "write to {} failed (report length {report_len}; try --report-len 32 or 64)",
                self.target.name()
            )
        })?;
        Ok(())
    }

    /// Collects all input reports arriving within `duration`.
    pub fn read_reports(&self, duration: Duration) -> Vec<Vec<u8>> {
        let deadline = Instant::now() + duration;
        let mut reports = Vec::new();
        let mut buf = [0u8; 65];
        while Instant::now() < deadline {
            match self.handle.read_timeout(&mut buf, 50) {
                Ok(0) => {}
                Ok(n) => reports.push(buf[..n].to_vec()),
                Err(e) => {
                    println!("read error on {}: {e}", self.target.name());
                    break;
                }
            }
        }
        reports
    }

    pub fn drain_answers(&self, duration: Duration) {
        for report in self.read_reports(duration) {
            println!("← {} answer", self.target.name());
            print!("{}", hex_dump(&report));
        }
    }
}

/// Classic hex dump, 16 bytes per row with offset.
pub fn hex_dump(data: &[u8]) -> String {
    let mut out = String::new();
    for (row, chunk) in data.chunks(16).enumerate() {
        let _ = write!(out, "  {:04x}  ", row * 16);
        for b in chunk {
            let _ = write!(out, "{b:02x} ");
        }
        let ascii: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
            .collect();
        let _ = writeln!(out, "{:width$} {ascii}", "", width = (16 - chunk.len()) * 3);
    }
    out
}
