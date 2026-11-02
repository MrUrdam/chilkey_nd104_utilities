//! Device manager with the firmware/protocol guard.
//!
//! Checks:
//! - P1 all interfaces of the model present (wired USB)
//! - P2/P4 reports have the expected length, writes are accepted
//! - P3 firmware version queries answered well-formed
//! - P5 every packet acknowledged with status OK; damaged or structurally
//!   unknown reports lock, well-formed unknown messages are logged and ignored
//! - P6 screen variant recognised where possible (informational)
//!
//! Lighting and the own picture are optional extras: an unsupported command
//! or a missing answer is reported to the caller without locking; damaged
//! reports lock like everywhere else.
//!
//! Any failed check locks the device until it is re-plugged or re-checked.

use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use crate::hid::{HidBackend, HidChannel, HidInterfaceInfo};
use crate::keyboard_settings::{KeyboardSetting, KeyboardSettings, SettingsOp};
use crate::keymap::{Keymap, KeymapOp, KeymapResult};
use crate::lighting::{LightChange, LightField, LightOp, LightZone, ZoneState};
use crate::model::KeymapReply;
use crate::model::{
    FirmwareInfo, KeyboardModel, Packet, PictureTransfer, ReportClass, Role, ScreenVariant,
    TransferAck,
};

const ANSWER_TIMEOUT: Duration = Duration::from_millis(1000);
/// The screen writes each page to its flash before it answers.
const PAGE_TIMEOUT: Duration = Duration::from_secs(5);
/// Pause between data reports (the web driver uses 2 ms).
const REPORT_GAP: Duration = Duration::from_millis(2);
const PAGE_RETRIES: usize = 3;
/// After the last page the screen usually stays silent (the web driver does
/// not wait for an answer either); a short wait catches a "send again".
const LAST_PAGE_TIMEOUT: Duration = Duration::from_millis(1500);
const FIRMWARE_RETRIES: usize = 3;

/// How far the connected firmware is trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trust {
    /// Version verified on hardware.
    Verified,
    /// Other version, all protocol checks passed so far.
    Compatible,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectedDevice {
    /// USB cable (keyboard and screen) or Bluetooth (keyboard only).
    pub connection: Connection,
    pub model_id: &'static str,
    pub model_name: &'static str,
    pub firmware: Vec<FirmwareInfo>,
    pub trust: Trust,
    pub screen_variant: ScreenVariant,
}

/// The device did not answer in time (as opposed to a wrong answer).
#[derive(Debug)]
struct NoAnswer(String);

impl std::fmt::Display for NoAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NoAnswer {}

/// The device answered something it must not (locks also over Bluetooth).
#[derive(Debug)]
struct WrongAnswer(String);

impl std::fmt::Display for WrongAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for WrongAnswer {}

/// Reading or writing the HID channel failed: over Bluetooth this means the
/// link is gone (keyboard asleep or out of range), not a protocol problem.
#[derive(Debug)]
struct LinkLost(String);

impl std::fmt::Display for LinkLost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LinkLost {}

