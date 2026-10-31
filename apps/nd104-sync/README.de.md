# nd104-sync

[English](README.md) · **Deutsch**

Hintergrund-Tool, das das Display der **Chilkey ND104** Tastatur mit **aktueller
Uhrzeit und aktuellem Wetter** versorgt – mit Tray-Icon und modernem
Einstellungs-/Statusfenster.

> 🚧 **Status: Planung.** Nichts von dem hier Beschriebenen ist bereits umgesetzt.
> Der detaillierte Plan steht in [docs/PLAN.md](../../docs/PLAN.md).

> 🔌 **Nur kabelgebunden (USB).** nd104-sync unterstützt die Tastatur
> ausschließlich per **USB-Kabel**. 2,4-GHz-Dongle und Bluetooth werden
> **nicht unterstützt** und nicht getestet.

> 🪟 **Vorerst nur Windows.** Windows 10/11 ist die einzige unterstützte
> Plattform. Die Architektur ist für Linux und macOS vorbereitet, die
> Unterstützung ist aber nicht umgesetzt.

---

## Warum?

Die ND104 hat keine Internetverbindung. Das Display kann Uhrzeit und Wetter
anzeigen, aber nur, wenn ein angeschlossener PC diese Infos schickt. Die
offizielle Lösung ist ein Browser-Treiber plus Wetter-Plugin mit festem
Wetter-Server. nd104-sync ersetzt das durch eine kleine Hintergrund-App, die du
einmal einrichtest: dein Wetterdienst, dein Standort, deine Zeitquelle.

## Was es macht

- Startet mit Windows und läuft im **Systemtray**
- **Synchronisiert die Uhrzeit** regelmäßig, beim Anstecken, nach dem Standby
  und genau zur Sommer-/Winterzeit-Umstellung
- **Holt das Wetter** für deinen Standort und sendet aktuelle, Höchst- und
  Tiefsttemperatur plus Wetter-Symbol ans Display
- Wetterdienst, **Standort**, **Zeitquelle**, **Zeitzone** und
  **Sommerzeit-Modus** im Einstellungsfenster wählbar
