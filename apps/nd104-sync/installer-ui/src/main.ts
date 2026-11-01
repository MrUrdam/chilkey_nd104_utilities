import "@fontsource/rajdhani/latin-500.css";
import "@fontsource/rajdhani/latin-600.css";
import "@fontsource/rajdhani/latin-700.css";
// same design tokens and controls as the settings window
import "../../ui/src/app.css";

import { mount } from "svelte";
import Setup from "./Setup.svelte";

// Browser-only demo with fake answers (`--mode demo`), never in the setup.
if (import.meta.env.MODE === "demo") {
  const { installDemo } = await import("./demo");
  installDemo(__APP_VERSION__);
}

mount(Setup, { target: document.getElementById("app")! });