/// How the keyboard is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Connection {
    /// Keyboard and screen controller over the USB cable: everything works.
    Usb,
    /// Keyboard over Bluetooth (report ID 2): time, weather, lighting,
    /// keymap and settings; the screen controller (own picture, screen
    /// version) is cable-only.
    Bluetooth,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeviceState {
    /// No supported keyboard found (USB cable required).
    NotConnected,
    /// Connected and all checks passed.
    Ready(ConnectedDevice),
    /// A check failed; nothing is sent until re-plug or re-check.
    Locked {
        model_name: Option<&'static str>,
        firmware: Vec<FirmwareInfo>,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventLevel {
    Info,
    Warning,
    Error,
}

/// Something worth showing in the "recent events" list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEvent {
    pub level: EventLevel,
    pub message: String,
}

struct Active {
    model: usize,
    connection: Connection,
    channels: HashMap<Role, (Box<dyn HidChannel>, usize)>,
}

pub struct DeviceManager {
    backend: Box<dyn HidBackend>,
    models: Vec<Box<dyn KeyboardModel>>,
    active: Option<Active>,
    state: DeviceState,
    events: Vec<DeviceEvent>,
    /// Screen firmware last verified over USB. Over Bluetooth the screen
    /// cannot be asked, so this is what its check relies on.
    known_screen: Option<FirmwareInfo>,
    /// Where `known_screen` is kept between runs (none in tests).
    screen_memory: Option<std::path::PathBuf>,
    /// The connection a lock happened on; the lock ends when that
    /// connection goes away (re-plug), e.g. by switching to the cable.
    locked_over: Option<Connection>,
    /// Over Bluetooth, waiting for one USB connection (screen unknown);
    /// said once in the events, not every poll.
    waiting_for_usb: bool,
}

impl DeviceManager {
    pub fn new(backend: Box<dyn HidBackend>, models: Vec<Box<dyn KeyboardModel>>) -> Self {
        Self {
            backend,
            models,
            active: None,
            state: DeviceState::NotConnected,
            events: Vec::new(),
            known_screen: None,
            screen_memory: None,
            locked_over: None,
            waiting_for_usb: false,
        }
    }

    /// Keeps the screen firmware seen over USB in `path`, for Bluetooth.
    pub fn with_screen_memory(mut self, path: std::path::PathBuf) -> Self {
        self.known_screen = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        self.screen_memory = Some(path);
        self
    }

    /// The screen firmware a Bluetooth connection relies on (tests).
    pub fn set_known_screen(&mut self, firmware: Option<FirmwareInfo>) {
        self.known_screen = firmware;
    }

    fn remember_screen(&mut self, firmware: &[FirmwareInfo]) {
        let Some(screen) = firmware.iter().find(|f| f.role == Role::Screen) else {
            return;
        };
        if self.known_screen.as_ref() == Some(screen) {
            return;
        }
        self.known_screen = Some(screen.clone());
        if let Some(path) = &self.screen_memory {
            let saved = serde_json::to_string(screen)
                .map_err(anyhow::Error::from)
                .and_then(|json| {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    Ok(std::fs::write(path, json)?)
                });
            if let Err(e) = saved {
                tracing::warn!("cannot remember the screen firmware: {e:#}");
            }
        }
    }

    pub fn state(&self) -> &DeviceState {
        &self.state
    }

    pub fn take_events(&mut self) -> Vec<DeviceEvent> {
        std::mem::take(&mut self.events)
    }

    fn event(&mut self, level: EventLevel, message: impl Into<String>) {
        let message = message.into();
        match level {
            EventLevel::Info => tracing::info!("{message}"),
            EventLevel::Warning => tracing::warn!("{message}"),
            EventLevel::Error => tracing::error!("{message}"),
        }
        self.events.push(DeviceEvent { level, message });
    }

    fn lock(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        let (model_name, firmware) = match &self.state {
            DeviceState::Ready(d) => (Some(d.model_name), d.firmware.clone()),
            DeviceState::Locked {
                model_name,
                firmware,
                ..
            } => (*model_name, firmware.clone()),
            DeviceState::NotConnected => (
                self.active
                    .as_ref()
                    .map(|a| self.models[a.model].display_name()),
                Vec::new(),
            ),
        };
        self.event(EventLevel::Error, format!("Locked: {reason}"));
        self.locked_over = self.active.as_ref().map(|a| a.connection);
        self.active = None;
        self.state = DeviceState::Locked {
            model_name,
            firmware,
            reason,
        };
    }

    /// A check failed: locks – unless the Bluetooth link is just gone (the
    /// keyboard sleeps or is out of range). Then the connection is dropped
    /// and the next poll connects again with all checks.
    fn fail(&mut self, check: &str, e: &anyhow::Error) {
        if self.is_bluetooth() && e.downcast_ref::<LinkLost>().is_some() {
            self.active = None;
            self.state = DeviceState::NotConnected;
            self.event(
                EventLevel::Info,
                format!("Bluetooth connection lost (keyboard asleep?): {e:#}"),
            );
        } else {
            self.lock(format!("{check}: {e:#}"));
        }
    }

    /// Detects plug/unplug and connects. Returns `true` if the state changed.
    pub fn poll(&mut self) -> bool {
        let before = self.state.clone();
        let all = match self.backend.enumerate() {
            Ok(list) => list,
            Err(e) => {
                self.event(EventLevel::Error, format!("HID enumeration failed: {e:#}"));
                return false;
            }
        };
        // Over Bluetooth the keyboard shows the same IDs as over USB (with
        // report ID 2), so the two are kept apart: the USB connection only
        // looks at USB interfaces, the Bluetooth one only at Bluetooth ones.
        let (bluetooth, interfaces): (Vec<_>, Vec<_>) =
            all.into_iter().partition(HidInterfaceInfo::is_bluetooth);

        let found_usb = self.models.iter().position(|m| {
            m.interfaces().iter().any(|spec| {
                interfaces
                    .iter()
                    .any(|i| i.vid == spec.vid && i.pid == spec.pid)
            })
        });
        let usb_complete = found_usb.is_some_and(|m| self.all_interfaces_present(m, &interfaces));
        let found_bluetooth = self
            .models
            .iter()
            .position(|m| Self::bluetooth_keyboard(m.as_ref(), &bluetooth).is_some());

        // the cable wins; a partial USB device (e.g. only the screen on a
        // charging cable) does not block Bluetooth
        if let DeviceState::Ready(d) = &self.state {
            let stay = match d.connection {
                Connection::Usb => found_usb.is_some(),
                Connection::Bluetooth => found_bluetooth.is_some() && !usb_complete,
            };
            if !stay {
                self.active = None;
                self.state = DeviceState::NotConnected;
                self.event(EventLevel::Info, "Keyboard disconnected");
            }
        }
        // a lock ends when its connection goes away: unplugged, or
        // Bluetooth replaced by the cable – all checks run again
        if matches!(self.state, DeviceState::Locked { .. }) {
            let gone = match self.locked_over {
                Some(Connection::Usb) => found_usb.is_none(),
                Some(Connection::Bluetooth) => found_bluetooth.is_none() || usb_complete,
                None => false,
            };
            if gone {
                self.locked_over = None;
                self.state = DeviceState::NotConnected;
                self.event(EventLevel::Info, "Keyboard re-connected – checking again");
            }
        }
        if matches!(self.state, DeviceState::NotConnected) && !usb_complete {
            if let Some(model) = found_bluetooth {
                self.connect_bluetooth(model, &bluetooth);
                return self.state != before;
            }
        }
        let found = found_usb.or(found_bluetooth);

        match (found, &self.state) {
            (None, DeviceState::NotConnected) => {}
            (None, _) => {
                self.active = None;
                self.state = DeviceState::NotConnected;
                self.event(EventLevel::Info, "Keyboard disconnected");
            }
            // A lock only ends with re-plug or re-check.
            (Some(_), DeviceState::Locked { .. }) => {}
            (Some(_), DeviceState::Ready(d)) if d.connection == Connection::Bluetooth => {}
            (Some(model), DeviceState::Ready(_)) => {
                if !self.all_interfaces_present(model, &interfaces) {
                    self.active = None;
                    self.state = DeviceState::NotConnected;
                    self.event(EventLevel::Info, "Keyboard disconnected");
                }
            }
            (Some(model), DeviceState::NotConnected) if found_usb == Some(model) => {
                self.connect(model, &interfaces)
            }
            (Some(_), DeviceState::NotConnected) => {}
        }
        self.state != before
    }

    /// Forgets a lock and runs all checks again.
    pub fn recheck(&mut self) -> bool {
        self.active = None;
        self.waiting_for_usb = false;
        self.state = DeviceState::NotConnected;
        self.event(EventLevel::Info, "Re-checking keyboard");
        self.poll();
        true
    }

    fn all_interfaces_present(&self, model: usize, interfaces: &[HidInterfaceInfo]) -> bool {
        self.models[model]
            .interfaces()
            .iter()
            .all(|spec| find_interface(interfaces, spec).is_some())
    }

    fn connect(&mut self, model_idx: usize, interfaces: &[HidInterfaceInfo]) {
        let specs = self.models[model_idx].interfaces();
        let name = self.models[model_idx].display_name();
        self.active = Some(Active {
            model: model_idx,
            connection: Connection::Usb,
            channels: HashMap::new(),
        });

        // P1: every interface present
        for spec in specs {
            let Some(info) = find_interface(interfaces, spec) else {
                // Partial enumeration right after plug-in is normal; try again next poll.
                if interfaces
                    .iter()
                    .any(|i| i.vid == spec.vid && i.pid == spec.pid)
                {
                    self.lock(format!(
                        "P1: {name} {} interface {:04x}:{:04x} usage {:04x}:{:04x} not found",
                        spec.role, spec.vid, spec.pid, spec.usage_page, spec.usage
                    ));
                } else {
                    self.active = None;
                }
                return;
            };
            match self.backend.open(info) {
                Ok(channel) => {
                    if let Some(active) = self.active.as_mut() {
                        active
                            .channels
                            .insert(spec.role, (channel, spec.report_len));
                    }
                }
                Err(e) => {
                    self.lock(format!("P1: cannot open {name} {}: {e:#}", spec.role));
                    return;
                }
            }
        }

        // P3: firmware versions
        let queries = self.models[model_idx].firmware_queries();
        let mut firmware = Vec::new();
        for query in queries {
            match self.query_firmware(&query) {
                Ok(info) => firmware.push(info),
                Err(e) => {
                    self.lock(format!("P3: {e:#}"));
                    return;
                }
            }
        }

        let model = &self.models[model_idx];
        let verified = firmware.iter().all(|f| {
            model
                .verified_firmware()
                .iter()
                .any(|v| v.role == f.role && v.version == f.version)
        });
        let trust = if verified {
            Trust::Verified
        } else {
            Trust::Compatible
        };
        let screen_variant = model.screen_variant(&firmware);
        let device = ConnectedDevice {
            connection: Connection::Usb,
            model_id: model.id(),
            model_name: model.display_name(),
            firmware: firmware.clone(),
            trust,
            screen_variant,
        };
        let versions: Vec<String> = firmware
            .iter()
            .map(|f| format!("{} {}", f.role, f.raw))
            .collect();
        self.state = DeviceState::Ready(device);
        self.remember_screen(&firmware);
        match trust {
            Trust::Verified => self.event(
                EventLevel::Info,
                format!("{name} connected, firmware verified ({})", versions.join(", ")),
            ),
            Trust::Compatible => self.event(
                EventLevel::Warning,
                format!(
                    "{name} connected with unverified firmware ({}); running as compatible while all checks pass",
                    versions.join(", ")
                ),
            ),
        }
    }

    /// The keyboard's command channel among the Bluetooth interfaces.
    fn bluetooth_keyboard<'a>(
        model: &dyn KeyboardModel,
        bluetooth: &'a [HidInterfaceInfo],
    ) -> Option<(&'static crate::model::InterfaceSpec, &'a HidInterfaceInfo)> {
        let spec = model
            .interfaces()
            .iter()
            .find(|s| s.role == Role::Keyboard)?;
        Some((spec, find_interface(bluetooth, spec)?))
    }

    /// Connects over Bluetooth: keyboard only. Its firmware is read and
    /// checked like over USB; the screen cannot be asked, so its firmware
    /// must have been verified over USB before.
    fn connect_bluetooth(&mut self, model_idx: usize, bluetooth: &[HidInterfaceInfo]) {
        let model = &self.models[model_idx];
        let name = model.display_name();
        let Some((spec, info)) = Self::bluetooth_keyboard(model.as_ref(), bluetooth) else {
            return;
        };
        let report_len = spec.report_len;
        let channel = match self.backend.open(info) {
            Ok(c) => c,
            // the Bluetooth link may just be rebuilding: try again next poll
            Err(e) => {
                tracing::debug!("cannot open {name} over Bluetooth yet: {e:#}");
                return;
            }
        };
        let mut channels = HashMap::new();
        channels.insert(Role::Keyboard, (channel, report_len));
        self.active = Some(Active {
            model: model_idx,
            connection: Connection::Bluetooth,
            channels,
        });

        // P3: keyboard firmware, read now
        let queries: Vec<Packet> = self.models[model_idx]
            .firmware_queries()
            .into_iter()
            .filter(|q| q.role == Role::Keyboard)
            .collect();
        let mut firmware = Vec::new();
        for query in queries {
            match self.query_firmware(&query) {
                Ok(info) => firmware.push(info),
                // only a wrong answer locks; no answer or a failed write means
                // asleep or the link is still rebuilding: try again next poll
                Err(e) if e.downcast_ref::<WrongAnswer>().is_none() => {
                    tracing::debug!("{name} over Bluetooth: {e:#}");
                    self.active = None;
                    return;
                }
                Err(e) => {
                    self.lock(format!("P3 (Bluetooth): {e:#}"));
                    return;
                }
            }
        }
        // the screen as last verified over USB
        let model = &self.models[model_idx];
        let Some(screen) = self.known_screen.clone().filter(|s| {
            model
                .verified_firmware()
                .iter()
                .any(|v| v.role == Role::Screen && v.version == s.version)
        }) else {
            // not a protocol problem: wait (not connected) until the cable
            // was connected once
            self.active = None;
            if !self.waiting_for_usb {
                self.waiting_for_usb = true;
                self.event(
                    EventLevel::Warning,
                    "Bluetooth: the screen firmware cannot be checked over Bluetooth – \
                     connect the keyboard once with the USB cable, then Bluetooth works",
                );
            }
            return;
        };
        self.waiting_for_usb = false;
        firmware.push(screen);
        let verified = firmware.iter().all(|f| {
            model
                .verified_firmware()
                .iter()
                .any(|v| v.role == f.role && v.version == f.version)
        });
        let trust = if verified {
            Trust::Verified
        } else {
            Trust::Compatible
        };
        let device = ConnectedDevice {
            connection: Connection::Bluetooth,
            model_id: model.id(),
            model_name: model.display_name(),
            firmware: firmware.clone(),
            trust,
            screen_variant: model.screen_variant(&firmware),
        };
        self.state = DeviceState::Ready(device);
        let keyboard = firmware
            .iter()
            .find(|f| f.role == Role::Keyboard)
            .map_or("?", |f| f.raw.as_str());
        self.event(
            EventLevel::Info,
            format!(
                "{name} connected via Bluetooth (keyboard {keyboard}{}; screen as last verified by cable)",
                if trust == Trust::Verified { ", verified" } else { ", unverified" }
            ),
        );
    }

    /// Connected over Bluetooth right now?
    pub fn is_bluetooth(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.connection == Connection::Bluetooth)
    }

    fn query_firmware(&mut self, query: &Packet) -> Result<FirmwareInfo> {
        for attempt in 1..=FIRMWARE_RETRIES {
            self.write(query)?;
            let deadline = Instant::now() + ANSWER_TIMEOUT;
            while let Some(report) = self.read_until(query.role, deadline)? {
                match self.classify(query.role, &report) {
                    ReportClass::Firmware(info) if info.role == query.role => return Ok(info),
                    ReportClass::Malformed(why) => return Err(WrongAnswer(why).into()),
                    other => self.note_unsolicited(other),
                }
            }
            tracing::debug!("no {} version answer (attempt {attempt})", query.role);
        }
        Err(NoAnswer(format!("{} did not answer the version query", query.role)).into())
    }

    /// Sends a time/weather packet and checks the acknowledgement (P4, P5).
    pub fn send(&mut self, packet: &Packet) -> Result<()> {
        if !matches!(self.state, DeviceState::Ready(_)) {
            return Err(anyhow!("keyboard not ready"));
        }
        // the screen controller is not reachable over Bluetooth: not a
        // protocol problem, so no lock
        if packet.role != Role::Keyboard && self.is_bluetooth() {
            return Err(anyhow!("this needs the USB cable"));
        }
        // Stale input first: anything damaged locks, the rest is logged.
        if let Err(e) = self.drain(packet.role) {
            self.fail("P5", &e);
            return Err(e);
        }
        if let Err(e) = self.write(packet) {
            self.fail("P4", &e);
            return Err(e);
        }
        match self.await_ack(packet) {
            Ok(()) => Ok(()),
            // over Bluetooth a sleeping keyboard does not answer: try again
            // later. A wrong answer still locks.
            Err(e) if self.is_bluetooth() && e.downcast_ref::<NoAnswer>().is_some() => {
                tracing::info!("no answer over Bluetooth (keyboard asleep?): {e:#}");
                Err(e)
            }
            Err(e) => {
                self.fail("P5", &e);
                Err(e)
            }
        }
    }

    fn await_ack(&mut self, packet: &Packet) -> Result<()> {
        let deadline = Instant::now() + ANSWER_TIMEOUT;
        while let Some(report) = self.read_until(packet.role, deadline)? {
            match self.classify(packet.role, &report) {
                ReportClass::Ack { command, ok } if command == packet.command => {
                    return if ok {
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "command 0x{command:02X} answered with an error status"
                        ))
                    };
                }
                ReportClass::Receipt => {}
                ReportClass::Malformed(why) => return Err(anyhow!(why)),
                other => self.note_unsolicited(other),
            }
        }
        Err(NoAnswer(format!(
            "no acknowledgement for command 0x{:02X}",
            packet.command
        ))
        .into())
    }

    fn model(&self) -> Result<&dyn KeyboardModel> {
        match (&self.state, &self.active) {
            (DeviceState::Ready(_), Some(active)) => Ok(self.models[active.model].as_ref()),
            _ => Err(anyhow!("keyboard not ready")),
        }
    }

    /// Writes a packet and returns the first answer `pick` accepts.
    /// `Ok(None)`: the firmware answered "unsupported". A missing answer is
    /// an error without lock (optional features); damaged reports lock.
    fn exchange<T>(
        &mut self,
        packet: &Packet,
        timeout: Duration,
        pick: impl Fn(&ReportClass) -> Option<T>,
    ) -> Result<Option<T>> {
        if let Err(e) = self.drain(packet.role) {
            self.fail("P5", &e);
            return Err(e);
        }
        if let Err(e) = self.write(packet) {
            self.fail("P4", &e);
            return Err(e);
        }
        let deadline = Instant::now() + timeout;
        loop {
            let report = match self.read_until(packet.role, deadline) {
                Ok(Some(report)) => report,
                Ok(None) => {
                    return Err(anyhow!(
                        "no answer from the {} to command 0x{:02X}",
                        packet.role,
                        packet.command
                    ))
                }
                Err(e) => {
                    self.fail("P4", &e);
                    return Err(e);
                }
            };
            let class = self.classify(packet.role, &report);
            if let Some(found) = pick(&class) {
                return Ok(Some(found));
            }
            match class {
                ReportClass::Unsupported => return Ok(None),
                ReportClass::Malformed(why) => {
                    self.lock(format!("P5: {why}"));
                    return Err(anyhow!(why));
                }
                other => self.note_unsolicited(other),
            }
        }
    }

    /// Reads a lighting zone. Values the firmware does not report stay `None`.
    pub fn light_read(&mut self, zone: LightZone) -> Result<ZoneState> {
        let mut state = ZoneState::default();
        for field in [
            LightField::Effect,
            LightField::Brightness,
            LightField::Speed,
            LightField::Color,
        ] {
            let packet = self
                .model()?
                .light_read(zone, field)
                .ok_or_else(|| anyhow!("this keyboard has no lighting control"))?;
            let data = self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
                ReportClass::Light {
                    command,
                    zone: z,
                    field: Some(f),
                    data,
                } if *command == packet.command && *z == zone && *f == field => Some(data.clone()),
                _ => None,
            })?;
            match (field, data.as_deref()) {
                (LightField::Effect, Some([v, ..])) => state.effect = Some(*v),
                (LightField::Brightness, Some([v, ..])) => state.brightness = Some(*v),
                (LightField::Speed, Some([v, ..])) => state.speed = Some(*v),
                (LightField::Color, Some([h, s, ..])) => {
                    state.hue = Some(*h);
                    state.sat = Some(*s);
                }
                _ => {}
            }
        }
        tracing::info!("lighting {zone:?} read: {state:?}");
        Ok(state)
    }

    /// Applies lighting changes, then stores them if `save`.
    pub fn light_apply(
        &mut self,
        zone: LightZone,
        changes: &[LightChange],
        save: bool,
    ) -> Result<()> {
        for change in changes {
            let packet = self
                .model()?
                .light_write(zone, *change)
                .ok_or_else(|| anyhow!("this keyboard has no lighting control"))?;
            let field = change.field();
            let echoed = self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
                ReportClass::Light {
                    command,
                    zone: z,
                    field: Some(f),
                    ..
                } if *command == packet.command && *z == zone && *f == field => Some(()),
                _ => None,
            })?;
            if echoed.is_none() {
                tracing::warn!(
                    "lighting {zone:?} {change:?}: [{}] not supported",
                    hex(&packet.bytes)
                );
                return Err(anyhow!(
                    "the keyboard does not support this lighting setting"
                ));
            }
            tracing::info!(
                "lighting {zone:?} {change:?}: sent [{}], echoed",
                hex(&packet.bytes)
            );
        }
        if save {
            let packet = self
                .model()?
                .light_save(zone)
                .ok_or_else(|| anyhow!("this keyboard has no lighting control"))?;
            let echoed = self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
                ReportClass::Light {
                    command, zone: z, ..
                } if *command == packet.command && *z == zone => Some(()),
                _ => None,
            })?;
            if echoed.is_none() {
                return Err(anyhow!("the keyboard cannot store lighting settings"));
            }
            tracing::info!(
                "lighting {zone:?} stored: sent [{}], echoed",
                hex(&packet.bytes)
            );
        }
        Ok(())
    }

    /// Reads the keyboard settings. Values the firmware does not report stay
    /// `None`.
    pub fn settings_read(&mut self) -> Result<KeyboardSettings> {
        let mut settings = KeyboardSettings::default();
        for setting in KeyboardSetting::ALL {
            let id = setting.read_id();
            let packet = self
                .model()?
                .setting_read(id)
                .ok_or_else(|| anyhow!("this keyboard has no settings"))?;
            let value = self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
                ReportClass::Setting {
                    command,
                    id: Some(i),
                    value,
                } if *command == packet.command && *i == id => Some(*value),
                _ => None,
            })?;
            settings.set(setting, value.flatten());
        }
        tracing::info!("keyboard settings read: {settings:?}");
        Ok(settings)
    }

    /// Writes one setting (only values the web driver offers), stores it and
    /// checks the echo of both.
    pub fn settings_write(&mut self, setting: KeyboardSetting, value: u8) -> Result<()> {
        if !setting.allows(value) {
            return Err(anyhow!("{value} is not a valid value for {setting:?}"));
        }
        let id = setting.write_id();
        let packet = self
            .model()?
            .setting_write(id, value)
            .ok_or_else(|| anyhow!("this keyboard has no settings"))?;
        let echoed = self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
            ReportClass::Setting {
                command,
                id: Some(i),
                value: v,
            } if *command == packet.command && *i == id && *v == Some(value) => Some(()),
            _ => None,
        })?;
        if echoed.is_none() {
            return Err(anyhow!("the keyboard does not support this setting"));
        }
        let save = self
            .model()?
            .settings_save()
            .ok_or_else(|| anyhow!("this keyboard has no settings"))?;
        let stored = self.exchange(&save, ANSWER_TIMEOUT, |c| match c {
            ReportClass::Setting {
                command, id: None, ..
            } if *command == save.command => Some(()),
            _ => None,
        })?;
        if stored.is_none() {
            return Err(anyhow!("the keyboard cannot store its settings"));
        }
        tracing::info!(
            "keyboard setting {setting:?} = {value}: sent [{}], stored",
            hex(&packet.bytes)
        );
        Ok(())
    }

    /// Reads all layers and the knob. `progress` gets 0.0 … 1.0.
    pub fn keymap_read(&mut self, progress: &mut dyn FnMut(f32)) -> Result<Keymap> {
        let model = self.model()?;
        let spec = model
            .keymap_spec()
            .ok_or_else(|| anyhow!("this keyboard does not support keymap backups"))?;
        let reads = model.keymap_reads();
        let knob_reads: Vec<(u8, u8, Packet)> = (0..spec.knob_layers as u8)
            .flat_map(|l| (0..spec.knob_directions as u8).map(move |d| (l, d)))
            .filter_map(|(l, d)| model.knob_read(l, d).map(|p| (l, d, p)))
            .collect();
        let total = (reads.len() + knob_reads.len()) as f32;

        let mut buffer = vec![0u8; spec.layers * spec.rows * spec.cols * 2];
        for (i, (packet, offset, size)) in reads.iter().enumerate() {
            let data = self
                .exchange(packet, ANSWER_TIMEOUT, |c| match c {
                    ReportClass::Keymap(KeymapReply::Buffer { offset: o, data })
                        if o == offset && data.len() == *size =>
                    {
                        Some(data.clone())
                    }
                    _ => None,
                })?
                .ok_or_else(|| anyhow!("the keyboard does not support reading the keymap"))?;
            let at = *offset as usize;
            buffer[at..at + size].copy_from_slice(&data);
            progress((i + 1) as f32 / total);
        }
        let per_layer = spec.rows * spec.cols;
        let layers = buffer
            .chunks(per_layer * 2)
            .map(|layer| {
                layer
                    .chunks(2)
                    .map(|b| u16::from_be_bytes([b[0], b[1]]))
                    .collect()
            })
            .collect();

        let mut knob = vec![vec![0u16; spec.knob_directions]; spec.knob_layers];
        for (i, (layer, direction, packet)) in knob_reads.iter().enumerate() {
            let code = self
                .exchange(packet, ANSWER_TIMEOUT, |c| match c {
                    ReportClass::Keymap(KeymapReply::Knob {
                        set: false,
                        layer: l,
                        direction: d,
                        code,
                    }) if l == layer && d == direction => Some(*code),
                    _ => None,
                })?
                .ok_or_else(|| anyhow!("the keyboard does not support reading the knob"))?;
            knob[*layer as usize][*direction as usize] = code;
            progress((reads.len() + i + 1) as f32 / total);
        }
        let keymap = Keymap { layers, knob };
        tracing::info!(
            "keymap read: {} layers, {} assigned keys",
            keymap.layers.len(),
            keymap.assigned_keys()
        );
        Ok(keymap)
    }

    /// Writes the keys and knob actions of `target` that differ from the
    /// keyboard, then reads everything back and compares. Returns the number
    /// of changes.
    pub fn keymap_restore(
        &mut self,
        target: &Keymap,
        progress: &mut dyn FnMut(f32),
    ) -> Result<usize> {
        let spec = self
            .model()?
            .keymap_spec()
            .ok_or_else(|| anyhow!("this keyboard does not support keymap backups"))?;
        target.validate(&spec)?;
        // reading takes the first and the last quarter of the progress
        let current = self.keymap_read(&mut |p| progress(p * 0.25))?;
        let changes = target.changes_from(&current, &spec);
        if changes.is_empty() {
            progress(1.0);
            return Ok(0);
        }
        self.event(
            EventLevel::Info,
            format!("Restoring keymap: {} changes", changes.len()),
        );
        let total = changes.len() as f32;
        for (i, &(layer, row, col, code)) in changes.keys.iter().enumerate() {
            let packet = self
                .model()?
                .key_write(layer, row, col, code)
                .ok_or_else(|| anyhow!("this keyboard cannot change keys"))?;
            self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
                ReportClass::Keymap(KeymapReply::KeySet {
                    layer: l,
                    row: r,
                    col: k,
                    code: v,
                }) if (*l, *r, *k, *v) == (layer, row, col, code) => Some(()),
                _ => None,
            })?
            .ok_or_else(|| anyhow!("the keyboard does not support changing keys"))?;
            progress(0.25 + 0.5 * (i + 1) as f32 / total);
        }
        for (i, &(layer, direction, code)) in changes.knob.iter().enumerate() {
            let packet = self
                .model()?
                .knob_write(layer, direction, code)
                .ok_or_else(|| anyhow!("this keyboard cannot change the knob"))?;
            self.exchange(&packet, ANSWER_TIMEOUT, |c| match c {
                ReportClass::Keymap(KeymapReply::Knob {
                    set: true,
                    layer: l,
                    direction: d,
                    code: v,
                }) if (*l, *d, *v) == (layer, direction, code) => Some(()),
                _ => None,
            })?
            .ok_or_else(|| anyhow!("the keyboard does not support changing the knob"))?;
            progress(0.25 + 0.5 * (changes.keys.len() + i + 1) as f32 / total);
        }
        let check = self.keymap_read(&mut |p| progress(0.75 + p * 0.25))?;
        let left = target.changes_from(&check, &spec);
        if !left.is_empty() {
            return Err(anyhow!(
                "verification failed: {} keys/knob actions differ after writing",
                left.len()
            ));
        }
        self.event(
            EventLevel::Info,
            format!("Keymap restored and verified ({} changes)", changes.len()),
        );
        Ok(changes.len())
    }

    /// Sends an own picture/GIF to the screen, page by page as the screen
    /// asks for them. `progress` gets 0.0 … 1.0.
    pub fn upload_picture(
        &mut self,
        transfer: &PictureTransfer,
        progress: &mut dyn FnMut(f32),
    ) -> Result<()> {
        self.model()?;
        if self.is_bluetooth() {
            return Err(anyhow!(
                "sending a picture needs the USB cable (the screen is not reachable over Bluetooth)"
            ));
        }
        let role = transfer.role;
        let ack = |c: &ReportClass| match c {
            ReportClass::Transfer(a) => Some(*a),
            _ => None,
        };
        let start = Packet {
            role,
            command: transfer.start[0],
            bytes: transfer.start.clone(),
        };
        self.event(
            EventLevel::Info,
            format!(
                "Sending own picture to the screen ({} pages)",
                transfer.pages.len()
            ),
        );
        let mut answer = self.exchange(&start, PAGE_TIMEOUT, ack)?;
        let mut next = 0usize;
        let mut retries = 0usize;
        loop {
            let page = match answer {
                Some(TransferAck::Next) if next == transfer.pages.len() => break,
                Some(TransferAck::Next) => {
                    retries = 0;
                    next += 1;
                    next - 1
                }
                Some(TransferAck::Resend) if next > 0 && retries < PAGE_RETRIES => {
                    retries += 1;
                    self.event(
                        EventLevel::Warning,
                        format!("Screen asked for page {next} again"),
                    );
                    next - 1
                }
                Some(TransferAck::Resend) => {
                    return Err(anyhow!("the screen rejected the picture data"))
                }
                None => return Err(anyhow!("the screen does not support own pictures")),
            };
            let reports = &transfer.pages[page];
            let last_page = page + 1 == transfer.pages.len();
            for (i, report) in reports.iter().enumerate() {
                let packet = Packet {
                    role,
                    command: report[0],
                    bytes: report.clone(),
                };
                if i + 1 < reports.len() {
                    if let Err(e) = self.write(&packet) {
                        self.lock(format!("P4: {e:#}"));
                        return Err(e);
                    }
                    thread::sleep(REPORT_GAP);
                } else if !last_page {
                    answer = self.exchange(&packet, PAGE_TIMEOUT, ack)?;
                } else {
                    // complete unless the screen asks for the page again
                    answer = match self.exchange(&packet, LAST_PAGE_TIMEOUT, ack) {
                        Ok(a) => a,
                        Err(_) if matches!(self.state, DeviceState::Ready(_)) => {
                            tracing::debug!("no answer after the last picture page (normal)");
                            Some(TransferAck::Next)
                        }
                        Err(e) => return Err(e),
                    };
                }
            }
            progress(next as f32 / transfer.pages.len() as f32);
        }
        progress(1.0);
        self.event(EventLevel::Info, "Own picture stored on the screen");
        Ok(())
    }

    fn drain(&mut self, role: Role) -> Result<()> {
        while let Some(report) = self.read_until(role, Instant::now())? {
            match self.classify(role, &report) {
                ReportClass::Malformed(why) => return Err(anyhow!(why)),
                other => self.note_unsolicited(other),
            }
        }
        Ok(())
    }

    fn note_unsolicited(&mut self, class: ReportClass) {
        match class {
            ReportClass::Ack { command, ok } => self.event(
                EventLevel::Info,
                format!(
                    "Ignored unsolicited answer for command 0x{command:02X} (status {})",
                    if ok { "OK" } else { "error" }
                ),
            ),
            ReportClass::Ignored(what) => self.event(EventLevel::Info, format!("Ignored: {what}")),
            ReportClass::Light { command, .. } => {
                tracing::debug!("ignored lighting answer 0x{command:02X}")
            }
            ReportClass::Setting { command, .. } => {
                tracing::debug!("ignored keyboard settings answer 0x{command:02X}")
            }
            ReportClass::Unsupported => {
                tracing::debug!("ignored unsupported-command answer")
            }
            ReportClass::Transfer(ack) => tracing::debug!("ignored transfer answer {ack:?}"),
            ReportClass::Keymap(reply) => tracing::debug!("ignored keymap answer {reply:?}"),
            ReportClass::Receipt | ReportClass::Firmware(_) => {}
            ReportClass::Malformed(_) => unreachable!("handled by caller"),
        }
    }

    /// Classifies a report; unusual ones carry the raw bytes for the log.
    fn classify(&self, role: Role, report: &[u8]) -> ReportClass {
        let model = self.active.as_ref().map(|a| a.model).unwrap_or(0);
        let class = self.models[model].classify_report(role, report);
        tracing::debug!("{role} report [{}] → {class:?}", hex(report));
        match class {
            ReportClass::Malformed(why) => {
                ReportClass::Malformed(format!("{why} [{}]", hex(report)))
            }
            ReportClass::Ignored(what) => ReportClass::Ignored(format!("{what} [{}]", hex(report))),
            ReportClass::Ack { command, ok } if !ok => {
                tracing::warn!("{role} error answer [{}]", hex(report));
                ReportClass::Ack { command, ok }
            }
            other => other,
        }
    }

    fn write(&mut self, packet: &Packet) -> Result<()> {
        let (channel, len) = self
            .active
            .as_mut()
            .and_then(|a| a.channels.get_mut(&packet.role))
            .ok_or_else(|| anyhow!("{} interface not open", packet.role))?;
        if packet.bytes.len() > *len {
            return Err(anyhow!("packet longer than {} report", packet.role));
        }
        let mut report = packet.bytes.clone();
        report.resize(*len, 0);
        channel
            .write(&report)
            .map_err(|e| LinkLost(format!("HID write failed: {e:#}")).into())
    }

    fn read_until(&mut self, role: Role, deadline: Instant) -> Result<Option<Vec<u8>>> {
        let (channel, _) = self
            .active
            .as_mut()
            .and_then(|a| a.channels.get_mut(&role))
            .ok_or_else(|| anyhow!("{role} interface not open"))?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        channel
            .read(remaining)
            .map_err(|e| LinkLost(format!("HID read failed: {e:#}")).into())
    }
}

