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
        Ok(Box::new(HidapiChannel { device }))
    }
}

struct HidapiChannel {
    device: hidapi::HidDevice,
}

impl HidChannel for HidapiChannel {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(report.len() + 1);
        buf.push(0); // report ID
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
            n => Ok(Some(buf[..n].to_vec())),
        }
    }
}
