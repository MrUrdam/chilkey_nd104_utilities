//! nd104-probe – milestone M0 diagnostic tool.
//!
//! Verifies the Chilkey ND104 HID protocol on real hardware before the
//! actual nd104-studio app is built. Primarily a USB-cable diagnostic.

mod extra;
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
    about = "Chilkey ND104 protocol probe (M0)"
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
    /// Send one READ command to the keyboard and print the answers. Only read
    /// commands are accepted: 08 (lighting/settings value), 12 (keymap
    /// buffer), 14 (knob action). Example: query 08 03 01
    Query {
        /// Command bytes in hex, e.g. 08 03 01
        #[arg(required = true, num_args = 1..)]
        bytes: Vec<String>,
        #[arg(long, default_value_t = 1500)]
        timeout_ms: u64,
    },
    /// Read a lighting zone; with values, change it (stored only with --store)
    Light {
        #[arg(long, value_enum, default_value_t = ZoneArg::Backlight)]
        zone: ZoneArg,
        /// Effect index (0 = off, 1 = solid colour)
        #[arg(long)]
        effect: Option<u8>,
        /// 0–255
        #[arg(long)]
        brightness: Option<u8>,
        /// 0–255
        #[arg(long)]
        speed: Option<u8>,
        /// Hue,saturation (0–255 each), e.g. 170,255
        #[arg(long, value_parser = parse_pair)]
        color: Option<(u8, u8)>,
        /// Store the changes in the keyboard (otherwise they last until a restart)
        #[arg(long)]
        store: bool,
    },
    /// Read a keyboard setting; with a value, change it (only the driver's values)
    Setting {
        #[arg(long, value_enum)]
        name: SettingArg,
        value: Option<u8>,
        /// Store the change in the keyboard
        #[arg(long)]
        store: bool,
    },
    /// Read one key of the keymap; with --code, change it (stored immediately!)
    Key {
        #[arg(long)]
        layer: u8,
        #[arg(long)]
        row: u8,
        #[arg(long)]
        col: u8,
        /// New key code in hex, e.g. 0029
        #[arg(long, value_parser = parse_code)]
        code: Option<u16>,
    },
    /// Read one knob action; with --code, change it (stored immediately!)
    Knob {
        #[arg(long)]
        layer: u8,
        #[arg(long, value_enum)]
        dir: DirArg,
        /// New key code in hex, e.g. 00a9
        #[arg(long, value_parser = parse_code)]
        code: Option<u16>,
    },
    /// Print one layer of the keymap as a table (read-only)
    Keymap {
        #[arg(long, default_value_t = 0)]
        layer: u8,
    },
    /// Upload a picture or GIF to the screen (USB only); --dry-run only checks it
    Picture {
        #[arg(long)]
        file: String,
        #[arg(long, value_enum, default_value_t = FitArg::Fill)]
        fit: FitArg,
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the HID report descriptor of a device (read-only)
    Descriptor {
        #[arg(long, value_enum, default_value_t = TargetArg::Keyboard)]
        device: TargetArg,
    },
    /// Print usage, report IDs and report descriptor of every collection of
    /// the keyboard, screen and receiver (read-only)
    Descriptors,
    /// Send one read command on the PC lighting channel (interface 4,
    /// FF59:0060, report ID 05): SignalRGB 21 22 23 27 28 or OpenRGB 01 02
    /// 03 05 – nothing that changes the lighting
    Rgb {
        /// Command bytes in hex, e.g. 27
        #[arg(required = true, num_args = 1..)]
        bytes: Vec<String>,
    },
    /// Test the SignalRGB colour stream: switch to PC colours (25), show one
    /// colour on all LEDs (24) for a few seconds, then give the lighting back
    /// (26). Nothing is stored in the keyboard.
    RgbTest {
        /// Colour as hex, e.g. ff0000
        #[arg(long, default_value = "ff0000")]
        color: String,
        #[arg(long, default_value_t = 5)]
        seconds: u64,
        /// LEDs per packet
        #[arg(long, default_value_t = 9)]
        per_packet: u8,
    },
    /// Read one feature report of a keyboard collection (read-only), e.g.
    /// --usage 0059:0001 --id 1 (LampArray attributes)
    Feature {
        /// Collection as page:usage in hex
        #[arg(long)]
        usage: String,
        /// Report ID in hex
        #[arg(long)]
        id: String,
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
enum SettingArg {
    DebounceMode,
    DebounceTime,
    Sleep,
    DeepSleep,
}

#[derive(Clone, Copy, ValueEnum)]
enum DirArg {
    Left,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
enum FitArg {
    Fill,
    Contain,
}

fn parse_code(text: &str) -> Result<u16, String> {
    u16::from_str_radix(text.trim_start_matches("0x"), 16).map_err(|e| e.to_string())
}

fn parse_pair(text: &str) -> Result<(u8, u8), String> {
    let (a, b) = text.split_once(',').ok_or("expected hue,saturation")?;
    Ok((
        a.trim().parse().map_err(|e| format!("{e}"))?,
        b.trim().parse().map_err(|e| format!("{e}"))?,
    ))
}

#[derive(Clone, Copy, ValueEnum)]
enum ZoneArg {
    /// Under the keys (VIA channel 3)
    Backlight,
    /// Around the keyboard and the light bar (VIA channel 2)
    Bottom,
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
        Command::Descriptor { device } => Probe::new()?.descriptor(device.into()),
        Command::Descriptors => extra::descriptors(),
        Command::Feature { usage, id } => extra::feature(&usage, &id),
        Command::Rgb { bytes } => extra::rgb(&bytes),
        Command::RgbTest {
            color,
            seconds,
            per_packet,
        } => extra::rgb_test(&color, seconds, per_packet),
        Command::Light {
            zone,
            effect,
            brightness,
            speed,
            color,
            store,
        } => extra::light(
            match zone {
                ZoneArg::Backlight => 3,
                ZoneArg::Bottom => 2,
            },
            extra::LightChange {
                effect,
                brightness,
                speed,
                color,
                store,
            },
        ),
        Command::Setting { name, value, store } => {
            use nd104_core::keyboard_settings::KeyboardSetting as S;
            let setting = match name {
                SettingArg::DebounceMode => S::DebounceMode,
                SettingArg::DebounceTime => S::DebounceTime,
                SettingArg::Sleep => S::SleepTime,
                SettingArg::DeepSleep => S::DeepSleepTime,
            };
            extra::setting(setting, value, store)
        }
        Command::Key {
            layer,
            row,
            col,
            code,
        } => extra::key(layer, row, col, code),
        Command::Knob { layer, dir, code } => {
            extra::knob(layer, matches!(dir, DirArg::Right), code)
        }
        Command::Keymap { layer } => extra::keymap(layer),
        Command::Picture { file, fit, dry_run } => extra::picture(
            &file,
            match fit {
                FitArg::Fill => nd104_core::picture::Fit::Fill,
                FitArg::Contain => nd104_core::picture::Fit::Contain,
            },
            dry_run,
        ),
        Command::Query { bytes, timeout_ms } => {
            let packet = bytes
                .iter()
                .map(|b| u8::from_str_radix(b.trim_start_matches("0x"), 16))
                .collect::<Result<Vec<u8>, _>>()
                .context("bytes must be hex, e.g. 08 03 01")?;
            // read commands only – nothing that changes the keyboard. 01/02
            // read in VIA and in the OpenRGB protocol alike (03 and 05 do not)
            const READS: [u8; 5] = [0x01, 0x02, 0x08, 0x12, 0x14];
            if !packet.first().is_some_and(|c| READS.contains(c)) {
                bail!("only read commands are allowed: 01, 02, 08, 12, 14");
            }
            let device = Probe::new()?.open(Target::Keyboard)?;
            println!("→ keyboard");
            print!("{}", hex_dump(&packet));
            device.send(&packet, 32)?;
            device.drain_answers(Duration::from_millis(timeout_ms));
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