/// Hex dump without trailing zero padding, e.g. `1c 03 00 … c7`.
fn hex(report: &[u8]) -> String {
    let end = report.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    report[..end]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_interface<'a>(
    interfaces: &'a [HidInterfaceInfo],
    spec: &crate::model::InterfaceSpec,
) -> Option<&'a HidInterfaceInfo> {
    interfaces.iter().find(|i| {
        i.vid == spec.vid
            && i.pid == spec.pid
            && i.usage_page == spec.usage_page
            && i.usage == spec.usage
    })
}

// ---------------------------------------------------------------------------
// Device thread: HID handles stay on one OS thread, async code talks to it.

enum Command {
    Poll(oneshot::Sender<(bool, DeviceState, Vec<DeviceEvent>)>),
    Recheck(oneshot::Sender<(DeviceState, Vec<DeviceEvent>)>),
    Send(
        Packet,
        oneshot::Sender<(Result<(), String>, DeviceState, Vec<DeviceEvent>)>,
    ),
    Light(LightOp, oneshot::Sender<Reply<Option<ZoneState>>>),
    Settings(SettingsOp, oneshot::Sender<Reply<KeyboardSettings>>),
    Keymap(
        KeymapOp,
        mpsc::UnboundedSender<f32>,
        oneshot::Sender<Reply<KeymapResult>>,
    ),
    Picture(
        PictureTransfer,
        mpsc::UnboundedSender<f32>,
        oneshot::Sender<Reply<()>>,
    ),
}

