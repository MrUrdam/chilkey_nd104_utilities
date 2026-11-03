// The operating system ND104 Studio runs on ("windows" or "linux"), from the
// app info. Platform-specific *texts* are already resolved by the backend
// (`key@linux` variants); this is for what the UI shows or hides.

export type Platform = "windows" | "linux";

const state = $state<{ platform: Platform }>({ platform: "windows" });

export function setPlatform(platform: string) {
  state.platform = platform === "linux" ? "linux" : "windows";
}

export function platform(): Platform {
  return state.platform;
}
