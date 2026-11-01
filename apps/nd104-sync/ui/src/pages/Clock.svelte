<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { api, type ClockSource, type Config, type DstMode, type TimePreview } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { civil, dateTime } from "../lib/format";

  let { draft = $bindable(), errors }: { draft: Config; errors: Record<string, string> } = $props();

  const sources: ClockSource[] = ["system", "timezone", "ntp", "manual"];
  const dstModes: DstMode[] = ["auto", "force_summer", "force_winter"];

  let zones = $state<string[]>([]);
  let preview = $state<TimePreview | null>(null);
  let previewError = $state<string | null>(null);
  let ntpResult = $state<string | null>(null);
  let ntpError = $state<string | null>(null);
  let ntpTesting = $state(false);

  let c = $derived(draft.clock);
  let needsZone = $derived(c.source === "timezone" || c.source === "ntp");

  async function refreshPreview() {
    try {
      preview = await api.previewTime($state.snapshot(draft.clock) as Config["clock"]);
      previewError = null;
    } catch (e) {
      preview = null;
      previewError = String(e);
    }
  }

  // Preview follows the draft immediately and ticks every second.
  $effect(() => {
    JSON.stringify(draft.clock);
    refreshPreview();
  });
  const timer = setInterval(refreshPreview, 1000);
  onDestroy(() => clearInterval(timer));
  onMount(async () => (zones = await api.timezones()));

  async function testNtp() {
    ntpTesting = true;
    ntpResult = null;
    ntpError = null;
    try {
      const ms = await api.testNtp(draft.clock.ntp_server);
      ntpResult = t("ui.clock.ntp_ok", { ms: (ms >= 0 ? "+" : "") + ms });
    } catch (e) {
      ntpError = String(e);
    } finally {
      ntpTesting = false;
    }
  }

  let shown = $derived(civil(preview?.local));
</script>

<div class="stack">
  <h1>{t("ui.nav.clock")}</h1>

  <div class="card preview-card">
    <div>
      <h2>{t("ui.clock.preview")}</h2>
      {#if shown}
        <div class="big">{preview?.local.slice(11, 19)}</div>
        <div class="muted">{shown.weekday} · {shown.date}</div>
      {:else if previewError}
        <div class="error-text selectable">{previewError}</div>
      {/if}
    </div>
    <dl class="kv">
      <dt>{t("ui.dash.next_dst")}</dt>
      <dd>{preview?.next_dst_change ? dateTime(preview.next_dst_change) : t("ui.clock.no_dst")}</dd>
    </dl>
  </div>

  <div class="card stack">
    <h2>{t("ui.clock.source")}</h2>
    <div class="sources">
      {#each sources as s (s)}
        <button class="source" aria-pressed={c.source === s} onclick={() => (draft.clock.source = s)}>
          <strong>{t(`ui.clock.src_${s}`)}</strong>
          <span class="muted">{t(`ui.clock.src_${s}_desc`)}</span>
        </button>
      {/each}
    </div>

    {#if needsZone}
      <label class="field"
        >{t("ui.clock.timezone")}
        <input
          list="tz-list"
          bind:value={draft.clock.timezone}
          placeholder="Europe/Berlin"
          aria-invalid={!!errors["clock.timezone"]}
        />
        <datalist id="tz-list">
          {#each zones as z (z)}<option value={z}></option>{/each}
        </datalist>
        {#if errors["clock.timezone"]}<span class="error-text">{t(errors["clock.timezone"])}</span>{/if}
      </label>
    {/if}

    {#if c.source === "ntp"}
      <div class="row end">
        <label class="field grow"
          >{t("ui.clock.ntp_server")}
          <input bind:value={draft.clock.ntp_server} aria-invalid={!!errors["clock.ntp_server"]} />
        </label>
        <button class="key" onclick={testNtp} disabled={ntpTesting || !c.ntp_server.trim()}>
          {ntpTesting ? "…" : t("ui.clock.ntp_test")}
        </button>
      </div>
      {#if ntpResult}<div class="notice">{ntpResult}</div>{/if}
      {#if ntpError}<div class="notice err selectable">{ntpError}</div>{/if}
    {/if}

    {#if c.source === "manual"}
      <label class="field"
        >{t("ui.clock.offset")}
        <input type="number" step="1" bind:value={draft.clock.manual_offset_min} aria-invalid={!!errors["clock.offset"]} />
        {#if errors["clock.offset"]}<span class="error-text">{t(errors["clock.offset"])}</span>{/if}
      </label>
    {/if}
  </div>

  <div class="card stack">
    <h2>{t("ui.clock.dst")}</h2>
    <div class="segmented">
      {#each dstModes as m (m)}
        <button aria-pressed={c.dst_mode === m} onclick={() => (draft.clock.dst_mode = m)}>{t(`ui.clock.dst_${m}`)}</button>
      {/each}
    </div>
    <p class="muted small">{t("ui.clock.dst_hint")}</p>

    <label class="field narrow"
      >{t("ui.clock.interval")}
      <input type="number" min="1" step="1" bind:value={draft.clock.sync_interval_min} aria-invalid={!!errors["clock.interval"]} />
    </label>
    <p class="muted small">{t("ui.clock.format_hint")}</p>
  </div>
</div>

<style>
  .preview-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
  }

  .big {
    font-family: var(--font-display);
    font-size: 44px;
    font-weight: 500;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .sources {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }

  .source {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    font: 13px var(--font-ui);
    color: var(--text);
    background: var(--panel-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    cursor: pointer;
    box-shadow: 0 2px 0 var(--key-edge);
  }

  .source strong {
    font-size: 14px;
  }

  .source[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .end {
    align-items: flex-end;
  }

  .grow {
    flex: 1;
  }

  .narrow {
    max-width: 260px;
  }

  .small {
    font-size: 12px;
    margin: 0;
  }

  @media (max-width: 960px) {
    .sources {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
