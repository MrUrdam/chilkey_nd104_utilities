# Chilkey ND104 Utilities

Sammlung von Skripten, Tools und (perspektivisch) Firmware-Experimenten rund um
die Chilkey ND104 Tastatur – vor allem für das integrierte Display.

Hauptziel: das Display unabhängig von der Hersteller-Software nutzen, z. B. um
dauerhaft das aktuelle Wetter für den eigenen Standort anzuzeigen, die Uhrzeit
zu synchronisieren und mit Themes/Bildern zu experimentieren.

> ⚠️ Inoffizielles Community-Projekt, nicht mit Chilkey verbunden.
> Nutzung auf eigene Gefahr – insbesondere beim Flashen von Firmware.

## Die Tastatur

Die [Chilkey ND104](https://chilkey.com/products/nd104-keyboard) ist eine
Full-Size-Tastatur (104 Tasten) mit Aluminiumgehäuse, Hot-Swap-Sockeln,
Drehregler und einem 1,47" Farbdisplay.

- Verbindung: USB-C, 2,4 GHz, Bluetooth 5.4
- Display: 320×172 TFT, zeigt Uhrzeit, Wetter, Bilder/GIFs
- Offizielle Software: [Chilkey Web-Driver / Console](https://chilkey.com/pages/console)
  (Chrome/Edge)
- Support & FAQ: [ND104 Support & Resources](https://chilkey.com/blogs/updates/nd104-faq-resources)

### Hardware (laut Community Reverse Engineering)

| Komponente | Chip / Info | USB VID:PID |
|---|---|---|
| Tastatur-MCU | WCH CH585 (RISC-V, BLE) | `6d67:016c` |
| Display-Controller | U-BEST UB32F341S (ARM, RTX + LVGL) | `5542:0001` |
| Display im DFU-Modus | – | `5542:1000` |

## Firmware

| Teil | Aktuelle Version (Chilkey) | Auf meinem Gerät |
|---|---|---|
| Tastatur | V3.0.4 | _tbd_ |
| Display | V1.2.1 | _tbd_ |

Die installierte Version lässt sich im Chilkey Web-Driver ablesen.

## Inhalt dieses Repos

| Ordner | Inhalt |
|---|---|
| `scripts/` | Python-Skripte für Display & Tastatur |
| `docs/` | Protokoll- und Hardware-Dokumentation |
| `firmware/` | Firmware-Notizen und -Experimente |
| `assets/` | Bilder & GIFs für das Display (320×172) |

## Quellen & Danksagung

- [ND104 Reverse Engineering](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b) – borrougagnou
- [ND104 Firmware Upgrade Guide](https://gist.github.com/borrougagnou/ab98c68cb6812d76fc0357e57f8541d9) – borrougagnou
- [zoom-sync](https://github.com/ozboar/zoom-sync) – Vorbild für Wetter/Zeit-Sync

## Lizenz

MIT – siehe [LICENSE](LICENSE).
