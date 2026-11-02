<script lang="ts">
  // Preview of the keyboard display. Connected by cable: what ND104 Studio
  // sent last, with the clock running on from the last sync like the
  // keyboard's own clock. Otherwise: the time as it would be sent (clock
  // settings) and the latest weather – dimmed only without any connection.
  import { onDestroy } from "svelte";
  import { api, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import DisplayScreen from "./DisplayScreen.svelte";
  import { bluetooth } from "../lib/bluetooth.svelte";

  const bt = bluetooth();

  let { status }: { status: Status } = $props();

  /** How often the planned time is asked for again (the clock runs on in between). */
  const PLAN_MS = 60_000;

  let now = $state(Date.now());
  const timer = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(timer));

  let connected = $derived(status.device.kind === "ready");

  /** "2026-11-02T18:42:00" (local wall clock) → Date. */
  function wallClock(text: string | null | undefined): Date | null {
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})/.exec(text ?? "");
    return m ? new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]) : null;
  }

  // the time ND104 Studio would send now, with the configured clock source
  let planned = $state<{ at: Date; fetched: number } | null>(null);

  async function plan() {
    try {
      const config = await api.config();
      const at = wallClock((await api.previewTime(config.clock)).local);
      planned = at ? { at, fetched: Date.now() } : null;
    } catch {
      planned = null;
    }
  }

  $effect(() => {
    if (connected) return;
    plan();
    const id = setInterval(plan, PLAN_MS);
    return () => clearInterval(id);
  });

  let clock = $derived.by(() => {
    if (connected) {
      const sent = wallClock(status.last_sent_time);
      if (!sent || !status.last_time_sync) return null;
      return new Date(sent.getTime() + (now - new Date(status.last_time_sync).getTime()));
    }
    return planned ? new Date(planned.at.getTime() + (now - planned.fetched)) : null;
  });
</script>

<DisplayScreen
  time={clock}
  weather={status.weather}
  dim={!connected && !bt.keyboard}
  battery={bt.keyboard?.battery ?? null}
  noWeather={t("ui.preview.no_weather")}
/>
