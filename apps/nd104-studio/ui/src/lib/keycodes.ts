// Key codes the keyboard understands (QMK numbering as used by Chilkey's
// web driver, VIA protocol 12) with names for the keymap page. Codes not
// listed here are shown as hex and stay untouched unless the user changes
// the key.

export type Category = "basic" | "mods" | "nav" | "numpad" | "media" | "mouse" | "light" | "layer" | "special";

export const CATEGORIES: Category[] = ["basic", "mods", "nav", "numpad", "media", "mouse", "light", "layer", "special"];

export interface KeyDef {
  code: number;
  /** English name; `de` when German differs. */
  en: string;
  de?: string;
  /** Short text for the key drawing; defaults to the name. */
  short?: string;
  /** German short text when it differs. */
  deShort?: string;
  category: Category;
}

export const KC_NO = 0x0000;
export const KC_TRNS = 0x0001;

const d = (code: number, en: string, category: Category, extra: Partial<KeyDef> = {}): KeyDef => ({ code, en, category, ...extra });

const LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((c, i) => d(0x04 + i, c, "basic"));
const DIGITS = "1234567890".split("").map((c, i) => d(0x1e + i, c, "basic"));
const FKEYS = Array.from({ length: 12 }, (_, i) => d(0x3a + i, `F${i + 1}`, "nav"));
const FKEYS_HIGH = Array.from({ length: 12 }, (_, i) => d(0x68 + i, `F${i + 13}`, "nav"));

const LAYER_COUNT = 6;
function layerKeys(): KeyDef[] {
  const out: KeyDef[] = [];
  for (let n = 0; n < LAYER_COUNT; n++) {
    out.push(d(0x5220 + n, `Hold: layer ${n}`, "layer", { de: `Halten: Ebene ${n}`, short: `MO${n}` }));
    out.push(d(0x5260 + n, `Toggle layer ${n}`, "layer", { de: `Ebene ${n} umschalten`, short: `TG${n}` }));
    out.push(d(0x5200 + n, `Switch to layer ${n}`, "layer", { de: `Zu Ebene ${n} wechseln`, short: `TO${n}` }));
    out.push(d(0x5280 + n, `Next key on layer ${n}`, "layer", { de: `Nächste Taste auf Ebene ${n}`, short: `OSL${n}` }));
    out.push(d(0x5240 + n, `Base layer ${n}`, "layer", { de: `Grundebene ${n}`, short: `DF${n}` }));
  }
  return out;
}

