<script lang="ts">
  import { api, type Config, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { condition, dateTime, temperature, time } from "../lib/format";
  import DisplayPreview from "../components/DisplayPreview.svelte";

  let { status, config }: { status: Status; config: Config } = $props();
  let unit = $derived(config.weather.ui_unit);
  let busy = $state(false);

  async function run(action: "sync_now" | "pause" | "resume" | "recheck") {
    busy = true;
    try {
      await api.engine(action);
    } finally {
      setTimeout(() => (busy = false), 600);
    }
  }

  const levelLed = { info: "accent", warning: "warn", error: "err" } as const;
</script>

<div class="stack">
  <div class="head">
    <h1>{t("ui.nav.dashboard")}</h1>
    <div class="row">
      <button class="key" disabled={busy} onclick={() => run(status.paused ? "resume" : "pause")}>
        {t(status.paused ? "menu.resume" : "menu.pause")}
      </button>
      <button class="key primary" disabled={busy || status.device.kind !== "ready"} onclick={() => run("sync_now")}>
        ⟳ {t("menu.sync_now")}
      </button>
    </div>
  </div>

  <div class="top">
    <div class="preview">
      <DisplayPreview {status} />
      <div class="caption muted">{t("ui.preview.caption")}</div>
    </div>

    <div class="card device">
      <h2>{t("ui.dash.keyboard")}</h2>
      {#if status.device.kind === "ready"}
        <div class="state"><span class="led ok"></span><strong>{status.device.model_name}</strong></div>
        <span class="badge {status.device.trust === 'verified' ? 'ok' : 'warn'}">
          {t(status.device.trust === "verified" ? "status.verified" : "status.compatible")}
        </span>
        <dl class="kv">
          {#each status.device.firmware as f (f.role)}
            <dt>{t(`ui.role.${f.role}`)}</dt>
            <dd class="mono">{f.raw}</dd>
          {/each}
        </dl>
      {:else if status.device.kind === "locked"}
        <div class="state"><span class="led warn"></span><strong>{t("ui.device.locked")}</strong></div>
        <div class="notice warn selectable">{status.device.reason}</div>
        <button class="key" disabled={busy} onclick={() => run("recheck")}>{t("menu.recheck")}</button>
      {:else}
        <div class="state"><span class="led"></span><strong>{t("status.not_connected")}</strong></div>
        <p class="muted">{t("ui.dash.connect_hint")}</p>
      {/if}
    </div>
  </div>

  <div class="grid-2">
    <div class="card">
      <h2>{t("ui.dash.sync")}</h2>
      {#if status.paused}<div class="notice warn">{t("ui.dash.paused")}</div>{/if}
      <dl class="kv">
        <dt>{t("ui.dash.last_time")}</dt>
        <dd>{time(status.last_time_sync, true)}</dd>
        <dt>{t("ui.dash.next_time")}</dt>
        <dd>{time(status.next_time_sync)}</dd>
        <dt>{t("ui.dash.next_dst")}</dt>
        <dd>{dateTime(status.next_dst_change)}</dd>
        <dt>{t("ui.dash.last_weather")}</dt>
        <dd>{time(status.last_weather_push, true)}</dd>
        <dt>{t("ui.dash.next_weather")}</dt>
        <dd>{time(status.next_weather_fetch)}</dd>
      </dl>
    </div>

    <div class="card">
      <h2>{t("ui.dash.weather")}</h2>
      {#if status.weather}
        <dl class="kv">
          <dt>{t("ui.weather.location")}</dt>
          <dd>{config.weather.location.name || "–"}</dd>
          <dt>{t("ui.dash.condition")}</dt>
          <dd>{condition(status.weather.condition)} ({t(status.weather.is_day ? "ui.day" : "ui.night")})</dd>
          <dt>{t("ui.dash.now")}</dt>
          <dd>{temperature(status.weather.current_c, unit)}</dd>
          <dt>{t("ui.dash.max_min")}</dt>
          <dd>{temperature(status.weather.max_c, unit)} / {temperature(status.weather.min_c, unit)}</dd>
          <dt>{t("ui.weather.provider")}</dt>
          <dd>{status.weather.provider}</dd>
        </dl>
      {:else}
        <p class="muted">{t("ui.preview.no_weather")}</p>
      {/if}
      {#if status.weather_error}
        <div class="notice {status.weather_stale ? 'err' : 'warn'} selectable">{status.weather_error}</div>
      {/if}
    </div>
  </div>

  <div class="card">
    <h2>{t("ui.dash.events")}</h2>
    {#if status.events.length === 0}
      <p class="muted">{t("ui.dash.no_events")}</p>
    {:else}
      <ul class="events">
        {#each status.events.slice(0, 30) as e, i (e.at + i)}
          <li>
            <span class="led {levelLed[e.level]}"></span>
            <span class="mono muted">{time(e.at, true)}</span>
            <span class="selectable">{e.message}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .top {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: 14px;
    align-items: start;
  }

  .preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .caption {
    font-size: 12px;
  }

  .device {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
  }

  .state {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
  }

  .events {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    max-height: 220px;
    overflow: auto;
  }

  .events li {
    display: grid;
    grid-template-columns: 12px 72px 1fr;
    gap: 8px;
    align-items: baseline;
  }

  @media (max-width: 960px) {
    .top,
    .grid-2 {
      grid-template-columns: 1fr;
    }
  }
</style>
