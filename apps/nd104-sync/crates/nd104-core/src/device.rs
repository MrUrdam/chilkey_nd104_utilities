//! Device manager with the firmware/protocol guard (PLAN.md § 5.2, § 5.3).
//!
//! Checks:
//! - P1 all interfaces of the model present (wired USB)
//! - P2/P4 reports have the expected length, writes are accepted
//! - P3 firmware version queries answered well-formed
//! - P5 every packet acknowledged with status OK; damaged or structurally
//!   unknown reports lock, well-formed unknown messages are logged and ignored
//! - P6 screen variant recognised where possible (informational)
//!
//! Any failed check locks the device until it is re-plugged or re-checked.

use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use crate::hid::{HidBackend, HidChannel, HidInterfaceInfo};
use crate::model::{FirmwareInfo, KeyboardModel, Packet, ReportClass, Role, ScreenVariant};

const ANSWER_TIMEOUT: Duration = Duration::from_millis(1000);
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
    pub model_id: &'static str,
    pub model_name: &'static str,
    pub firmware: Vec<FirmwareInfo>,
    pub trust: Trust,
    pub screen_variant: ScreenVariant,
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
    channels: HashMap<Role, (Box<dyn HidChannel>, usize)>,
}

pub struct DeviceManager {
    backend: Box<dyn HidBackend>,
    models: Vec<Box<dyn KeyboardModel>>,
    active: Option<Active>,
    state: DeviceState,
    events: Vec<DeviceEvent>,
}

impl DeviceManager {
    pub fn new(backend: Box<dyn HidBackend>, models: Vec<Box<dyn KeyboardModel>>) -> Self {
        Self {
            backend,
            models,
            active: None,
            state: DeviceState::NotConnected,
            events: Vec::new(),
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
        self.active = None;
        self.state = DeviceState::Locked {
            model_name,
            firmware,
            reason,
        };
    }

    /// Detects plug/unplug and connects. Returns `true` if the state changed.
    pub fn poll(&mut self) -> bool {
        let before = self.state.clone();
        let interfaces = match self.backend.enumerate() {
            Ok(list) => list,
            Err(e) => {
                self.event(EventLevel::Error, format!("HID enumeration failed: {e:#}"));
                return false;
            }
        };

        let found = self.models.iter().position(|m| {
            m.interfaces().iter().any(|spec| {
                interfaces
                    .iter()
                    .any(|i| i.vid == spec.vid && i.pid == spec.pid)
            })
        });

        match (found, &self.state) {
            (None, DeviceState::NotConnected) => {}
            (None, _) => {
                self.active = None;
                self.state = DeviceState::NotConnected;
                self.event(EventLevel::Info, "Keyboard disconnected");
            }
            // A lock only ends with re-plug or re-check.
            (Some(_), DeviceState::Locked { .. }) => {}
            (Some(model), DeviceState::Ready(_)) => {
                if !self.all_interfaces_present(model, &interfaces) {
                    self.active = None;
                    self.state = DeviceState::NotConnected;
                    self.event(EventLevel::Info, "Keyboard disconnected");
                }
            }
            (Some(model), DeviceState::NotConnected) => self.connect(model, &interfaces),
        }
        self.state != before
    }

    /// Forgets a lock and runs all checks again.
    pub fn recheck(&mut self) -> bool {
        self.active = None;
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

    fn query_firmware(&mut self, query: &Packet) -> Result<FirmwareInfo> {
        for attempt in 1..=FIRMWARE_RETRIES {
            self.write(query)?;
            let deadline = Instant::now() + ANSWER_TIMEOUT;
            while let Some(report) = self.read_until(query.role, deadline)? {
                match self.classify(query.role, &report) {
                    ReportClass::Firmware(info) if info.role == query.role => return Ok(info),
                    ReportClass::Malformed(why) => return Err(anyhow!(why)),
                    other => self.note_unsolicited(other),
                }
            }
            tracing::debug!("no {} version answer (attempt {attempt})", query.role);
        }
        Err(anyhow!("{} did not answer the version query", query.role))
    }

    /// Sends a time/weather packet and checks the acknowledgement (P4, P5).
    pub fn send(&mut self, packet: &Packet) -> Result<()> {
        if !matches!(self.state, DeviceState::Ready(_)) {
            return Err(anyhow!("keyboard not ready"));
        }
        // Stale input first: anything damaged locks, the rest is logged.
        if let Err(e) = self.drain(packet.role) {
            self.lock(format!("P5: {e:#}"));
            return Err(e);
        }
        if let Err(e) = self.write(packet) {
            self.lock(format!("P4: {e:#}"));
            return Err(e);
        }
        match self.await_ack(packet) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.lock(format!("P5: {e:#}"));
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
        Err(anyhow!(
            "no acknowledgement for command 0x{:02X}",
            packet.command
        ))
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
        channel.write(&report)
    }

    fn read_until(&mut self, role: Role, deadline: Instant) -> Result<Option<Vec<u8>>> {
        let (channel, _) = self
            .active
            .as_mut()
            .and_then(|a| a.channels.get_mut(&role))
            .ok_or_else(|| anyhow!("{role} interface not open"))?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        channel.read(remaining)
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
}

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
                    Ok(b) => DeviceManager::new(b, models),
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
    }

    struct Shared {
        present: bool,
        keyboard_fw: &'static str,
        behaviour: Behaviour,
        written: Vec<(Role, Vec<u8>)>,
        queues: HashMap<Role, VecDeque<Vec<u8>>>,
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
            Ok(if s.present {
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
            present: true,
            keyboard_fw,
            behaviour,
            written: Vec::new(),
            queues: HashMap::new(),
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
