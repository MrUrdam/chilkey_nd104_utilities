// Which key codes (QMK numbering, see keycodes.ts) a browser key event can
// stand for. `KeyboardEvent.code` names the key the keyboard *sent*, so a
// remapped key shows up under its new function – the keymap page looks the
// code up in the keyboard's keymap to find the physical key.

const TABLE: Record<string, number[]> = {
  Enter: [0x28],
  Escape: [0x29],
  Backspace: [0x2a],
  Tab: [0x2b],
  Space: [0x2c],
  Minus: [0x2d],
  Equal: [0x2e],
  BracketLeft: [0x2f],
  BracketRight: [0x30],
  // the ISO # key (0x32) arrives as "Backslash" too
  Backslash: [0x31, 0x32],
  Semicolon: [0x33],
  Quote: [0x34],
  Backquote: [0x35],
  Comma: [0x36],
  Period: [0x37],
  Slash: [0x38],
  CapsLock: [0x39],
  PrintScreen: [0x46],
  ScrollLock: [0x47],
  Pause: [0x48],
  Insert: [0x49],
  Home: [0x4a],
  PageUp: [0x4b],
  Delete: [0x4c],
  End: [0x4d],
  PageDown: [0x4e],
  ArrowRight: [0x4f],
  ArrowLeft: [0x50],
  ArrowDown: [0x51],
  ArrowUp: [0x52],
  NumLock: [0x53],
  NumpadDivide: [0x54],
  NumpadMultiply: [0x55],
  NumpadSubtract: [0x56],
  NumpadAdd: [0x57],
  NumpadEnter: [0x58],
  Numpad0: [0x62],
  NumpadDecimal: [0x63],
  IntlBackslash: [0x64],
  ContextMenu: [0x65],
  ControlLeft: [0xe0],
  ShiftLeft: [0xe1],
  AltLeft: [0xe2],
  MetaLeft: [0xe3],
  ControlRight: [0xe4],
  ShiftRight: [0xe5],
  AltRight: [0xe6],
  MetaRight: [0xe7],
  AudioVolumeMute: [0xa8],
  AudioVolumeUp: [0xa9],
  AudioVolumeDown: [0xaa],
  MediaTrackNext: [0xab],
  MediaTrackPrevious: [0xac],
  MediaStop: [0xad],
  MediaPlayPause: [0xae],
  MediaSelect: [0xaf],
  Eject: [0xb0],
  LaunchMail: [0xb1],
  LaunchApp2: [0xb2],
  LaunchApp1: [0xb3],
  BrowserSearch: [0xb4],
  BrowserHome: [0xb5],
  BrowserBack: [0xb6],
  BrowserForward: [0xb7],
  BrowserRefresh: [0xb9],
};

for (let i = 0; i < 26; i++) TABLE[`Key${String.fromCharCode(65 + i)}`] = [0x04 + i];
for (let i = 1; i <= 9; i++) TABLE[`Digit${i}`] = [0x1e + i - 1];
TABLE.Digit0 = [0x27];
for (let i = 1; i <= 9; i++) TABLE[`Numpad${i}`] = [0x59 + i - 1];
for (let i = 1; i <= 12; i++) TABLE[`F${i}`] = [0x3a + i - 1];
for (let i = 13; i <= 24; i++) TABLE[`F${i}`] = [0x68 + i - 13];

/** Key codes a `KeyboardEvent.code` can come from (empty: unknown). */
export function codesFor(eventCode: string): number[] {
  return TABLE[eventCode] ?? [];
}
