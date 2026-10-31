//! Scheduler: polls the device, syncs time and weather, checks for app
//! updates (PLAN.md § 5.7, § 7). UI and CLI talk to it through
//! [`EngineHandle`]: commands in, [`Status`] snapshots out.

use std::collections::VecDeque;
use std::time::Duration;

use anyhow::{anyhow, Result};
use jiff::civil::DateTime;
use jiff::{SignedDuration, Timestamp};
use serde::Serialize;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep_until, Instant, MissedTickBehavior};

use crate::app_update::{self, UpdateStatus};
use crate::clock::{ntp_offset, ClockSource};
use crate::config::Config;
use crate::device::{DeviceEvent, DeviceHandle, DeviceState, EventLevel};
use crate::firmware_update::{self, FirmwareReport};
use crate::model::{self, KeyboardModel};
use crate::weather::{self, WeatherReport};

const MAX_EVENTS: usize = 100;
const STALE_AFTER: SignedDuration = SignedDuration::from_hours(3);
const FIRST_UPDATE_CHECK: Duration = Duration::from_secs(30);
const RESUME_JUMP: SignedDuration = SignedDuration::from_secs(60);
const FAR_FUTURE: Duration = Duration::from_secs(365 * 24 * 3600);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub at: Timestamp,
    pub level: EventLevel,
    pub message: String,
}

/// Snapshot for UI/CLI.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    pub device: DeviceState,
    pub paused: bool,
    pub last_time_sync: Option<Timestamp>,
    /// Wall-clock time sent with the last sync.
    pub last_sent_time: Option<DateTime>,
    pub next_time_sync: Option<Timestamp>,
    pub next_dst_change: Option<Timestamp>,
    pub weather: Option<WeatherReport>,
    pub weather_stale: bool,
    pub weather_error: Option<String>,
    pub last_weather_push: Option<Timestamp>,
    pub next_weather_fetch: Option<Timestamp>,
    pub app_update: Option<Result<UpdateStatus, String>>,
    pub app_update_checked: Option<Timestamp>,
    /// Result of the last firmware update check (versions.json in the repo).
    pub firmware_update: Option<Result<FirmwareReport, String>>,
    pub firmware_update_checked: Option<Timestamp>,
    pub events: VecDeque<Event>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            device: DeviceState::NotConnected,
            paused: false,
            last_time_sync: None,
            last_sent_time: None,
            next_time_sync: None,
            next_dst_change: None,
            weather: None,
            weather_stale: false,
            weather_error: None,
            last_weather_push: None,
            next_weather_fetch: None,
            app_update: None,
            app_update_checked: None,
            firmware_update: None,
            firmware_update_checked: None,
            events: VecDeque::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// Send time and weather now (re-fetches the weather).
    SyncNow,
    Pause,
    Resume,
    /// Lift a lock and run all checks again.
    Recheck,
    /// Apply a changed configuration.
    Reload(Box<Config>),
    /// Manual "Check for updates".
    CheckAppUpdate,
    /// Manual firmware update check.
    CheckFirmwareUpdate,
    Shutdown,
}

pub struct EngineHandle {
    pub commands: mpsc::Sender<EngineCommand>,
    pub status: watch::Receiver<Status>,
    pub task: JoinHandle<()>,
}

/// Starts the engine on the current tokio runtime.
pub fn start(config: Config, device: DeviceHandle) -> EngineHandle {
    let (commands, rx) = mpsc::channel(32);
    let (status_tx, status) = watch::channel(Status::default());
    let engine = Engine {
        config,
        device,
        models: model::registry(),
        status: status_tx,
        ntp: None,
        weather_failures: 0,
    };
    let task = tokio::spawn(engine.run(rx));
    EngineHandle {
        commands,
        status,
        task,
    }
}

struct Engine {
    config: Config,
    device: DeviceHandle,
    models: Vec<Box<dyn KeyboardModel>>,
    status: watch::Sender<Status>,
    /// Last NTP offset and when it was measured.
    ntp: Option<(SignedDuration, Instant)>,
    weather_failures: u32,
}

