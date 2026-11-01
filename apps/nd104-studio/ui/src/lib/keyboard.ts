// Physical layout of the Chilkey ND104 (ISO and ANSI) and the legend
// presets shown on the keys. Every key knows its matrix position(s) in the
// keymap (row, column) and the HID usage it sends by default.
//
// Matrix positions come from the keyboard's own default keymap. A few keys
// have alternative positions for other board variants (split Backspace,
// split right Shift, split Space); they are listed as aliases and always
// written together, so a change works whichever switch is fitted.

export type Shape = "iso" | "ansi";

export interface PhysicalKey {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** [row, col] positions; the first one is read, all are written. */
  pos: [number, number][];
  /** HID usage this key sends by default (for legends and names). */
  hid: number;
  mod?: boolean;
  round?: boolean;
  /** Not remappable on layer 0 (keeps the other layers reachable). */
  protectedOnBase?: boolean;
  /** Not a rectangle: the ISO Enter (wide on top, narrower below). */
  shape?: "iso-enter";
}

function row(ids: string[], x0: number, y: number, rowNo: number, col0: number, hid0: number[] | number): PhysicalKey[] {
  return ids.map((id, i) => ({
    id,
    x: x0 + i,
    y,
    w: 1,
    h: 1,
    pos: [[rowNo, col0 + i]],
    hid: Array.isArray(hid0) ? hid0[i] : hid0 + i,
  }));
}

const k = (
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
  pos: [number, number][],
  hid: number,
  extra: Partial<PhysicalKey> = {},
): PhysicalKey => ({ id, x, y, w, h, pos, hid, ...extra });

/** Keys shared by ISO and ANSI. */
function common(): PhysicalKey[] {
  const keys: PhysicalKey[] = [
    k("esc", 0, 0, 1, 1, [[0, 0]], 0x29, { mod: true }),
    ...row(["f1", "f2", "f3", "f4"], 1.25, 0, 0, 1, 0x3a),
    ...row(["f5", "f6", "f7", "f8"], 5.5, 0, 0, 5, 0x3e).map((x) => ({ ...x, mod: true })),
    ...row(["f9", "f10", "f11", "f12"], 9.75, 0, 0, 9, 0x42),
    k("disp", 14, 0, 1, 1, [[7, 9]], 0x7e35, { mod: true }),
    k("prtsc", 15.25, 0, 1, 1, [[0, 13]], 0x46, { mod: true }),
    k("scrlk", 16.25, 0, 1, 1, [[0, 14]], 0x47, { mod: true }),
    k("pause", 17.25, 0, 1, 1, [[0, 15]], 0x48, { mod: true }),
    k("round", 18.5, 0, 1, 1, [[7, 12]], 0xb2, { mod: true, round: true }),

    k("grave", 0, 1.25, 1, 1, [[1, 0]], 0x35),
    ...row(["d1", "d2", "d3", "d4", "d5", "d6", "d7", "d8", "d9", "d0", "minus"], 1, 1.25, 1, 1, [
      0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2d,
    ]),
    k("equal", 12, 1.25, 1, 1, [[6, 8]], 0x2e),
    k("bksp", 13, 1.25, 2, 1, [[7, 10], [7, 11]], 0x2a, { mod: true }),
    k("ins", 15.25, 1.25, 1, 1, [[1, 12]], 0x49, { mod: true }),
    k("home", 16.25, 1.25, 1, 1, [[1, 13]], 0x4a, { mod: true }),
    k("pgup", 17.25, 1.25, 1, 1, [[6, 4]], 0x4b, { mod: true }),
    k("num", 18.5, 1.25, 1, 1, [[6, 5]], 0x53, { mod: true }),
    k("kpslash", 19.5, 1.25, 1, 1, [[1, 14]], 0x54, { mod: true }),
    k("kpstar", 20.5, 1.25, 1, 1, [[1, 15]], 0x55, { mod: true }),
    k("kpminus", 21.5, 1.25, 1, 1, [[6, 13]], 0x56, { mod: true }),

    k("tab", 0, 2.25, 1.5, 1, [[2, 0]], 0x2b, { mod: true }),
    ...row(["q", "w", "e", "r", "t", "y", "u", "i", "o", "p", "lbr"], 1.5, 2.25, 2, 1, [
      0x14, 0x1a, 0x08, 0x15, 0x17, 0x1c, 0x18, 0x0c, 0x12, 0x13, 0x2f,
    ]),
    k("rbr", 12.5, 2.25, 1, 1, [[6, 10]], 0x30),
    k("del", 15.25, 2.25, 1, 1, [[2, 12]], 0x4c, { mod: true }),
    k("end", 16.25, 2.25, 1, 1, [[2, 13]], 0x4d, { mod: true }),
    k("pgdn", 17.25, 2.25, 1, 1, [[6, 6]], 0x4e, { mod: true }),
    k("kp7", 18.5, 2.25, 1, 1, [[6, 7]], 0x5f),
    k("kp8", 19.5, 2.25, 1, 1, [[2, 14]], 0x60),
    k("kp9", 20.5, 2.25, 1, 1, [[2, 15]], 0x61),
    k("kpplus", 21.5, 2.25, 1, 2, [[6, 14]], 0x57, { mod: true }),

    k("caps", 0, 3.25, 1.75, 1, [[3, 0]], 0x39, { mod: true }),
    ...row(["a", "s", "d", "f", "g", "h", "j", "k", "l", "semi", "quote"], 1.75, 3.25, 3, 1, [
      0x04, 0x16, 0x07, 0x09, 0x0a, 0x0b, 0x0d, 0x0e, 0x0f, 0x33, 0x34,
    ]),
    ...row(["kp4", "kp5", "kp6"], 18.5, 3.25, 3, 13, 0x5c),

    ...row(["z", "x", "c", "v", "b", "n", "m", "comma", "dot", "slash"], 2.25, 4.25, 4, 1, [
      0x1d, 0x1b, 0x06, 0x19, 0x05, 0x11, 0x10, 0x36, 0x37, 0x38,
    ]),
    k("rshift", 12.25, 4.25, 2.75, 1, [[4, 11], [7, 8]], 0xe5, { mod: true }),
    k("up", 16.25, 4.25, 1, 1, [[4, 12]], 0x52, { mod: true }),
    ...row(["kp1", "kp2", "kp3"], 18.5, 4.25, 4, 13, 0x59),
    k("kpenter", 21.5, 4.25, 1, 2, [[6, 15]], 0x58, { mod: true }),

    k("lctrl", 0, 5.25, 1.25, 1, [[5, 0]], 0xe0, { mod: true }),
    k("lwin", 1.25, 5.25, 1.25, 1, [[5, 1]], 0xe3, { mod: true }),
    k("lalt", 2.5, 5.25, 1.25, 1, [[5, 2]], 0xe2, { mod: true }),
    k("space", 3.75, 5.25, 6.25, 1, [[5, 3], [5, 4], [5, 5], [5, 6]], 0x2c),
    k("ralt", 10, 5.25, 1.25, 1, [[5, 8]], 0xe6, { mod: true }),
    k("menu", 11.25, 5.25, 1.25, 1, [[5, 9]], 0x65, { mod: true }),
    k("fn", 12.5, 5.25, 1.25, 1, [[5, 10]], 0x5221, { mod: true, protectedOnBase: true }),
    k("rctrl", 13.75, 5.25, 1.25, 1, [[5, 11]], 0xe4, { mod: true }),
    k("left", 15.25, 5.25, 1, 1, [[6, 12]], 0x50, { mod: true }),
    k("down", 16.25, 5.25, 1, 1, [[5, 12]], 0x51, { mod: true }),
    k("right", 17.25, 5.25, 1, 1, [[5, 13]], 0x4f, { mod: true }),
    k("kp0", 18.5, 5.25, 2, 1, [[5, 14]], 0x62),
    k("kpdot", 20.5, 5.25, 1, 1, [[5, 15]], 0x63),
  ];
  return keys;
}

