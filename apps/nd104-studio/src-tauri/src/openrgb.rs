//! OpenRGB SDK server: offers the ND104 to OpenRGB (as a client), Artemis,
//! Aurora and other SDK clients and passes their colours on to the
//! keyboard's lighting channel (protocol part: `nd104_core::openrgb`).
//!
//! - Listens on 127.0.0.1 only, off unless switched on (Device settings).
//! - The keyboard is offered only while it is connected by USB cable and its
//!   checks passed; clients are told when that changes.
//! - When the last client disconnects, the keyboard gets its own lighting
//!   back. Nothing is ever stored in the keyboard.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use nd104_core::device::{Connection, DeviceState};
use nd104_core::engine::Status;
use nd104_core::hid::{HidBackend, HidChannel, HidapiBackend};
use nd104_core::openrgb::{self as sdk, packet, Led};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use crate::AppState;

/// What the settings show.
#[derive(Debug, Clone, Default, Serialize)]
pub struct OpenRgbStatus {
    pub running: bool,
    pub port: u16,
    pub clients: usize,
    /// The keyboard is offered to clients right now.
    pub offered: bool,
    /// E.g. the port is in use.
    pub error: Option<String>,
}

/// The running server, kept in Tauri's state.
#[derive(Default)]
pub struct OpenRgbServer {
    running: Mutex<Option<Running>>,
    status: Arc<Mutex<OpenRgbStatus>>,
}

struct Running {
    port: u16,
    stop: Arc<AtomicBool>,
}

#[tauri::command]
pub fn openrgb_status(app: AppHandle) -> OpenRgbStatus {
    app.state::<OpenRgbServer>().status.lock().unwrap().clone()
}

