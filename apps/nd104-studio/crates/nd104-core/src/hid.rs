//! HID access behind small traits so the device manager can be tested
//! without hardware.

#[cfg(windows)]
use std::collections::HashMap;
#[cfg(not(windows))]
use std::ffi::CString;
#[cfg(windows)]
use std::sync::mpsc;
use std::time::Duration;
#[cfg(windows)]
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
#[cfg(not(windows))]
use hidapi::HidApi;

/// How an interface is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    Usb,
    Bluetooth,
    Unknown,
}

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
    /// Windows: from the device path; Linux: from hidraw (hidapi).
    pub bus: Bus,
}

/// Bluetooth "HID over GATT" service: part of the Windows device path of
/// every HID interface that is connected over Bluetooth instead of USB.
#[cfg_attr(not(windows), allow(dead_code))]
const BLUETOOTH_HID_SERVICE: &str = "{00001812-0000-1000-8000-00805f9b34fb}";

/// The bus of a Windows HID device path.
#[cfg_attr(not(windows), allow(dead_code))]
fn bus_from_path(path: &str) -> Bus {
    if path.to_lowercase().contains(BLUETOOTH_HID_SERVICE) {
        Bus::Bluetooth
    } else {
        Bus::Usb
    }
}

impl HidInterfaceInfo {
    /// Connected over Bluetooth? The ND104 reports the same IDs and usages
    /// there, but its vendor collection uses report ID 2 (docs/protocol.md
    /// § 5.5), so it must not be mixed up with the USB interface.
    pub fn is_bluetooth(&self) -> bool {
        self.bus == Bus::Bluetooth
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
#[cfg(windows)]
const INFO_TIMEOUT: Duration = Duration::from_secs(3);
/// When to try an interface again that could not be opened.
#[cfg(windows)]
const INFO_RETRY: Duration = Duration::from_secs(30);

/// Real backend.
///
/// On Windows it does not use hidapi at all: hidapi's context always scans
/// every HID device of the PC when it is created (`HidApi::new` calls
/// `add_devices(0, 0)`, also via `new_without_enumerate`), which takes
/// minutes while Windows answers slowly (e.g. while OpenRGB scans). Instead
/// it takes the HID device paths from Windows, reads vendor/product ID from
/// the path, reads the details of only the interfaces of supported
/// keyboards and receivers, and talks to them with Windows' HID functions
/// ([`win::WinHid`]). Elsewhere it uses hidapi.
pub struct HidapiBackend {
    #[cfg(not(windows))]
    api: HidApi,
    /// Vendor/product IDs worth opening (keyboards, screens, receivers).
    wanted: Vec<(u16, u16)>,
    /// Details of interfaces opened before, by path.
    #[cfg(windows)]
    known: HashMap<String, HidInterfaceInfo>,
    /// Interfaces that were too slow or failed, and when to try again.
    #[cfg(windows)]
    skipped: HashMap<String, Instant>,
    /// Interfaces whose details did not come in time: their answer is still
    /// awaited (not asked again), e.g. while another program scans every HID
    /// device of the PC and Windows answers slowly.
    #[cfg(windows)]
    pending: HashMap<String, mpsc::Receiver<Result<HidInterfaceInfo>>>,
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
            // on Linux hidapi reads sysfs: creating it is cheap (no slow
            // scan as on Windows)
            #[cfg(not(windows))]
            api: HidApi::new().context("cannot initialise HID API")?,
            wanted,
            #[cfg(windows)]
            known: HashMap::new(),
            #[cfg(windows)]
            skipped: HashMap::new(),
            #[cfg(windows)]
            pending: HashMap::new(),
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

/// Reads one interface's details in a helper thread, so that a device that
/// does not answer cannot hold up the others; a receiver that stays empty
/// means too slow (the thread finishes on its own later).
#[cfg(windows)]
fn interface_info(path: &str) -> mpsc::Receiver<Result<HidInterfaceInfo>> {
    let (tx, rx) = mpsc::channel();
    let path = path.to_owned();
    std::thread::spawn(move || {
        let _ = tx.send(read_interface_info(path));
    });
    rx
}

/// Interface number from the path (`…&MI_01…`), -1 without one (Windows
/// paths; on Linux hidapi reports it).
#[cfg(windows)]
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
        bus: bus_from_path(&path),
        path,
        product: None,
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

/// The report ID of one top-level collection (`usage_page:usage`) in a
/// report descriptor: the first Report ID item (`85 nn`) inside that
/// collection, 0 without one.
///
/// On Linux one hidraw device covers a whole USB interface, so its
/// descriptor holds several collections – the ND104's interface 4 has five
/// (report IDs 03, 06, 02, 04 and 05 for the lighting channel), and over
/// Bluetooth the keyboard collections come before the command channel
/// (ID 2). Taking the first ID of the whole descriptor would pick the wrong
/// one. Walks the items properly (no data byte is mistaken for an item).
#[cfg_attr(windows, allow(dead_code))]
pub fn report_id_for_collection(desc: &[u8], usage_page: u16, usage: u16) -> u8 {
    let mut i = 0;
    let mut page = 0u16;
    let mut last_usage: Option<u16> = None;
    let mut depth = 0u32;
    let mut inside = false;
    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // long item: 0xFE, size, tag, data
            let size = usize::from(*desc.get(i + 1).unwrap_or(&0));
            i += 3 + size;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => usize::from(n),
        };
        let data = desc.get(i + 1..i + 1 + size).unwrap_or(&[]);
        let value = data
            .iter()
            .rev()
            .fold(0u32, |v, &b| (v << 8) | u32::from(b));
        match prefix & 0xFC {
            // Usage Page (global)
            0x04 => page = value as u16,
            // Usage (local); a 4-byte usage carries its own page
            0x08 => {
                last_usage = Some(if size == 4 {
                    (value & 0xFFFF) as u16
                } else {
                    value as u16
                });
                if size == 4 {
                    page = (value >> 16) as u16;
                }
            }
            // Collection
            0xA0 => {
                if depth == 0 {
                    inside = page == usage_page && last_usage == Some(usage);
                }
                depth += 1;
                last_usage = None;
            }
            // End Collection
            0xC0 => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    inside = false;
                }
            }
            // Report ID (global)
            0x84 if inside && size >= 1 => return data[0],
            // Main items (Input/Output/Feature) end the local usages
            0x80 | 0x90 | 0xB0 => last_usage = None,
            _ => {}
        }
        i += 1 + size;
    }
    0
}

