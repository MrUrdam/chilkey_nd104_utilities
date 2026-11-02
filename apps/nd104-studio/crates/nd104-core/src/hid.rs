//! HID access behind small traits so the device manager can be tested
//! without hardware.

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::mpsc;
use std::time::{Duration, Instant};

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

/// How long opening one interface for its details may take. A paired
/// Bluetooth keyboard that is not connected stays in Windows' device list,
/// and opening it blocks for many seconds each time.
const INFO_TIMEOUT: Duration = Duration::from_secs(3);
/// When to try an interface again that was too slow or could not be opened.
const INFO_RETRY: Duration = Duration::from_secs(30);

/// Real backend using hidapi (pure-Rust Windows backend).
///
/// On Windows it does not use hidapi's enumeration, which opens every HID
/// device of the PC (and waits for each stale Bluetooth one): it takes the
/// HID device paths from Windows, reads vendor/product ID from the path,
/// and opens only the interfaces of supported keyboards and receivers –
/// each with a time limit, and only once while it stays plugged in.
pub struct HidapiBackend {
    api: HidApi,
    /// Vendor/product IDs worth opening (keyboards, screens, receivers).
    wanted: Vec<(u16, u16)>,
    /// Details of interfaces opened before, by path.
    known: HashMap<String, HidInterfaceInfo>,
    /// Interfaces that were too slow or failed, and when to try again.
    skipped: HashMap<String, Instant>,
}

impl HidapiBackend {
    pub fn new() -> Result<Self> {
        let mut wanted = Vec::new();
        for model in crate::model::registry() {
            for spec in model.interfaces().iter().chain(model.receiver()) {
                if !wanted.contains(&(spec.vid, spec.pid)) {
                    wanted.push((spec.vid, spec.pid));
                }
            }
        }
        Ok(Self {
            api: hidapi_without_scan().context("cannot initialise HID API")?,
            wanted,
            known: HashMap::new(),
            skipped: HashMap::new(),
        })
    }
}

/// Vendor and product ID from a HID device path: USB
/// `HID#VID_6D67&PID_016C&MI_01#…`, Bluetooth LE
/// `HID#{00001812-…}_Dev_VID&016d67_PID&016c&Col02#…` (2 hex digits of
/// ID source before the vendor ID).
pub fn ids_from_path(path: &str) -> Option<(u16, u16)> {
    let p = path.to_ascii_lowercase();
    let hex = |s: &str| u16::from_str_radix(s, 16).ok();
    if let Some(i) = p.find("vid_") {
        let vid = hex(p.get(i + 4..i + 8)?)?;
        let j = p.find("pid_")?;
        return Some((vid, hex(p.get(j + 4..j + 8)?)?));
    }
    let i = p.find("vid&")?;
    let vid = hex(p.get(i + 6..i + 10)?)?;
    let j = p.find("pid&")?;
    Some((vid, hex(p.get(j + 4..j + 8)?)?))
}

/// A hidapi context that does not scan all HID devices of the PC (that is
/// what blocks on stale Bluetooth devices). ND104 Studio, its setup and the
/// device thread are the only hidapi users in their processes, so the
/// global switch behind it is fine here.
#[allow(deprecated)]
fn hidapi_without_scan() -> hidapi::HidResult<HidApi> {
    HidApi::new_without_enumerate()
}

/// Reads one interface's details in a helper thread, so that a device that
/// does not answer cannot hold up the others; a receiver that stays empty
/// means too slow (the thread finishes on its own later).
fn interface_info(path: &str) -> mpsc::Receiver<Result<HidInterfaceInfo>> {
    let (tx, rx) = mpsc::channel();
    let path = path.to_owned();
    std::thread::spawn(move || {
        let _ = tx.send(read_interface_info(path));
    });
    rx
}

/// Interface number from the path (`…&MI_01…`), -1 without one.
fn interface_number(path: &str) -> i32 {
    let p = path.to_ascii_lowercase();
    p.find("&mi_")
        .and_then(|i| p.get(i + 4..i + 6))
        .and_then(|n| i32::from_str_radix(n, 16).ok())
        .unwrap_or(-1)
}

