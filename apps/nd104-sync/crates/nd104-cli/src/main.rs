//! nd104-cli – headless console runner for the nd104-sync core (M1).
//!
//! Wired USB only. Uses the same configuration file as the app.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use nd104_core::app_update::{self, UpdateStatus};
use nd104_core::config::{self, Config};
use nd104_core::device::{DeviceHandle, DeviceState, Trust};
use nd104_core::engine::{self, EngineCommand, Status};
use nd104_core::hid::{HidBackend, HidapiBackend};
use nd104_core::secrets;
use nd104_core::weather::{self, geocoding, LocationMode};
use nd104_core::{model, APP_VERSION};

#[derive(Parser)]
#[command(
    name = "nd104-cli",
    version = APP_VERSION,
    about = "nd104-sync core on the console (Chilkey ND104, wired USB only)"
)]
struct Cli {
    /// Config file (default: %APPDATA%\nd104-sync\config.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run continuously: keep time and weather in sync (default)
    Run,
    /// Show keyboard, firmware and protocol check result
    Status,
    /// Sync time and weather once, then exit
    Sync,
    /// Fetch and print the weather (nothing is sent)
    Weather,
    /// Search a city; with --set, store the result in the config
    Geocode {
        query: String,
        /// Number of the result to store (1 = first)
        #[arg(long)]
        set: Option<usize>,
        #[arg(long, default_value = "en")]
        language: String,
    },
    /// Check GitHub for a newer nd104-sync release
    CheckUpdate,
    /// Manage API keys of weather providers (Windows Credential Manager)
    ApiKey {
        #[command(subcommand)]
        action: KeyAction,
    },
    /// Configuration helpers
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// Store a key; it is read from the console, not from the command line
    Set { provider: String },
    /// Delete the stored key
    Delete { provider: String },
    /// Show which providers have a stored key
    Status,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print the config file path
    Path,
    /// Print the effective configuration
    Show,
    /// Write a config file with default values (if none exists)
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,hyper=warn,reqwest=warn".into()),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let path = match cli.config {
        Some(p) => p,
        None => config::default_path()?,
    };

    match cli.command.unwrap_or(Command::Run) {
        Command::Run => run(&path).await,
        Command::Status => status(&path).await,
        Command::Sync => sync_once(&path).await,
        Command::Weather => print_weather(&path).await,
        Command::Geocode {
            query,
            set,
            language,
        } => geocode(&path, &query, set, &language).await,
        Command::CheckUpdate => check_update().await,
        Command::ApiKey { action } => api_key(action),
        Command::Config { action } => config_action(&path, action),
    }
}

fn load(path: &Path) -> Result<Config> {
    Config::load(path)
}

fn device() -> DeviceHandle {
    DeviceHandle::spawn(
        || Ok(Box::new(HidapiBackend::new()?) as Box<dyn HidBackend>),
        model::registry(),
    )
}

async fn run(path: &Path) -> Result<()> {
    let config = load(path)?;
    println!("nd104-sync core {APP_VERSION} – config {}", path.display());
    println!("Press Ctrl+C to stop.\n");
    let handle = engine::start(config, device());
    let mut status = handle.status.clone();
    let mut last_summary = String::new();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            changed = status.changed() => {
                if changed.is_err() { break; }
                let summary = summary(&status.borrow());
                if summary != last_summary {
                    println!("{summary}");
                    last_summary = summary;
                }
            }
        }
    }
    let _ = handle.commands.send(EngineCommand::Shutdown).await;
    let _ = handle.task.await;
    Ok(())
}

async fn status(path: &Path) -> Result<()> {
    let _ = load(path)?;
    let device = device();
    let (_, state, events) = device.poll().await?;
    for e in events {
        println!("[{:?}] {}", e.level, e.message);
    }
    println!();
    print_device(&state);
    Ok(())
}

async fn sync_once(path: &Path) -> Result<()> {
    let config = load(path)?;
    let has_location = config.weather.location.location().is_some()
        || config.weather.provider == weather::ManualProvider::ID;
    let handle = engine::start(config, device());
    let mut status = handle.status.clone();

    let result = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            status.changed().await?;
            let s = status.borrow().clone();
            if let DeviceState::Locked { reason, .. } = &s.device {
                bail!("locked: {reason}");
            }
            let weather_done =
                !has_location || s.last_weather_push.is_some() || s.weather_error.is_some();
            if s.last_time_sync.is_some() && weather_done {
                return Ok(s);
            }
        }
    })
    .await;

    let _ = handle.commands.send(EngineCommand::Shutdown).await;
    let _ = handle.task.await;
    let s = result.context("timed out – is the keyboard connected with a USB cable?")??;
    println!("{}", summary(&s));
    if let Some(e) = &s.weather_error {
        println!("Weather not sent: {e}");
    } else if !has_location {
        println!(
            "Weather not sent: no location configured (use `nd104-cli geocode <city> --set 1`)."
        );
    }
    Ok(())
}

async fn print_weather(path: &Path) -> Result<()> {
    let config = load(path)?;
    let settings = config.weather;
    let provider = weather::create(&settings.provider)
        .with_context(|| format!("unknown provider {:?}", settings.provider))?;
    let ctx = weather::context(provider.as_ref(), &settings, reqwest_client()?, None)?;
    let r = provider.fetch(&ctx).await?;
    println!(
        "{} ({}): {:?} ({}), {:.1} °C, max {:.1} °C, min {:.1} °C",
        settings.location.name,
        provider.display_name(),
        r.condition,
        if r.is_day { "day" } else { "night" },
        r.current_c,
        r.max_c,
        r.min_c
    );
    Ok(())
}

