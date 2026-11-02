// Bluetooth connection of the keyboard and its battery level, as Windows
// reports them. Refreshed regularly while the window is open.
import { api, type BluetoothKeyboard } from "./api";

const REFRESH_MS = 20_000;

const state = $state<{ keyboard: BluetoothKeyboard | null }>({ keyboard: null });
let timer: ReturnType<typeof setInterval> | null = null;

async function refresh() {
  try {
    state.keyboard = await api.bluetoothStatus();
  } catch {
    state.keyboard = null;
  }
}

/** Current Bluetooth state (reactive). */
export function bluetooth(): { keyboard: BluetoothKeyboard | null } {
  return state;
}

/** Starts refreshing; returns the stop function. */
export function watchBluetooth(): () => void {
  if (!timer) {
    refresh();
    timer = setInterval(refresh, REFRESH_MS);
  }
  return () => {
    if (timer) clearInterval(timer);
    timer = null;
  };
}
