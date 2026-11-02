# nd104-probe

Diagnostic command-line tool for **milestone M0**. It checks the Chilkey ND104
protocol on real hardware before the ND104 Studio app is built.

> 🔌 **USB cable or Bluetooth** – the same as ND104 Studio, see
> [Connection modes](../../README.md#connection-modes) (`picture` and the screen version: cable only).
> 🪟 Tested target: Windows 10/11.
> ⚠️ Close the official web driver (nd104.chilkey.com) first. It holds the
> same HID interfaces open.

## What it does

| Command | Purpose | Writes to the device? |
|---|---|---|
| `list` | Show all ND104 HID interfaces (keyboard, screen, screen DFU) with usage page, release number, path | no |
| `version` | Send the firmware version queries and print raw answers plus parsed versions | only the read-only version query |
| `listen` | Print every input report of a device for N seconds | no |
| `time` | Send the current (or a given) local time | yes |
| `weather` | Send icon + current/max/min temperature | yes |
| `icons` | Cycle icons 0–9 (current temperature = icon index) | yes |
| `descriptor` | Print the HID report descriptor of the vendor collection (shows the report ID used over Bluetooth) | no |
| `query 08 03 01` | Send one **read** command (`08`, `12` or `14` only) and print the answers | no |
| `light --zone backlight\|bottom` | Read effect, brightness, speed and colour; `--effect/--brightness/--speed/--color h,s` change them, `--store` keeps them | only with values (stored only with `--store`) |
| `setting --name debounce-mode\|debounce-time\|sleep\|deep-sleep [value]` | Read a keyboard setting; a value changes it (only the values Chilkey's driver offers), `--store` keeps it | only with a value |
| `key --layer L --row R --col C [--code XXXX]` | Read one key; `--code` changes it, reads it back and prints the command to undo it | only with `--code` (**stored immediately**) |
| `knob --layer L --dir left\|right [--code XXXX]` | The same for the knob (layers 0–3) | only with `--code` (**stored immediately**) |
| `keymap --layer L` | Print one layer of the keymap as an 8 × 16 table | no |
| `picture --file F [--fit fill\|contain] [--dry-run]` | Check and convert a picture/GIF like the app, then upload it to the screen (USB only) | yes (`--dry-run`: no) |

All keyboard commands work over the USB cable **and over Bluetooth**: the tool
reads the report ID from the descriptor (over Bluetooth the vendor collection
uses report ID 2) and adds/removes it automatically. The screen controller –
needed for `picture` and the screen version – is reachable by cable only.

What it does **not** do: flash firmware, change macros, or work over 2.4 GHz.
Keymap, lighting and settings change only with an explicit value (see the
table).

## Firmware safety

Before `time`, `weather` and `icons` write anything, the probe reads both
firmware versions:
- **Verified version:** it sends.
- **Other (older/newer) version, both queries answered well-formed:** it sends
  and prints a "compatible (unverified)" warning.
- **Query not answered or malformed:** the protocol may be different, so it
  stops and sends nothing.

Any write error aborts immediately.

`--force` skips the stop for a failed version query. Use it **only** to
investigate a new firmware during M0. Unlike the probe, the ND104 Studio app has
no such override.

`--dry-run` prints the packets without opening any device.

## Usage

```bash
cd apps/nd104-studio
cargo run -p nd104-probe -- list
cargo run -p nd104-probe -- version
cargo run -p nd104-probe -- listen --device screen --seconds 15
cargo run -p nd104-probe -- time --dry-run
cargo run -p nd104-probe -- time --target keyboard
cargo run -p nd104-probe -- time --target screen --at 2026-12-24T18:30:00
cargo run -p nd104-probe -- weather --icon 5 --current -3.5 --max 1 --min -7.2
cargo run -p nd104-probe -- icons --target keyboard --delay-s 5
```

Options for the writing commands:

| Option | Default | Meaning |
|---|---|---|
| `--target keyboard\|screen` | `keyboard` | Device that receives the packet (web driver: keyboard, community notes: screen) |
| `--dry-run` | off | Print packets only |
| `--force` | off | Write despite unknown firmware (verification only) |
| `--report-len N` | 32 / 64 | Override the HID report length if Windows rejects the write |

## M0 test plan

Work through these in order and note the results in
[docs/protocol.md § 6](../../../../docs/protocol.md#6-open-points-for-m0-hardware-verification):

1. `list`: do both devices appear with usage page `ff60` and usage `0061`?
2. `version`: which strings come back? Note the raw answers. → fill in the
   README firmware table.
3. `time --target keyboard`, then `time --target screen`: which one updates the clock?
4. `weather --icon 0 --current 21.5 --max 24 --min 12`: are the values shown correctly?
5. `weather --icon 5 --current -3.5 --max 1 --min -7.2`: are negative values right?
6. `icons`: photograph or describe each icon 0–9.
7. `listen --device screen` while switching display pages with Fn + knob: what does the screen send?

## Build

Requirements: Rust stable (MSVC toolchain) and the Visual Studio Build Tools
(C++ workload). HID access uses the pure-Rust Windows backend of `hidapi`
(`windows-native`), so no C compiler is needed for HID itself.

```bash
cd apps/nd104-studio
cargo build -p nd104-probe --release   # → target/release/nd104-probe.exe
cargo test -p nd104-protocol           # protocol unit tests, no hardware needed
```