/// Result of a device-thread operation plus the state and events after it.
pub type Reply<T> = (Result<T, String>, DeviceState, Vec<DeviceEvent>);

/// Async handle to the device thread.
#[derive(Clone)]
pub struct DeviceHandle {
    tx: mpsc::Sender<Command>,
}

impl DeviceHandle {
    /// Starts the device thread. `backend` is created on that thread.
    pub fn spawn<F>(backend: F, models: Vec<Box<dyn KeyboardModel>>) -> Self
    where
        F: FnOnce() -> Result<Box<dyn HidBackend>> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<Command>(16);
        thread::Builder::new()
            .name("nd104-device".into())
            .spawn(move || {
                let mut manager = match backend() {
                    Ok(b) => {
                        let manager = DeviceManager::new(b, models);
                        match crate::config::local_data_dir() {
                            Ok(dir) => manager.with_screen_memory(dir.join("screen-firmware.json")),
                            Err(_) => manager,
                        }
                    }
                    Err(e) => {
                        tracing::error!("HID backend unavailable: {e:#}");
                        return;
                    }
                };
                while let Some(cmd) = rx.blocking_recv() {
                    match cmd {
                        Command::Poll(reply) => {
                            let changed = manager.poll();
                            let _ = reply.send((
                                changed,
                                manager.state().clone(),
                                manager.take_events(),
                            ));
                        }
                        Command::Recheck(reply) => {
                            manager.recheck();
                            let _ = reply.send((manager.state().clone(), manager.take_events()));
                        }
                        Command::Send(packet, reply) => {
                            let result = manager.send(&packet).map_err(|e| format!("{e:#}"));
                            let _ = reply.send((
                                result,
                                manager.state().clone(),
                                manager.take_events(),
                            ));
                        }
                        Command::Keymap(op, progress, reply) => {
                            let mut report = |p: f32| {
                                let _ = progress.send(p);
                            };
                            let result = match op {
                                KeymapOp::Read => {
                                    manager.keymap_read(&mut report).map(KeymapResult::Read)
                                }
                                KeymapOp::Restore(target) => manager
                                    .keymap_restore(&target, &mut report)
                                    .map(KeymapResult::Restored),
                            }
                            .map_err(|e| format!("{e:#}"));
                            let _ = reply.send((
                                result,
                                manager.state().clone(),
                                manager.take_events(),
                            ));
                        }
                        Command::Light(op, reply) => {
                            let result = match op {
                                LightOp::Read(zone) => manager.light_read(zone).map(Some),
                                LightOp::Apply {
                                    zone,
                                    changes,
                                    save,
                                } => manager.light_apply(zone, &changes, save).map(|()| None),
                            }
                            .map_err(|e| format!("{e:#}"));
                            let _ = reply.send((
                                result,
                                manager.state().clone(),
                                manager.take_events(),
                            ));
                        }
                        Command::Settings(op, reply) => {
                            let result = match op {
                                SettingsOp::Read => manager.settings_read(),
                                SettingsOp::Write(setting, value) => manager
                                    .settings_write(setting, value)
                                    .and_then(|()| manager.settings_read()),
                            }
                            .map_err(|e| format!("{e:#}"));
                            let _ = reply.send((
                                result,
                                manager.state().clone(),
                                manager.take_events(),
                            ));
                        }
                        Command::Picture(transfer, progress, reply) => {
                            let result = manager
                                .upload_picture(&transfer, &mut |p| {
                                    let _ = progress.send(p);
                                })
                                .map_err(|e| format!("{e:#}"));
                            let _ = reply.send((
                                result,
                                manager.state().clone(),
                                manager.take_events(),
                            ));
                        }
                    }
                }
            })
            .expect("cannot start device thread");
        Self { tx }
    }

    pub async fn poll(&self) -> Result<(bool, DeviceState, Vec<DeviceEvent>)> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Poll(reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }

    pub async fn recheck(&self) -> Result<(DeviceState, Vec<DeviceEvent>)> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Recheck(reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }

    pub async fn send(
        &self,
        packet: Packet,
    ) -> Result<(Result<(), String>, DeviceState, Vec<DeviceEvent>)> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Send(packet, reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }
}

