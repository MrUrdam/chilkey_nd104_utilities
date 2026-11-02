import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Mode = "install" | "update" | "uninstall";
export type Relation = "fresh" | "upgrade" | "same" | "downgrade";
export type Step = "preparing" | "closing" | "copying" | "finishing" | "removing" | "done";

export interface SetupInfo {
  mode: Mode;
  version: string;
  installed: string | null;
  relation: Relation;
  msi: boolean;
  payload: boolean;
  autostart: boolean;
  desktop_shortcut: boolean;
  /** Style stored in the app's settings (calm / gamer). */
  style: string;
}

export interface Progress {
  step: Step;
  value: number;
}

export type Texts = Record<string, string>;

export const setupInfo = () => invoke<SetupInfo>("setup_info");
export const texts = () => invoke<Texts>("texts");
export const runInstall = (autostart: boolean, desktopShortcut: boolean, style: string) =>
  invoke<void>("run_install", { options: { autostart, desktop_shortcut: desktopShortcut, style } });
export const runUninstall = (keepSettings: boolean) => invoke<void>("run_uninstall", { keepSettings });
export const launchApp = () => invoke<void>("launch_app");
/** How the keyboard is connected: "usb", "bluetooth", "wireless" (2.4 GHz) or not at all (null). */
export const keyboardConnected = () => invoke<"usb" | "bluetooth" | "wireless" | null>("keyboard_connected");
export const openLink = (which: "project" | "license" | "changelog") => invoke<void>("open_link", { which });
export const onProgress = (handler: (p: Progress) => void): Promise<UnlistenFn> =>
  listen<Progress>("progress", (e) => handler(e.payload));
