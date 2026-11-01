import "@fontsource/rajdhani/latin-500.css";
import "@fontsource/rajdhani/latin-600.css";
import "@fontsource/rajdhani/latin-700.css";
import "./app.css";

import { mount } from "svelte";
import App from "./App.svelte";

// Browser-only demo with sample data (`vite build --mode demo`, used for the
// README screenshots). Removed from normal builds.
if (import.meta.env.MODE === "demo") {
  const { installDemo } = await import("./demo/demo");
  installDemo(__APP_VERSION__);
}

mount(App, { target: document.getElementById("app")! });
