import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Window of the setup (nd104-sync-setup.exe). It shares the design system
// and components with the settings window in ui/.
//
// `--mode demo` builds a browser-only version with fake answers into
// installer-ui/dist-demo (see installer-ui/src/demo.ts); never embedded.
const appVersion = /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/.exec(
  readFileSync(new URL("Cargo.toml", import.meta.url), "utf8"),
)?.[1];

export default defineConfig(({ mode }) => ({
  root: "installer-ui",
  publicDir: "../ui/public",
  plugins: [svelte()],
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(appVersion ?? "0.0.0"),
  },
  server: {
    port: 1421,
    strictPort: true,
    fs: { allow: [".."] },
  },
  build: {
    outDir: mode === "demo" ? "dist-demo" : "dist",
    emptyOutDir: true,
    target: "es2022",
  },
}));