/// Starts, stops or moves the server to match the settings.
pub fn apply(app: &AppHandle, enabled: bool, port: u16) {
    let server = app.state::<OpenRgbServer>();
    let mut running = server.running.lock().unwrap();
    if let Some(r) = running.as_ref() {
        if enabled && r.port == port {
            return;
        }
        r.stop.store(true, Ordering::Relaxed);
        *running = None;
    }
    *server.status.lock().unwrap() = OpenRgbStatus {
        port,
        ..Default::default()
    };
    if !enabled {
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let status_rx = app.state::<AppState>().status.clone();
    let info = server.status.clone();
    let stop_thread = stop.clone();
    thread::spawn(move || serve(port, stop_thread, status_rx, info));
    *running = Some(Running { port, stop });
}

/// State shared by the server and its client threads. Nothing here talks to
/// the keyboard: the connections only note what they want, and the writer
/// thread sends it at a pace the keyboard can take.
struct Shared {
    leds: Vec<Led>,
    colors: Vec<u32>,
    /// Keyboard firmware while the keyboard is offered.
    offered: Option<String>,
    /// Show the clients' colours (`true`) or the keyboard's own lighting.
    want_direct: bool,
    /// Something changed that the writer has not sent yet.
    dirty: bool,
    clients: Vec<Client>,
    next_id: u64,
}

struct Client {
    id: u64,
    stream: Arc<Mutex<TcpStream>>,
    protocol: u32,
}

impl Shared {
    /// The clients' colours should be shown.
    fn show(&mut self) {
        self.want_direct = true;
        self.dirty = true;
    }

    /// The keyboard's own lighting should come back.
    fn release(&mut self) {
        self.want_direct = false;
        self.dirty = true;
    }

    fn notify_all(&mut self, id: u32) {
        for c in &self.clients {
            if c.protocol >= 1 {
                let _ = c.stream.lock().unwrap().write_all(&sdk::packet(0, id, &[]));
            }
        }
    }
}

/// How often the writer sends the newest colours (about 30 frames/s).
const FRAME: Duration = Duration::from_millis(33);
/// Pause between the reports of one frame: the keyboard needs a moment for
/// each (sent back to back, it stops accepting and writes time out).
const REPORT_GAP: Duration = Duration::from_millis(2);
/// After a failed write, wait this long before opening the channel again.
const REOPEN_DELAY: Duration = Duration::from_secs(1);

/// The keyboard's lighting channel: interface 4, FF59:0060, report ID 05
/// (the channel adds the report ID itself).
fn open_channel(backend: &mut Option<HidapiBackend>) -> Option<Box<dyn HidChannel>> {
    if backend.is_none() {
        *backend = HidapiBackend::new().ok();
    }
    let backend = backend.as_mut()?;
    let info = backend.enumerate().ok()?.into_iter().find(|i| {
        i.vid == 0x6D67 && !i.is_bluetooth() && i.usage_page == 0xFF59 && i.interface_number == 4
    })?;
    match backend.open(&info) {
        Ok(channel) => Some(channel),
        Err(e) => {
            tracing::warn!("OpenRGB server: cannot open the lighting channel: {e:#}");
            None
        }
    }
}

/// Sends one report (`sdk` reports start with the report ID, which the
/// channel adds itself) and reads away the keyboard's answer, like
/// `nd104-probe rgb-test`, which was checked on the keyboard.
fn send(channel: &mut Box<dyn HidChannel>, report: &[u8]) -> bool {
    if channel.write(&report[1..]).is_err() {
        return false;
    }
    while matches!(channel.read(Duration::ZERO), Ok(Some(_))) {}
    thread::sleep(REPORT_GAP);
    true
}

/// The only thread that talks to the lighting channel: sends the newest
/// colours (or gives the lighting back) whenever something changed.
fn writer(shared: Arc<Mutex<Shared>>, stop: Arc<AtomicBool>) {
    let mut backend = None;
    let mut channel: Option<Box<dyn HidChannel>> = None;
    let mut direct = false;
    let mut retry_at: Option<std::time::Instant> = None;
    loop {
        let stopping = stop.load(Ordering::Relaxed);
        if !stopping {
            thread::sleep(FRAME);
        }
        let (offered, want_direct, colors) = {
            let mut s = shared.lock().unwrap();
            if !s.dirty && !stopping {
                continue;
            }
            s.dirty = false;
            let want = s.want_direct && !stopping;
            (s.offered.is_some(), want, s.colors.clone())
        };
        if !offered {
            // unplugged: the keyboard is in its own mode again
            channel = None;
            direct = false;
        } else if want_direct || direct {
            if channel.is_none() && !retry_at.is_some_and(|t| std::time::Instant::now() < t) {
                channel = open_channel(&mut backend);
            }
            let ok = match &mut channel {
                None => false,
                Some(c) if want_direct => {
                    (direct || send(c, &sdk::report_direct_on()))
                        && sdk::reports_colors(&colors).iter().all(|r| send(c, r))
                }
                Some(c) => send(c, &sdk::report_direct_off()),
            };
            if ok {
                direct = want_direct;
                retry_at = None;
            } else {
                tracing::debug!(
                    "OpenRGB server: the lighting channel does not accept, trying again"
                );
                channel = None;
                direct = false;
                retry_at = Some(std::time::Instant::now() + REOPEN_DELAY);
                // keep the wish, send again
                shared.lock().unwrap().dirty = true;
            }
        }
        if stopping {
            return;
        }
    }
}

/// The keyboard is offered while connected by cable and checked; returns
/// its firmware for the description.
fn offered(status: &Status) -> Option<String> {
    match &status.device {
        DeviceState::Ready(d) if d.connection == Connection::Usb => Some(
            d.firmware
                .iter()
                .find(|f| f.role == nd104_core::model::Role::Keyboard)
                .map(|f| f.raw.clone())
                .unwrap_or_default(),
        ),
        _ => None,
    }
}

fn serve(
    port: u16,
    stop: Arc<AtomicBool>,
    status: watch::Receiver<Status>,
    info: Arc<Mutex<OpenRgbStatus>>,
) {
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("OpenRGB server: cannot listen on port {port}: {e}");
            info.lock().unwrap().error = Some(e.to_string());
            return;
        }
    };
    let _ = listener.set_nonblocking(true);
    tracing::info!("OpenRGB server listening on 127.0.0.1:{port}");
    info.lock().unwrap().running = true;

    let leds = sdk::layout();
    let shared = Arc::new(Mutex::new(Shared {
        colors: vec![0; leds.len()],
        leds,
        offered: offered(&status.borrow()),
        want_direct: false,
        dirty: false,
        clients: Vec::new(),
        next_id: 0,
    }));

    let stop_writer = Arc::new(AtomicBool::new(false));
    let writer_thread = {
        let (shared, stop) = (shared.clone(), stop_writer.clone());
        thread::spawn(move || writer(shared, stop))
    };

    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, addr)) => {
                tracing::info!("OpenRGB client connected from {addr}");
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_nodelay(true);
                let writer = match stream.try_clone() {
                    Ok(w) => Arc::new(Mutex::new(w)),
                    Err(_) => continue,
                };
                let id = {
                    let mut s = shared.lock().unwrap();
                    s.next_id += 1;
                    let id = s.next_id;
                    s.clients.push(Client {
                        id,
                        stream: writer.clone(),
                        protocol: 0,
                    });
                    id
                };
                let shared = shared.clone();
                let info = info.clone();
                thread::spawn(move || {
                    if let Err(e) = client(stream, id, &writer, &shared) {
                        tracing::debug!("OpenRGB client {id}: {e}");
                    }
                    let mut s = shared.lock().unwrap();
                    s.clients.retain(|c| c.id != id);
                    if s.clients.is_empty() {
                        s.release();
                    }
                    info.lock().unwrap().clients = s.clients.len();
                    tracing::info!("OpenRGB client disconnected");
                });
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100))
            }
            Err(e) => {
                tracing::warn!("OpenRGB server: {e}");
                thread::sleep(Duration::from_millis(500));
            }
        }
        // keyboard plugged in / out: tell the clients
        let now = offered(&status.borrow());
        let mut s = shared.lock().unwrap();
        if now.is_some() != s.offered.is_some() {
            s.offered = now;
            s.dirty = true;
            s.notify_all(packet::DEVICE_LIST_UPDATED);
        }
        let mut i = info.lock().unwrap();
        i.clients = s.clients.len();
        i.offered = s.offered.is_some();
    }

    // stopped: give the lighting back and close the connections
    let mut s = shared.lock().unwrap();
    s.release();
    for c in s.clients.drain(..) {
        let _ = c.stream.lock().unwrap().shutdown(std::net::Shutdown::Both);
    }
    drop(s);
    stop_writer.store(true, Ordering::Relaxed);
    let _ = writer_thread.join();
    *info.lock().unwrap() = OpenRgbStatus {
        port,
        ..Default::default()
    };
    tracing::info!("OpenRGB server stopped");
}

