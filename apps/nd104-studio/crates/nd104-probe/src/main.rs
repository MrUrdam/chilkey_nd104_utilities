//! nd104-probe – milestone M0 diagnostic tool.
//!
//! Verifies the Chilkey ND104 USB HID protocol on real hardware before the
//! actual nd104-studio app is built. Wired USB connection only.

mod hid;

use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use nd104_protocol::{
    normalize_version, theme_packet, time_packet, weather_packet, ClockTime, Rgb, Target, Theme,
    Weather, WeatherIcon,
};

use crate::hid::{hex_dump, Probe};

/// Firmware versions verified on hardware with the M0 checklist (normalized).
/// Other versions run as "compatible (unverified)".
const KNOWN_KEYBOARD_FIRMWARE: &[&str] = &["3.0.14"];
const KNOWN_SCREEN_FIRMWARE: &[&str] = &["1.2.4"];

#[derive(Parser)]
#[command(
    name = "nd104-probe",
    version,
    about = "Chilkey ND104 protocol probe (M0, wired USB only)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all ND104 HID interfaces (keyboard, screen, screen DFU)
    List,
    /// Read the firmware versions of keyboard and screen
    Version {
        /// How long to wait for answers per device
        #[arg(long, default_value_t = 1000)]
        timeout_ms: u64,
    },
    /// Print every input report a device sends
    Listen {
        #[arg(long, value_enum, default_value_t = TargetArg::Keyboard)]
        device: TargetArg,
        #[arg(long, default_value_t = 10)]
        seconds: u64,
    },
    /// Send the current local time (or --at) to the display
    Time {
        /// Local date/time instead of now, e.g. 2026-10-31T14:05:00
        #[arg(long)]
        at: Option<String>,
        #[command(flatten)]
        write: WriteOpts,
    },
    /// Send weather values to the display
    Weather {
        /// Icon index 0–9
        #[arg(long, default_value_t = 0)]
        icon: u8,
        /// Current temperature in °C
        #[arg(long, allow_negative_numbers = true, default_value_t = 12.3)]
        current: f32,
        /// Max temperature in °C
        #[arg(long, allow_negative_numbers = true, default_value_t = 15.0)]
        max: f32,
        /// Min temperature in °C
        #[arg(long, allow_negative_numbers = true, default_value_t = -3.5)]
        min: f32,
        #[command(flatten)]
        write: WriteOpts,
    },
    /// Cycle through all icons 0–9 (current temperature = icon index)
    Icons {
        /// Seconds per icon
        #[arg(long, default_value_t = 4)]
        delay_s: u64,
        #[command(flatten)]
        write: WriteOpts,
    },
    /// Set display theme and colours (web driver defaults: 1, AA7ECC, 000000)
    Theme {
        /// 1 = Neon Grid, 2 = Mechanical Age, 3 = Emotional Moment
        #[arg(long, default_value_t = 1)]
        theme: u8,
        /// Background colour as RRGGBB
        #[arg(long, default_value = "AA7ECC")]
        background: String,
        /// Text colour as RRGGBB
        #[arg(long, default_value = "000000")]
        text: String,
        #[command(flatten)]
        write: WriteOpts,
    },
}

#[derive(Args)]
struct WriteOpts {
    /// Device that receives the packet (web driver: keyboard, community notes: screen)
    #[arg(long, value_enum, default_value_t = TargetArg::Keyboard)]
    target: TargetArg,
    /// Only print the packet, do not open any device
    #[arg(long)]
    dry_run: bool,
    /// Write even if the version query was not answered as expected (verification only!)
    #[arg(long)]
    force: bool,
    /// Override the HID output report length (without report ID)
    #[arg(long)]
    report_len: Option<usize>,
}

#[derive(Clone, Copy, ValueEnum)]
enum TargetArg {
    Keyboard,
    Screen,
}

