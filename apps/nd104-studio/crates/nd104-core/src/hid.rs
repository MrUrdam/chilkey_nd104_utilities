//! HID access behind small traits so the device manager can be tested
//! without hardware.

use std::ffi::CString;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use hidapi::HidApi;

/// One enumerated HID interface (collection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidInterfaceInfo {
    pub vid: u16,
    pub pid: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
    pub path: String,
    pub product: Option<String>,
}

/// Bluetooth "HID over GATT" service: part of the device path of every HID
/// interface that is connected over Bluetooth instead of USB.
const BLUETOOTH_HID_SERVICE: &str = "{00001812-0000-1000-8000-00805f9b34fb}";

impl HidInterfaceInfo {
    /// Connected over Bluetooth? The ND104 reports the same IDs and usages
    /// there, but its vendor collection uses report ID 2 (docs/protocol.md
    /// § 5.5), so it must not be mixed up with the USB interface.
    pub fn is_bluetooth(&self) -> bool {
        self.path.to_lowercase().contains(BLUETOOTH_HID_SERVICE)
    }
}

/// An opened interface.
pub trait HidChannel {
    /// Writes one output report (without report ID, already padded).
    fn write(&mut self, report: &[u8]) -> Result<()>;
    /// Reads one input report; `None` on timeout.
    fn read(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>>;
}

pub trait HidBackend {
    fn enumerate(&mut self) -> Result<Vec<HidInterfaceInfo>>;
    fn open(&mut self, info: &HidInterfaceInfo) -> Result<Box<dyn HidChannel>>;
}

/// Real backend using hidapi (pure-Rust Windows backend).
pub struct HidapiBackend {
    api: HidApi,
}

impl HidapiBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            api: HidApi::new().context("cannot initialise HID API")?,
        })
    }
}

impl HidBackend for HidapiBackend {
    fn enumerate(&mut self) -> Result<Vec<HidInterfaceInfo>> {
        self.api
            .refresh_devices()
            .context("HID enumeration failed")?;
        Ok(self
            .api
            .device_list()
            .map(|d| HidInterfaceInfo {
                vid: d.vendor_id(),
                pid: d.product_id(),
                usage_page: d.usage_page(),
                usage: d.usage(),
                interface_number: d.interface_number(),
                path: d.path().to_string_lossy().into_owned(),
                product: d.product_string().map(str::to_owned),
            })
            .collect())
    }

    fn open(&mut self, info: &HidInterfaceInfo) -> Result<Box<dyn HidChannel>> {
        let path = CString::new(info.path.clone()).map_err(|_| anyhow!("invalid HID path"))?;
        let device = self
            .api
            .open_path(&path)
            .with_context(|| format!("cannot open {:04x}:{:04x}", info.vid, info.pid))?;
        // over USB the vendor collection has no report ID; over Bluetooth it
        // has one (ND104: 2), declared in the report descriptor (tag 0x85)
        let report_id = if info.is_bluetooth() {
            let mut desc = [0u8; 4096];
            let len = device
                .get_report_descriptor(&mut desc)
                .context("cannot read the Bluetooth report descriptor")?;
            desc[..len]
                .windows(2)
                .find(|w| w[0] == 0x85)
                .map(|w| w[1])
                .ok_or_else(|| anyhow!("Bluetooth interface without report ID"))?
        } else {
            0
        };
        Ok(Box::new(HidapiChannel { device, report_id }))
    }
}

struct HidapiChannel {
    device: hidapi::HidDevice,
    /// 0 over USB; the collection's report ID over Bluetooth.
    report_id: u8,
}

impl HidChannel for HidapiChannel {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(report.len() + 1);
        buf.push(self.report_id);
        buf.extend_from_slice(report);
        let written = self.device.write(&buf).context("HID write failed")?;
        if written < buf.len() {
            return Err(anyhow!(
                "short HID write ({written} of {} bytes)",
                buf.len()
            ));
        }
        Ok(())
    }

    fn read(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let mut buf = [0u8; 65];
        let ms = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
        match self
            .device
            .read_timeout(&mut buf, ms)
            .context("HID read failed")?
        {
            0 => Ok(None),
            // with a report ID, Windows puts it in front of the data
            n if self.report_id != 0 && buf[0] == self.report_id => Ok(Some(buf[1..n].to_vec())),
            n => Ok(Some(buf[..n].to_vec())),
        }
    }
}