/// Answers one client until it disconnects.
fn client(
    mut stream: TcpStream,
    id: u64,
    writer: &Arc<Mutex<TcpStream>>,
    shared: &Arc<Mutex<Shared>>,
) -> std::io::Result<()> {
    let reply = |device: u32, packet_id: u32, data: &[u8]| -> std::io::Result<()> {
        writer
            .lock()
            .unwrap()
            .write_all(&sdk::packet(device, packet_id, data))
    };
    let u32_at = |data: &[u8], at: usize| -> Option<u32> {
        Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
    };
    loop {
        let mut head = [0u8; sdk::HEADER_LEN];
        stream.read_exact(&mut head)?;
        let Some(h) = sdk::parse_header(&head) else {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "not an OpenRGB client",
            ));
        };
        if h.size > 1 << 20 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "packet too large",
            ));
        }
        let mut data = vec![0u8; h.size as usize];
        stream.read_exact(&mut data)?;

        match h.id {
            packet::REQUEST_PROTOCOL_VERSION => {
                let theirs = u32_at(&data, 0).unwrap_or(0);
                let protocol = theirs.min(sdk::SERVER_PROTOCOL);
                let mut s = shared.lock().unwrap();
                if let Some(c) = s.clients.iter_mut().find(|c| c.id == id) {
                    c.protocol = protocol;
                }
                drop(s);
                reply(0, h.id, &sdk::SERVER_PROTOCOL.to_le_bytes())?;
            }
            packet::SET_CLIENT_NAME => {
                let name = String::from_utf8_lossy(&data);
                tracing::info!("OpenRGB client: {}", name.trim_end_matches('\0'));
            }
            packet::REQUEST_CONTROLLER_COUNT => {
                let count = u32::from(shared.lock().unwrap().offered.is_some());
                reply(0, h.id, &count.to_le_bytes())?;
            }
            packet::REQUEST_CONTROLLER_DATA => {
                let wanted = u32_at(&data, 0).unwrap_or(0).min(sdk::SERVER_PROTOCOL);
                let s = shared.lock().unwrap();
                if h.device == 0 {
                    if let Some(version) = &s.offered {
                        let block = sdk::controller_data(
                            wanted,
                            &sdk::DeviceInfo {
                                name: "Chilkey ND104",
                                vendor: "Chilkey",
                                description: "Chilkey ND104 via ND104 Studio",
                                version,
                                location: "ND104 Studio (USB, lighting channel)",
                                active_mode: if s.want_direct {
                                    sdk::MODE_DIRECT
                                } else {
                                    sdk::MODE_KEYBOARD
                                },
                            },
                            &s.leds,
                            &s.colors,
                        );
                        drop(s);
                        reply(0, h.id, &block)?;
                    }
                }
            }
            packet::PROFILE_LIST => {
                // no profiles: data_size 6, count 0
                let mut body = 6u32.to_le_bytes().to_vec();
                body.extend(0u16.to_le_bytes());
                reply(0, h.id, &body)?;
            }
            packet::UPDATE_LEDS if h.device == 0 => {
                if let Some(colors) = sdk::parse_colors(&data, 4) {
                    let mut s = shared.lock().unwrap();
                    let n = colors.len().min(s.colors.len());
                    s.colors[..n].copy_from_slice(&colors[..n]);
                    s.show();
                }
            }
            packet::UPDATE_ZONE_LEDS if h.device == 0 => {
                let zone = u32_at(&data, 4);
                if let (Some(zone), Some(colors)) = (zone, sdk::parse_colors(&data, 8)) {
                    let mut s = shared.lock().unwrap();
                    if let Some(start) = sdk::zone_start(&s.leds, zone) {
                        let n = colors.len().min(s.colors.len() - start);
                        s.colors[start..start + n].copy_from_slice(&colors[..n]);
                        s.show();
                    }
                }
            }
            packet::UPDATE_SINGLE_LED if h.device == 0 => {
                if let (Some(led), Some(color)) = (u32_at(&data, 0), u32_at(&data, 4)) {
                    let mut s = shared.lock().unwrap();
                    if let Some(c) = s.colors.get_mut(led as usize) {
                        *c = color;
                        s.show();
                    }
                }
            }
            packet::SET_CUSTOM_MODE if h.device == 0 => shared.lock().unwrap().show(),
            packet::UPDATE_MODE | packet::SAVE_MODE if h.device == 0 => {
                let mode = u32_at(&data, 4).map(|m| m as i32);
                let mut s = shared.lock().unwrap();
                if mode == Some(sdk::MODE_KEYBOARD) {
                    s.release();
                } else {
                    s.show();
                }
            }
            // resize zone, profiles, plugins …: nothing to do here
            other => tracing::debug!("OpenRGB client: packet {other} ignored"),
        }
    }
}