- **Status-Dashboard** mit Live-Vorschau dessen, was das Display zeigt
- **Schützt deine Tastatur:** funktioniert auch mit älterer/neuerer Firmware,
  stoppt aber sofort, sobald sich das Protokoll anders verhält
  (siehe [Firmware-Schutz](#firmware-schutz))
- **Zeigt die installierte Firmware** von Tastatur und Display (Dashboard und
  Seite „Gerät & Firmware“)
- **Prüft auf Firmware-Updates** nach dem Start und benachrichtigt dich;
  Button **„Nach Updates suchen“** in der Oberfläche und im Tray-Menü
- Oberfläche auf **Deutsch und Englisch** (folgt der Systemsprache, umschaltbar)
- **Dark / Light Mode** folgt Windows, in den Einstellungen änderbar

## Was es NICHT macht

- ❌ **Keine Funkverbindung** – kein 2,4-GHz-Dongle, kein Bluetooth. Nur USB-Kabel.
- ❌ **Kein Linux / macOS** bisher (zukünftig möglich, nicht umgesetzt).
- ❌ **Kein Firmware-Flashen.** Es lädt, installiert oder verändert keine
  Firmware, sondern informiert nur über verfügbare Updates.
- ❌ **Keine riskante Kommunikation.** An ungeprüfte Firmware werden nur Uhrzeit
  und Wetter gesendet, und nur solange alle Protokoll-Prüfungen bestehen.
  Sieht irgendetwas anders aus, hört das Tool sofort auf zu senden und sagt dir
  warum. Einen Schalter „ignorieren und weitermachen“ gibt es nicht.
- ❌ **Keine Tastatur-Konfiguration** – Tastenbelegung, RGB, Makros, Drehregler
  usw. bleiben beim offiziellen [Chilkey-Treiber](https://chilkey.com/pages/console).
- ❌ **Kein Bild-/GIF-Upload** in der ersten Version (später geplant).
- ❌ **Keine °C/°F- oder 12/24-h-Umschaltung am Display.** Chilkey liefert zwei
  Display-Firmware-Varianten (24 h + °C, 12 h + °F), und das Tool ändert nicht,
  welche du installiert hast (siehe [docs/firmware.md](../../docs/firmware.md)).
  Die Einheit in der Oberfläche ist davon unabhängig.
- ❌ **Keine Telemetrie.** Es kontaktiert nur die von dir eingestellten Dienste
  (siehe [Datenschutz](#datenschutz--netzwerkzugriffe)).
- ❌ **Nicht parallel** zum offiziellen Chilkey-Wetter-Plugin gedacht – beide
  würden aufs Display schreiben.

---

## Voraussetzungen

| | |
|---|---|
| Betriebssystem | Windows 10 (21H2+) oder Windows 11, 64 Bit |
| Verbindung | Chilkey ND104 per **USB-Kabel** |
| Firmware | Geprüfte Versionen, oder andere Versionen solange die Protokoll-Prüfungen bestehen (siehe unten) |
| Laufzeit | Microsoft Edge WebView2 (bei Windows 11 und aktuellem Windows 10 vorinstalliert) |

### Unterstützte Firmware

| Teil | Geprüfte Version | Status |
|---|---|---|
| Tastatur | – | noch nicht geprüft (Meilenstein M0) |
| Display (24 h / °C) | – | noch nicht geprüft (Meilenstein M0) |
| Display (12 h / °F) | – | vorerst nicht geplant |

Firmware-Quellen und neueste Dateien: [docs/firmware.md](../../docs/firmware.md).

## Firmware-Schutz

Bevor irgendetwas gesendet wird, liest nd104-sync die Firmware-Versionen von
Tastatur und Display – mit denselben Abfragen wie der offizielle Web-Treiber
(siehe [docs/protocol.md § 3](../../docs/protocol.md#3-firmware-version-queries)) –
und vergleicht sie mit einer Liste geprüfter Versionen.

Ältere oder neuere Firmware ist willkommen, aber das Protokoll muss sich genau
wie erwartet verhalten. nd104-sync prüft das beim Anstecken und bei jedem
Senden:
- beide USB-Geräte haben die erwarteten Schnittstellen und Reportgrößen,
- die Versionsabfragen werden korrekt beantwortet,
- Schreibzugriffe werden angenommen,
- Antworten sehen so aus wie bei geprüfter Firmware.

| Situation | Verhalten |
|---|---|
| Geprüfte Version, alle Prüfungen bestanden | **Geprüft:** Normalbetrieb |
| Ältere/neuere Version, alle Prüfungen bestanden | **Kompatibel (ungeprüft):** Uhrzeit und Wetter werden synchronisiert; einmalige Benachrichtigung, die Statusseite zeigt „ungeprüfte Firmware“ |
| Eine Prüfung schlägt fehl (vorher oder im Betrieb) | **Gesperrt:** sofortiger Sendestopp. Das Tray-Icon wird orange, die Benachrichtigung nennt die fehlgeschlagene Prüfung, und die Statusseite erklärt, was zu tun ist |
| Version nicht lesbar | Gesperrt |

Zusatzfunktionen wie Themes oder Bilder (geplant) werden nur mit geprüfter
Firmware genutzt. Eine Sperre wird nur durch erneutes Anstecken oder „Erneut
prüfen“ aufgehoben, und in beiden Fällen laufen alle Prüfungen von vorn.
Einen Schalter „ignorieren und weitermachen“ gibt es bewusst **nicht**.

## Firmware-Update-Prüfung

Kurz nach dem Start (und danach einmal täglich) prüft nd104-sync eine
Versionsliste in diesem Repo ([`firmware/versions.json`](../../firmware/)). Gibt es
eine neuere Firmware von Chilkey, erhältst du eine Benachrichtigung mit Link zur
offiziellen Anleitung – und den Hinweis, ob nd104-sync diese Version schon
unterstützt. Das Update selbst bleibt deine Entscheidung und läuft über die
Chilkey-Tools.

## Wetterdienste

| Anbieter | API-Key | Abdeckung | Status |
|---|---|---|---|
| [Open-Meteo](https://open-meteo.com/) (Standard) | nein | weltweit | geplant (M1) |
| Manuell / Test | – | – | geplant (M1) |
| [Bright Sky](https://brightsky.dev/) (DWD-Daten) | nein | Deutschland | geplant (M4) |
| [OpenWeatherMap](https://openweathermap.org/) | ja | weltweit | geplant (M4) |
| [WeatherAPI.com](https://www.weatherapi.com/) | ja | weltweit | geplant (M4) |

Wetterdienste sind Plug-ins mit einer gemeinsamen Schnittstelle, weitere lassen
sich ergänzen (siehe [docs/PLAN.md § 5.6](../../docs/PLAN.md#56-weather-service)).
API-Keys liegen in der Windows-Anmeldeinformationsverwaltung, nicht in der
Konfigurationsdatei.

## Zeitquellen

| Quelle | Beschreibung |
|---|---|
| System (Standard) | Lokale PC-Zeit |
| Zeitzone | PC-Zeit umgerechnet in eine gewählte Zeitzone |
| NTP | Zeit von einem NTP-Server (Standard `pool.ntp.org`) |
| Manuell | Systemzeit plus fester Versatz |

Sommer-/Winterzeit: **Automatisch** (aus den Zeitzonen-Regeln) · Sommerzeit erzwingen · Winterzeit erzwingen.

---

## Installation

> Noch nicht verfügbar – Releases erscheinen auf der
> [GitHub-Release-Seite](https://github.com/MrUrdam/chilkey_nd104_utilities/releases)
> als Windows-Installer (`.msi` / `.exe`).

## Benutzung

1. ND104 per **USB-Kabel** anschließen.
2. Offiziellen Chilkey-Web-Treiber bzw. das Wetter-Plugin schließen.
3. nd104-sync starten. Beim ersten Start öffnet sich das Einstellungsfenster –
   Standort wählen (Stadtsuche) und bei Bedarf einen anderen Wetterdienst.
4. Ab dann startet nd104-sync mit Windows und läuft im Tray.

**Tray-Menü**

| Eintrag | Funktion |
|---|---|
| Statuszeile | Verbindung, Wetter und letzte Synchronisierung auf einen Blick |
| Jetzt synchronisieren | Zeit und Wetter sofort senden |
| Pause / Fortsetzen | Alle Updates anhalten / fortsetzen |
| Nach Firmware-Updates suchen | Manuelle Prüfung, zeigt das Ergebnis an |
| Einstellungen… | Einstellungs-/Statusfenster öffnen |
| Sprache ▸ | Automatisch / Deutsch / English |
| Log-Ordner öffnen | Zur Fehlersuche |
| Beenden | App schließen |

**Tray-Icon-Farben:** Akzentfarbe = verbunden & synchron · grau = nicht verbunden ·
orange = gesperrt (Firmware) · rot = Fehler.

**Kommandozeilen-Optionen (geplant)**

```
nd104-sync.exe                # normaler Start (Tray)
nd104-sync.exe --settings     # Einstellungsfenster direkt öffnen
nd104-sync.exe --minimized    # ohne Fenster starten (für Autostart)
nd104-sync.exe --sync-once    # einmal synchronisieren und beenden
```

**Dateien**

| Was | Wo |
|---|---|
| Konfiguration | `%APPDATA%\nd104-sync\config.toml` |
| Logs | `%LOCALAPPDATA%\nd104-sync\logs\` |
| API-Keys | Windows-Anmeldeinformationsverwaltung (`nd104-sync/<anbieter>`) |

## Datenschutz / Netzwerkzugriffe

nd104-sync verbindet sich nur mit:
- dem gewählten Wetterdienst (und der Open-Meteo-Ortssuche),
- dem NTP-Server, falls NTP als Zeitquelle gewählt ist,
- GitHub (Rohdatei dieses Repos) für die Firmware-Update-Prüfung – abschaltbar.

Keine Telemetrie, keine Analyse, keine Standortermittlung über die IP, außer du
schaltest sie ein.

## Fehlerbehebung

| Problem | Lösung |
|---|---|
| „Nicht verbunden“ | USB-**Kabel** verwenden (Funk wird nicht unterstützt); anderen Port probieren; keine passiven Hubs |
| Oranges Icon / gesperrt | Eine Protokoll-Prüfung ist fehlgeschlagen (Grund auf der Seite „Gerät & Firmware“). Kabel neu anstecken oder „Erneut prüfen“; bleibt es gesperrt, verhält sich deine Firmware anders – bitte ein Issue mit dem Log anlegen |
| Display zeigt andere Werte | Vermutlich läuft noch das offizielle Chilkey-Wetter-Plugin – schließen |
| Falsche Uhrzeit nach Zeitumstellung | Zeitzone / Sommerzeit-Modus im Tab „Uhr“ prüfen |

---

## Aus dem Quellcode bauen

### Voraussetzungen (Windows)

| Werkzeug | Version | Hinweis |
|---|---|---|
| [Rust](https://rustup.rs/) | stable, `x86_64-pc-windows-msvc` | über `rustup` |
| Visual Studio Build Tools | 2022 | Workload „Desktopentwicklung mit C++“ |
| [Node.js](https://nodejs.org/) | LTS (≥ 20) | für den UI-Build |
| WebView2 Runtime | aktuell | bei Windows 11 vorinstalliert |
| Tauri CLI | v2 | `cargo install tauri-cli --version "^2"` |

### Bauen

```bash
cd apps/nd104-sync
npm install --prefix ui
cargo tauri dev        # Entwicklungsmodus mit Hot Reload
cargo tauri build      # Release-Build + Installer in src-tauri/target/release/bundle
cargo test --workspace # Unit-Tests (ohne Hardware)
```

### Wichtigste Abhängigkeiten

| Crate / Paket | Zweck |
|---|---|
| `tauri` v2 + Plugins `autostart`, `single-instance`, `notification` | App-Gerüst, Tray, Fenster |
| `hidapi` | USB-HID-Kommunikation |
| `tokio` | Async-Laufzeit, Scheduler |
| `reqwest` (rustls) | HTTP für Wetter / Ortssuche / Versionsliste |
| `jiff` | Zeitzonen und Sommerzeit |
| `sntpc` | NTP-Client |
| `serde`, `toml`, `directories` | Konfiguration |
| `keyring` | API-Keys in der Anmeldeinformationsverwaltung |
| `tracing`, `tracing-appender` | Logging |
| `sys-locale` | Erkennung der Systemsprache |
| Svelte + TypeScript + Vite | Einstellungsoberfläche |

### Projektstruktur

```
apps/nd104-sync/
├── crates/
│   ├── nd104-protocol/   # Paketaufbau, CRC – ohne I/O, mit Unit-Tests
│   └── nd104-core/       # Gerät, Firmware-Schutz, Uhr, Wetter, Scheduler, Konfiguration
├── src-tauri/            # Tauri-App: Tray, Commands, Installer
├── ui/                   # Svelte-Frontend
├── locales/              # en.json, de.json
└── packaging/            # Installer-Konfiguration
```

## Mitmachen

- **Neuer Wetterdienst:** `WeatherProvider`-Trait implementieren, registrieren,
  Übersetzungen ergänzen und Zeile in der Tabelle oben hinzufügen.
- **Neue Sprache:** `locales/<code>.json` mit allen Schlüsseln aus `en.json` anlegen.
- **Neue Plattform (Linux/macOS):** `Platform`-Trait implementieren
  (siehe [docs/PLAN.md § 5.10](../../docs/PLAN.md#510-platform-layer-r2)).
- **Neue Firmware-Version:** Protokoll an echter Hardware prüfen, in
  `docs/protocol.md` dokumentieren, zur Liste geprüfter Versionen hinzufügen.

## Danksagung

Das Protokollwissen basiert auf den
[ND104-Reverse-Engineering-Notizen](https://gist.github.com/borrougagnou/a21613093f323a05167a92212ab1ff7b)
von borrougagnou und einer Analyse des offiziellen
[Chilkey-Web-Treibers](https://nd104.chilkey.com/) – gesammelt in
[docs/protocol.md](../../docs/protocol.md). Inspiriert von
[zoom-sync](https://github.com/ozboar/zoom-sync).

Inoffizielles Projekt, nicht mit Chilkey verbunden. MIT-Lizenz – siehe [LICENSE](../../LICENSE).
