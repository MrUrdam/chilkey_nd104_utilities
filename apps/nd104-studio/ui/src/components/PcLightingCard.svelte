<script lang="ts">
  // Whether PC lighting software (OpenRGB, SignalRGB) may take over the
  // keyboard's LEDs – read from the keyboard, a change is stored right away.
  // It is the gate for everything below: the OpenRGB server and the SignalRGB
  // plugin only work (and are only shown) while it is allowed.
  import { api, type Config, type SignalRgbStatus, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import OpenRgbCard from "./OpenRgbCard.svelte";

  let {
    status,
    draft = $bindable(),
    errors,
  }: { status: Status; draft: Config; errors: Record<string, string> } = $props();

  let ready = $derived(status.device.kind === "ready");
  /** 1: PC software may take over, 0: blocked, null: not read / not reported. */
  let allowed = $state<number | null>(null);
  let loading = $state(false);
  let saving = $state(false);
  let error = $state("");

  async function read() {
    loading = true;
    error = "";
    try {
      allowed = (await api.keyboardSettingsRead()).pc_lighting;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (ready && allowed === null && !loading && !error) read();
    if (!ready) allowed = null;
  });

  // --- SignalRGB plugin ---------------------------------------------------
  let plugin = $state<SignalRgbStatus | null>(null);
  let pluginBusy = $state(false);
  let pluginError = $state("");

  api.signalRgbStatus().then((s) => (plugin = s), (e) => (pluginError = String(e)));

  async function pluginAction(install: boolean) {
    pluginBusy = true;
    pluginError = "";
    try {
      plugin = install ? await api.signalRgbInstall() : await api.signalRgbRemove();
    } catch (e) {
      pluginError = String(e);
    } finally {
      pluginBusy = false;
    }
  }

  async function change(on: boolean) {
    saving = true;
    error = "";
    try {
      allowed = (await api.keyboardSettingsWrite("pc_lighting", on ? 1 : 0)).pc_lighting;
    } catch (e) {
      error = String(e);
      await read();
    } finally {
      saving = false;
    }
  }
</script>

<div class="card stack" class:disabled={!ready}>
  <div class="head">
    <div>
      <h2>{t("ui.pcl.title")}</h2>
      <span class="muted small">{t("ui.pcl.subtitle")}</span>
    </div>
    <label class="switch">
      <input
        type="checkbox"
        checked={allowed === 1}
        disabled={!ready || loading || saving || allowed === null}
        onchange={(e) => change(e.currentTarget.checked)}
      />{allowed === 1 ? t("ui.pcl.allowed") : t("ui.pcl.blocked")}
    </label>
  </div>
  <p class="muted small">{t("ui.pcl.explain")}</p>
  <ul class="muted small notes">
    <li>{t("ui.pcl.note_volatile")}</li>
    <li>{t("ui.pcl.note_ends")}</li>
    <li>{t("ui.pcl.note_key")}</li>
  </ul>
  {#if ready && allowed === null && !loading && !error}
    <p class="muted small">{t("ui.pcl.unknown")}</p>
  {/if}
  {#if error}<div class="notice err selectable"><Icon name="alert" />{error}</div>{/if}

  {#if allowed === 0}
    <p class="muted small">{t("ui.pcl.blocked_hint")}</p>
    {#if draft.device.openrgb_server}<p class="muted small">{t("ui.pcl.server_waits")}</p>{/if}
  {/if}

  {#if allowed === 1}
  <OpenRgbCard bind:draft {errors} />

  <div class="plugin stack">
    <div class="head">
      <div>
        <h3>{t("ui.pcl.srgb_title")}</h3>
        <span class="muted small">{t("ui.pcl.srgb_subtitle")}</span>
      </div>
      {#if plugin}
        <span class="badge {plugin.current ? 'ok' : plugin.installed ? 'warn' : ''}">
          {t(plugin.current ? "ui.pcl.srgb_installed" : plugin.installed ? "ui.pcl.srgb_outdated" : "ui.pcl.srgb_missing")}
        </span>
      {/if}
    </div>
    <ol class="muted small notes">
      <li>{t("ui.pcl.srgb_step1")}</li>
      <li>{t("ui.pcl.srgb_step2")}</li>
      <li>{t("ui.pcl.srgb_step3")}</li>
    </ol>
    {#if plugin && !plugin.signalrgb_found}
      <p class="muted small">{t("ui.pcl.srgb_not_found")}</p>
    {/if}
    <div class="row">
      <button class="key primary" disabled={pluginBusy || !plugin || plugin.current} onclick={() => pluginAction(true)}>
        <Icon name="download" />{t(plugin?.installed ? "ui.pcl.srgb_update" : "ui.pcl.srgb_install")}
      </button>
      {#if plugin?.installed}
        <button class="key" disabled={pluginBusy} onclick={() => pluginAction(false)}>
          <Icon name="trash" />{t("ui.pcl.srgb_remove")}
        </button>
      {/if}
    </div>
    {#if plugin}<p class="faint small selectable">{plugin.folder}</p>{/if}
    {#if pluginError}<div class="notice err selectable"><Icon name="alert" />{pluginError}</div>{/if}
  </div>
  {/if}
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  .head h2 {
    margin: 0;
  }

  .notes {
    margin: 0;
    padding-left: 18px;
  }

  .plugin {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
  }

  .row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  p {
    margin: 0;
  }
</style>
