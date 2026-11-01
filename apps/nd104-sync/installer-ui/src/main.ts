import "@fontsource/rajdhani/latin-500.css";
import "@fontsource/rajdhani/latin-600.css";
import "@fontsource/rajdhani/latin-700.css";
// same design tokens and controls as the settings window
import "../../ui/src/app.css";

import { mount } from "svelte";
import Setup from "./Setup.svelte";

mount(Setup, { target: document.getElementById("app")! });
