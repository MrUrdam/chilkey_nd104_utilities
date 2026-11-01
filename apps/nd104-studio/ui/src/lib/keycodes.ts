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

  d(0x7820, "Lighting on/off", "light", { de: "Licht an/aus", short: "RGB" }),
  d(0x7821, "Next effect", "light", { de: "Nächster Effekt", short: "Eff+" }),
  d(0x7822, "Previous effect", "light", { de: "Vorheriger Effekt", short: "Eff−" }),
  d(0x7823, "Hue +", "light", { de: "Farbton +", short: "Hue+" }),
  d(0x7824, "Hue −", "light", { de: "Farbton −", short: "Hue−" }),
  d(0x7825, "Saturation +", "light", { de: "Sättigung +", short: "Sat+" }),
  d(0x7826, "Saturation −", "light", { de: "Sättigung −", short: "Sat−" }),
  d(0x7827, "Brightness +", "light", { de: "Helligkeit +", short: "Bri+" }),
  d(0x7828, "Brightness −", "light", { de: "Helligkeit −", short: "Bri−" }),
  d(0x7829, "Speed +", "light", { de: "Tempo +", short: "Spd+" }),
  d(0x782a, "Speed −", "light", { de: "Tempo −", short: "Spd−" }),

  ...layerKeys(),

  d(0x7e0f, "Display page up", "special", { de: "Display: Seite hoch", short: "LCD↑" }),
  d(0x7e10, "Display page down", "special", { de: "Display: Seite runter", short: "LCD↓" }),
  d(0x7e0e, "Display on/off", "special", { de: "Display an/aus", short: "LCD" }),
  d(0x7e16, "Display back", "special", { de: "Display: Zurück", short: "LCD⟲" }),
  d(0x7e17, "Display confirm", "special", { de: "Display: Bestätigen", short: "LCD✓" }),
  d(0x7e21, "Display calculator", "special", { de: "Display-Rechner", short: "LCalc" }),
  d(0x700b, "Lock Win key", "special", { de: "Win-Taste sperren", short: "WinLk" }),
  d(0x7e04, "Windows mode", "special", { de: "Windows-Modus", short: "Win" }),
  d(0x7e05, "Mac mode", "special", { de: "Mac-Modus", short: "Mac" }),
  d(0x7e00, "Mission Control", "special", { short: "MissC" }),
  d(0x7e01, "Launchpad", "special", { short: "LPad" }),
  d(0x7c16, "Esc / ` (grave escape)", "special", { de: "Esc / ^ (Grave-Escape)", short: "GEsc" }),
];

const BY_CODE = new Map(KEYS.map((k) => [k.code, k]));

/** Display names as reported by the driver for codes we know but don't offer. */
const EXTRA: Record<number, string> = {
  0x7e35: "Display key",
  0x7e18: "LcdLet",
  0x7e22: "Display reset",
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
}

/**
 * Name of a key code. `basicName` gives the name of a basic key in the
 * current label preset (so KC_Y can read "Z" on a German layout).
 */
export function describe(code: number, lang: string, basicName: (code: number) => string | null = () => null): Described {
  const def = BY_CODE.get(code);
  if (def) {
    const fromPreset = def.category === "basic" ? basicName(code) : null;
    const name = fromPreset ?? (lang === "de" && def.de ? def.de : def.en);
    const short = (lang === "de" ? def.deShort : undefined) ?? def.short ?? def.en;
    return { name, short: fromPreset ?? short, known: true, category: def.category };
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
    return { name, short: `${inner.short}/${mods}`, known: false, category: "mods" };
  }
  // Layer-tap: layer when held, key when tapped
  if (code >= 0x4000 && code <= 0x4fff) {
    const inner = describe(code & 0xff, lang, basicName);
    const layer = (code >> 8) & 0x0f;
    const name = lang === "de" ? `${inner.name} / gehalten Ebene ${layer}` : `${inner.name} / hold layer ${layer}`;
    return { name, short: `${inner.short}/L${layer}`, known: false, category: "layer" };
  }
  const extra = EXTRA[code];
  return { name: extra ? `${extra} (${hex(code)})` : hex(code), short: hex(code), known: false, category: null };
}

export function keysIn(category: Category): KeyDef[] {
  return KEYS.filter((k) => k.category === category && k.code !== KC_NO && k.code !== KC_TRNS);
}
