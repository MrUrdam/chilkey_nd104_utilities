<script lang="ts">
  // Debounce and sleep times of the keyboard. Like in Chilkey's web driver,
  // a change is sent and stored in the keyboard right away; afterwards the
  // values are read back.
  import { api, type KeyboardSetting, type KeyboardSettings, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let { status }: { status: Status } = $props();

  // the values Chilkey's web driver offers (same lists as the app's core)
  const range = (from: number, to: number, step = 1) =>
    Array.from({ length: Math.floor((to - from) / step) + 1 }, (_, i) => from + i * step);
  const DEBOUNCE_TIMES = [...range(1, 20), ...range(25, 50, 5), ...range(60, 100, 10), 200];
  const SLEEP_TIMES = [...range(1, 20), ...range(25, 50, 5), 60];
  const MODES = [0, 1, 2];

  let ready = $derived(status.device.kind === "ready");
  let values = $state<KeyboardSettings | null>(null);
  let loading = $state(false);
  let saving = $state<KeyboardSetting | null>(null);
  let saved = $state<KeyboardSetting | null>(null);
  let error = $state("");

  async function read() {
    loading = true;
    error = "";
    try {
      values = await api.keyboardSettingsRead();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // read once the keyboard is there (also after plugging it in later)
  $effect(() => {
    if (ready && !values && !loading && !error) read();
    if (!ready) values = null;
  });

  async function change(setting: KeyboardSetting, value: number) {
    saving = setting;
    saved = null;
    error = "";
    try {
      values = await api.keyboardSettingsWrite(setting, value);
      saved = setting;
      setTimeout(() => {
        if (saved === setting) saved = null;
      }, 1800);
    } catch (e) {
      error = String(e);
      // show what the keyboard really has
      await read();
    } finally {
      saving = null;
    }
  }

  let modeOff = $derived(values?.debounce_mode === 0);
</script>

{#snippet field(setting: KeyboardSetting, label: string, current: number | null | undefined, options: number[], text: (v: number) => string, disabled = false)}
  <label>
    <span class="label-row">
      {label}
      {#if saving === setting}<span class="muted small">…</span>
      {:else if saved === setting}<span class="saved small"><Icon name="check" size={13} />{t("ui.kbs.saved")}</span>{/if}
    </span>
    <select
      value={current ?? ""}
      disabled={!ready || !values || saving !== null || disabled}
      onchange={(e) => change(setting, Number(e.currentTarget.value))}
    >
      {#if current === null || current === undefined || !options.includes(current)}
        <option value="" disabled>{current ? text(current) : "–"}</option>
      {/if}
      {#each options as v (v)}<option value={v}>{text(v)}</option>{/each}
    </select>
  </label>
{/snippet}

<div class="card stack">
  <h2>{t("ui.kbs.title")}</h2>
  <p class="muted small">{t("ui.kbs.intro")}</p>

  {#if !ready}
    <div class="notice warn"><Icon name="plug" />{t("ui.light.not_connected")}</div>
  {/if}

  <div class="grid-2">
    {@render field("debounce_mode", t("ui.kbs.debounce_mode"), values?.debounce_mode, MODES, (v) => t(`ui.kbs.mode.${v}`))}
    {@render field(
      "debounce_time",
      t("ui.kbs.debounce_time"),
      modeOff ? null : values?.debounce_time,
      DEBOUNCE_TIMES,
      (v) => `${v} ms`,
      modeOff,
    )}
    {@render field("sleep_time", t("ui.kbs.sleep"), values?.sleep_time, SLEEP_TIMES, (v) => t("ui.kbs.minutes", { n: v }))}
    {@render field("deep_sleep_time", t("ui.kbs.deep_sleep"), values?.deep_sleep_time, SLEEP_TIMES, (v) =>
      t("ui.kbs.minutes", { n: v }),
    )}
  </div>

  <p class="muted small">{t("ui.kbs.hint")}</p>
  {#if error}<div class="notice err selectable"><Icon name="alert" /><span>{error}</span></div>{/if}
</div>

<style>
  .small {
    font-size: 12px;
  }

  p.small {
    margin: 0;
  }

  .label-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .saved {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--ok);
  }
</style>
