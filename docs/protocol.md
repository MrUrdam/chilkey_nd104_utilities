# ND104 USB HID Protocol Notes

> **Status:** Collected from public sources and static analysis of the official
> web driver. **Nothing on this page has been verified on hardware by this
> project yet** – verification is milestone M0 of [nd104-sync](PLAN.md).
>
> Sources:
> - Official web driver [nd104.chilkey.com](https://nd104.chilkey.com/)
>   (WebHID, Chromium only), bundle `assets/index-BpBkeHgw.js`, analysed 2026-10-31
> - [ND104 Reverse Engineering](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b) by borrougagnou
>
> Scope: **wired USB connection only.**

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

**Response** (input report, byte 0 = `0x11`):
- bytes **8–14** = ASCII version string, 7 characters → shown as *Device Version*.

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

**Consequence for nd104-sync:** use only the ASCII answer of the `0x11` query
(§ 3.1 / § 3.2) and ignore the numeric byte. If a version string is not
received within a timeout, treat it as unknown, which means locked mode.

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

> ⚠️ **Target device:** the web driver sends time and weather packets to the
> **keyboard** device (`6D67:016C`). The gist sends them to the **screen**
> device (`5542:0001`). Both may work, with the keyboard passing packets on to
> the screen. M0 has to find out which path is reliable.

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

**Icon index** (mapping in the web driver, from WeatherAPI condition codes):

| Index | Meaning | WeatherAPI codes |
|---|---|---|
| 0 | Sunny (day) – also default | 1000 |
| 2 | Partly cloudy (day) | 1003 |
| 4 | Rain / drizzle / thunder | 1063, 1072, 1150–1201, 1240–1246, 1087, 1273, 1276 |
| 5 | Snow / sleet / ice | 1066, 1069, 1114, 1117, 1204–1225, 1249–1258, 1279, 1282 |
| 6 | Clear (night, after 18:00) | 1000 |
| 7 | Partly cloudy (night, after 18:00) | 1003 |
| 8 | Cloudy / overcast / mist / fog | 1006, 1009, 1030, 1135, 1147 |
| 1, 3 | not used by the web driver (gist: icons 0–9 exist) | – |

> 🐞 In the analysed bundle, the web driver works out the icon index but then
> sends a **constant `0x02`** at offset 13. The icon is never actually used.
> nd104-sync will send the real index at offset 13, as the gist describes.

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
| `0x39` | `00 02` (len 2) | unknown, sent from the screen page |
| `0x58` (raw, not wrapped) | `[0x58, n/2, data…]` | debug/raw send; answer `58 xx 00` = done |
| `BA 06 AA B0 …` | | image/GIF upload (see gist); answers `BA ?? A5 B0/FC …` control chunking (67 or 137 packets per block) |

---

## 5. Keyboard commands (not used by nd104-sync)

These are listed only for completeness. nd104-sync never configures the keyboard.

| Byte 0 | Meaning |
|---|---|
| `0x01` | protocol/layout version |
| `0x08 0x03 n` | RGB: brightness / effect / speed / colour |
| `0x08 0x00 0x04 n` | debounce, polling rate, sleep times, … |
| `0x12` | read keymap |
| `0x14` | read knob mapping |

---

## 6. Open points for M0 (hardware verification)

- [ ] Do the version queries return the expected strings (keyboard `V3.0.4`?,
      screen `V1.2.1`?) and in which format exactly?
- [ ] Should time/weather go to the keyboard device or the screen device, or does either work?
- [ ] Does icon index at offset 13 work, and how does each index look on the display?
- [ ] Are negative temperatures shown correctly with sign + magnitude encoding?
- [ ] Is the CRC actually checked by the firmware (gist: zero CRC often accepted)?
- [ ] What does command `0x39 00 02` do?
