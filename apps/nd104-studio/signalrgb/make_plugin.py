"""Generates Chilkey_ND104.js (SignalRGB plugin) from leds.json – the LED info
the keyboard reports (OpenRGB command 05 on the lighting channel:
`nd104-probe rgb 05 <first> <count>`, read 2026-11-02, firmware V3.0.14).
Each LED: index, x/y position (QMK points), flags, three unknown bytes and
the keycode of the key above it.

    python make_plugin.py leds.json Chilkey_ND104.js
"""
import json, sys
from collections import Counter

leds = json.load(open(sys.argv[1]))
out_path = sys.argv[2]

names = {0x28: "Enter", 0x29: "Esc", 0x2a: "Backspace", 0x2b: "Tab", 0x2c: "Space", 0x2d: "-", 0x2e: "=",
         0x2f: "[", 0x30: "]", 0x31: "\\\\", 0x32: "#", 0x33: ";", 0x34: "'", 0x35: "`", 0x36: ",", 0x37: ".",
         0x38: "/", 0x39: "Caps Lock", 0x46: "Print Screen", 0x47: "Scroll Lock", 0x48: "Pause", 0x49: "Insert",
         0x4a: "Home", 0x4b: "Page Up", 0x4c: "Delete", 0x4d: "End", 0x4e: "Page Down", 0x4f: "Right",
         0x50: "Left", 0x51: "Down", 0x52: "Up", 0x53: "Num Lock", 0x54: "Num /", 0x55: "Num *", 0x56: "Num -",
         0x57: "Num +", 0x58: "Num Enter", 0x62: "Num 0", 0x63: "Num .", 0x64: "ISO \\\\", 0x65: "Menu",
         0xe0: "Left Ctrl", 0xe1: "Left Shift", 0xe2: "Left Alt", 0xe3: "Left Win", 0xe4: "Right Ctrl",
         0xe5: "Right Shift", 0xe6: "Right Alt", 0xe7: "Right Win", 0xa8: "Mute", 0xa9: "Volume Up",
         0xaa: "Volume Down", 0xb2: "Calculator"}
for c in range(0x04, 0x1e):
    names[c] = chr(ord("A") + c - 4)
for c in range(0x1e, 0x27):
    names[c] = str(c - 0x1d)
names[0x27] = "0"
for c in range(0x3a, 0x46):
    names[c] = f"F{c - 0x39}"
for c in range(0x59, 0x62):
    names[c] = f"Num {c - 0x58}"

W, H = 23, 8
led_names, positions, seen = [], [], Counter()
bar = edge = 0
for l in leds:
    if l["i"] >= 120:
        edge += 1
        n = f"Edge light {edge}"
    elif l["i"] >= 115:
        bar += 1
        n = f"Light bar {bar}"
    else:
        n = names.get(l["keycode"], f"Key {l['keycode']:02X}")
        seen[n] += 1
        if seen[n] > 1:
            n = f"{n} {seen[n]}"
    led_names.append(n)
    # QMK LED points: x 0-226, y 12-84 → canvas W x H
    positions.append([min(W - 1, round(l["x"] / 226 * (W - 1))), min(H - 1, round((l["y"] - 12) / 72 * (H - 1)))])

js = f'''// SignalRGB plugin for the Chilkey ND104 – made by ND104 Studio
// (https://github.com/MrUrdam/chilkey_nd104_utilities). MIT licence.
//
// The ND104 firmware (V3.0.14) understands SignalRGB's QMK protocol on its
// lighting channel: USB interface 4, usage page FF59, report ID 05
// (docs/protocol.md § 5.4). PC lighting must be allowed on the keyboard
// (ND104 Studio → Lighting → PC lighting software, or Fn + Shift + L).
// Colours are not stored in the keyboard; when SignalRGB stops, the
// keyboard's own lighting returns.

export function Name() {{ return "Chilkey ND104"; }}
export function Publisher() {{ return "ND104 Studio"; }}
export function VendorId() {{ return 0x6D67; }}
export function ProductId() {{ return 0x016C; }}
export function Size() {{ return [{W}, {H}]; }}
export function DefaultPosition() {{ return [10, 100]; }}
export function DefaultScale() {{ return 8.0; }}
export function ControllableParameters() {{
\treturn [
\t\t{{"property": "LightingMode", "label": "Lighting Mode", "type": "combobox", "values": ["Canvas", "Forced"], "default": "Canvas"}},
\t\t{{"property": "forcedColor", "label": "Forced Color", "min": "0", "max": "360", "type": "color", "default": "#009bde"}},
\t];
}}

// SignalRGB QMK protocol, sent with report ID 05
const REPORT_ID = 0x05;
const REPORT_LENGTH = 64; // report ID + 63 bytes
const STREAM_LEDS = 0x24;
const EFFECT_ENABLE = 0x25;
const EFFECT_DISABLE = 0x26;
const LEDS_PER_PACKET = 9;

const vLedNames = {json.dumps(led_names)};
const vLedPositions = {json.dumps(positions)};

export function LedNames() {{ return vLedNames; }}
export function LedPositions() {{ return vLedPositions; }}

export function Validate(endpoint) {{
\treturn endpoint.interface === 4 && endpoint.usage_page === 0xFF59;
}}

export function Initialize() {{
\tdevice.write([REPORT_ID, EFFECT_ENABLE], REPORT_LENGTH);
}}

export function Render() {{
\tsendColors(false);
}}

export function Shutdown(SystemSuspend) {{
\t// give the lighting back to the keyboard (its stored effect returns)
\tdevice.write([REPORT_ID, EFFECT_DISABLE], REPORT_LENGTH);
}}

function sendColors(shutdown) {{
\tconst colors = [];
\tfor (let i = 0; i < vLedPositions.length; i++) {{
\t\tconst [x, y] = vLedPositions[i];
\t\tcolors.push(LightingMode === "Forced" ? hexToRgb(forcedColor) : device.color(x, y));
\t}}
\tfor (let start = 0; start < colors.length; start += LEDS_PER_PACKET) {{
\t\tconst count = Math.min(LEDS_PER_PACKET, colors.length - start);
\t\tconst packet = [REPORT_ID, STREAM_LEDS, start, count];
\t\tfor (let i = 0; i < count; i++) {{
\t\t\tpacket.push(...colors[start + i]);
\t\t}}
\t\tdevice.write(packet, REPORT_LENGTH);
\t}}
}}

function hexToRgb(hex) {{
\tconst v = parseInt(hex.replace("#", ""), 16);
\treturn [(v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF];
}}
'''
open(out_path, "w", encoding="utf-8", newline="\n").write(js)
print("ok", len(led_names), "LEDs", len(set(map(tuple, positions))), "distinct positions")