impl Engine {
    async fn run(mut self, mut rx: mpsc::Receiver<EngineCommand>) {
        let mut poll = interval(Duration::from_secs(
            self.config.device.poll_interval_s.max(1).into(),
        ));
        poll.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let now = Instant::now();
        let mut next_time = now + FAR_FUTURE;
        let mut next_weather = now; // fetch right away
        let mut next_app_check = now + FIRST_UPDATE_CHECK;
        let mut last_wall = Timestamp::now();
        let mut last_mono = Instant::now();

        loop {
            let dst = self.dst_instant();
            let paused = self.status.borrow().paused;

            tokio::select! {
                _ = poll.tick() => {
                    // Wall clock jumped more than the monotonic clock: resumed from sleep.
                    let wall = Timestamp::now().duration_since(last_wall);
                    let mono = SignedDuration::try_from(last_mono.elapsed()).unwrap_or(SignedDuration::ZERO);
                    last_wall = Timestamp::now();
                    last_mono = Instant::now();
                    let resumed = wall - mono > RESUME_JUMP;
                    if resumed {
                        self.event(EventLevel::Info, "Resumed from sleep, syncing again");
                    }
                    let became_ready = self.poll_device().await;
                    // First connection: compare the firmware with the version list once.
                    let never_checked = self.status.borrow().firmware_update_checked.is_none();
                    if became_ready && never_checked && self.config.updates.firmware_check {
                        self.check_firmware_update(false).await;
                    }
                    if (became_ready || resumed) && !paused {
                        next_time = Instant::now();
                        if resumed || !self.push_cached_weather().await {
                            next_weather = Instant::now();
                        }
                    }
                }
                _ = sleep_until(next_time), if !paused => {
                    self.sync_time().await;
                    next_time = Instant::now() + self.time_interval();
                }
                _ = sleep_until(dst.unwrap_or(next_time)), if dst.is_some() && !paused => {
                    self.event(EventLevel::Info, "Summer/winter time change");
                    self.sync_time().await;
                }
                _ = sleep_until(next_weather), if !paused => {
                    let ok = self.update_weather().await;
                    next_weather = Instant::now() + self.weather_delay(ok);
                }
                _ = sleep_until(next_app_check), if self.config.updates.app_check || self.config.updates.firmware_check => {
                    if self.config.updates.app_check {
                        self.check_app_update().await;
                    }
                    if self.config.updates.firmware_check {
                        self.check_firmware_update(false).await;
                    }
                    next_app_check = Instant::now() + Duration::from_secs(u64::from(self.config.updates.check_interval_h.max(1)) * 3600);
                }
                cmd = rx.recv() => match cmd {
                    None | Some(EngineCommand::Shutdown) => break,
                    Some(EngineCommand::SyncNow) => {
                        self.status.send_modify(|s| s.paused = false);
                        self.sync_time().await;
                        let ok = self.update_weather().await;
                        next_time = Instant::now() + self.time_interval();
                        next_weather = Instant::now() + self.weather_delay(ok);
                    }
                    Some(EngineCommand::Pause) => {
                        self.status.send_modify(|s| s.paused = true);
                        self.event(EventLevel::Info, "Updates paused");
                    }
                    Some(EngineCommand::Resume) => {
                        self.status.send_modify(|s| s.paused = false);
                        self.event(EventLevel::Info, "Updates resumed");
                        next_time = Instant::now();
                        next_weather = Instant::now();
                    }
                    Some(EngineCommand::Recheck) => {
                        if self.recheck_device().await {
                            next_time = Instant::now();
                            next_weather = Instant::now();
                        }
                    }
                    Some(EngineCommand::Reload(config)) => {
                        self.config = *config;
                        self.ntp = None;
                        self.event(EventLevel::Info, "Configuration applied");
                        next_time = Instant::now();
                        next_weather = Instant::now();
                    }
                    Some(EngineCommand::CheckAppUpdate) => self.check_app_update().await,
                    Some(EngineCommand::CheckFirmwareUpdate) => self.check_firmware_update(true).await,
                },
            }
        }
    }

    fn time_interval(&self) -> Duration {
        Duration::from_secs(u64::from(self.config.clock.sync_interval_min.max(1)) * 60)
    }

    /// Normal interval after success; exponential backoff (1, 2, 4 … min) after errors.
    fn weather_delay(&self, ok: bool) -> Duration {
        let normal = Duration::from_secs(u64::from(self.config.weather.interval_min()) * 60);
        if ok {
            return normal;
        }
        let backoff =
            Duration::from_secs(60 * 2u64.pow(self.weather_failures.saturating_sub(1).min(6)));
        backoff.min(normal)
    }

    fn dst_instant(&self) -> Option<Instant> {
        let next = self.status.borrow().next_dst_change?;
        let wait = next.duration_since(Timestamp::now());
        let wait = Duration::try_from(wait).unwrap_or(Duration::ZERO) + Duration::from_secs(1);
        Some(Instant::now() + wait)
    }

    fn event(&self, level: EventLevel, message: impl Into<String>) {
        let event = Event {
            at: Timestamp::now(),
            level,
            message: message.into(),
        };
        match level {
            EventLevel::Info => tracing::info!("{}", event.message),
            EventLevel::Warning => tracing::warn!("{}", event.message),
            EventLevel::Error => tracing::error!("{}", event.message),
        }
        self.push_event(event);
    }