/// Usage page and usage of a HID collection. Opened without access rights,
/// which Windows answers from its own data – nothing is sent to the device,
/// and it is fast (hidapi's details take about a second per interface).
#[cfg(windows)]
fn read_interface_info(path: String) -> Result<HidInterfaceInfo> {
    use windows::core::HSTRING;
    use windows::Win32::Devices::HumanInterfaceDevice::{
        HidD_FreePreparsedData, HidD_GetPreparsedData, HidP_GetCaps, HIDP_CAPS,
        HIDP_STATUS_SUCCESS, PHIDP_PREPARSED_DATA,
    };
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    let (vid, pid) = ids_from_path(&path).ok_or_else(|| anyhow!("no IDs in {path}"))?;
    let handle = unsafe {
        CreateFileW(
            &HSTRING::from(path.as_str()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
    .context("cannot open the HID interface")?;
    let caps = unsafe {
        let mut data = PHIDP_PREPARSED_DATA::default();
        let caps = if HidD_GetPreparsedData(handle, &mut data) {
            let mut caps = HIDP_CAPS::default();
            let status = HidP_GetCaps(data, &mut caps);
            HidD_FreePreparsedData(data);
            (status == HIDP_STATUS_SUCCESS).then_some(caps)
        } else {
            None
        };
        let _ = CloseHandle(handle);
        caps
    };
    let caps = caps.ok_or_else(|| anyhow!("no HID capabilities"))?;
    Ok(HidInterfaceInfo {
        vid,
        pid,
        usage_page: caps.UsagePage,
        usage: caps.Usage,
        interface_number: interface_number(&path),
        path,
        product: None,
    })
}

#[cfg(not(windows))]
fn read_interface_info(path: String) -> Result<HidInterfaceInfo> {
    let api = hidapi_without_scan()?;
    let c_path = CString::new(path.clone()).map_err(|_| anyhow!("invalid HID path"))?;
    let d = api.open_path(&c_path)?.get_device_info()?;
    Ok(HidInterfaceInfo {
        vid: d.vendor_id(),
        pid: d.product_id(),
        usage_page: d.usage_page(),
        usage: d.usage(),
        interface_number: d.interface_number(),
        path,
        product: d.product_string().map(str::to_owned),
    })
}

/// All present HID device paths, straight from Windows (nothing is opened).
#[cfg(windows)]
fn hid_paths() -> Result<Vec<String>> {
    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Device_Interface_ListW, CM_Get_Device_Interface_List_SizeW,
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CR_BUFFER_SMALL, CR_SUCCESS,
    };
    const HID_INTERFACE: GUID = GUID::from_u128(0x4d1e55b2_f16f_11cf_88cb_001111000030);
    // the list can grow between the two calls: try again then
    for _ in 0..3 {
        let mut len = 0u32;
        let cr = unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut len,
                &HID_INTERFACE,
                PCWSTR::null(),
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if cr != CR_SUCCESS {
            return Err(anyhow!("cannot list HID devices ({})", cr.0));
        }
        let mut buffer = vec![0u16; len as usize];
        let cr = unsafe {
            CM_Get_Device_Interface_ListW(
                &HID_INTERFACE,
                PCWSTR::null(),
                &mut buffer,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if cr == CR_BUFFER_SMALL {
            continue;
        }
        if cr != CR_SUCCESS {
            return Err(anyhow!("cannot list HID devices ({})", cr.0));
        }
        return Ok(buffer
            .split(|&c| c == 0)
            .filter(|s| !s.is_empty())
            .map(String::from_utf16_lossy)
            .collect());
    }
    Err(anyhow!("cannot list HID devices (list kept changing)"))
}

/// Is the Bluetooth device behind a HID path connected right now? A paired
/// keyboard stays in Windows' device list while it is on USB, 2.4 GHz or off,
/// and opening it then blocks. `true` when Windows does not say.
#[cfg(windows)]
fn bluetooth_connected(path: &str) -> bool {
    use windows::core::{GUID, HSTRING, PCWSTR};
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_DevNode_PropertyW, CM_Get_Device_Interface_PropertyW, CM_Get_Parent,
        CM_Locate_DevNodeW, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
    };
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_InstanceId, DEVPROPTYPE};
    use windows::Win32::Foundation::DEVPROPKEY;
    // "connected right now" of a Bluetooth device (boolean, 0xFF = true)
    const CONNECTED: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0x83da6326_97a6_4088_9453_a1923f573b29),
        pid: 15,
    };
    unsafe {
        // the interface's device instance …
        let mut kind = DEVPROPTYPE::default();
        let mut buf = [0u8; 1024];
        let mut size = buf.len() as u32;
        if CM_Get_Device_Interface_PropertyW(
            &HSTRING::from(path),
            &DEVPKEY_Device_InstanceId,
            &mut kind,
            Some(buf.as_mut_ptr()),
            &mut size,
            0,
        ) != CR_SUCCESS
        {
            return true;
        }
        let id: Vec<u16> = buf[..size as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let mut node = 0u32;
        if CM_Locate_DevNodeW(&mut node, PCWSTR(id.as_ptr()), CM_LOCATE_DEVNODE_NORMAL)
            != CR_SUCCESS
        {
            return true;
        }
        // … and up the tree to the Bluetooth device that knows its state
        for _ in 0..4 {
            let mut value = [0u8; 1];
            let mut size = 1u32;
            if CM_Get_DevNode_PropertyW(
                node,
                &CONNECTED,
                &mut kind,
                Some(value.as_mut_ptr()),
                &mut size,
                0,
            ) == CR_SUCCESS
            {
                return value[0] != 0;
            }
            let mut parent = 0u32;
            if CM_Get_Parent(&mut parent, node, 0) != CR_SUCCESS {
                return true;
            }
            node = parent;
        }
    }
    true
}

#[cfg(not(windows))]
fn bluetooth_connected(_path: &str) -> bool {
    true
}

#[cfg(not(windows))]
fn hid_paths() -> Result<Vec<String>> {
    let mut api = HidApi::new().context("cannot initialise HID API")?;
    api.refresh_devices().context("HID enumeration failed")?;
    Ok(api
        .device_list()
        .map(|d| d.path().to_string_lossy().into_owned())
        .collect())
}

impl HidBackend for HidapiBackend {
    fn enumerate(&mut self) -> Result<Vec<HidInterfaceInfo>> {
        let started = Instant::now();
        let paths: Vec<String> = hid_paths()?
            .into_iter()
            .filter(|p| ids_from_path(p).is_some_and(|id| self.wanted.contains(&id)))
            .filter(|p| {
                let usable =
                    !p.to_lowercase().contains(BLUETOOTH_HID_SERVICE) || bluetooth_connected(p);
                if !usable {
                    tracing::debug!("HID interface {p}: Bluetooth not connected, skipped");
                }
                usable
            })
            .collect();
        // forget what is gone
        self.known.retain(|p, _| paths.contains(p));
        self.skipped.retain(|p, _| paths.contains(p));
        let now = Instant::now();
        let mut found = Vec::new();
        let mut opening = Vec::new();
        for path in paths {
            if let Some(info) = self.known.get(&path) {
                found.push(info.clone());
            } else if !self.skipped.get(&path).is_some_and(|at| now < *at) {
                let rx = interface_info(&path);
                opening.push((path, rx));
            }
        }
        // new interfaces: all opened in parallel, one time limit for all
        let deadline = now + INFO_TIMEOUT;
        for (path, rx) in opening {
            match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Ok(info)) => {
                    self.known.insert(path, info.clone());
                    found.push(info);
                }
                Ok(Err(e)) => {
                    // e.g. keyboard collections Windows keeps for itself
                    tracing::debug!("HID interface {path} skipped: {e:#}");
                    self.skipped.insert(path, now + INFO_RETRY);
                }
                Err(_) => {
                    tracing::debug!("HID interface {path} too slow (stale Bluetooth?), skipped");
                    self.skipped.insert(path, now + INFO_RETRY);
                }
            }
        }
        tracing::debug!("HID enumeration took {:?}", started.elapsed());
        Ok(found)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_come_from_usb_and_bluetooth_paths() {
        assert_eq!(
            ids_from_path(
                r"\\?\HID#VID_6D67&PID_016C&MI_01#a&14b42412&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            ),
            Some((0x6D67, 0x016C))
        );
        assert_eq!(
            ids_from_path(
                r"\\?\HID#{00001812-0000-1000-8000-00805f9b34fb}_Dev_VID&016d67_PID&016c&Col02#b&1&0&0001#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            ),
            Some((0x6D67, 0x016C))
        );
        assert_eq!(
            ids_from_path(
                r"\\?\hid#vid_6d66&pid_8572&mi_01#b&17b130a2&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            ),
            Some((0x6D66, 0x8572))
        );
        assert_eq!(
            ids_from_path(r"\\?\HID#ConvertedDevice&Col01#5&1#{4d1e55b2}"),
            None
        );
    }
}
