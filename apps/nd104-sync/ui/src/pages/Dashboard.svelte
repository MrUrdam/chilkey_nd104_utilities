<script lang="ts">
  // Display replica as the hero; the facts that matter right next to it.
  import { api, type Config, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { condition, dateTime, temperature, time } from "../lib/format";
  import DisplayPreview from "../components/DisplayPreview.svelte";
  import Icon from "../components/Icon.svelte";

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

  const levelLed = { info: "", warning: "warn", error: "err" } as const;
</script>

<div class="stack page">
  <div class="head">
    <h1>{t("ui.nav.dashboard")}</h1>
    <div class="row">
      <button class="key" disabled={busy} onclick={() => run(status.paused ? "resume" : "pause")}>
        <Icon name={status.paused ? "play" : "pause"} />{t(status.paused ? "menu.resume" : "menu.pause")}
      </button>
      <button class="key primary" disabled={busy || status.device.kind !== "ready"} onclick={() => run("sync_now")}>
        <Icon name="sync" />{t("menu.sync_now")}
      </button>
    </div>
  </div>

  <section class="hero">
    <figure class="replica">
      <DisplayPreview {status} />
      <figcaption>
        <span>{t("ui.preview.caption")}</span>
        <span class="faint">{t("ui.preview.battery_hint")}</span>
      </figcaption>
    </figure>

    <div class="facts card">
      <div class="fact">
        <span class="label">{t("ui.dash.keyboard")}</span>
        {#if status.device.kind === "ready"}
          <span class="value"><span class="led ok"></span>{status.device.model_name}</span>
          <span class="badge {status.device.trust === 'verified' ? 'ok' : 'warn'}">
            {t(status.device.trust === "verified" ? "status.verified" : "status.compatible")}
          </span>
        {:else if status.device.kind === "locked"}
          <span class="value"><span class="led warn"></span>{t("ui.device.locked")}</span>
          <button class="key" disabled={busy} onclick={() => run("recheck")}>{t("menu.recheck")}</button>
        {:else}
          <span class="value"><span class="led"></span>{t("status.not_connected")}</span>
          <span class="faint small">{t("ui.dash.connect_hint")}</span>
        {/if}
      </div>

      {#if status.paused}
        <div class="notice warn"><Icon name="pause" />{t("ui.dash.paused")}</div>
      {/if}

      <div class="fact">
        <span class="label">{t("ui.dash.last_time")}</span>
        <span class="value big">{time(status.last_time_sync, true)}</span>
        <span class="faint small">{t("ui.dash.next_at", { time: time(status.next_time_sync) })}</span>
      </div>

      <div class="fact">
        <span class="label">{t("ui.dash.last_weather")}</span>
        <span class="value big">{time(status.last_weather_push, true)}</span>
        <span class="faint small">{t("ui.dash.next_at", { time: time(status.next_weather_fetch) })}</span>
      </div>

      <div class="fact">
        <span class="label">{t("ui.dash.next_dst")}</span>
        <span class="value">{dateTime(status.next_dst_change)}</span>
      </div>
    </div>
  </section>

  {#if status.device.kind === "locked"}
    <div class="notice warn selectable"><Icon name="alert" />{status.device.reason}</div>
  {/if}

  <div class="grid-2">
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
        <div class="notice {status.weather_stale ? 'err' : 'warn'} selectable">
          <Icon name="alert" />{status.weather_error}
        </div>
      {/if}
    </div>

    <div class="card">
      <h2>{t("ui.dash.firmware")}</h2>
      {#if status.device.kind !== "not_connected" && status.device.firmware.length}
        <dl class="kv">
          {#each status.device.firmware as f (f.role)}
            <dt>{t(`ui.role.${f.role}`)}</dt>
            <dd class="mono small">{f.raw}</dd>
          {/each}
        </dl>
      {:else}
        <p class="muted">{t("ui.dash.connect_hint")}</p>
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
            <span class="mono faint">{time(e.at, true)}</span>
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

  .small {
    font-size: 12px;
  }

  .hero {
    display: grid;
    grid-template-columns: minmax(0, 1.65fr) minmax(220px, 1fr);
    gap: 24px;
    align-items: stretch;
  }

  .replica {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  figcaption {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--muted);
    padding: 0 4px;
  }

  .facts {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px;
  }

  .fact {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }

  .fact + .fact {
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .label {
    font-size: 12px;
    color: var(--muted);
  }

  .value {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 500;
  }

  .value.big {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }

  :global(:root[data-style="gamer"]) .value.big {
    font-size: 28px;
    font-weight: 600;
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

  @media (max-width: 900px) {
    .hero,
    .grid-2 {
      grid-template-columns: 1fr;
    }
  }
</style>
