// Demo mode: the setup window runs in a plain browser with fake answers
// (`vite build --config vite.installer.config.ts --mode demo`). Nothing is
// installed. Only included in demo builds.
//
// URL parameters: ?lang=en|de  ?theme=dark|light  ?style=calm|gamer
//                 ?mode=install|update|uninstall  ?relation=fresh|upgrade|same|downgrade

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import en from "../../installer/locales/en.json";
import de from "../../installer/locales/de.json";

const params = new URLSearchParams(location.search);
const lang = params.get("lang") === "de" ? "de" : "en";
const theme = params.get("theme");
const mode = params.get("mode") ?? "install";
const relation = params.get("relation") ?? (mode === "install" ? "fresh" : "upgrade");

export function installDemo(version: string) {
  if (theme === "dark" || theme === "light") document.documentElement.dataset.theme = theme;
  mockWindows("main");

  const progress = async (steps: [string, number][]) => {
    for (const [step, value] of steps) {
      await new Promise((r) => setTimeout(r, 450));
      await emit("progress", { step, value });
    }
  };

  mockIPC(
    async (cmd) => {
      switch (cmd) {
        case "texts":
          return { ...(lang === "de" ? de : en), lang };
        case "setup_info":
          return {
            mode,
            version,
            installed: relation === "fresh" ? null : relation === "downgrade" ? "9.9.9" : "1.2.0",
            relation,
            msi: false,
            payload: true,
            autostart: true,
            desktop_shortcut: false,
            style: params.get("style") === "gamer" ? "gamer" : "calm",
          };
        case "run_install":
          await progress([
            ["closing", 0.05],
            ["copying", 0.3],
            ["copying", 0.6],
            ["copying", 0.8],
            ["finishing", 0.92],
            ["done", 1],
          ]);
          return null;
        case "run_uninstall":
          await progress([
            ["closing", 0.2],
            ["removing", 0.6],
            ["done", 1],
          ]);
          return null;
        case "keyboard_connected":
          return true;
        default:
          // launch_app, open_link, window commands …
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
