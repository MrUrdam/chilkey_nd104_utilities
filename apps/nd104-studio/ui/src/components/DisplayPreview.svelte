<script lang="ts">
  // Dashboard preview of the keyboard display: what nd104-studio sent last.
  // The clock keeps running from the last sync, like the keyboard's own clock.
  import { onDestroy } from "svelte";
  import type { Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import DisplayScreen from "./DisplayScreen.svelte";

  let { status }: { status: Status } = $props();

  let now = $state(Date.now());
  const timer = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(timer));

  let clock = $derived.by(() => {
    if (!status.last_sent_time || !status.last_time_sync) return null;
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})/.exec(status.last_sent_time);
    if (!m) return null;
    const sent = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);
    return new Date(sent.getTime() + (now - new Date(status.last_time_sync).getTime()));
  });

  let connected = $derived(status.device.kind === "ready");
</script>

<DisplayScreen
  time={clock}
  weather={status.weather}
  dim={!connected}
  overlay={connected ? null : t("ui.preview.offline")}
  noWeather={t("ui.preview.no_weather")}
/>