fn reqwest_client() -> Result<nd104_core::weather::HttpClient> {
    nd104_core::weather::new_http_client()
}

async fn geocode(path: &Path, query: &str, set: Option<usize>, language: &str) -> Result<()> {
    let places = geocoding::search(query, language).await?;
    if places.is_empty() {
        println!("No results for {query:?}.");
        return Ok(());
    }
    for (i, p) in places.iter().enumerate() {
        println!(
            "{:>2}. {}  ({:.4}, {:.4}){}",
            i + 1,
            p.label(),
            p.latitude,
            p.longitude,
            p.timezone
                .as_deref()
                .map(|t| format!("  {t}"))
                .unwrap_or_default()
        );
    }
    if let Some(n) = set {
        let place = places
            .get(n.wrapping_sub(1))
            .with_context(|| format!("no result number {n}"))?;
        let mut config = load(path)?;
        let location = place.to_location();
        config.weather.location.mode = LocationMode::Search;
        config.weather.location.name = location.name.clone();
        config.weather.location.lat = Some(location.lat);
        config.weather.location.lon = Some(location.lon);
        config.save(path)?;
        println!("\nSaved location {} to {}", location.name, path.display());
    }
    Ok(())
}

async fn check_update() -> Result<()> {
    println!("Installed: nd104-sync {APP_VERSION}");
    match app_update::check(APP_VERSION).await? {
        UpdateStatus::Available(r) => println!("Update available: {} → {}", r.version, r.url),
        UpdateStatus::UpToDate { latest: Some(r) } => {
            println!("Up to date (latest release {}).", r.version)
        }
        UpdateStatus::UpToDate { latest: None } => {
            println!(
                "Up to date (no releases published yet: {}).",
                app_update::RELEASES_PAGE
            )
        }
    }
    Ok(())
}

fn api_key(action: KeyAction) -> Result<()> {
    let known = |id: &str| -> Result<()> {
        match weather::info(id) {
            Some(p) if p.requires_api_key => Ok(()),
            Some(p) => bail!("{} needs no API key", p.name),
            None => bail!(
                "unknown provider {id:?} – key-based providers: {}",
                weather::PROVIDERS
                    .iter()
                    .filter(|p| p.requires_api_key)
                    .map(|p| p.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    };
    match action {
        KeyAction::Set { provider } => {
            known(&provider)?;
            println!("Paste the API key for {provider} and press Enter:");
            let mut key = String::new();
            std::io::stdin().read_line(&mut key)?;
            secrets::set(&provider, &key)?;
            println!("Stored in the Windows Credential Manager.");
        }
        KeyAction::Delete { provider } => {
            known(&provider)?;
            secrets::delete(&provider)?;
            println!("Deleted.");
        }
        KeyAction::Status => {
            for p in weather::PROVIDERS.iter().filter(|p| p.requires_api_key) {
                let state = if secrets::has(p.id) { "stored" } else { "–" };
                println!("{:<16} {state}", p.id);
            }
        }
    }
    Ok(())
}

fn config_action(path: &Path, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Path => println!("{}", path.display()),
        ConfigAction::Show => print!("{}", load(path)?.to_toml()?),
        ConfigAction::Init => {
            if path.exists() {
                println!("{} already exists – left unchanged.", path.display());
            } else {
                Config::default().save(path)?;
                println!("Created {}", path.display());
            }
        }
    }
    Ok(())
}

fn print_device(state: &DeviceState) {
    match state {
        DeviceState::NotConnected => {
            println!("No supported keyboard found. Connect the ND104 with a USB cable (wireless is not supported).")
        }
        DeviceState::Ready(d) => {
            let trust = match d.trust {
                Trust::Verified => "verified",
                Trust::Compatible => "compatible (unverified)",
            };
            println!("{} – firmware {trust}", d.model_name);
            for f in &d.firmware {
                println!("  {:<8} {}", f.role.to_string(), f.raw);
            }
            println!("  screen variant: {:?}", d.screen_variant);
        }
        DeviceState::Locked {
            model_name,
            firmware,
            reason,
        } => {
            println!("{} – LOCKED: {reason}", model_name.unwrap_or("Keyboard"));
            for f in firmware {
                println!("  {:<8} {}", f.role.to_string(), f.raw);
            }
            println!("Nothing is sent. Re-plug the keyboard or run again to re-check.");
        }
    }
}

fn summary(s: &Status) -> String {
    let device = match &s.device {
        DeviceState::NotConnected => "not connected".to_string(),
        DeviceState::Ready(d) => format!(
            "{} ({})",
            d.model_name,
            if d.trust == Trust::Verified {
                "verified"
            } else {
                "compatible"
            }
        ),
        DeviceState::Locked { reason, .. } => format!("LOCKED: {reason}"),
    };
    let time = s
        .last_sent_time
        .map(|t| t.strftime("%H:%M:%S").to_string())
        .unwrap_or_else(|| "–".into());
    let weather = s
        .weather
        .as_ref()
        .map(|w| {
            format!(
                "{:?} {:.1} °C ({:.1}/{:.1})",
                w.condition, w.current_c, w.max_c, w.min_c
            )
        })
        .unwrap_or_else(|| "–".into());
    let pushed = if s.last_weather_push.is_some() {
        ""
    } else {
        " (not sent)"
    };
    format!(
        "[{}] device: {device} | time sent: {time} | weather: {weather}{pushed}{}",
        jiff::Zoned::now().strftime("%H:%M:%S"),
        if s.paused { " | PAUSED" } else { "" }
    )
}
