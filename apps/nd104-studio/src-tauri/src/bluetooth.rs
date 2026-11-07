//! Is the keyboard connected over Bluetooth, and what battery level does it
//! report? Read from the system – Windows' device properties (the values of
//! "Settings → Bluetooth & devices"), on Linux BlueZ over D-Bus. Nothing is
//! sent to the keyboard.
//!
//! The ND104 shows up as one Bluetooth LE device per slot ("ND104 BT1" …
//! "ND104 BT3"); its GATT services carry the keyboard's USB IDs, e.g.
//! `BTHLEDEVICE\{…}_DEV_VID&016D67_PID&016C…`. The battery level comes from
//! the standard Bluetooth battery service.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BluetoothKeyboard {
    /// Name as Windows shows it, e.g. "ND104 BT2".
    pub name: String,
    /// Bluetooth slot 1–3, read from the name.
    pub slot: Option<u8>,
    /// Battery level in percent, if the keyboard reports one.
    pub battery: Option<u8>,
}

/// The keyboard if it is connected over Bluetooth right now.
#[tauri::command]
pub fn bluetooth_status() -> Option<BluetoothKeyboard> {
    imp::find()
}

/// "ND104 BT2" → 2.
#[cfg(any(windows, target_os = "linux", test))]
fn slot_of(name: &str) -> Option<u8> {
    let at = name.rfind("BT")?;
    let digit = name[at + 2..].trim().chars().next()?.to_digit(10)?;
    u8::try_from(digit).ok().filter(|d| (1..=9).contains(d))
}

#[cfg(windows)]
mod imp {
    use super::{slot_of, BluetoothKeyboard};
    use nd104_core::model::{self, Role};
    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_DevNode_PropertyW, CM_Get_Device_ID_ListW, CM_Get_Device_ID_List_SizeW,
        CM_Get_Parent, CM_Locate_DevNodeW, CM_GETIDLIST_FILTER_ENUMERATOR,
        CM_GETIDLIST_FILTER_PRESENT, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
    };
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_FriendlyName, DEVPROPTYPE};
    use windows::Win32::Foundation::DEVPROPKEY;

    /// Battery level (byte, percent) of a Bluetooth device.
    const BATTERY: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5),
        pid: 2,
    };
    /// "Connected right now" (boolean); paired devices stay listed when they
    /// are not connected.
    const CONNECTED: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0x83da6326_97a6_4088_9453_a1923f573b29),
        pid: 15,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Present devices of one enumerator ("BTHLEDEVICE").
    fn device_ids(enumerator: &str) -> Vec<String> {
        let filter = wide(enumerator);
        let flags = CM_GETIDLIST_FILTER_ENUMERATOR | CM_GETIDLIST_FILTER_PRESENT;
        let mut len = 0u32;
        unsafe {
            if CM_Get_Device_ID_List_SizeW(&mut len, PCWSTR(filter.as_ptr()), flags) != CR_SUCCESS {
                return Vec::new();
            }
            let mut buf = vec![0u16; len as usize];
            if CM_Get_Device_ID_ListW(PCWSTR(filter.as_ptr()), &mut buf, flags) != CR_SUCCESS {
                return Vec::new();
            }
            buf.split(|&c| c == 0)
                .filter(|s| !s.is_empty())
                .map(String::from_utf16_lossy)
                .collect()
        }
    }

    fn property(devinst: u32, key: &DEVPROPKEY) -> Option<Vec<u8>> {
        let mut kind = DEVPROPTYPE::default();
        let mut size = 0u32;
        unsafe {
            // first call: size only
            let _ = CM_Get_DevNode_PropertyW(devinst, key, &mut kind, None, &mut size, 0);
            if size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            if CM_Get_DevNode_PropertyW(
                devinst,
                key,
                &mut kind,
                Some(buf.as_mut_ptr()),
                &mut size,
                0,
            ) != CR_SUCCESS
            {
                return None;
            }
            buf.truncate(size as usize);
            Some(buf)
        }
    }

    fn text(devinst: u32, key: &DEVPROPKEY) -> Option<String> {
        let bytes = property(devinst, key)?;
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        Some(String::from_utf16_lossy(&units))
    }

    pub fn find() -> Option<BluetoothKeyboard> {
        // GATT services of the keyboard carry its USB IDs
        let ids: Vec<String> = model::registry()
            .iter()
            .flat_map(|m| m.interfaces())
            .filter(|i| i.role == Role::Keyboard)
            .map(|i| format!("VID&01{:04X}_PID&{:04X}", i.vid, i.pid))
            .collect();
        let mut parents = Vec::new();
        for id in device_ids("BTHLEDEVICE") {
            let upper = id.to_uppercase();
            if !ids.iter().any(|want| upper.contains(want)) {
                continue;
            }
            let wid = wide(&id);
            let mut node = 0u32;
            let mut parent = 0u32;
            unsafe {
                if CM_Locate_DevNodeW(&mut node, PCWSTR(wid.as_ptr()), CM_LOCATE_DEVNODE_NORMAL)
                    != CR_SUCCESS
                    || CM_Get_Parent(&mut parent, node, 0) != CR_SUCCESS
                {
                    continue;
                }
            }
            if !parents.contains(&parent) {
                parents.push(parent);
            }
        }
        // one device per Bluetooth slot; take the connected one
        parents.into_iter().find_map(|dev| {
            let connected = property(dev, &CONNECTED).map(|b| b.first() == Some(&0xFF));
            if connected == Some(false) {
                return None;
            }
            let name = text(dev, &DEVPKEY_Device_FriendlyName).unwrap_or_default();
            let battery = property(dev, &BATTERY)
                .and_then(|b| b.first().copied())
                .filter(|&p| p <= 100);
            Some(BluetoothKeyboard {
                slot: slot_of(&name),
                name,
                battery,
            })
        })
    }
}