export const KEYS: KeyDef[] = [
  d(KC_NO, "Nothing", "special", { de: "Nichts", short: "" }),
  d(KC_TRNS, "Same as layer below", "special", { de: "Wie Ebene darunter", short: "▽" }),
  ...LETTERS,
  ...DIGITS,
  d(0x28, "Enter", "basic"),
  d(0x29, "Esc", "basic"),
  d(0x2a, "Backspace", "basic", { de: "Rücktaste", short: "Bksp" }),
  d(0x2b, "Tab", "basic"),
  d(0x2c, "Space", "basic", { de: "Leertaste" }),
  d(0x2d, "-", "basic"),
  d(0x2e, "=", "basic"),
  d(0x2f, "[", "basic"),
  d(0x30, "]", "basic"),
  d(0x31, "\\", "basic"),
  d(0x32, "# (ISO)", "basic", { short: "#" }),
  d(0x33, ";", "basic"),
  d(0x34, "'", "basic"),
  d(0x35, "`", "basic"),
  d(0x36, ",", "basic"),
  d(0x37, ".", "basic"),
  d(0x38, "/", "basic"),
  d(0x64, "\\ (ISO, next to Shift)", "basic", { de: "< (ISO, neben Shift)", short: "ISO\\" }),
  d(0x39, "Caps Lock", "basic", { de: "Feststelltaste", short: "Caps", deShort: "Fest" }),

  d(0xe0, "Left Ctrl", "mods", { de: "Strg links", short: "Ctrl", deShort: "Strg" }),
  d(0xe1, "Left Shift", "mods", { de: "Shift links", short: "Shift" }),
  d(0xe2, "Left Alt", "mods", { de: "Alt links", short: "Alt" }),
  d(0xe3, "Left Win", "mods", { de: "Win links", short: "Win" }),
  d(0xe4, "Right Ctrl", "mods", { de: "Strg rechts", short: "RCtrl", deShort: "RStrg" }),
  d(0xe5, "Right Shift", "mods", { de: "Shift rechts", short: "RShift" }),
  d(0xe6, "Right Alt", "mods", { de: "AltGr", short: "RAlt", deShort: "AltGr" }),
  d(0xe7, "Right Win", "mods", { de: "Win rechts", short: "RWin" }),
  d(0x65, "Menu", "mods", { de: "Menü" }),

  ...FKEYS,
  ...FKEYS_HIGH,
  d(0x46, "Print Screen", "nav", { de: "Druck", short: "PrtSc", deShort: "Druck" }),
  d(0x47, "Scroll Lock", "nav", { de: "Rollen", short: "ScrLk" }),
  d(0x48, "Pause", "nav"),
  d(0x49, "Insert", "nav", { de: "Einfg", short: "Ins", deShort: "Einfg" }),
  d(0x4a, "Home", "nav", { de: "Pos1" }),
  d(0x4b, "Page Up", "nav", { de: "Bild ↑", short: "PgUp" }),
  d(0x4c, "Delete", "nav", { de: "Entf", short: "Del", deShort: "Entf" }),
  d(0x4d, "End", "nav", { de: "Ende" }),
  d(0x4e, "Page Down", "nav", { de: "Bild ↓", short: "PgDn" }),
  d(0x4f, "Right", "nav", { de: "Rechts", short: "→" }),
  d(0x50, "Left", "nav", { de: "Links", short: "←" }),
  d(0x51, "Down", "nav", { de: "Runter", short: "↓" }),
  d(0x52, "Up", "nav", { de: "Hoch", short: "↑" }),

  d(0x53, "Num Lock", "numpad", { short: "Num" }),
  d(0x54, "Numpad /", "numpad", { de: "Ziffernblock /", short: "/" }),
  d(0x55, "Numpad *", "numpad", { de: "Ziffernblock *", short: "*" }),
  d(0x56, "Numpad -", "numpad", { de: "Ziffernblock -", short: "-" }),
  d(0x57, "Numpad +", "numpad", { de: "Ziffernblock +", short: "+" }),
  d(0x58, "Numpad Enter", "numpad", { de: "Ziffernblock Enter", short: "Ent" }),
  ...Array.from({ length: 9 }, (_, i) => d(0x59 + i, `Numpad ${i + 1}`, "numpad", { de: `Ziffernblock ${i + 1}`, short: `${i + 1}` })),
  d(0x62, "Numpad 0", "numpad", { de: "Ziffernblock 0", short: "0" }),
  d(0x63, "Numpad .", "numpad", { de: "Ziffernblock ,", short: "." }),

  d(0xa8, "Mute", "media", { de: "Stumm", short: "Mute" }),
  d(0xa9, "Volume up", "media", { de: "Lauter", short: "Vol+" }),
  d(0xaa, "Volume down", "media", { de: "Leiser", short: "Vol−" }),
  d(0xae, "Play / Pause", "media", { de: "Wiedergabe / Pause", short: "⏯" }),
  d(0xab, "Next track", "media", { de: "Nächster Titel", short: "⏭" }),
  d(0xac, "Previous track", "media", { de: "Vorheriger Titel", short: "⏮" }),
  d(0xad, "Stop", "media", { short: "⏹" }),
  d(0xbb, "Fast forward", "media", { de: "Vorspulen", short: "⏩" }),
  d(0xbc, "Rewind", "media", { de: "Zurückspulen", short: "⏪" }),
  d(0xaf, "Media player", "media", { de: "Medienplayer", short: "Media" }),
  d(0xb0, "Eject", "media", { de: "Auswerfen", short: "⏏" }),
  d(0xb1, "Mail", "media", { de: "E-Mail", short: "Mail" }),
  d(0xb2, "Calculator", "media", { de: "Rechner", short: "Calc" }),
  d(0xb3, "My Computer", "media", { de: "Arbeitsplatz", short: "PC" }),
  d(0xb4, "Browser search", "media", { de: "Browser: Suche", short: "Search" }),
  d(0xb5, "Browser home", "media", { de: "Browser: Startseite", short: "WWW" }),
  d(0xb6, "Browser back", "media", { de: "Browser: Zurück", short: "Back" }),
  d(0xb7, "Browser forward", "media", { de: "Browser: Vor", short: "Fwd" }),
  d(0xb9, "Browser refresh", "media", { de: "Browser: Neu laden", short: "Rfsh" }),
  d(0xbd, "Screen brightness +", "media", { de: "Bildschirm heller", short: "Scr+" }),
  d(0xbe, "Screen brightness −", "media", { de: "Bildschirm dunkler", short: "Scr−" }),

  d(0xcd, "Mouse up", "mouse", { de: "Maus hoch", short: "M↑" }),
  d(0xce, "Mouse down", "mouse", { de: "Maus runter", short: "M↓" }),
  d(0xcf, "Mouse left", "mouse", { de: "Maus links", short: "M←" }),
  d(0xd0, "Mouse right", "mouse", { de: "Maus rechts", short: "M→" }),
  d(0xd1, "Left click", "mouse", { de: "Linksklick", short: "LMB" }),
  d(0xd2, "Right click", "mouse", { de: "Rechtsklick", short: "RMB" }),
  d(0xd3, "Middle click", "mouse", { de: "Mittelklick", short: "MMB" }),
  d(0xd4, "Mouse button 4", "mouse", { de: "Maustaste 4", short: "MB4" }),
  d(0xd5, "Mouse button 5", "mouse", { de: "Maustaste 5", short: "MB5" }),
  d(0xd9, "Wheel up", "mouse", { de: "Mausrad hoch", short: "W↑" }),
  d(0xda, "Wheel down", "mouse", { de: "Mausrad runter", short: "W↓" }),
  d(0xdb, "Wheel left", "mouse", { de: "Mausrad links", short: "W←" }),
  d(0xdc, "Wheel right", "mouse", { de: "Mausrad rechts", short: "W→" }),

  d(0x7820, "Backlight on/off", "light", { de: "Hintergrundbeleuchtung an/aus", short: "RGB" }),
  d(0x7821, "Backlight: next effect", "light", { de: "Hintergrundbeleuchtung: nächster Effekt", short: "Eff+" }),
  d(0x7822, "Backlight: previous effect", "light", { de: "Hintergrundbeleuchtung: vorheriger Effekt", short: "Eff−" }),
  d(0x7823, "Backlight hue +", "light", { de: "Hintergrundbeleuchtung: Farbton +", short: "Hue+" }),
  d(0x7824, "Backlight hue −", "light", { de: "Hintergrundbeleuchtung: Farbton −", short: "Hue−" }),
  d(0x7825, "Backlight saturation +", "light", { de: "Hintergrundbeleuchtung: Sättigung +", short: "Sat+" }),
  d(0x7826, "Backlight saturation −", "light", { de: "Hintergrundbeleuchtung: Sättigung −", short: "Sat−" }),
  d(0x7827, "Backlight brightness +", "light", { de: "Hintergrundbeleuchtung heller", short: "Bri+" }),
  d(0x7828, "Backlight brightness −", "light", { de: "Hintergrundbeleuchtung dunkler", short: "Bri−" }),
  d(0x7829, "Backlight speed +", "light", { de: "Hintergrundbeleuchtung schneller", short: "Spd+" }),
  d(0x782a, "Backlight speed −", "light", { de: "Hintergrundbeleuchtung langsamer", short: "Spd−" }),
  // Chilkey functions (checked on the keyboard; the test ends with any other key)
  d(0x7e06, "Lighting test (hold 3 s)", "light", { de: "Lichttest (3 s halten)", short: "LTest" }),
  d(0x7e15, "Battery brightness limit (50 % / 100 %)", "light", {
    de: "Helligkeit im Akkubetrieb (50 % / 100 %)",
    short: "BatBri",
  }),

  ...layerKeys(),

  d(0x7e0f, "Display page up", "special", { de: "Display: Seite hoch", short: "LCD↑" }),
  d(0x7e10, "Display page down", "special", { de: "Display: Seite runter", short: "LCD↓" }),
  d(0x7e0e, "Display on/off", "special", { de: "Display an/aus", short: "LCD" }),
  d(0x7e16, "Display back", "special", { de: "Display: Zurück", short: "LCD⟲" }),
  d(0x7e17, "Display confirm", "special", { de: "Display: Bestätigen", short: "LCD✓" }),
  d(0x7e21, "Display calculator", "special", { de: "Display-Rechner", short: "LCalc" }),
  d(0x7e18, "Show typed keys on the display", "special", { de: "Getippte Zeichen im Display", short: "LcdKey" }),
  d(0x7e22, "Restart display", "special", { de: "Display neu starten", short: "LcdRst" }),
  d(0x7e35, "Show desktop", "special", { de: "Desktop anzeigen", short: "Desk" }),
  // connection and keyboard functions of Chilkey's firmware (names checked on the keyboard)
  d(0x7e08, "USB mode", "special", { de: "USB-Modus", short: "USB" }),
  d(0x7e09, "Bluetooth device 1", "special", { de: "Bluetooth-Gerät 1", short: "BT1" }),
  d(0x7e0a, "Bluetooth device 2", "special", { de: "Bluetooth-Gerät 2", short: "BT2" }),
  d(0x7e0b, "Bluetooth device 3", "special", { de: "Bluetooth-Gerät 3", short: "BT3" }),
  d(0x7e0d, "2.4 GHz wireless", "special", { de: "2,4-GHz-Funk", short: "2.4G" }),
  d(0x7e0c, "Lock Win key", "special", { de: "Win-Taste sperren", short: "WinLk" }),
  d(0x7e07, "Keyboard sleep", "special", { de: "Tastatur-Ruhezustand", short: "Sleep" }),
  d(0x7013, "N-key rollover on/off", "special", { de: "N-Key-Rollover an/aus", short: "NKRO" }),
  d(0x7e14, "Charging indicator on/off", "special", { de: "Ladeanzeige an/aus", short: "ChgLED" }),
  // QMK's own Win lock; the ND104 uses 0x7E0C instead
  d(0x700b, "Lock Win key (alternative)", "special", { de: "Win-Taste sperren (Alternative)", short: "WinLk2" }),
  d(0x7e04, "Windows mode", "special", { de: "Windows-Modus", short: "Win" }),
  d(0x7e05, "Mac mode", "special", { de: "Mac-Modus", short: "Mac" }),
  d(0x7e00, "Mission Control", "special", { short: "MissC" }),
  d(0x7e01, "Launchpad", "special", { short: "LPad" }),
  d(0x7c16, "Esc / ` (grave escape)", "special", { de: "Esc / ^ (Grave-Escape)", short: "GEsc" }),
];