impl DeviceHandle {
    pub async fn keymap(
        &self,
        op: KeymapOp,
        progress: mpsc::UnboundedSender<f32>,
    ) -> Result<Reply<KeymapResult>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Keymap(op, progress, reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }

    pub async fn light(&self, op: LightOp) -> Result<Reply<Option<ZoneState>>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Light(op, reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }

    pub async fn settings(&self, op: SettingsOp) -> Result<Reply<KeyboardSettings>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Settings(op, reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }

    pub async fn upload_picture(
        &self,
        transfer: PictureTransfer,
        progress: mpsc::UnboundedSender<f32>,
    ) -> Result<Reply<()>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Picture(transfer, progress, reply))
            .await
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())
    }
}

fn gone() -> anyhow::Error {
    anyhow!("device thread stopped (HID backend unavailable?)")
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use nd104_protocol::{checksum8, crc16};

    use super::*;
    use crate::model::Nd104;

    #[derive(Clone, Copy, PartialEq)]
    enum Behaviour {
        Normal,
        NoAck,
        BadStatus,
        Damaged,
        UnknownWellFormedFirst,
        SilentVersion,
        WriteFails,
        /// Lighting commands answered with FF (unsupported).
        NoLighting,
        /// The screen asks for the first data page twice.
        ResendOnce,
        /// The keyboard keeps one key unchanged (write echoed, not stored).
        StuckKey,
    }

    struct Shared {
        /// Keyboard on Bluetooth, screen on a (charging) USB cable.
        bluetooth: bool,
        /// With `bluetooth`: the keyboard is on the USB cable as well.
        cable_too: bool,
        /// Reads and writes fail like a dropped Bluetooth link (os error 1167).
        link_down: bool,
        present: bool,
        keyboard_fw: &'static str,
        behaviour: Behaviour,
        written: Vec<(Role, Vec<u8>)>,
        queues: HashMap<Role, VecDeque<Vec<u8>>>,
        /// (channel, value id) → data
        lighting: HashMap<(u8, u8), Vec<u8>>,
        /// Keyboard settings by read id (channel 0).
        settings: HashMap<u8, u8>,
        /// Picture bytes left to receive in the current page / in total.
        page_left: usize,
        picture_left: usize,
        resent: bool,
        /// 6 layers × 128 keys × 2 bytes
        keymap: Vec<u8>,
        knob: [[u16; 2]; 4],
        key_writes: usize,
    }

    fn picture_ack(code: u8) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[..7].copy_from_slice(&[0xBA, 5, 0xA5, 0xB0, 0, 2, code]);
        r
    }

    struct MockBackend(Arc<Mutex<Shared>>);
    struct MockChannel(Arc<Mutex<Shared>>, Role);

    fn iface(vid: u16, pid: u16, n: i32) -> HidInterfaceInfo {
        HidInterfaceInfo {
            vid,
            pid,
            usage_page: 0xFF60,
            usage: 0x61,
            interface_number: n,
            path: format!("{vid:04x}:{pid:04x}"),
            product: None,
        }
    }

    impl HidBackend for MockBackend {
        fn enumerate(&mut self) -> Result<Vec<HidInterfaceInfo>> {
            let s = self.0.lock().unwrap();
            Ok(if s.bluetooth {
                let mut keyboard = iface(0x6D67, 0x016C, -1);
                keyboard.path =
                    r"\\?\HID#{00001812-0000-1000-8000-00805f9b34fb}_Dev_VID&016d67_PID&016c&Col02"
                        .into();
                let mut list = vec![keyboard, iface(0x5542, 0x0001, 0)];
                if s.cable_too {
                    list.push(iface(0x6D67, 0x016C, 1));
                }
                list
            } else if s.present {
                vec![iface(0x6D67, 0x016C, 1), iface(0x5542, 0x0001, 0)]
            } else {
                vec![]
            })
        }
        fn open(&mut self, info: &HidInterfaceInfo) -> Result<Box<dyn HidChannel>> {
            let role = if info.vid == 0x6D67 {
                Role::Keyboard
            } else {
                Role::Screen
            };
            Ok(Box::new(MockChannel(self.0.clone(), role)))
        }
    }

    fn with_crc(mut r: Vec<u8>) -> Vec<u8> {
        r[6] = 0;
        r[7] = 0;
        let crc = crc16(&r);
        r[6] = (crc & 0xFF) as u8;
        r[7] = (crc >> 8) as u8;
        r
    }

    fn version_answer(len: usize, text: &str) -> Vec<u8> {
        let mut r = vec![0u8; len];
        r[0] = 0x11;
        r[1] = 0x02;
        r[5] = text.len() as u8;
        r[8..8 + text.len()].copy_from_slice(text.as_bytes());
        with_crc(r)
    }

    fn relayed_ack(kind: u8, command: u8, status: u8) -> Vec<u8> {
        let mut r = vec![0u8; 32];
        r[..6].copy_from_slice(&[0x1C, kind, 0, 0, 0, 7]);
        r[8..14].copy_from_slice(&[0xA5, command, 0x00, 0x02, status, 0x00]);
        r[14] = checksum8(&r[9..]);
        with_crc(r)
    }

    impl HidChannel for MockChannel {
        fn write(&mut self, report: &[u8]) -> Result<()> {
            let mut s = self.0.lock().unwrap();
            if s.link_down {
                return Err(anyhow!("the device is not connected (os error 1167)"));
            }
            if s.behaviour == Behaviour::WriteFails && report[0] == 0x1C {
                return Err(anyhow!("device rejected write"));
            }
            s.written.push((self.1, report.to_vec()));
            let mut answers = Vec::new();
            match (self.1, report[0]) {
                (Role::Keyboard, 0x11) => answers.push(version_answer(
                    32,
                    &format!("V{} 20260710_123331", s.keyboard_fw),
                )),
                (Role::Screen, 0x11) if s.behaviour != Behaviour::SilentVersion => {
                    answers.push(version_answer(64, "V1.2.4 Mar 17 2026 21:37:00"))
                }
                (Role::Keyboard, 0x1C) => {
                    let (kind, command) = (report[1], report[9]);
                    answers.push(with_crc(vec![
                        0x1C, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ]));
                    match s.behaviour {
                        Behaviour::NoAck => {}
                        Behaviour::BadStatus => answers.push(relayed_ack(kind, command, 0x01)),
                        Behaviour::Damaged => {
                            let mut a = relayed_ack(kind, command, 0xFE);
                            a[14] ^= 0xFF;
                            answers.push(a);
                        }
                        Behaviour::UnknownWellFormedFirst => {
                            answers.push(relayed_ack(0x03, 0x35, 0xFE));
                            answers.push(relayed_ack(kind, command, 0xFE));
                        }
                        _ => answers.push(relayed_ack(kind, command, 0xFE)),
                    }
                }
                (Role::Keyboard, 0x07..=0x09) if s.behaviour == Behaviour::NoLighting => {
                    let mut a = report.to_vec();
                    a[0] = 0xFF;
                    answers.push(a);
                }
                // keyboard settings (like V3.0.14: sleep lives under 0B,
                // reading 06 answers 0)
                (Role::Keyboard, 0x07) if report[1] == 0 => {
                    s.settings.insert(report[3], report[4]);
                    answers.push(report.to_vec());
                }
                (Role::Keyboard, 0x08) if report[1] == 0 => {
                    let mut a = report.to_vec();
                    a[4] = s.settings.get(&report[3]).copied().unwrap_or(0);
                    answers.push(a);
                }
                (Role::Keyboard, 0x07) => {
                    let len = if report[2] == 4 { 2 } else { 1 };
                    s.lighting
                        .insert((report[1], report[2]), report[3..3 + len].to_vec());
                    answers.push(report.to_vec());
                }
                (Role::Keyboard, 0x08) => {
                    let mut a = report.to_vec();
                    let data = s
                        .lighting
                        .get(&(report[1], report[2]))
                        .cloned()
                        .unwrap_or(vec![7, 7]);
                    a[3..3 + data.len()].copy_from_slice(&data);
                    answers.push(a);
                }
                (Role::Keyboard, 0x09) => answers.push(report.to_vec()),
                (Role::Keyboard, 0x12 | 0x05 | 0x14 | 0x15)
                    if s.behaviour == Behaviour::NoLighting =>
                {
                    let mut a = report.to_vec();
                    a[0] = 0xFF;
                    answers.push(a);
                }
                (Role::Keyboard, 0x12) => {
                    let (offset, size) = (
                        usize::from(report[1]) << 8 | usize::from(report[2]),
                        report[3] as usize,
                    );
                    let mut a = report.to_vec();
                    a[4..4 + size].copy_from_slice(&s.keymap[offset..offset + size]);
                    answers.push(a);
                }
                (Role::Keyboard, 0x05) => {
                    s.key_writes += 1;
                    let at =
                        (report[1] as usize * 128 + report[2] as usize * 16 + report[3] as usize)
                            * 2;
                    let stuck = s.behaviour == Behaviour::StuckKey && at == 0;
                    if !stuck {
                        s.keymap[at] = report[4];
                        s.keymap[at + 1] = report[5];
                    }
                    answers.push(report.to_vec());
                }
                (Role::Keyboard, 0x14) => {
                    let mut a = report.to_vec();
                    let code = s.knob[report[1] as usize][report[3] as usize];
                    a[4..6].copy_from_slice(&code.to_be_bytes());
                    answers.push(a);
                }
                (Role::Keyboard, 0x15) => {
                    s.knob[report[1] as usize][report[3] as usize] =
                        u16::from_be_bytes([report[4], report[5]]);
                    answers.push(report.to_vec());
                }
                (Role::Screen, 0xBA) if report[1] == 6 && report[2] == 0xAA => {
                    s.picture_left =
                        u32::from_be_bytes([report[4], report[5], report[6], report[7]]) as usize;
                    s.page_left = s.picture_left.min(4096);
                    answers.push(picture_ack(0xFE));
                }
                (Role::Screen, 0xBA) => {
                    let n = report[1] as usize;
                    s.page_left -= n;
                    s.picture_left -= n;
                    if s.page_left == 0 && s.picture_left == 0 {
                        // the real screen does not answer the last page
                    } else if s.page_left == 0 {
                        if s.behaviour == Behaviour::ResendOnce && !s.resent {
                            s.resent = true;
                            s.picture_left += 4096.min(s.picture_left + 4096);
                            s.page_left = 4096;
                            answers.push(picture_ack(0xFD));
                        } else {
                            s.page_left = s.picture_left.min(4096);
                            answers.push(picture_ack(0xFE));
                        }
                    }
                }
                _ => {}
            }
            let role = self.1;
            s.queues.entry(role).or_default().extend(answers);
            Ok(())
        }

        fn read(&mut self, _timeout: Duration) -> Result<Option<Vec<u8>>> {
            let mut s = self.0.lock().unwrap();
            let role = self.1;
            Ok(s.queues.entry(role).or_default().pop_front())
        }
    }

    fn manager(
        behaviour: Behaviour,
        keyboard_fw: &'static str,
    ) -> (DeviceManager, Arc<Mutex<Shared>>) {
        let shared = Arc::new(Mutex::new(Shared {
            bluetooth: false,
            cable_too: false,
            link_down: false,
            present: true,
            keyboard_fw,
            behaviour,
            written: Vec::new(),
            queues: HashMap::new(),
            lighting: HashMap::new(),
            settings: HashMap::from([(1, 5), (2, 1), (0x0B, 10), (7, 30)]),
            page_left: 0,
            picture_left: 0,
            resent: false,
            keymap: (0..1536).map(|i| (i % 7) as u8).collect(),
            knob: [[0xE9, 0xEA]; 4],
            key_writes: 0,
        }));
        let m = DeviceManager::new(Box::new(MockBackend(shared.clone())), vec![Box::new(Nd104)]);
        (m, shared)
    }

    fn time_packet() -> Packet {
        Nd104
            .encode_time(&"2026-10-31T12:00:00".parse().unwrap())
            .unwrap()
    }

    #[test]
    fn lighting_is_set_saved_and_read_back() {
        use crate::lighting::{LightChange, LightZone};
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        m.poll();
        m.light_apply(
            LightZone::Bottom,
            &[
                LightChange::Effect { value: 1 },
                LightChange::Brightness { value: 200 },
                LightChange::Color { hue: 85, sat: 255 },
            ],
            true,
        )
        .unwrap();
        let state = m.light_read(LightZone::Bottom).unwrap();
        assert_eq!(state.effect, Some(1));
        assert_eq!(state.brightness, Some(200));
        assert_eq!((state.hue, state.sat), (Some(85), Some(255)));
        let written: Vec<Vec<u8>> = shared
            .lock()
            .unwrap()
            .written
            .iter()
            .filter(|(r, b)| *r == Role::Keyboard && (7..=9).contains(&b[0]))
            .map(|(_, b)| b[..5].to_vec())
            .collect();
        assert_eq!(written[0], [7, 2, 2, 1, 0]);
        assert_eq!(written[2], [7, 2, 4, 85, 255]);
        assert_eq!(written[3], [9, 2, 0, 0, 0]);
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    #[test]
    fn unsupported_lighting_does_not_lock() {
        use crate::lighting::{LightChange, LightZone};
        let (mut m, _) = manager(Behaviour::NoLighting, "3.0.14");
        m.poll();
        let state = m.light_read(LightZone::PerKey).unwrap();
        assert_eq!(state, crate::lighting::ZoneState::default());
        assert!(m
            .light_apply(
                LightZone::PerKey,
                &[LightChange::Brightness { value: 1 }],
                false
            )
            .is_err());
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    fn connection(m: &DeviceManager) -> Option<Connection> {
        match m.state() {
            DeviceState::Ready(d) => Some(d.connection),
            _ => None,
        }
    }

    /// Connects over USB once (remembers the screen), then switches to
    /// Bluetooth (keyboard on Bluetooth, screen on a charging cable).
    fn over_bluetooth(behaviour: Behaviour) -> (DeviceManager, Arc<Mutex<Shared>>) {
        let (mut m, shared) = manager(behaviour, "3.0.14");
        m.poll();
        assert_eq!(connection(&m), Some(Connection::Usb));
        shared.lock().unwrap().bluetooth = true;
        m.poll(); // the USB keyboard is gone
        m.poll(); // Bluetooth
        assert_eq!(connection(&m), Some(Connection::Bluetooth));
        (m, shared)
    }

    #[test]
    fn bluetooth_waits_for_one_usb_connection_without_locking() {
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        shared.lock().unwrap().bluetooth = true;
        m.poll();
        // not a lock (no red state, no notification), just waiting
        assert_eq!(*m.state(), DeviceState::NotConnected);
        let events = m.take_events();
        assert!(events.iter().any(|e| e.message.contains("USB cable")));
        m.poll();
        assert!(m.take_events().is_empty(), "the hint is said once");
        // nothing but the keyboard version query was sent
        let written = shared.lock().unwrap().written.clone();
        assert!(written
            .iter()
            .all(|(r, b)| *r == Role::Keyboard && b[0] == 0x11));
        // the cable plugged in while Bluetooth stays connected
        shared.lock().unwrap().cable_too = true;
        m.poll();
        assert_eq!(connection(&m), Some(Connection::Usb));
    }

    #[test]
    fn a_bluetooth_lock_ends_when_the_cable_is_plugged_in() {
        let (mut m, shared) = over_bluetooth(Behaviour::BadStatus);
        assert!(m.send(&time_packet()).is_err());
        assert!(matches!(m.state(), DeviceState::Locked { .. }));
        m.poll();
        assert!(
            matches!(m.state(), DeviceState::Locked { .. }),
            "Bluetooth alone keeps it"
        );
        {
            let mut s = shared.lock().unwrap();
            s.behaviour = Behaviour::Normal;
            s.cable_too = true;
        }
        m.poll(); // re-connected by cable: all checks again
        assert_eq!(connection(&m), Some(Connection::Usb));
    }

    #[test]
    fn bluetooth_syncs_time_and_reads_the_keyboard_only() {
        let (mut m, _shared) = over_bluetooth(Behaviour::Normal);
        let DeviceState::Ready(d) = m.state() else {
            unreachable!()
        };
        // keyboard read now, screen as remembered from USB
        assert!(d.firmware.iter().any(|f| f.role == Role::Screen));
        assert_eq!(d.trust, Trust::Verified);
        m.send(&time_packet()).unwrap();
        assert_eq!(connection(&m), Some(Connection::Bluetooth));
    }

    #[test]
    fn bluetooth_refuses_the_picture_without_locking() {
        let (mut m, _shared) = over_bluetooth(Behaviour::Normal);
        let err = m.upload_picture(&picture(9000), &mut |_| {}).unwrap_err();
        assert!(format!("{err:#}").contains("USB cable"));
        assert_eq!(connection(&m), Some(Connection::Bluetooth));
    }

    #[test]
    fn bluetooth_without_answer_does_not_lock() {
        let (mut m, _shared) = over_bluetooth(Behaviour::NoAck);
        assert!(m.send(&time_packet()).is_err());
        // a sleeping keyboard: still connected, nothing locked
        assert_eq!(connection(&m), Some(Connection::Bluetooth));
    }

    #[test]
    fn bluetooth_wrong_answer_still_locks() {
        let (mut m, _shared) = over_bluetooth(Behaviour::BadStatus);
        assert!(m.send(&time_packet()).is_err());
        assert!(matches!(m.state(), DeviceState::Locked { .. }));
    }

    #[test]
    fn a_sleeping_keyboard_over_bluetooth_reconnects_without_lock() {
        let (mut m, shared) = over_bluetooth(Behaviour::Normal);
        shared.lock().unwrap().link_down = true;
        assert!(m.send(&time_packet()).is_err());
        assert_eq!(*m.state(), DeviceState::NotConnected, "dropped, not locked");
        m.poll(); // still asleep: stays not connected
        assert_eq!(*m.state(), DeviceState::NotConnected);
        shared.lock().unwrap().link_down = false;
        m.poll(); // awake: connected again with all checks
        assert_eq!(connection(&m), Some(Connection::Bluetooth));
        m.send(&time_packet()).unwrap();
    }

    #[test]
    fn a_failed_usb_write_still_locks() {
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        m.poll();
        shared.lock().unwrap().link_down = true;
        assert!(m.send(&time_packet()).is_err());
        assert!(matches!(m.state(), DeviceState::Locked { .. }));
    }

    #[test]
    fn the_cable_wins_over_bluetooth() {
        let (mut m, shared) = over_bluetooth(Behaviour::Normal);
        shared.lock().unwrap().bluetooth = false;
        m.poll(); // leaves Bluetooth
        m.poll(); // USB
        assert_eq!(connection(&m), Some(Connection::Usb));
    }

    #[test]
    fn keyboard_settings_are_read_written_and_stored() {
        use crate::keyboard_settings::KeyboardSetting;
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        m.poll();
        let s = m.settings_read().unwrap();
        assert_eq!(
            (
                s.debounce_mode,
                s.debounce_time,
                s.sleep_time,
                s.deep_sleep_time
            ),
            (Some(1), Some(5), Some(10), Some(30))
        );
        m.settings_write(KeyboardSetting::SleepTime, 15).unwrap();
        assert_eq!(m.settings_read().unwrap().sleep_time, Some(15));
        let written: Vec<Vec<u8>> = shared
            .lock()
            .unwrap()
            .written
            .iter()
            .filter(|(r, b)| *r == Role::Keyboard && (b[0] == 7 || b[0] == 9))
            .map(|(_, b)| b[..5].to_vec())
            .collect();
        assert_eq!(written, [vec![7, 0, 4, 0x0B, 15], vec![9, 0, 0, 0, 0]]);
        // values the web driver does not offer are never sent
        assert!(m.settings_write(KeyboardSetting::SleepTime, 61).is_err());
        assert!(m.settings_write(KeyboardSetting::DebounceMode, 3).is_err());
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    #[test]
    fn unsupported_settings_do_not_lock() {
        use crate::keyboard_settings::{KeyboardSetting, KeyboardSettings};
        let (mut m, _) = manager(Behaviour::NoLighting, "3.0.14");
        m.poll();
        assert_eq!(m.settings_read().unwrap(), KeyboardSettings::default());
        assert!(m.settings_write(KeyboardSetting::SleepTime, 10).is_err());
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    fn picture(len: usize) -> crate::model::PictureTransfer {
        let frame = crate::model::EncodedFrame {
            jpeg: vec![0x5A; len],
            delay: 1,
        };
        Nd104.picture_transfer(&[frame]).unwrap()
    }

    #[test]
    fn picture_is_sent_page_by_page() {
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        m.poll();
        let transfer = picture(9000);
        let mut seen = Vec::new();
        m.upload_picture(&transfer, &mut |p| seen.push(p)).unwrap();
        assert_eq!(transfer.pages.len(), 3);
        assert_eq!(seen.last(), Some(&1.0));
        let s = shared.lock().unwrap();
        assert_eq!(s.picture_left, 0);
        let data = s
            .written
            .iter()
            .filter(|(r, b)| *r == Role::Screen && b[0] == 0xBA)
            .count();
        assert_eq!(data, 1 + transfer.pages.iter().map(Vec::len).sum::<usize>());
    }

    #[test]
    fn picture_page_is_resent_on_request() {
        let (mut m, shared) = manager(Behaviour::ResendOnce, "3.0.14");
        m.poll();
        let transfer = picture(9000);
        m.upload_picture(&transfer, &mut |_| {}).unwrap();
        let s = shared.lock().unwrap();
        assert!(s.resent);
        let data = s
            .written
            .iter()
            .filter(|(r, b)| *r == Role::Screen && b[0] == 0xBA)
            .count();
        assert_eq!(
            data,
            1 + transfer.pages.iter().map(Vec::len).sum::<usize>() + transfer.pages[0].len()
        );
    }

    #[test]
    fn keymap_is_read_completely() {
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        m.poll();
        let mut last = 0.0;
        let keymap = m.keymap_read(&mut |p| last = p).unwrap();
        assert_eq!(last, 1.0);
        assert_eq!(keymap.layers.len(), 6);
        assert!(keymap.layers.iter().all(|l| l.len() == 128));
        let raw = shared.lock().unwrap().keymap.clone();
        assert_eq!(
            keymap.layers[2][5],
            u16::from_be_bytes([raw[(256 + 5) * 2], raw[(256 + 5) * 2 + 1]])
        );
        assert_eq!(keymap.knob, vec![vec![0xE9, 0xEA]; 4]);
    }

    #[test]
    fn keymap_restore_writes_only_differences_and_verifies() {
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        m.poll();
        let mut target = m.keymap_read(&mut |_| {}).unwrap();
        target.layers[0][17] = 0x0029; // row 1, col 1
        target.layers[5][127] = 0x7E40;
        target.knob[3][1] = 0x0080;
        assert_eq!(m.keymap_restore(&target, &mut |_| {}).unwrap(), 3);
        let s = shared.lock().unwrap();
        assert_eq!(s.key_writes, 2);
        assert_eq!(&s.keymap[34..36], &[0x00, 0x29]);
        assert_eq!(s.knob[3][1], 0x0080);
        drop(s);
        // nothing differs any more
        assert_eq!(m.keymap_restore(&target, &mut |_| {}).unwrap(), 0);
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    #[test]
    fn keymap_restore_reports_a_key_that_did_not_change() {
        let (mut m, _) = manager(Behaviour::StuckKey, "3.0.14");
        m.poll();
        let mut target = m.keymap_read(&mut |_| {}).unwrap();
        target.layers[0][0] = 0x1234;
        let err = m.keymap_restore(&target, &mut |_| {}).unwrap_err();
        assert!(err.to_string().contains("verification failed"), "{err}");
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    #[test]
    fn unsupported_keymap_does_not_lock() {
        let (mut m, _) = manager(Behaviour::NoLighting, "3.0.14");
        m.poll();
        assert!(m.keymap_read(&mut |_| {}).is_err());
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    #[test]
    fn verified_firmware_connects_and_sends() {
        let (mut m, shared) = manager(Behaviour::Normal, "3.0.14");
        assert!(m.poll());
        let DeviceState::Ready(d) = m.state() else {
            panic!("{:?}", m.state())
        };
        assert_eq!(d.trust, Trust::Verified);
        assert_eq!(d.screen_variant, ScreenVariant::Celsius24h);
        m.send(&time_packet()).unwrap();
        let s = shared.lock().unwrap();
        let last = s.written.last().unwrap();
        assert_eq!(last.0, Role::Keyboard);
        assert_eq!(last.1.len(), 32);
    }

    #[test]
    fn other_firmware_runs_as_compatible() {
        let (mut m, _) = manager(Behaviour::Normal, "3.0.15");
        m.poll();
        let DeviceState::Ready(d) = m.state() else {
            panic!()
        };
        assert_eq!(d.trust, Trust::Compatible);
        assert!(m.send(&time_packet()).is_ok());
    }

    #[test]
    fn silent_version_query_locks() {
        let (mut m, _) = manager(Behaviour::SilentVersion, "3.0.14");
        m.poll();
        assert!(
            matches!(m.state(), DeviceState::Locked { reason, .. } if reason.starts_with("P3"))
        );
    }

    #[test]
    fn missing_ack_locks() {
        let (mut m, _) = manager(Behaviour::NoAck, "3.0.14");
        m.poll();
        assert!(m.send(&time_packet()).is_err());
        assert!(
            matches!(m.state(), DeviceState::Locked { reason, .. } if reason.contains("no acknowledgement"))
        );
    }

    #[test]
    fn error_status_locks() {
        let (mut m, _) = manager(Behaviour::BadStatus, "3.0.14");
        m.poll();
        assert!(m.send(&time_packet()).is_err());
        assert!(matches!(m.state(), DeviceState::Locked { .. }));
    }

    #[test]
    fn damaged_answer_locks() {
        let (mut m, _) = manager(Behaviour::Damaged, "3.0.14");
        m.poll();
        assert!(m.send(&time_packet()).is_err());
        assert!(matches!(m.state(), DeviceState::Locked { .. }));
    }

    #[test]
    fn write_error_locks() {
        let (mut m, _) = manager(Behaviour::WriteFails, "3.0.14");
        m.poll();
        assert!(m.send(&time_packet()).is_err());
        assert!(
            matches!(m.state(), DeviceState::Locked { reason, .. } if reason.starts_with("P4"))
        );
    }

    #[test]
    fn unknown_wellformed_message_is_logged_and_ignored() {
        let (mut m, _) = manager(Behaviour::UnknownWellFormedFirst, "3.0.14");
        m.poll();
        m.take_events();
        m.send(&time_packet()).unwrap();
        assert!(matches!(m.state(), DeviceState::Ready(_)));
        let events = m.take_events();
        assert!(
            events.iter().any(|e| e.message.contains("0x35")),
            "{events:?}"
        );
    }

    #[test]
    fn lock_survives_poll_until_replug_or_recheck() {
        let (mut m, shared) = manager(Behaviour::NoAck, "3.0.14");
        m.poll();
        let _ = m.send(&time_packet());
        m.poll();
        assert!(matches!(m.state(), DeviceState::Locked { .. }));

        // re-plug
        shared.lock().unwrap().present = false;
        m.poll();
        assert_eq!(m.state(), &DeviceState::NotConnected);
        shared.lock().unwrap().present = true;
        shared.lock().unwrap().behaviour = Behaviour::Normal;
        m.poll();
        assert!(matches!(m.state(), DeviceState::Ready(_)));

        // recheck
        shared.lock().unwrap().behaviour = Behaviour::NoAck;
        let _ = m.send(&time_packet());
        shared.lock().unwrap().behaviour = Behaviour::Normal;
        m.recheck();
        assert!(matches!(m.state(), DeviceState::Ready(_)));
    }

    #[test]
    fn nothing_is_sent_while_locked() {
        let (mut m, shared) = manager(Behaviour::NoAck, "3.0.14");
        m.poll();
        let _ = m.send(&time_packet());
        let writes = shared.lock().unwrap().written.len();
        assert!(m.send(&time_packet()).is_err());
        assert_eq!(shared.lock().unwrap().written.len(), writes);
    }
}