function iso(): PhysicalKey[] {
  return [
    ...common(),
    // spans 1.5 units on the Tab row and 1.25 units on the Caps row
    k("enter", 13.5, 2.25, 1.5, 2, [[3, 12]], 0x28, { mod: true, shape: "iso-enter" }),
    // the ISO # key sends the US backslash code (Windows maps both to #)
    k("hash", 12.75, 3.25, 1, 1, [[6, 11]], 0x31),
    k("lshift", 0, 4.25, 1.25, 1, [[4, 0]], 0xe1, { mod: true }),
    k("iso", 1.25, 4.25, 1, 1, [[6, 0]], 0x64),
  ];
}

function ansi(): PhysicalKey[] {
  return [
    ...common(),
    k("bslash", 13.5, 2.25, 1.5, 1, [[6, 11]], 0x31),
    k("enter", 12.75, 3.25, 2.25, 1, [[3, 12]], 0x28, { mod: true }),
    k("lshift", 0, 4.25, 2.25, 1, [[4, 0]], 0xe1, { mod: true }),
  ];
}

export function layout(shape: Shape): PhysicalKey[] {
  return shape === "ansi" ? ansi() : iso();
}

/** Total size in key units (for scaling). */
export const LAYOUT_WIDTH = 22.5;
export const LAYOUT_HEIGHT = 6.25;

/** Elements that are not keys, for orientation. */
export const DECOR = [
  { kind: "screen", x: 19.65, y: 0, w: 1.7, h: 1 },
  { kind: "bar", x: 15.4, y: 3.5, w: 2.7, h: 0.25 },
] as const;
export const KNOB = { x: 21.5, y: 0, w: 1, h: 1 };

// ---------------------------------------------------------------------------
// Legend presets: what is printed on the keys (display only)

export type Legends = Record<string, string>;