    fn push_event(&self, event: Event) {
        self.status.send_modify(|s| {
            s.events.push_front(event);
            s.events.truncate(MAX_EVENTS);
        });
    }

    fn device_events(&self, events: Vec<DeviceEvent>) {
        // Device events are already logged by the device manager.
        for e in events {
            self.push_event(Event {
                at: Timestamp::now(),
                level: e.level,
                message: e.message,
            });
        }
    }

    fn set_device_state(&self, state: DeviceState) {
        self.status.send_modify(|s| s.device = state);
    }

    fn ready_model(&self) -> Option<&dyn KeyboardModel> {
        let status = self.status.borrow();
        let DeviceState::Ready(d) = &status.device else {
            return None;
        };
        let id = d.model_id;
        drop(status);
        self.models
            .iter()
            .find(|m| m.id() == id)
            .map(|m| m.as_ref())
    }

    /// Returns `true` if the device just became ready.
    async fn poll_device(&self) -> bool {
        match self.device.poll().await {
            Ok((changed, state, events)) => {
                self.device_events(events);
                let ready = matches!(state, DeviceState::Ready(_));
                self.set_device_state(state);
                changed && ready
            }
            Err(e) => {
                self.event(EventLevel::Error, format!("{e:#}"));
                false
            }
        }
    }

    async fn recheck_device(&self) -> bool {
        match self.device.recheck().await {
            Ok((state, events)) => {
                self.device_events(events);
                let ready = matches!(state, DeviceState::Ready(_));
                self.set_device_state(state);
                ready
            }
            Err(e) => {
                self.event(EventLevel::Error, format!("{e:#}"));
                false
            }
        }
    }

    async fn send(&self, packet: crate::model::Packet) -> bool {
        match self.device.send(packet).await {
            Ok((result, state, events)) => {
                self.device_events(events);
                self.set_device_state(state);
                result.is_ok()
            }
            Err(e) => {
                self.event(EventLevel::Error, format!("{e:#}"));
                false
            }
        }
    }

    async fn now(&mut self) -> Result<Timestamp> {
        let now = Timestamp::now();
        if self.config.clock.source != ClockSource::Ntp {
            return Ok(now);
        }
        // Re-measure the NTP offset at most every 6 hours.
        let fresh =
            matches!(self.ntp, Some((_, at)) if at.elapsed() < Duration::from_secs(6 * 3600));
        if !fresh {
            let server = self.config.clock.ntp_server.clone();
            let offset =
                tokio::task::spawn_blocking(move || ntp_offset(&server, Duration::from_secs(5)))
                    .await
                    .map_err(|e| anyhow!("NTP task failed: {e}"))??;
            self.event(EventLevel::Info, format!("NTP offset {offset:#}"));
            self.ntp = Some((offset, Instant::now()));
        }
        let (offset, _) = self.ntp.expect("set above");
        Ok(now.checked_add(offset)?)
    }

    async fn sync_time(&mut self) {
        let instant = match self.now().await {
            Ok(t) => t,
            Err(e) => {
                self.event(
                    EventLevel::Warning,
                    format!("NTP failed, using PC time: {e:#}"),
                );
                Timestamp::now()
            }
        };
        let clock = self.config.clock.clone();
        let next_dst = clock.next_transition(instant);
        let next_sync = instant
            .checked_add(SignedDuration::try_from(self.time_interval()).unwrap_or_default())
            .ok();
        self.status.send_modify(|s| {
            s.next_dst_change = next_dst;
            s.next_time_sync = next_sync;
        });

        let local = match clock.local_time(instant) {
            Ok(t) => t,
            Err(e) => {
                self.event(EventLevel::Error, format!("Clock: {e:#}"));
                return;
            }
        };
        let Some(model) = self.ready_model() else {
            return;
        };
        let packet = match model.encode_time(&local) {
            Ok(p) => p,
            Err(e) => {
                self.event(EventLevel::Error, format!("Cannot encode time: {e:#}"));
                return;
            }
        };
        if self.send(packet).await {
            self.status.send_modify(|s| {
                s.last_time_sync = Some(Timestamp::now());
                s.last_sent_time = Some(local);
            });
            tracing::info!("time synced: {local}");
        }
    }

    /// Fetches (with fallback) and pushes the weather. Returns `true` on a successful fetch.
    async fn update_weather(&mut self) -> bool {
        let report = match self.fetch_weather().await {
            Ok(r) => {
                self.weather_failures = 0;
                r
            }
            Err(e) => {
                self.weather_failures += 1;
                let message = format!("{e:#}");
                self.event(EventLevel::Warning, format!("Weather: {message}"));
                self.status.send_modify(|s| {
                    s.weather_error = Some(message);
                    s.weather_stale = s.weather.as_ref().map_or(true, |w| {
                        Timestamp::now().duration_since(w.observed_at) > STALE_AFTER
                    });
                });
                return false;
            }
        };
        let next = Timestamp::now()
            .checked_add(SignedDuration::from_mins(
                self.config.weather.interval_min().into(),
            ))
            .ok();
        self.status.send_modify(|s| {
            s.weather = Some(report);
            s.weather_error = None;
            s.weather_stale = false;
            s.next_weather_fetch = next;
        });
        self.push_cached_weather().await;
        true
    }

