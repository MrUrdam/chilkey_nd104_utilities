// Takes the README screenshots of the settings window from the demo build.
//
//   npm run build:demo               # → ui/dist-demo (sample data, no keyboard)
//   npm i --no-save playwright && npx playwright install chromium
//   node scripts/screenshots.mjs     # → docs/images/nd104-sync/*.png
//
// Options: --out <dir>  --lang en|de  --theme dark|light
// Runs in CI via .github/workflows/nd104-sync-screenshots.yml (Windows, so the
// window uses the real Segoe UI font).

import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const here = fileURLToPath(new URL(".", import.meta.url));
const app = resolve(here, "..");
const args = process.argv.slice(2);
const option = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};

const root = join(app, "ui", "dist-demo");
const out = resolve(option("out", join(app, "..", "..", "docs", "images", "nd104-sync")));
const lang = option("lang", "en");
const theme = option("theme", "dark");
const texts = JSON.parse(await readFile(join(app, "locales", `${lang}.json`), "utf8"));

// Same size as the app window (src-tauri/src/main.rs), rendered at 2× for sharp images.
const WINDOW = { width: 1000, height: 680 };
// Fixed demo time so every run produces the same pictures.
const DEMO_TIME = new Date("2026-11-01T10:42:00+01:00");

const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".json": "application/json",
};

const server = createServer(async (req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://x").pathname);
  const file = normalize(join(root, path === "/" ? "index.html" : path));
  if (!file.startsWith(root)) return res.writeHead(403).end();
  try {
    const body = await readFile(file);
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" }).end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/?lang=${lang}&theme=${theme}`;

const SHOTS = [
  { file: "dashboard.png", page: "dashboard" },
  {
    file: "weather.png",
    page: "weather",
    // Show a test result, like after pressing "Test".
    prepare: async (page) => {
      await page.getByRole("button", { name: texts["ui.weather.test"], exact: true }).first().click();
      await page.locator(".notice strong").first().waitFor();
    },
  },
  { file: "clock.png", page: "clock" },
  {
    file: "lighting.png",
    page: "lighting",
    // wait until both zones show the values read from the "keyboard"
    prepare: async (page) => {
      await page.locator("select:not([disabled])").nth(1).waitFor();
    },
  },
  {
    file: "picture.png",
    page: "picture",
    // a picked GIF with its checks and the playing preview
    prepare: async (page) => {
      await page.getByRole("button", { name: texts["ui.pic.choose"], exact: true }).click();
      await page.locator(".checks").waitFor();
    },
  },
  { file: "device.png", page: "device" },
  { file: "general.png", page: "general" },
];

const browser = await chromium.launch();
try {
  const context = await browser.newContext({
    viewport: WINDOW,
    deviceScaleFactor: 2,
    locale: lang === "de" ? "de-DE" : "en-GB",
    timezoneId: "Europe/Berlin",
    colorScheme: theme === "light" ? "light" : "dark",
  });
  const page = await context.newPage();
  page.on("pageerror", (e) => console.error("page error:", e.message));
  await page.clock.install({ time: DEMO_TIME });
  await page.goto(url);
  await page.locator(".shell").waitFor();
  await page.evaluate(() => document.fonts.ready);
  await mkdir(out, { recursive: true });

  for (const shot of SHOTS) {
    await page.locator("nav").getByRole("button", { name: texts[`ui.nav.${shot.page}`] }).click();
    if (shot.prepare) await shot.prepare(page);
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(out, shot.file) });
    console.log("saved", join(out, shot.file));
  }
} finally {
  await browser.close();
  server.close();
}