impl HidBackend for HidapiBackend {
    /// Linux and others: hidapi lists the hidraw devices with all details
    /// (IDs, usage, interface number, bus) – only the supported ones are kept.
    #[cfg(not(windows))]
    fn enumerate(&mut self) -> Result<Vec<HidInterfaceInfo>> {
        self.api
            .refresh_devices()
            .context("HID enumeration failed")?;
        Ok(self
            .api
            .device_list()
            .filter(|d| self.wanted.contains(&(d.vendor_id(), d.product_id())))
            .map(|d| HidInterfaceInfo {
                vid: d.vendor_id(),
                pid: d.product_id(),
                usage_page: d.usage_page(),
                usage: d.usage(),
                interface_number: d.interface_number(),
                path: d.path().to_string_lossy().into_owned(),
                product: d.product_string().map(str::to_owned),
                bus: match d.bus_type() {
                    hidapi::BusType::Usb => Bus::Usb,
                    hidapi::BusType::Bluetooth => Bus::Bluetooth,
                    _ => Bus::Unknown,
                },
            })
            .collect())
    }

    #[cfg(windows)]
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
        self.pending.retain(|p, _| paths.contains(p));
        let now = Instant::now();
        let mut found = Vec::new();
        let mut opening = Vec::new();
        for path in paths {
            if let Some(info) = self.known.get(&path) {
                found.push(info.clone());
            } else if let Some(rx) = self.pending.remove(&path) {
                // still waiting for the earlier request
                opening.push((path, rx));
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
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    tracing::debug!("HID interface {path}: Windows answers slowly, still waiting");
                    self.pending.insert(path, rx);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.skipped.insert(path, now + INFO_RETRY);
                }
            }
        }
        tracing::debug!("HID enumeration took {:?}", started.elapsed());
        Ok(found)
    }

    #[cfg(windows)]
    fn open(&mut self, info: &HidInterfaceInfo) -> Result<Box<dyn HidChannel>> {
        Ok(Box::new(win::WinHid::open(&info.path).with_context(
            || format!("cannot open {:04x}:{:04x}", info.vid, info.pid),
        )?))
    }

    #[cfg(not(windows))]
    fn open(&mut self, info: &HidInterfaceInfo) -> Result<Box<dyn HidChannel>> {
        let path = CString::new(info.path.clone()).map_err(|_| anyhow!("invalid HID path"))?;
        let device = self
            .api
            .open_path(&path)
            .with_context(|| format!("cannot open {:04x}:{:04x}", info.vid, info.pid))?;
        // the collection's report ID from its descriptor: none for the
        // command channel over USB, 2 over Bluetooth, 5 for the lighting
        // channel (docs/protocol.md § 5.4, § 5.5). The hidraw descriptor
        // can hold several collections – take the one being opened.
        let mut desc = [0u8; 4096];
        let len = device
            .get_report_descriptor(&mut desc)
            .context("cannot read the report descriptor")?;
        let report_id = report_id_for_collection(&desc[..len], info.usage_page, info.usage);
        Ok(Box::new(HidapiChannel { device, report_id }))
    }
}

