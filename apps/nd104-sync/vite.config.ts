import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The settings window lives in ui/; locale files are shared with the tray
// (../locales) and therefore outside the Vite root.
export default defineConfig({
  root: "ui",
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    fs: { allow: [".."] },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
  },
});
