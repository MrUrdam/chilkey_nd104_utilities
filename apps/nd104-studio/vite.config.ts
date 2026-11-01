import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The settings window lives in ui/; locale files are shared with the tray
// (../locales) and therefore outside the Vite root.
//
// `--mode demo` builds a browser-only version with sample data into
// ui/dist-demo (see ui/src/demo/demo.ts). It is used for the README
// screenshots and is never embedded into the app.
const appVersion = /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/.exec(
  readFileSync(new URL("Cargo.toml", import.meta.url), "utf8"),
)?.[1];

export default defineConfig(({ mode }) => ({
  root: "ui",
  plugins: [svelte()],
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(appVersion ?? "0.0.0"),
  },
  server: {
    port: 1420,
    strictPort: true,
    fs: { allow: [".."] },
  },
  build: {
    outDir: mode === "demo" ? "dist-demo" : "dist",
    emptyOutDir: true,
    target: "es2022",
  },
}));