/// Windows' own HID functions, without hidapi (see [`HidapiBackend`]).
#[cfg(windows)]
pub mod win {
    use std::time::Duration;

    use anyhow::{anyhow, Context, Result};
    use windows::core::HSTRING;
    use windows::Win32::Devices::HumanInterfaceDevice::{
        HidD_FreePreparsedData, HidD_GetPreparsedData, HidP_GetCaps, HidP_GetValueCaps,
        HidP_Output, HIDP_CAPS, HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS, PHIDP_PREPARSED_DATA,
    };
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE, WAIT_OBJECT_0,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };
    use windows::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject};
    use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

    /// How long a write may take (as in hidapi).
    const WRITE_TIMEOUT_MS: u32 = 1000;

    /// One opened HID collection.
    pub struct WinHid {
        handle: HANDLE,
        event: HANDLE,
        /// Report lengths including the report ID byte.
        input_len: usize,
        output_len: usize,
        /// 0 if the collection has no report ID.
        report_id: u8,
    }

    // The handles may be used from another thread (one user at a time).
    unsafe impl Send for WinHid {}

    impl WinHid {
        pub fn open(path: &str) -> Result<Self> {
            let handle = unsafe {
                CreateFileW(
                    &HSTRING::from(path),
                    (GENERIC_READ | GENERIC_WRITE).0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED,
                    None,
                )
            }
            .context("CreateFile")?;
            let caps = unsafe { caps(handle) };
            let event = unsafe { CreateEventW(None, true, false, None) };
            match (caps, event) {
                (Some((caps, report_id)), Ok(event)) => Ok(Self {
                    handle,
                    event,
                    input_len: usize::from(caps.InputReportByteLength),
                    output_len: usize::from(caps.OutputReportByteLength),
                    report_id,
                }),
                (_, event) => {
                    unsafe {
                        let _ = CloseHandle(handle);
                        if let Ok(e) = event {
                            let _ = CloseHandle(e);
                        }
                    }
                    Err(anyhow!("no HID capabilities"))
                }
            }
        }

        /// The report ID this collection's reports carry (0: none).
        pub fn report_id(&self) -> u8 {
            self.report_id
        }

        /// Runs one overlapped transfer, waiting up to `timeout_ms`; `None`
        /// on timeout (the transfer is cancelled and waited for).
        fn transfer(
            &self,
            timeout_ms: u32,
            start: impl FnOnce(*mut OVERLAPPED) -> windows::core::Result<()>,
        ) -> Result<Option<usize>> {
            let mut ov = OVERLAPPED {
                hEvent: self.event,
                ..Default::default()
            };
            unsafe {
                let _ = ResetEvent(self.event);
                if let Err(e) = start(&mut ov) {
                    if e.code() != ERROR_IO_PENDING.to_hresult() {
                        return Err(anyhow!(e));
                    }
                }
                let mut done = 0u32;
                if WaitForSingleObject(self.event, timeout_ms) != WAIT_OBJECT_0 {
                    let _ = CancelIoEx(self.handle, Some(&ov));
                    // the cancelled transfer must finish before `ov` goes away
                    let _ = GetOverlappedResult(self.handle, &ov, &mut done, true);
                    return Ok(None);
                }
                GetOverlappedResult(self.handle, &ov, &mut done, false)?;
                Ok(Some(done as usize))
            }
        }
    }

    /// Report lengths and the output report ID from Windows' parsed
    /// descriptor (nothing is sent to the device).
    unsafe fn caps(handle: HANDLE) -> Option<(HIDP_CAPS, u8)> {
        let mut data = PHIDP_PREPARSED_DATA::default();
        if !HidD_GetPreparsedData(handle, &mut data) {
            return None;
        }
        let mut caps = HIDP_CAPS::default();
        let ok = HidP_GetCaps(data, &mut caps) == HIDP_STATUS_SUCCESS;
        let mut report_id = 0;
        if ok && caps.NumberOutputValueCaps > 0 {
            let mut values =
                vec![HIDP_VALUE_CAPS::default(); usize::from(caps.NumberOutputValueCaps)];
            let mut n = caps.NumberOutputValueCaps;
            if HidP_GetValueCaps(HidP_Output, values.as_mut_ptr(), &mut n, data)
                == HIDP_STATUS_SUCCESS
                && n > 0
            {
                report_id = values[0].ReportID;
            }
        }
        HidD_FreePreparsedData(data);
        ok.then_some((caps, report_id))
    }

    impl super::HidChannel for WinHid {
        fn write(&mut self, report: &[u8]) -> Result<()> {
            // Windows wants exactly the output report length, ID in front
            let mut buf = vec![0u8; self.output_len.max(1)];
            buf[0] = self.report_id;
            let n = report.len().min(buf.len() - 1);
            buf[1..=n].copy_from_slice(&report[..n]);
            match self
                .transfer(WRITE_TIMEOUT_MS, |ov| unsafe {
                    WriteFile(self.handle, Some(&buf), None, Some(ov))
                })
                .context("HID write failed")?
            {
                Some(_) => Ok(()),
                None => Err(anyhow!("HID write failed: no answer within 1 s")),
            }
        }

        fn read(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
            let mut buf = vec![0u8; self.input_len.max(1)];
            let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
            let ptr = buf.as_mut_ptr();
            let len = buf.len();
            let n = self
                .transfer(ms, |ov| unsafe {
                    // `buf` outlives the transfer (it is cancelled and waited for)
                    ReadFile(
                        self.handle,
                        Some(std::slice::from_raw_parts_mut(ptr, len)),
                        None,
                        Some(ov),
                    )
                })
                .context("HID read failed")?;
            // Windows always puts the report ID (or 0) in front
            Ok(n.filter(|&n| n > 1).map(|n| buf[1..n].to_vec()))
        }
    }

    impl Drop for WinHid {
        fn drop(&mut self) {
            unsafe {
                let _ = CancelIoEx(self.handle, None);
                let _ = CloseHandle(self.handle);
                let _ = CloseHandle(self.event);
            }
        }
    }
}