const BY_CODE = new Map(KEYS.map((k) => [k.code, k]));

/** Codes found on the keyboard that are not offered (no known function). */
const EXTRA: Record<number, { en: string; de: string; short: string }> = {
  0x7e23: { en: "Undocumented function", de: "Undokumentierte Funktion", short: "Undoc." },
};

const MOD_NAMES = ["Ctrl", "Shift", "Alt", "Win"];

/** Modifier bits (QMK: 1 Ctrl, 2 Shift, 4 Alt, 8 Gui, 0x10 right side). */
function modText(bits: number): string {
  const right = bits & 0x10 ? "R" : "";
  return MOD_NAMES.filter((_, i) => bits & (1 << i))
    .map((m) => right + m)
    .join("+");
}

export function hex(code: number): string {
  return "0x" + code.toString(16).toUpperCase().padStart(4, "0");
}

export interface Described {
  name: string;
  short: string;
  known: boolean;
  category: Category | null;
  /** What the function does, for functions that need more than a name. */
  hint?: string;
}

/** Short explanations (English, German) where the name alone is not enough. */
const HINTS: Record<number, [string, string]> = {
  [KC_NO]: ["The key does nothing.", "Die Taste tut nichts."],
  [KC_TRNS]: ["Does what this key does on the layer below.", "Tut, was die Taste auf der Ebene darunter tut."],
  0x7c16: ["Esc; with Shift or Win: ^ (German) or `.", "Esc; mit Shift oder Win: ^ (bzw. ` auf Englisch)."],
  0x7e09: ["Short press: switch to device 1. Hold: pair a new device.", "Kurz: zu Gerät 1 wechseln. Lang halten: neues Gerät koppeln."],
  0x7e0a: ["Short press: switch to device 2. Hold: pair a new device.", "Kurz: zu Gerät 2 wechseln. Lang halten: neues Gerät koppeln."],
  0x7e0b: ["Short press: switch to device 3. Hold: pair a new device.", "Kurz: zu Gerät 3 wechseln. Lang halten: neues Gerät koppeln."],
  0x7e0d: [
    "Short press: switch to the 2.4 GHz receiver. Hold: pair the receiver.",
    "Kurz: zum 2,4-GHz-Empfänger wechseln. Lang halten: Empfänger koppeln.",
  ],
  0x7e0c: ["E.g. so it cannot open the Start menu in games.", "Z. B. damit sie im Spiel nicht das Startmenü öffnet."],
  0x700b: ["Not used by the ND104 – use \"Lock Win key\".", "Von der ND104 nicht genutzt – nimm „Win-Taste sperren“."],
  0x7013: ["On: any number of keys at once. Off: works with every PC.", "An: beliebig viele Tasten gleichzeitig. Aus: läuft an jedem PC."],
  0x7e06: ["Hold 3 s; any other key ends the test.", "3 s halten; jede andere Taste beendet den Test."],
  0x7e15: ["On battery: backlight at most 50 % or 100 %.", "Im Akkubetrieb: Hintergrundbeleuchtung höchstens 50 % oder 100 %."],
  0x7e22: ["Hold long: update mode (ends after 3 min).", "Lange halten: Update-Modus (endet nach 3 Min.)."],
  0x7e17: ["Calculator on the display on/off.", "Rechner im Display an/aus."],
  0x7e35: ["Minimises all windows, or brings them back.", "Minimiert alle Fenster oder holt sie zurück."],
  0x7e04: ["Hold 3 s.", "3 s halten."],
  0x7e05: ["Hold 3 s.", "3 s halten."],
  0x7e00: ["macOS only.", "Nur macOS."],
  0x7e01: ["macOS only.", "Nur macOS."],
};

