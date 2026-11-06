// The operating system ND104 Studio runs on ("windows" or "linux"), from the
// app info. Platform-specific *texts* are already resolved by the backend
// (`key@linux` variants); this is for what the UI shows or hides.

export type Platform = "windows" | "linux";

const state = $state<{ platform: Platform; secureStore: boolean }>({
  platform: "windows",
  secureStore: true,
});

export function setPlatform(platform: string) {
  state.platform = platform === "linux" ? "linux" : "windows";
}

/** Whether the OS credential store is reachable. On Linux `false` means the
 * Secret Service is not running, so API keys go to the 0600 fallback file. */
export function setSecureStore(available: boolean) {
  state.secureStore = available;
}

export function platform(): Platform {
  return state.platform;
}

export function secureStore(): boolean {
  return state.secureStore;
}