impl From<TargetArg> for Target {
    fn from(t: TargetArg) -> Self {
        match t {
            TargetArg::Keyboard => Target::Keyboard,
            TargetArg::Screen => Target::Screen,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List => Probe::new()?.list(),
        Command::Version { timeout_ms } => {
            let probe = Probe::new()?;
            let versions = probe.read_versions(Duration::from_millis(timeout_ms), true);
            println!();
            println!(
                "Keyboard firmware: {}",
                versions
                    .keyboard
                    .as_ref()
                    .map_or("unknown", |v| v.raw.as_str())
            );
            println!(
                "Screen firmware:   {}",
                versions
                    .screen
                    .as_ref()
                    .map_or("unknown", |v| v.raw.as_str())
            );
            Ok(())
        }
        Command::Listen { device, seconds } => {
            Probe::new()?.listen(device.into(), Duration::from_secs(seconds))
        }
        Command::Time { at, write } => {
            let now = match at {
                Some(text) => text
                    .parse::<jiff::civil::DateTime>()
                    .with_context(|| format!("cannot parse --at {text:?}"))?,
                None => jiff::Zoned::now().datetime(),
            };
            let packet = time_packet(&clock_time(now)?)?;
            println!("Time: {now} (weekday {:?})", now.weekday());
            send(&write, &[packet.to_vec()], Duration::ZERO)
        }
        Command::Weather {
            icon,
            current,
            max,
            min,
            write,
        } => {
            let packet = weather_packet(&Weather {
                icon: WeatherIcon::new(icon)?,
                current_c: current,
                max_c: max,
                min_c: min,
            })?;
            println!("Weather: icon {icon}, current {current} °C, max {max} °C, min {min} °C");
            send(&write, &[packet.to_vec()], Duration::ZERO)
        }
        Command::Theme {
            theme,
            background,
            text,
            write,
        } => {
            let bg = Rgb::from_hex(&background).context("--background must be RRGGBB")?;
            let fg = Rgb::from_hex(&text).context("--text must be RRGGBB")?;
            let packet = theme_packet(Theme::new(theme)?, bg, fg)?;
            println!(
                "Theme {theme}, background #{background} (RGB565 {:04X}), text #{text} (RGB565 {:04X})",
                bg.to_rgb565(),
                fg.to_rgb565()
            );
            send(&write, &[packet.to_vec()], Duration::ZERO)
        }
        Command::Icons { delay_s, write } => {
            let packets = (0..=WeatherIcon::MAX_INDEX)
                .map(|i| {
                    weather_packet(&Weather {
                        icon: WeatherIcon::new(i)?,
                        current_c: i as f32,
                        max_c: 20.0,
                        min_c: -5.0,
                    })
                    .map(|p| p.to_vec())
                })
                .collect::<Result<Vec<_>, _>>()?;
            println!("Cycling icons 0–9, {delay_s} s each (current temperature = icon index)");
            send(&write, &packets, Duration::from_secs(delay_s))
        }
    }
}

fn clock_time(dt: jiff::civil::DateTime) -> Result<ClockTime> {
    Ok(ClockTime {
        year: u16::try_from(dt.year()).context("year out of range")?,
        month: dt.month() as u8,
        day: dt.day() as u8,
        hour: dt.hour() as u8,
        minute: dt.minute() as u8,
        second: dt.second() as u8,
        weekday: dt.weekday().to_sunday_zero_offset() as u8,
    })
}

/// Sends packets after the firmware check. Each packet is followed by
/// `delay` and a short read of any answers.
fn send(opts: &WriteOpts, packets: &[Vec<u8>], delay: Duration) -> Result<()> {
    let target: Target = opts.target.into();

    if opts.dry_run {
        for (i, p) in packets.iter().enumerate() {
            println!("\nPacket {i} → {} (dry run)", target.name());
            print!("{}", hex_dump(p));
        }
        return Ok(());
    }

    let probe = Probe::new()?;
    firmware_gate(&probe, opts.force)?;

    let report_len = opts.report_len.unwrap_or(target.report_len());
    let device = probe.open(target)?;
    for (i, p) in packets.iter().enumerate() {
        println!("\nPacket {i} → {}", target.name());
        print!("{}", hex_dump(p));
        device.send(p, report_len)?;
        device.drain_answers(Duration::from_millis(300));
        if i + 1 < packets.len() {
            thread::sleep(delay);
        }
    }
    println!("\nDone. Check the display.");
    Ok(())
}

/// Protocol safety: verified firmware → go; other firmware → go with a warning
/// as long as both version queries were answered well-formed; otherwise the
/// protocol looks different → stop.
fn firmware_gate(probe: &Probe, force: bool) -> Result<()> {
    let versions = probe.read_versions(Duration::from_millis(1000), false);
    let keyboard = versions
        .keyboard
        .as_ref()
        .and_then(|v| normalize_version(&v.version));
    let screen = versions
        .screen
        .as_ref()
        .and_then(|v| normalize_version(&v.version));
    println!(
        "Firmware: keyboard {}, screen {}",
        versions
            .keyboard
            .as_ref()
            .map_or("unknown", |v| v.raw.as_str()),
        versions
            .screen
            .as_ref()
            .map_or("unknown", |v| v.raw.as_str())
    );

    let (Some(keyboard), Some(screen)) = (keyboard, screen) else {
        if force {
            println!(
                "⚠ Version query not answered as expected – writing anyway because of --force."
            );
            return Ok(());
        }
        bail!(
            "version query not answered as expected – the protocol may differ, nothing was sent.\n\
             Run `nd104-probe version` for the raw answers. Use --force only for verification."
        );
    };

    let verified = KNOWN_KEYBOARD_FIRMWARE.contains(&keyboard.as_str())
        && KNOWN_SCREEN_FIRMWARE.contains(&screen.as_str());
    if verified {
        println!("Firmware verified.");
    } else {
        println!(
            "⚠ Firmware not verified (known: keyboard {KNOWN_KEYBOARD_FIRMWARE:?}, screen \
             {KNOWN_SCREEN_FIRMWARE:?}) – protocol answers look normal, continuing as \
             compatible (unverified)."
        );
    }
    Ok(())
}