/** Explanations of the layer functions (n = layer). */
/** Name of layer `n` as the keymap page shows it (set by the page). */
let layerLabel = (n: number, lang: string) => (lang === "de" ? `Ebene ${n}` : `Layer ${n}`);

/** Lets the layer functions use the page's layer names ("Zu Eigene 1 wechseln"). */
export function setLayerLabel(label: (n: number, lang: string) => string) {
  layerLabel = label;
}

type LayerKind = "hold" | "toggle" | "to" | "oneshot" | "base";

function layerKind(code: number): LayerKind | null {
  if (code >= 0x5220 && code <= 0x523f) return "hold";
  if (code >= 0x5260 && code <= 0x527f) return "toggle";
  if (code >= 0x5200 && code <= 0x521f) return "to";
  if (code >= 0x5280 && code <= 0x529f) return "oneshot";
  if (code >= 0x5240 && code <= 0x525f) return "base";
  return null;
}

/** Name and explanation of a layer function, with the layer's name. */
function layerFunction(code: number, lang: string): { name: string; hint: string } | null {
  const kind = layerKind(code);
  if (!kind) return null;
  const n = layerLabel(code & 0x1f, lang);
  const de = lang === "de";
  switch (kind) {
    case "hold":
      return de
        ? { name: `Halten: ${n}`, hint: "Aktiv, solange die Taste gehalten wird." }
        : { name: `Hold: ${n}`, hint: "Active while the key is held." };
    case "toggle":
      return de
        ? { name: `${n} umschalten`, hint: "An oder aus; bleibt bis zum nächsten Druck." }
        : { name: `Toggle ${n}`, hint: "On or off; stays until pressed again." };
    case "to":
      return de
        ? { name: `Zu ${n} wechseln`, hint: "Schaltet die anderen Ebenen aus." }
        : { name: `Switch to ${n}`, hint: "Turns the other layers off." };
    case "oneshot":
      return de
        ? { name: `Nächste Taste auf ${n}`, hint: "Nur der nächste Tastendruck nutzt diese Ebene." }
        : { name: `Next key on ${n}`, hint: "Only the next key press uses this layer." };
    case "base":
      return de
        ? { name: `Grundebene: ${n}`, hint: "Macht diese Ebene zur Grundebene." }
        : { name: `Base layer: ${n}`, hint: "Makes this layer the base layer." };
  }
}