const MODS_EN: Legends = {
  esc: "Esc", bksp: "Backspace", tab: "Tab", caps: "Caps", enter: "Enter",
  lshift: "Shift", rshift: "Shift", lctrl: "Ctrl", rctrl: "Ctrl", lwin: "Win",
  menu: "Menu", lalt: "Alt", ralt: "Alt", fn: "Fn", space: "",
  ins: "Ins", home: "Home", pgup: "PgUp", del: "Del", end: "End", pgdn: "PgDn",
  prtsc: "PrtSc", scrlk: "ScrLk", pause: "Pause", num: "Num", kpdot: ".",
  round: "Calc", disp: "Disp", up: "↑", down: "↓", left: "←", right: "→",
};

const MODS_DE: Legends = {
  ...MODS_EN,
  bksp: "⟵", caps: "Feststell", lshift: "⇧", rshift: "⇧", lctrl: "Strg",
  rctrl: "Strg", ralt: "AltGr", menu: "Menü", ins: "Einfg", home: "Pos1",
  pgup: "Bild↑", del: "Entf", end: "Ende", pgdn: "Bild↓", prtsc: "Druck",
  scrlk: "Rollen", kpdot: ",",
};

const COMMON: Legends = {
  f1: "F1", f2: "F2", f3: "F3", f4: "F4", f5: "F5", f6: "F6", f7: "F7", f8: "F8",
  f9: "F9", f10: "F10", f11: "F11", f12: "F12",
  kpslash: "/", kpstar: "*", kpminus: "-", kpplus: "+", kpenter: "Enter",
  kp0: "0", kp1: "1", kp2: "2", kp3: "3", kp4: "4", kp5: "5", kp6: "6", kp7: "7", kp8: "8", kp9: "9",
  d1: "1", d2: "2", d3: "3", d4: "4", d5: "5", d6: "6", d7: "7", d8: "8", d9: "9", d0: "0",
};
for (const c of "abcdefghijklmnopqrstuvwxyz") COMMON[c] = c.toUpperCase();

export interface LegendPreset {
  id: string;
  name: string;
  shape: Shape;
  legends: Legends;
}

export const BUILTIN_PRESETS: LegendPreset[] = [
  {
    id: "iso-uk",
    name: "ISO UK",
    shape: "iso",
    legends: {
      ...COMMON, ...MODS_EN,
      grave: "¬\n`", d1: "!\n1", d2: '"\n2', d3: "£\n3", d4: "$\n4", d5: "%\n5", d6: "^\n6",
      d7: "&\n7", d8: "*\n8", d9: "(\n9", d0: ")\n0", minus: "_\n-", equal: "+\n=",
      lbr: "{\n[", rbr: "}\n]", semi: ":\n;", quote: "@\n'", hash: "~\n#",
      iso: "|\n\\", comma: "<\n,", dot: ">\n.", slash: "?\n/",
    },
  },
  {
    id: "iso-de",
    name: "ISO DE",
    shape: "iso",
    legends: {
      ...COMMON, ...MODS_DE,
      grave: "°\n^", d1: "!\n1", d2: '"\n2', d3: "§\n3", d4: "$\n4", d5: "%\n5", d6: "&\n6",
      d7: "/\n7", d8: "(\n8", d9: ")\n9", d0: "=\n0", minus: "?\nß", equal: "`\n´",
      y: "Z", z: "Y", lbr: "Ü", rbr: "*\n+", semi: "Ö", quote: "Ä", hash: "'\n#",
      iso: ">\n<", comma: ";\n,", dot: ":\n.", slash: "_\n-",
    },
  },
  {
    id: "ansi-us",
    name: "ANSI US",
    shape: "ansi",
    legends: {
      ...COMMON, ...MODS_EN,
      grave: "~\n`", d1: "!\n1", d2: "@\n2", d3: "#\n3", d4: "$\n4", d5: "%\n5", d6: "^\n6",
      d7: "&\n7", d8: "*\n8", d9: "(\n9", d0: ")\n0", minus: "_\n-", equal: "+\n=",
      lbr: "{\n[", rbr: "}\n]", bslash: "|\n\\", semi: ":\n;", quote: "\"\n'",
      comma: "<\n,", dot: ">\n.", slash: "?\n/",
    },
  },
];

/** A user's own legends: a copy of a built-in preset with changes. */
export interface CustomLegendPreset {
  id: string;
  name: string;
  base: string;
  labels: Record<string, string>;
}

/** Legends of a built-in or custom preset (custom: base + own labels). */
export function resolveLegends(id: string, custom: CustomLegendPreset[]): { preset: LegendPreset; custom: CustomLegendPreset | null } {
  const own = custom.find((c) => c.id === id);
  const base = BUILTIN_PRESETS.find((p) => p.id === (own ? own.base : id)) ?? BUILTIN_PRESETS[0];
  if (!own) return { preset: base, custom: null };
  return { preset: { ...base, id: own.id, name: own.name, legends: { ...base.legends, ...own.labels } }, custom: own };
}