#[cfg(not(windows))]
struct HidapiChannel {
    device: hidapi::HidDevice,
    /// 0 over USB; the collection's report ID over Bluetooth.
    report_id: u8,
}

#[cfg(not(windows))]
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
        let deadline = std::time::Instant::now() + timeout;
        let mut buf = [0u8; 65];
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let ms = i32::try_from(left.as_millis()).unwrap_or(i32::MAX);
            let n = self
                .device
                .read_timeout(&mut buf, ms)
                .context("HID read failed")?;
            if n == 0 {
                return Ok(None);
            }
            // another collection of the same hidraw device (e.g. a key
            // press): never passed on or logged
            if let Some(report) = own_report(&buf[..n], self.report_id) {
                return Ok(Some(report.to_vec()));
            }
            if left.is_zero() {
                return Ok(None);
            }
        }
    }
}

/// The part of a report read from a hidraw device that belongs to the opened
/// collection, without its report ID; `None` for reports of other
/// collections. On Linux one hidraw device carries every collection of a
/// USB interface (over Bluetooth: of the whole keyboard), so key presses
/// (report ID 1) arrive next to the command channel's answers (ID 2) –
/// they must not be taken for answers. A collection without report ID is
/// alone on its device (the command channel over USB).
#[cfg_attr(windows, allow(dead_code))]
fn own_report(report: &[u8], report_id: u8) -> Option<&[u8]> {
    match report.split_first() {
        _ if report_id == 0 => Some(report),
        Some((&id, data)) if id == report_id => Some(data),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udev_rules_cover_every_supported_device() {
        let rules = [
            include_str!("../../../linux/70-nd104-studio.rules"),
            include_str!("../../../linux/70-nd104-studio-plugdev.rules"),
        ];
        for model in crate::model::registry() {
            for spec in model.interfaces().iter().chain(model.receiver()) {
                let id = format!("*:{:04X}:{:04X}.*", spec.vid, spec.pid);
                for r in rules {
                    assert!(r.contains(&id), "udev rule for {id} missing");
                }
            }
        }
    }

    #[test]
    fn reports_of_other_collections_are_dropped() {
        // Bluetooth: a key press (ID 1) next to the command channel (ID 2)
        assert_eq!(own_report(&[0x01, 0x00, 0x00, 0x18], 2), None);
        assert_eq!(own_report(&[0x02, 0x11, 0x02], 2), Some(&[0x11, 0x02][..]));
        // lighting channel (ID 5) on interface 4 next to the mouse (ID 3)
        assert_eq!(own_report(&[0x03, 0x01], 5), None);
        // command channel over USB: no report ID, the report as it is
        assert_eq!(own_report(&[0x11, 0x02], 0), Some(&[0x11, 0x02][..]));
    }

    #[test]
    fn report_id_is_taken_from_the_right_collection() {
        // USB command channel alone: no report ID
        let usb = [
            0x06, 0x60, 0xFF, 0x09, 0x61, 0xA1, 0x01, 0x09, 0x62, 0x15, 0x00, 0x26, 0xFF, 0x00,
            0x95, 0x20, 0x75, 0x08, 0x81, 0x02, 0xC0,
        ];
        assert_eq!(report_id_for_collection(&usb, 0xFF60, 0x61), 0);
        // Linux hidraw of interface 4: system control (ID 3) first, then
        // the lighting channel (ID 5)
        let interface4 = [
            0x05, 0x01, 0x09, 0x80, 0xA1, 0x01, 0x85, 0x03, 0x19, 0x81, 0x29, 0x83, 0x15, 0x00,
            0x25, 0x01, 0x75, 0x01, 0x95, 0x03, 0x81, 0x02, 0x95, 0x05, 0x81, 0x01, 0xC0, //
            0x06, 0x59, 0xFF, 0x09, 0x60, 0xA1, 0x01, 0x85, 0x05, 0x09, 0x62, 0x15, 0x00, 0x26,
            0xFF, 0x00, 0x75, 0x08, 0x95, 0x3F, 0x81, 0x02, 0x09, 0x63, 0x91, 0x02, 0xC0,
        ];
        assert_eq!(report_id_for_collection(&interface4, 0xFF59, 0x60), 5);
        assert_eq!(report_id_for_collection(&interface4, 0x0001, 0x80), 3);
        assert_eq!(
            report_id_for_collection(&interface4, 0x000C, 0x01),
            0,
            "not there"
        );
        // nested collections and a data byte 0x85 do not confuse it
        let nested = [
            0x09, 0x85, 0x06, 0x60, 0xFF, 0x09, 0x61, 0xA1, 0x01, 0xA1, 0x02, 0x85, 0x02, 0xC0,
            0xC0,
        ];
        assert_eq!(report_id_for_collection(&nested, 0xFF60, 0x61), 2);
    }

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
