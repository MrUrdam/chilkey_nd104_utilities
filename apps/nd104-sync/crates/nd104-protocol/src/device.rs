/// USB vendor/product ID pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbId {
    pub vid: u16,
    pub pid: u16,
}

/// Keyboard MCU (keys, RGB, settings; also relays display commands).
pub const KEYBOARD: UsbId = UsbId { vid: 0x6D67, pid: 0x016C };
/// Display controller.
pub const SCREEN: UsbId = UsbId { vid: 0x5542, pid: 0x0001 };
/// Display controller in DFU (firmware update) mode. Never written to.
pub const SCREEN_DFU: UsbId = UsbId { vid: 0x5542, pid: 0x1000 };

/// Vendor HID collection used by the web driver on both devices.
pub const VENDOR_USAGE_PAGE: u16 = 0xFF60;
pub const VENDOR_USAGE: u16 = 0x61;

/// Which of the two USB devices a packet is sent to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Keyboard,
    Screen,
}

impl Target {
    pub fn usb_id(self) -> UsbId {
        match self {
            Target::Keyboard => KEYBOARD,
            Target::Screen => SCREEN,
        }
    }

    /// HID report length (without report ID) as used by the web driver.
    pub fn report_len(self) -> usize {
        match self {
            Target::Keyboard => 32,
            Target::Screen => 64,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Target::Keyboard => "keyboard",
            Target::Screen => "screen",
        }
    }
}
