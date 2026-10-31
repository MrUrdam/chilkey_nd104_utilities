<script lang="ts">
  // Mock-up of the 320×172 keyboard display: what nd104-sync sent last.
  // The clock keeps running from the last sync, like the keyboard's own clock.
  import { onDestroy } from "svelte";
  import type { Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import WeatherIcon from "./WeatherIcon.svelte";

  let { status }: { status: Status } = $props();

  let now = $state(Date.now());
  const timer = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(timer));

  let clock = $derived.by(() => {
    if (!status.last_sent_time || !status.last_time_sync) return null;
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})/.exec(status.last_sent_time);
    if (!m) return null;
    const sent = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);
    const shown = new Date(sent.getTime() + (now - new Date(status.last_time_sync).getTime()));
    return shown;
  });

  const pad = (n: number) => String(n).padStart(2, "0");
  let connected = $derived(status.device.kind === "ready");
</script>

<div class="bezel">
  <div class="screen" class:dim={!connected}>
    {#if clock}
      <div class="left">
        <div class="time">{pad(clock.getHours())}<span class="colon">:</span>{pad(clock.getMinutes())}</div>
        <div class="date">
          {clock.toLocaleDateString(undefined, { weekday: "short" })}
          · {pad(clock.getDate())}.{pad(clock.getMonth() + 1)}.{clock.getFullYear()}
        </div>
      </div>
    {:else}
      <div class="left placeholder">--:--</div>
    {/if}

    <div class="right">
      {#if status.weather}
        <WeatherIcon condition={status.weather.condition} isDay={status.weather.is_day} size={70} />
        <div class="temp">{status.weather.current_c.toFixed(1)}°</div>
        <div class="minmax">
          <span>↑ {status.weather.max_c.toFixed(1)}°</span>
          <span>↓ {status.weather.min_c.toFixed(1)}°</span>
        </div>
      {:else}
        <div class="placeholder small">{t("ui.preview.no_weather")}</div>
      {/if}
    </div>

    {#if !connected}
      <div class="overlay">{t("ui.preview.offline")}</div>
    {/if}
  </div>
</div>

<style>
  .bezel {
    background: linear-gradient(180deg, #2b313b, #14171c);
    border: 1px solid #3b4350;
    border-radius: 18px;
    padding: 14px;
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.35),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
    width: 100%;
    max-width: 500px;
  }

  .screen {
    position: relative;
    aspect-ratio: 320 / 172;
    background: radial-gradient(120% 120% at 20% 0%, #0f1a22 0%, var(--screen-bg) 60%);
    border-radius: 8px;
    border: 1px solid #000;
    display: grid;
    grid-template-columns: 1.25fr 1fr;
    align-items: center;
    padding: 0 6%;
    color: #f8fafc;
    font-family: var(--font-display);
    overflow: hidden;
    box-shadow: inset 0 0 30px rgba(34, 211, 238, 0.08);
  }

  .screen.dim > :not(.overlay) {
    opacity: 0.35;
  }

  .time {
    font-size: clamp(40px, 7.5vw, 64px);
    font-weight: 700;
    line-height: 1;
    color: #e0faff;
    text-shadow: 0 0 18px rgba(34, 211, 238, 0.55);
    letter-spacing: 0.02em;
  }

  .colon {
    animation: blink 1s steps(1) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }

  .date {
    margin-top: 6px;
    font-size: clamp(13px, 1.8vw, 17px);
    color: #8be9f7;
    font-weight: 500;
  }

  .right {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .temp {
    font-size: clamp(24px, 4vw, 34px);
    font-weight: 700;
  }

  .minmax {
    display: flex;
    gap: 10px;
    font-size: 14px;
    color: #94a3b8;
  }

  .placeholder {
    font-size: 48px;
    color: #334155;
    font-weight: 700;
  }

  .placeholder.small {
    font-size: 14px;
    font-weight: 500;
    text-align: center;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 600;
    color: #cbd5e1;
    background: rgba(0, 0, 0, 0.35);
  }
</style>