/// One Bluetooth device as BlueZ describes it.
#[cfg(any(target_os = "linux", test))]
struct BlueZDevice {
    name: String,
    /// E.g. `usb:v6D67p016Cd0130` – the keyboard's USB IDs.
    modalias: String,
    connected: bool,
    battery: Option<u8>,
}

/// The connected ND104 among BlueZ's devices (one per Bluetooth slot).
#[cfg(any(target_os = "linux", test))]
fn pick_bluez(devices: Vec<BlueZDevice>, ids: &[(u16, u16)]) -> Option<BluetoothKeyboard> {
    let wanted: Vec<String> = ids
        .iter()
        .map(|(vid, pid)| format!("V{vid:04X}P{pid:04X}"))
        .collect();
    devices.into_iter().find_map(|d| {
        let modalias = d.modalias.to_uppercase();
        if !d.connected || !wanted.iter().any(|w| modalias.contains(w.as_str())) {
            return None;
        }
        Some(BluetoothKeyboard {
            slot: slot_of(&d.name),
            battery: d.battery.filter(|&p| p <= 100),
            name: d.name,
        })
    })
}

/// Linux: BlueZ on the system bus (`org.bluez.Device1`, `org.bluez.Battery1`).
#[cfg(target_os = "linux")]
mod imp {
    use std::time::Duration;

    use dbus::arg::prop_cast;
    use dbus::blocking::stdintf::org_freedesktop_dbus::ObjectManager;
    use dbus::blocking::Connection;
    use nd104_core::model::{self, Role};

    use super::{pick_bluez, BlueZDevice, BluetoothKeyboard};

    pub fn find() -> Option<BluetoothKeyboard> {
        let connection = Connection::new_system().ok()?;
        let proxy = connection.with_proxy("org.bluez", "/", Duration::from_millis(500));
        let objects = proxy.get_managed_objects().ok()?;
        let devices = objects
            .values()
            .filter_map(|interfaces| {
                let device = interfaces.get("org.bluez.Device1")?;
                let text = |key: &str| prop_cast::<String>(device, key).cloned();
                Some(BlueZDevice {
                    name: text("Alias").or_else(|| text("Name")).unwrap_or_default(),
                    modalias: text("Modalias").unwrap_or_default(),
                    connected: prop_cast::<bool>(device, "Connected")
                        .copied()
                        .unwrap_or(false),
                    battery: interfaces
                        .get("org.bluez.Battery1")
                        .and_then(|b| prop_cast::<u8>(b, "Percentage").copied()),
                })
            })
            .collect();
        let ids: Vec<(u16, u16)> = model::registry()
            .iter()
            .flat_map(|m| m.interfaces())
            .filter(|i| i.role == Role::Keyboard)
            .map(|i| (i.vid, i.pid))
            .collect();
        pick_bluez(devices, &ids)
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn find() -> Option<super::BluetoothKeyboard> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{pick_bluez, slot_of, BlueZDevice};

    #[test]
    fn bluez_finds_the_connected_keyboard() {
        let dev = |name: &str, modalias: &str, connected: bool| BlueZDevice {
            name: name.into(),
            modalias: modalias.into(),
            connected,
            battery: Some(87),
        };
        let ids = [(0x6D67, 0x016C)];
        // paired but not connected (other slot), a mouse, the connected keyboard
        let found = pick_bluez(
            vec![
                dev("ND104 BT1", "usb:v6D67p016Cd0130", false),
                dev("Mouse", "usb:v046Dp4082d0001", true),
                dev("ND104 BT2", "usb:v6D67p016Cd0130", true),
            ],
            &ids,
        )
        .unwrap();
        assert_eq!((found.slot, found.battery), (Some(2), Some(87)));
        assert!(pick_bluez(vec![dev("ND104 BT1", "usb:v6D67p016Cd0130", false)], &ids).is_none());
    }

    #[test]
    fn slot_from_name() {
        assert_eq!(slot_of("ND104 BT2"), Some(2));
        assert_eq!(slot_of("ND104 BT 3"), Some(3));
        assert_eq!(slot_of("ND104"), None);
    }
}