function hintFor(code: number, lang: string): string | undefined {
  const h = HINTS[code];
  return h ? (lang === "de" ? h[1] : h[0]) : layerFunction(code, lang)?.hint;
}

/**
 * Name of a key code. `basicName` gives the name of a basic key in the
 * current label preset (so KC_Y can read "Z" on a German layout).
 */
/**
 * The ND104 firmware gives some codes another meaning on layer 2 (Fn + right
 * Shift): the lighting keys control the bottom lighting, the display page keys
 * the display brightness, the display restart becomes a factory reset of the
 * display (checked on the keyboard).
 */
export const UNDERGLOW_LAYER = 2;

const ON_LAYER_2: Record<number, { en: string; de: string; short?: string; hint: [string, string] }> = {
  0x7820: { en: "Bottom lighting on/off", de: "Untere Beleuchtung an/aus", hint: ["", ""] },
  0x7821: { en: "Bottom lighting: next effect", de: "Untere Beleuchtung: nächster Effekt", hint: ["", ""] },
  0x7822: { en: "Bottom lighting: previous effect", de: "Untere Beleuchtung: vorheriger Effekt", hint: ["", ""] },
  0x7823: { en: "Bottom lighting hue +", de: "Untere Beleuchtung: Farbton +", hint: ["", ""] },
  0x7824: { en: "Bottom lighting hue −", de: "Untere Beleuchtung: Farbton −", hint: ["", ""] },
  0x7825: { en: "Bottom lighting saturation +", de: "Untere Beleuchtung: Sättigung +", hint: ["", ""] },
  0x7826: { en: "Bottom lighting saturation −", de: "Untere Beleuchtung: Sättigung −", hint: ["", ""] },
  0x7827: { en: "Bottom lighting brightness +", de: "Untere Beleuchtung heller", hint: ["", ""] },
  0x7828: { en: "Bottom lighting brightness −", de: "Untere Beleuchtung dunkler", hint: ["", ""] },
  0x7829: { en: "Bottom lighting speed +", de: "Untere Beleuchtung schneller", hint: ["", ""] },
  0x782a: { en: "Bottom lighting speed −", de: "Untere Beleuchtung langsamer", hint: ["", ""] },
  0x7e0f: { en: "Display brightness +", de: "Display heller", short: "LCD+", hint: ["", ""] },
  0x7e10: { en: "Display brightness −", de: "Display dunkler", short: "LCD−", hint: ["", ""] },
  0x7e22: {
    en: "Display factory reset",
    de: "Display auf Werkszustand",
    short: "LcdRes",
    hint: ["", ""],
  },
};

