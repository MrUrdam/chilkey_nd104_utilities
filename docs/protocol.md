# ND104 HID Protocol Notes (USB and Bluetooth)

> **Status:** Collected from public sources and static analysis of the official
> web driver. Parts marked ✅ have been verified on real hardware (wired USB,
> keyboard V3.0.14, screen V1.2.4). Everything else is unverified – see the
> M0 checklist in § 6.
>
> Sources:
> - Official web driver [nd104.chilkey.com](https://nd104.chilkey.com/)
>   (WebHID, Chromium only), bundle `assets/index-BpBkeHgw.js`, analysed 2026-10-31
> - [ND104 Reverse Engineering](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b) by borrougagnou
>
> Scope: **USB cable** (§ 1–5) and **Bluetooth** (§ 5.5); the 2.4 GHz
> receiver has not been analysed. What ND104 Studio supports on each:
> [Connection modes](../apps/nd104-studio/README.md#connection-modes).

---

## 1. USB devices

The ND104 enumerates as two independent USB devices when connected by cable.

| Device | VID:PID | Vendor HID collection used by the web driver |
|---|---|---|
| Keyboard | `0x6D67:0x016C` (28007:364) | usage page `0xFF60`, usage `0x61` |
| Screen | `0x5542:0x0001` (21826:1) | usage page `0xFF60`, usage `0x61` |
| Screen in DFU mode | `0x5542:0x1000` | – (firmware flashing only, not used) |

WebHID filters used by the web driver:

```js
// keyboard
{ vendorId: 28007, productId: 364, usagePage: 65376, usage: 97 }
// screen ("device3" in the code)
{ vendorId: 21826, productId: 1,   usagePage: 65376, usage: 97 }
```

The web driver connects the screen as a **separate, second device**. It only
asks for it when the user opens the *Screen* page.

Reports have no report ID (`sendReport(0, …)`). Keyboard packets are 32 bytes
long, screen packets 64 bytes.

---

## 2. Checksums

### 2.1 CRC-16 (packet CRC)

CRC-16/CCITT-FALSE: poly `0x1021`, init `0xFFFF`, no reflection, no final XOR.

```js
// web driver
let crc = 0xFFFF;
for (const b of packet) {          // whole packet, CRC bytes still 0
  crc ^= b << 8;
  for (let i = 0; i < 8; i++)
    crc = (crc & 0x8000) ? (crc << 1) ^ 0x1021 : crc << 1, crc &= 0xFFFF;
}
packet[6] = crc & 0xFF;   // low byte first
packet[7] = crc >> 8;
```

- Calculated over the **entire packet** (32 or 64 bytes) while bytes 6–7 are still 0.
- Stored **low byte first** at offset 6, high byte at offset 7. This applies to
  **every** packet in the web driver, including the time packet. The gist
  describes the time CRC as big-endian; the web driver does not do that.
- Check value: `11 02` followed by 62 zero bytes (64 bytes) → CRC `0x1BFD`.
  This is the constant hard-coded into the screen version query (§ 3.2).

### 2.2 8-bit payload checksum (`1C` packets)

```
sum = Σ packet[9 .. end]   (with the checksum byte itself still 0)
checksum = (sum XOR 0xFF) & 0xFF
```

---

## 3. Firmware version queries

The web driver shows *Device Name*, *Device Version* and *Screen Version* in an
info dialog. These are the queries behind it.

### 3.1 Keyboard firmware version

**Request:** send to the keyboard (`6D67:016C`), 32 bytes:

| Offset | Value | Meaning |
|---|---|---|
| 0 | `0x11` | command: get version |
| 1 | `0x02` | |
| 2–5 | `0x00` | |
| 6–7 | CRC-16 (low, high) | over all 32 bytes, see § 2.1 (= `0x60BC` → `BC 60`) |
| 8–31 | `0x00` | |

**Response** (input report, 32 bytes). ✅ **Verified on hardware**
(2026-10-31, keyboard firmware V3.0.14):

```
11 02 00 00 00 17 91 f6 56 33 2e 30 2e 31 34 20   ........V3.0.14.
32 30 32 36 30 37 31 30 5f 31 32 33 33 33 31 00   20260710_123331.
```

| Offset | Value | Meaning |
|---|---|---|
| 0–1 | `11 02` | echo of the command |
| 5 | `0x17` | length of the version text (23) |
| 6–7 | `91 F6` | CRC-16 over all 32 bytes (bytes 6–7 = 0), **low byte first**. Valid: `0xF691` |
| 8… | ASCII | `V3.0.14 20260710_123331` → version + build date/time |

The web driver shows only bytes 8–14 (7 characters), i.e. `V3.0.14`.
ND104 Studio reads the full text using the length byte and checks the CRC.

Right after that, the web driver sends `01 00 …` (32 bytes). The answer has
byte 0 = `0x01`, and byte 2 holds a layout/protocol version that selects the
key code table. This is not the firmware version.

### 3.2 Screen firmware version

**Request:** send to the screen (`5542:0001`), 64 bytes:

| Offset | Value |
|---|---|
| 0 | `0x11` |
| 1 | `0x02` |
| 2–5 | `0x00` |
| 6 | `0xFD` |
| 7 | `0x1B` |
| 8–63 | `0x00` |

(`FD 1B` is simply the CRC-16 of this 64-byte packet, low byte first.)

**Response** (input report, 64 bytes). ✅ **Verified on hardware**
(2026-10-31, screen firmware V1.2.4):

```
11 02 00 00 00 1b e0 d9 56 31 2e 32 2e 34 20 4d   ........V1.2.4.M
61 72 20 31 37 20 32 30 32 36 20 32 31 3a 33 37   ar.17.2026.21:37
3a 30 30 00 …                                     :00 (zero-padded to 64)
```

The layout is the same as for the keyboard: length `0x1B` (27) at offset 5,
CRC `0xD9E0` over all 64 bytes, low byte first, at 6–7, and the text
`V1.2.4 Mar 17 2026 21:37:00` from offset 8.

**Responses handled by the web driver (screen device):**

| Condition | Content |
|---|---|
| byte 0 = `0x11` | bytes **8–13** = ASCII version string (6 chars) → *Screen Version* |
| byte 0 = `0x1C` and byte 9 = `0xFB` | bytes 14–18 = device address (shown as MAC `04:xx:xx:xx:xx:xx`); byte 21 = a **numeric** value that is *also* written to *Screen Version* |

The keyboard handler also writes *Screen Version*: when the keyboard sends a
`0x1C` report with byte 9 = `'A'`, bytes 6–12 are read as ASCII. This looks
like the keyboard passing on screen information.

### 3.3 Why the screen version looks broken on the web driver

Three code paths write to the same *Screen Version* field (`SVtype`):

1. the ASCII string from the screen's `0x11` answer,
2. a single **number** (byte 21) from the screen's `0x1C … FB` answer, which
   overwrites (1) whenever it arrives later,
3. the `0x1C … 'A'` answer of the keyboard.

In addition, the screen only gets queried after it has been connected as a
second WebHID device (Screen page). Without that, the field stays empty.
Whichever answer arrives last decides what the dialog shows, so you get an empty
field, a bare number, or the right string. That fits the broken display you
observed. The firmware itself probably answers correctly.

**Consequence for ND104 Studio:** use only the ASCII answer of the `0x11` query
(§ 3.1 / § 3.2) and ignore the numeric byte. If a version string is not
received within a timeout, the protocol check fails and the tool locks (firmware guard, see the [ND104 Studio README](../apps/nd104-studio/README.md#firmware-safety)).

---

## 4. Screen commands (`1C` packet family)

### 4.1 Generic wrapper (web driver function `J(payload, len)`)

32-byte packet:

| Offset | Value |
|---|---|
| 0 | `0x1C` |
| 1 | `0x02` |
| 2–4 | `0x00` |
| 5 | `len + 5` |
| 6–7 | CRC-16 (low, high) |
| 8 | `0xA5` (magic) |
| 9 | command byte (`payload[0]`) |
| 10 | `0x00` |
| 11 | `len` |
| 12 … 11+len | payload bytes `payload[1..]` |
| 12+len | 8-bit checksum (§ 2.2) |

Order of calculation: fill in the bytes → 8-bit checksum → CRC-16 last.

✅ **Target device, verified on hardware:** time and weather packets work through
**both** routes, the **keyboard** (`6D67:016C`, 32-byte report, like the web
driver) and the **screen** (`5542:0001`, the same 32 bytes zero-padded to 64,
like the gist). The keyboard passes the packet on to the screen. ND104 Studio uses
the keyboard route, as the web driver does.

### 4.1.1 Answers (acknowledgement) ✅

Every `1C` command is acknowledged. The screen's answer is a small `A5` frame:

```
A5 <cmd> 00 02 <status> 00 <checksum>
```

| Field | Value |
|---|---|
| `cmd` | echo of the command (`38` time, `FE` weather) |
| `status` | `FE` = OK; `FC` = rejected (seen for the theme command on screen V1.2.4, display unchanged) |
| checksum | 8-bit checksum (§ 2.2) over `cmd 00 02 status 00` |

How it arrives depends on the route:

| Route | Answers |
|---|---|
| Keyboard | 1. `1C 01 00 00 00 00 EB 24 …`: keyboard "received" (CRC valid)<br>2. `1C <kind> 00 00 00 07 <CRC> A5 <cmd> 00 02 FE 00 <chk> …`: the screen's answer, wrapped in a `1C` frame (kind `03` for time, `02` for weather) |
| Screen | `A5 <cmd> 00 02 FE 00 <chk> …` directly at offset 0 (64-byte report) |

Recorded examples (keyboard route):
- time: `1C 03 00 00 00 07 90 7E A5 38 00 02 FE 00 C7`
- weather: `1C 02 00 00 00 07 8F F0 A5 FE 00 02 FE 00 01`

ND104 Studio's protocol check P5 expects exactly this pattern after every write.
Missing answers, another status or a different structure lock the tool.

**Unsolicited answers:** besides the acknowledgements for our packets, the
keyboard sometimes sends well-formed OK answers for commands we never sent:

| Seen | Frame |
|---|---|
| `0x35` (frame kind 03) | `1C 03 00 00 00 07 54 76 A5 35 00 02 FE 00 CA` |
| `0x35` (frame kind 02) | `1C 02 00 00 00 07 7D F8 A5 35 00 02 FE 00 CA` |
| `0x3A` (frame kind 03) | `1C 03 00 00 00 07 C5 44 A5 3A 00 02 FE 00 C5` |

The first one arrived That is command `0x35`,
status OK, which ND104 Studio never sends. It is not caused by page changes or by
the time packet itself, and the web driver has no handling for it either. Its
origin is still open, see § 6. **ND104 Studio logs and ignores it** (decision
2026-10-31).

### 4.2 Weather (command `0xFE`, len 8)

Payload (offsets in the 32-byte packet):

| Offset | Value |
|---|---|
| 9 | `0xFE` |
| 11 | `0x08` |
| 12 | `0x00` |
| 13 | weather icon index |
| 14–15 | current temperature (big-endian) |
| 16–17 | max temperature (big-endian) |
| 18–19 | min temperature (big-endian) |
| 20 | 8-bit checksum |

**Temperature encoding: sign + magnitude, not two's complement.**

```js
// web driver
value = t > 0 ? (t * 10) | 0 : (-t * 10) | 0x8000;   // °C × 10, bit 15 = negative
```

-3.5 °C → `0x8023`. The gist assumes a signed int16 (two's complement), which
would give wrong values below 0 °C.

✅ **Verified on hardware:** 21.5 / 24 / 12 °C and -3.5 / 1 / -7.2 °C are
shown correctly, including the decimal.

**Icon index.** ✅ All 10 icons verified on hardware (screen V1.2.4, 24 h / °C):

| Index | Shown on the display | Meaning | WeatherAPI codes used by the web driver |
|---|---|---|---|
| 0 | Sun | Clear / sunny (day) | 1000 (day) |
| 1 | Two clouds, one behind the other | Overcast | – (unused) |
| 2 | Cloud + sun | Partly cloudy (day) | 1003 (day) |
| 3 | Single cloud | Cloudy | – (unused) |
| 4 | Cloud + rain | Rain / drizzle | 1063, 1072, 1150–1201, 1240–1246, 1087, 1273, 1276 |
| 5 | Cloud + snow | Snow / sleet / ice | 1066, 1069, 1114, 1117, 1204–1225, 1249–1258, 1279, 1282 |
| 6 | Purple moon + stars | Clear (night) | 1000 (after 18:00) |
| 7 | Cloud + purple moon + stars | Partly cloudy (night) | 1003 (after 18:00) |
| 8 | Cloud with bars underneath | Fog / mist (the web driver also uses it for overcast) | 1006, 1009, 1030, 1135, 1147 |
| 9 | Cloud + lightning | **Thunderstorm** | – (the web driver maps thunder to 4) |

Notes:
- **Icon 9 is a real thunderstorm icon.** The web driver and the community
  notes map thunderstorms to rain, but ND104 Studio will use 9.
- Icons 1 and 3 are not used by the web driver. ND104 Studio uses 1 for overcast
  and 3 for cloudy, and 8 only for fog/mist.
- Providers that only report "cloudy or overcast" (Open-Meteo WMO code 3,
  Bright Sky icon `cloudy`) are split by their cloud cover: below 88 % (7/8)
  → 3 (cloudy), from 88 % → 1 (overcast). OpenWeatherMap (803/804) and
  WeatherAPI (1006/1009) already tell the two apart.
- Indices above 9 are rejected by `nd104-protocol` and have not been tried on
  purpose.

> 🐞 In the analysed bundle, the web driver works out the icon index but then
> sends a **constant `0x02`** at offset 13. The icon is never actually used.
> ND104 Studio will send the real index at offset 13, as the gist describes.

### 4.3 Time (command `0x38`)

The web driver builds this one by hand, without the wrapper:

| Offset | Value |
|---|---|
| 0 | `0x1C` |
| 1 | `0x03` |
| 5 | `0x0F` |
| 6–7 | CRC-16 (low, high) |
| 8 | `0xA5` |
| 9 | `0x38` |
| 10–13 | `00 0A 00 01` |
| 14–15 | year (big-endian) |
| 16 | month (1–12) |
| 17 | day |
| 18 | hour (0–23) |
| 19 | minute |
| 20 | second |
| 21 | weekday (0 = Sunday, JS `getDay()`) |
| 22 | 8-bit checksum (§ 2.2) |

Local time of the PC is sent, so there is no time-zone information in the packet.

### 4.4 Other commands seen

| Command | Payload | Meaning |
|---|---|---|
| `0x39` | `00 01` / `00 02` / `00 03` / `00 04` | display menu navigation: up / down / switch / back (web driver buttons) – not tested |
| `0xFD` | `00 bgH bgL fgH fgL id` | theme + colours (RGB565 BE, themes 1–3). ❌ **Rejected by screen V1.2.4** (answer status `FC`, display unchanged; tested 2026-10-31). Kept in `nd104-protocol`/`nd104-probe theme` for diagnosis only |
| `0x34` | `00 01` | sent by the web driver before an image upload ("test/upload mode"), followed by `0xFB` |
| `0xFB` | – | screen info; answer `1C … FB` with an address and a number |
| `55 42 01 …` (raw) | width/height, frame count, frame delays | image/GIF upload header, followed by the `BA 06 AA B0` chunks |
| `0x58` (raw, not wrapped) | `[0x58, n/2, data…]` | debug/raw send; answer `58 xx 00` = done |
| `BA 06 AA B0 …` | | image/GIF upload (see gist); answers `BA ?? A5 B0/FC …` control chunking (67 or 137 packets per block) |

---

### 4.5 Own picture / GIF (raw reports to the screen) – used by ND104 Studio

From the web driver (`ScreenView`, function `R`) and the screen's input
handler. Sent straight to the **screen interface** (`5542:0001`, 64-byte
reports, not wrapped in `1C`). No other command is needed before.

```text
payload = 55 42 | width u16 BE (0x0140) | height u16 BE (0x00AC) | 00 | frames u8
        | one delay byte per frame (10 ms units; a still picture has one byte 01)
        | one size per frame (u32 BE)
        | the frames: baseline JPEG, 320 × 172 each
start   = BA 06 AA B0 | payload length u32 BE              (one report)
data    = BA n | n payload bytes, n ≤ 62                    (one report each)
```

- The data reports are grouped in **pages of 4096 payload bytes** (67 reports).
- After the start report and after each page the screen answers
  `BA ?? A5 B0 ?? 02 FE` → send the next page, or `BA ?? A5 B0 ?? 02 FD` → send
  the last page again. After the last page the screen sends **no answer**
  (verified on hardware); the web driver does not wait for one either.
- The driver accepts files up to 2 MB and asks for fewer than 60 GIF frames;
  still pictures are encoded with JPEG quality 1.0, GIF frames with 0.8.
- Answers `BA ?? A5 FC …` belong to a second transfer type of the driver with
  137 reports per block (not used).

## 5. Keyboard commands

The keyboard firmware is QMK based (strings `rgb_matrix_config`,
`rgblight_config`) and speaks the **VIA** protocol on its vendor interface
(`6D67:016C`, 32-byte reports, no `1C` frame). Every command is answered with
an echo of the request; `FF` as the first byte means "not supported".

### 5.1 Lighting – used by ND104 Studio (on/off, brightness, colour)

| Bytes | Meaning |
|---|---|
| `07 zone id data…` | set a value (live, not stored) |
| `08 zone id` | read a value; the answer repeats the bytes and adds the data |
| `09 zone` | store the zone in the keyboard's EEPROM |

| Zone | Web driver name | QMK |
|---|---|---|
| `03` | Background lighting (per key) | `rgb_matrix` |
| `02` | Bottom lighting | `rgblight` (underglow) |

| id | Value |
|---|---|
| `01` | brightness 0–255 |
| `02` | effect: `0` = all off, `1` = solid colour, more effects compiled into the firmware (≈ 18 per-key, 42 bottom) |
| `03` | effect speed 0–255 |
| `04` | colour: hue, saturation (0–255 each) |

The effects are **built into the firmware**; the driver only sends their index
and shows names for them. The driver reads zone `03` on connect but never zone
`02` (it remembers that one in the browser).

### 5.2 Keyboard settings (channel `00`) – debounce and sleep used by ND104 Studio

Write `07 00 04 id value`, then store with `09 00`; read `08 00 04 id`, the
answer carries the value in byte 4.

| id | Driver label | Values |
|---|---|---|
| `00` | USB polling rate | 1–8 (kHz) |
| `01` | debounce speed | ms |
| `02` | debounce mode | 0 none, 1 "send to debounce", 2 "debounce to send" |
| `04` | **keyboard colour** | 0 red, 1 pink, 2 yellow, 3 green, 4 blue, 5 black, 6 white |
| `05` | **online RGB** | 1 enable (default), 0 disable |
| `06` / `07` | sleep / deep-sleep time | minutes (the driver writes sleep time with id `0B`; `06` always reads 0 on V3.0.14) |

The driver reads only ids `00`–`02` on connect; keyboard colour and online RGB
come from the browser's storage (defaults 0 and 1), not from the keyboard.

What the driver shows and sends (ND104 Studio does the same, on the Device &
Firmware page):

| Setting | Written as | Read as | Values offered |
|---|---|---|---|
| Debounce mode | id `02` | id `02` | 0, 1, 2 |
| Debounce time | id `01` (0 while the mode is 0) | id `01` | 1–20, 25–50 (step 5), 60–100 (step 10), 200 ms |
| Sleep time | id **`0B`** | id **`0B`** ¹ | 1–20, 25–50 (step 5), 60 min |
| Deep-sleep time | id `07` | id `07` | same as sleep |

¹ The driver reads id `06`, which always answers 0 on V3.0.14; id `0B` reads
back the written value (checked 2026-11-02: write 5 → read 5, write 1 → read 1).
ND104 Studio reads `0B`.

Each write is followed by `09 00` (store). The polling-rate row exists in the
driver but is hidden; keyboard colour and online RGB are not used. No
text in the keyboard firmware names these two settings, so their effect is not
documented yet.

### 5.3 Keymap – used by ND104 Studio (keymap page, backup and restore)

VIA "dynamic keymap" commands, answered with an echo of the request (reads
carry the data in the echo):

| Bytes | Meaning |
|---|---|
| `12 off_hi off_lo size` | read `size` (≤ 28) bytes of the keymap buffer; data from byte 4 |
| `05 layer row col kc_hi kc_lo` | set one key (stored immediately) |
| `14 layer 00 dir` | read the knob action (`dir` 0 = left, 1 = right); code at bytes 4–5 |
| `15 layer 00 dir kc_hi kc_lo` | set the knob action |

- The buffer holds **6 layers × 8 rows × 16 columns**, 2 bytes per key
  (big-endian), so key `row × 16 + col` of `layer` is at offset
  `(layer × 128 + row × 16 + col) × 2` – 1536 bytes in total. The web driver
  reads it in 28-byte steps that never cross a layer.
- The knob is read and set for **layers 0–3**, left and right.
- Macros: `0D` buffer size, `10` clear all, `0F off_hi off_lo len data…`
  write (the driver ends with one byte at offset `0x0A3F`, probably the
  playback mode). The driver never reads macros back, so ND104 Studio does not
  back them up.

**Other programs:** Windows hands every input report to all programs that
have the interface open. While Chilkey's web driver or VIA is connected,
ND104 Studio receives their answers too (e.g. the keymap read `12 …` the web
driver sends on connect). The guard treats answers in the VIA range
`0x01`–`0x16` (and the screen's `58 …`) as well-formed foreign traffic:
logged, ignored, no lock.

### 5.4 Other keyboard commands (not used)

| Byte 0 | Meaning |
|---|---|
| `0x01` | protocol/layout version |

---

## 5.5 Over Bluetooth (analysis, 2026-11-02)

> Used by ND104 Studio since 2.1.0 – see [Connection modes](../apps/nd104-studio/README.md#connection-modes).

**In short:** Bluetooth is not "USB over the air". The keyboard is a
Bluetooth LE HID device (HID over GATT) that offers the same HID collections
as its USB keyboard interface – including the vendor command channel
`FF60:0061`. Windows turns it into a normal HID device, so the same 32-byte
commands work; only one report ID byte (`02`) is added. The screen controller
is a separate USB device inside the keyboard (`5542:0001`): over Bluetooth it
is not exposed, but the keyboard still forwards time and weather (`1C`
packets) to it, as over the cable. Battery level comes from the standard BLE
Battery Service, which Windows reads by itself.

Connected over Bluetooth LE, the keyboard shows up once per slot ("ND104 BT1"
… "BT3") with the **same IDs** (`6D67:016C`, release `0x0130`) through the
HID-over-GATT service (`{00001812-…}` in the device path). Windows exposes:

- keyboard, consumer control, system control and mouse collections;
- the **vendor collection `FF60:0061`** – the same command channel as over
  USB, but with **report ID 2** (descriptor `85 02`, 32-byte input and output
  reports). Writes therefore start with `02` instead of `00`; answers arrive
  with `02` in front (hidapi on Windows), the 32 bytes after it are identical
  to USB;
- the standard **battery service** (`180F`); Windows keeps the level as a
  device property (`{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2`, byte, percent);
- **not** the screen controller `5542:0001` – it is reachable by cable only.

Verified (read-only, `nd104-probe version` / `query`):

| Command | Result over Bluetooth |
|---|---|
| `11 02 …` keyboard version | `V3.0.14 20260710_123331`, identical to USB |
| `08 03 01/02/04`, `08 02 02/04` lighting | brightness, effect, colour answered |
| `08 00 04 01/02/07` debounce, deep sleep | answered (5 ms, mode 2, 10 min) |
| `08 00 04 06` sleep | `00` – always 0 (also over USB); `08 00 04 0B` reads the real sleep time |
| `14 00 00 dir` knob | answered (Vol− / Vol+) |
| `12 …` keymap buffer | answered, same data as over USB |

Verified writes (`nd104-probe time` / `weather`, one packet each, checked on
the display):

| Packet | Answers over Bluetooth | Display |
|---|---|---|
| Time `1C 03 … A5 38 …` (also a date in 2030) | `1C 01` receipt, then `A5 38 00 02 FE 00` (OK) | ✅ shows the sent time |
| Weather `1C 02 … A5 FE …` (snow, −7.5 °C) | `1C 01` receipt, then `A5 FE 00 02 FE 00` (OK) | ✅ shows the sent weather |
| Lighting `07 03 01 40` (brightness), `07 03 04 00 ff` (colour), not stored | echo, read back the new value | ✅ backlight darker / red |
| Setting `07 00 04 01 0a` (debounce 10 ms), not stored | echo, read back 10 | – (restored to 5) |
| Key `05 03 03 01 00 29` (layer 3) | echo, read back `0x0029` | – (restored to `0x0001`) |

So the keyboard relays `1C` packets to the screen over Bluetooth exactly as
over USB, and its own commands (lighting, settings, keymap) behave the same.
Only the screen controller is missing: the screen version cannot be read and
pictures cannot be uploaded over Bluetooth, so the firmware guard can only
check the keyboard there.

## 6. Open points for M0 (hardware verification)

- [x] Both devices enumerate with the vendor interface `FF60:0061` over USB
      (keyboard interface 1, screen interface 0). *(2026-10-31)*
- [x] Version queries work on both devices and return `version + build`, with
      a length byte and a valid CRC (§ 3.1 / § 3.2): keyboard
      `V3.0.14 20260710_123331`, screen `V1.2.4 Mar 17 2026 21:37:00`.
      The support page values (V3.0.4 / V1.2.1) are outdated. *(2026-10-31)*
- [x] Time and weather work through **both** the keyboard and the screen
      device, and each write is acknowledged (§ 4.1.1). *(2026-10-31)*
- [x] Icon index at offset 13 works; all 10 icons documented (§ 4.2).
      Icon 9 = thunderstorm. *(2026-10-31)*
- [x] Negative temperatures are shown correctly with sign + magnitude
      encoding. *(2026-10-31)*
- [x] The time is shown correctly (two test times, then the current time). *(2026-10-31)*
- [x] Icon 1 = two clouds (overcast), icon 3 = single cloud (cloudy). *(2026-10-31)*
- [ ] Origin of the unsolicited `A5 35 … FE 00` answer (§ 4.1.1).
      Not seen again: 25 s while switching pages, 3 time syncs, 5 min idle
      listening (11:39–11:44). Logged and ignored by ND104 Studio.
- [ ] *Not tested on purpose:* does the firmware check the CRC? Sending broken
      packets is an avoidable risk, and ND104 Studio always sends a valid CRC.
- [x] Command `0x39` identified from the web driver: display menu navigation
      (§ 4.4). Not sent by ND104 Studio.
- [x] Theme command `0xFD` rejected by screen V1.2.4 (status `FC`). *(2026-10-31)*
- [ ] *Not tested on purpose:* command `0x39` (menu navigation). ND104 Studio
      sends no unknown commands.

**Result:** M0 passed for keyboard **V3.0.14** and screen **V1.2.4**
(24 h / °C). These are the first verified firmware versions.
