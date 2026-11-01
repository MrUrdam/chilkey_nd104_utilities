import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Window of the setup (nd104-sync-setup.exe). It shares the design system
// and components with the settings window in ui/.
export default defineConfig({
  root: "installer-ui",
  publicDir: "../ui/public",
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    fs: { allow: [".."] },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
  },
});
