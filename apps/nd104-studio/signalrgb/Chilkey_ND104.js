// SignalRGB plugin for the Chilkey ND104 – made by ND104 Studio
// (https://github.com/MrUrdam/chilkey_nd104_utilities). MIT licence.
//
// The ND104 firmware (V3.0.14) understands SignalRGB's QMK protocol on its
// lighting channel: USB interface 4, usage page FF59, report ID 05
// (docs/protocol.md § 5.4). PC lighting must be allowed on the keyboard
// (ND104 Studio → Lighting → PC lighting software, or Fn + Shift + L).
// Colours are not stored in the keyboard; when SignalRGB stops, the
// keyboard's own lighting returns.

export function Name() { return "Chilkey ND104"; }
export function Publisher() { return "ND104 Studio"; }
export function VendorId() { return 0x6D67; }
export function ProductId() { return 0x016C; }
export function Size() { return [23, 8]; }
export function DefaultPosition() { return [10, 100]; }
export function DefaultScale() { return 8.0; }
export function ControllableParameters() {
	return [
		{"property": "LightingMode", "label": "Lighting Mode", "type": "combobox", "values": ["Canvas", "Forced"], "default": "Canvas"},
		{"property": "forcedColor", "label": "Forced Color", "min": "0", "max": "360", "type": "color", "default": "#009bde"},
	];
}

// SignalRGB QMK protocol, sent with report ID 05
const REPORT_ID = 0x05;
const REPORT_LENGTH = 64; // report ID + 63 bytes
const STREAM_LEDS = 0x24;
const EFFECT_ENABLE = 0x25;
const EFFECT_DISABLE = 0x26;
const LEDS_PER_PACKET = 9;

const vLedNames = ["Esc", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "`", "Print Screen", "Scroll Lock", "Pause", "Calculator", "Volume Down", "Volume Up", "Mute", "` 2", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "Backspace", "Backspace 2", "Insert", "Home", "Page Up", "Num Lock", "Num /", "Num *", "Num -", "Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "\\", "Delete", "End", "Page Down", "Num 7", "Num 8", "Num 9", "Num +", "Caps Lock", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "Enter", "Num 4", "Num 5", "Num 6", "Left Shift", "ISO \\", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "Right Shift", "Right Shift 2", "Up", "Num 1", "Num 2", "Num Enter", "Num 3", "Left Ctrl", "Left Win", "Left Alt", "Space", "Space 2", "Space 3", "Space 4", "Right Alt", "Menu", "4 2", "Right Ctrl", "Left", "Down", "Right", "Num 0", "Num .", "Light bar 1", "Light bar 2", "Light bar 3", "Light bar 4", "Light bar 5", "Edge light 1", "Edge light 2", "Edge light 3", "Edge light 4", "Edge light 5", "Edge light 6", "Edge light 7", "Edge light 8", "Edge light 9", "Edge light 10"];
const vLedPositions = [[0, 0], [1, 0], [2, 0], [3, 0], [4, 0], [6, 0], [7, 0], [8, 0], [9, 0], [10, 0], [11, 0], [12, 0], [13, 0], [14, 0], [16, 0], [17, 0], [18, 0], [19, 0], [20, 0], [21, 0], [22, 0], [0, 1], [1, 1], [2, 1], [3, 1], [4, 1], [5, 1], [6, 1], [7, 1], [8, 1], [9, 1], [10, 1], [11, 1], [12, 1], [13, 1], [14, 1], [16, 1], [17, 1], [18, 1], [19, 1], [20, 1], [21, 1], [22, 1], [0, 3], [1, 3], [3, 3], [4, 3], [5, 3], [6, 3], [7, 3], [8, 3], [9, 3], [10, 3], [11, 3], [12, 3], [13, 3], [14, 3], [16, 3], [17, 3], [18, 3], [19, 3], [20, 3], [21, 3], [22, 3], [1, 4], [2, 4], [3, 4], [4, 4], [5, 4], [6, 4], [7, 4], [8, 4], [9, 4], [10, 4], [11, 4], [12, 4], [14, 4], [19, 4], [20, 4], [21, 4], [0, 5], [1, 5], [3, 5], [4, 5], [5, 5], [6, 5], [7, 5], [8, 5], [9, 5], [10, 5], [11, 5], [12, 5], [13, 5], [14, 5], [17, 5], [19, 5], [20, 5], [21, 5], [22, 5], [0, 6], [1, 6], [3, 6], [5, 6], [7, 6], [8, 6], [8, 6], [11, 6], [12, 6], [13, 6], [14, 6], [16, 6], [17, 6], [18, 6], [19, 6], [20, 6], [16, 3], [16, 3], [17, 3], [17, 3], [17, 3], [1, 1], [1, 2], [1, 2], [1, 3], [1, 4], [1, 4], [1, 5], [1, 6], [1, 6], [1, 7]];

export function LedNames() { return vLedNames; }
export function LedPositions() { return vLedPositions; }

export function Validate(endpoint) {
	return endpoint.interface === 4 && endpoint.usage_page === 0xFF59;
}

export function Initialize() {
	device.write([REPORT_ID, EFFECT_ENABLE], REPORT_LENGTH);
}

export function Render() {
	sendColors(false);
}

export function Shutdown(SystemSuspend) {
	// give the lighting back to the keyboard (its stored effect returns)
	device.write([REPORT_ID, EFFECT_DISABLE], REPORT_LENGTH);
}

function sendColors(shutdown) {
	const colors = [];
	for (let i = 0; i < vLedPositions.length; i++) {
		const [x, y] = vLedPositions[i];
		colors.push(LightingMode === "Forced" ? hexToRgb(forcedColor) : device.color(x, y));
	}
	for (let start = 0; start < colors.length; start += LEDS_PER_PACKET) {
		const count = Math.min(LEDS_PER_PACKET, colors.length - start);
		const packet = [REPORT_ID, STREAM_LEDS, start, count];
		for (let i = 0; i < count; i++) {
			packet.push(...colors[start + i]);
		}
		device.write(packet, REPORT_LENGTH);
	}
}

function hexToRgb(hex) {
	const v = parseInt(hex.replace("#", ""), 16);
	return [(v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF];
}
