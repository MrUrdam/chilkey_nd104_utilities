//! Is the keyboard connected over Bluetooth, and what battery level does it
//! report? Read from Windows' device properties – the same values as in
//! "Settings → Bluetooth & devices". Nothing is sent to the keyboard.
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

#[cfg(not(windows))]
mod imp {
    pub fn find() -> Option<super::BluetoothKeyboard> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::slot_of;

    #[test]
    fn slot_from_name() {
        assert_eq!(slot_of("ND104 BT2"), Some(2));
        assert_eq!(slot_of("ND104 BT 3"), Some(3));
        assert_eq!(slot_of("ND104"), None);
    }
}