    async fn fetch_weather(&self) -> Result<WeatherReport> {
        let settings = self.config.weather.clone();
        let client = weather::new_http_client()?;
        // Each provider gets its own context: key-based ones need their own key.
        let fetch = |id: &str| {
            let provider = weather::create(id);
            let ctx = provider
                .as_ref()
                .map(|p| weather::context(p.as_ref(), &settings, client.clone(), None));
            let id = id.to_string();
            async move {
                let provider =
                    provider.ok_or_else(|| anyhow!("unknown weather provider {id:?}"))?;
                provider.fetch(&ctx.expect("created with provider")?).await
            }
        };
        match fetch(&settings.provider).await {
            Ok(r) => Ok(r),
            Err(e) if !settings.fallback_provider.is_empty() => {
                self.event(
                    EventLevel::Warning,
                    format!(
                        "{} failed ({e:#}), trying {}",
                        settings.provider, settings.fallback_provider
                    ),
                );
                fetch(&settings.fallback_provider).await
            }
            Err(e) => Err(e),
        }
    }

    /// Sends the last weather report if the device is ready. Returns `true` if sent.
    async fn push_cached_weather(&self) -> bool {
        let Some(report) = self.status.borrow().weather.clone() else {
            return false;
        };
        let Some(model) = self.ready_model() else {
            return false;
        };
        let packet = match model.encode_weather(&report) {
            Ok(p) => p,
            Err(e) => {
                self.event(EventLevel::Error, format!("Cannot encode weather: {e:#}"));
                return false;
            }
        };
        let sent = self.send(packet).await;
        if sent {
            self.status
                .send_modify(|s| s.last_weather_push = Some(Timestamp::now()));
            tracing::info!(
                "weather sent: {:?} {:.1} °C ({:.1} / {:.1})",
                report.condition,
                report.current_c,
                report.max_c,
                report.min_c
            );
        }
        sent
    }

    /// Compares the keyboard firmware with `firmware/versions.json`.
    /// Automatic checks are skipped silently while no keyboard is known.
    async fn check_firmware_update(&self, manual: bool) {
        let (firmware, model_id, model_name) = match &self.status.borrow().device {
            DeviceState::Ready(d) => (d.firmware.clone(), Some(d.model_id), Some(d.model_name)),
            DeviceState::Locked {
                firmware,
                model_name,
                ..
            } if !firmware.is_empty() => (firmware.clone(), None, *model_name),
            _ => (Vec::new(), None, None),
        };
        let model = self.models.iter().find(|m| {
            model_id.is_some_and(|id| m.id() == id)
                || model_name.is_some_and(|n| m.display_name() == n)
        });
        let result = match model {
            None | Some(_) if firmware.is_empty() => {
                if !manual {
                    return;
                }
                Err("keyboard not connected – its firmware is unknown".to_string())
            }
            None => Err("no version list entry for this keyboard".to_string()),
            Some(model) => firmware_update::fetch_manifest()
                .await
                .map(|m| firmware_update::evaluate(&m, model.as_ref(), &firmware))
                .map_err(|e| format!("{e:#}")),
        };
        match &result {
            Ok(r) if r.update_available() => {
                self.event(EventLevel::Warning, "New keyboard firmware is available")
            }
            Ok(_) => self.event(
                EventLevel::Info,
                "Keyboard firmware checked against the version list",
            ),
            Err(e) => self.event(
                EventLevel::Warning,
                format!("Firmware update check failed: {e}"),
            ),
        }
        self.status.send_modify(|s| {
            s.firmware_update = Some(result);
            s.firmware_update_checked = Some(Timestamp::now());
        });
    }

    async fn check_app_update(&self) {
        let result = app_update::check(crate::APP_VERSION)
            .await
            .map_err(|e| format!("{e:#}"));
        match &result {
            Ok(UpdateStatus::Available(r)) => self.event(
                EventLevel::Warning,
                format!("nd104-sync {} is available: {}", r.version, r.url),
            ),
            Ok(UpdateStatus::UpToDate { .. }) => {
                self.event(EventLevel::Info, "nd104-sync is up to date")
            }
            Err(e) => self.event(EventLevel::Warning, format!("App update check failed: {e}")),
        }
        self.status.send_modify(|s| {
            s.app_update = Some(result);
            s.app_update_checked = Some(Timestamp::now());
        });
    }
}
