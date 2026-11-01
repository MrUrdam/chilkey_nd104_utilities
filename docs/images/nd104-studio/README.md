# ND104 Studio screenshots

Used by the [main README](../../../README.md) and the
[ND104 Studio README](../../../apps/nd104-studio/README.md).

**Generated, do not edit by hand.** The workflow
[`nd104-studio-screenshots.yml`](../../../.github/workflows/nd104-studio-screenshots.yml)
renders them from the demo build of the settings window (sample data, no
keyboard; the sample GIF is drawn by the demo itself) and commits them whenever the UI changes. It can also be started by
hand under *Actions*.

| File | Page |
|---|---|
| `dashboard.png` | Dashboard with the live display preview (hero image) |
| `keymap.png` | Keymap: basic layer with two remapped keys, lighting look, Caps Lock selected with the function picker |
| `weather.png` | Weather (location and provider) |
| `clock.png` | Clock |
| `lighting.png` | Lighting: background lighting in solid colour, bottom lighting with a rainbow effect |
| `picture.png` | Screen picture: a sample animated GIF with its checks and preview |
| `device.png` | Device & Firmware, verified state |
| `general.png` | General |

Locally (Windows recommended for the right font):

```bash
cd apps/nd104-studio
npm ci
npm i --no-save playwright && npx playwright install chromium
npm run build:demo
npm run screenshots          # options: -- --lang de --theme light --out <dir>
```