const LAYER_NOTE: [string, string] = [
  "With Fn + Shift: bottom lighting.",
  "Mit Fn + Shift: untere Beleuchtung.",
];

export function describe(
  code: number,
  lang: string,
  basicName: (code: number) => string | null = () => null,
  layer = 0,
): Described {
  const de = lang === "de";
  const special = layer === UNDERGLOW_LAYER ? ON_LAYER_2[code] : undefined;
  if (special) {
    const base = describe(code, lang, basicName, 0);
    const hint = special.hint[0] ? (de ? special.hint[1] : special.hint[0]) : undefined;
    return { ...base, name: de ? special.de : special.en, short: special.short ?? base.short, hint };
  }
  const def = BY_CODE.get(code);
  if (def) {
    const fromPreset = def.category === "basic" ? basicName(code) : null;
    const name = fromPreset ?? layerFunction(code, lang)?.name ?? (lang === "de" && def.de ? def.de : def.en);
    const short = (lang === "de" ? def.deShort : undefined) ?? def.short ?? def.en;
    let hint = hintFor(code, lang);
    // the codes that mean something else on layer 2 say so
    if (ON_LAYER_2[code] && code >= 0x7820 && code <= 0x782a) {
      hint = [hint, de ? LAYER_NOTE[1] : LAYER_NOTE[0]].filter(Boolean).join(" ");
    } else if (ON_LAYER_2[code]) {
      const other = de ? ON_LAYER_2[code].de : ON_LAYER_2[code].en;
      hint = [hint, de ? `Mit Fn + Shift: ${other}.` : `With Fn + Shift: ${other}.`].filter(Boolean).join(" ");
    }
    return { name, short: fromPreset ?? short, known: true, category: def.category, hint };
  }
  // Modifier + key (e.g. Ctrl+C): 0x0100–0x1FFF
  if (code >= 0x0100 && code <= 0x1fff) {
    const inner = describe(code & 0xff, lang, basicName);
    const mods = modText(code >> 8);
    return { name: `${mods}+${inner.name}`, short: `${mods}+${inner.short}`, known: inner.known, category: "mods" };
  }
  // Mod-tap: modifier when held, key when tapped
  if (code >= 0x2000 && code <= 0x3fff) {
    const inner = describe(code & 0xff, lang, basicName);
    const mods = modText((code >> 8) & 0x1f);
    const name = lang === "de" ? `${inner.name} / gehalten ${mods}` : `${inner.name} / hold ${mods}`;
    const hint =
      lang === "de" ? `Tippen: ${inner.name}. Halten: ${mods}.` : `Tap: ${inner.name}. Hold: ${mods}.`;
    return { name, short: `${inner.short}/${mods}`, known: false, category: "mods", hint };
  }
  // Layer-tap: layer when held, key when tapped
  if (code >= 0x4000 && code <= 0x4fff) {
    const inner = describe(code & 0xff, lang, basicName);
    const layer = (code >> 8) & 0x0f;
    const name = lang === "de" ? `${inner.name} / gehalten Ebene ${layer}` : `${inner.name} / hold layer ${layer}`;
    const hint =
      lang === "de"
        ? `Tippen: ${inner.name}. Halten: Ebene ${layer} ist aktiv.`
        : `Tap: ${inner.name}. Hold: layer ${layer} is active.`;
    return { name, short: `${inner.short}/L${layer}`, known: false, category: "layer", hint };
  }
  const extra = EXTRA[code];
  if (extra) {
    const hint = lang === "de" ? "Von Chilkey nicht beschrieben; bleibt, wie sie ist." : "Not described by Chilkey; stays as it is.";
    return { name: lang === "de" ? extra.de : extra.en, short: extra.short, known: false, category: null, hint };
  }
  const hint = lang === "de" ? "Unbekannte Funktion; bleibt, wie sie ist." : "Unknown function; stays as it is.";
  return { name: lang === "de" ? "Unbekannte Funktion" : "Unknown function", short: "?", known: false, category: null, hint };
}

export function keysIn(category: Category): KeyDef[] {
  return KEYS.filter((k) => k.category === category && k.code !== KC_NO && k.code !== KC_TRNS);
}
