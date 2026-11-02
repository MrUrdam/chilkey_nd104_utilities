<script lang="ts">
  // ND104 Studio's OpenRGB SDK server: lets OpenRGB, Artemis or Aurora
  // colour every LED. A section of the PC lighting card, shown only while PC
  // lighting software is allowed. Switched in the settings draft (saved with
  // the footer button); the status line is read from the running server.
  import { onDestroy } from "svelte";
  import { api, type Config, type OpenRgbStatus } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let { draft = $bindable(), errors }: { draft: Config; errors: Record<string, string> } = $props();

  let server = $state<OpenRgbStatus | null>(null);
  const refresh = () => api.openRgbStatus().then((s) => (server = s), () => (server = null));
  refresh();
  const timer = setInterval(refresh, 2000);
  onDestroy(() => clearInterval(timer));

  let line = $derived.by(() => {
    if (!server) return "";
    if (server.error) return t("ui.orgb.error", { port: server.port, error: server.error });
    if (!server.running) return t("ui.orgb.off");
    const key = server.offered ? "ui.orgb.running" : "ui.orgb.running_no_keyboard";
    return t(key, { port: server.port, n: server.clients });
  });
</script>

<div class="section stack">
  <div class="head">
    <div>
      <h3>{t("ui.orgb.title")}</h3>
      <span class="muted small">{t("ui.orgb.subtitle")}</span>
    </div>
    <label class="switch">
      <input type="checkbox" bind:checked={draft.device.openrgb_server} />{draft.device.openrgb_server
        ? t("ui.light.on")
        : t("ui.light.off")}
    </label>
  </div>
  <p class="muted small">{t("ui.orgb.explain")}</p>
  <ol class="muted small notes">
    <li>{t("ui.orgb.step1")}</li>
    <li>{t("ui.orgb.step2", { port: draft.device.openrgb_port })}</li>
    <li>{t("ui.orgb.step3")}</li>
  </ol>
  <label class="field narrow"
    >{t("ui.orgb.port")}
    <input
      type="number"
      min="1024"
      max="65535"
      step="1"
      bind:value={draft.device.openrgb_port}
      aria-invalid={!!errors["device.openrgb_port"]}
    />
    {#if errors["device.openrgb_port"]}<span class="error-text">{t(errors["device.openrgb_port"])}</span>{/if}
  </label>
  {#if line}
    <p class="small status" class:err={!!server?.error}>
      <span class="led {server?.error ? 'err' : server?.running ? (server.offered ? 'ok' : 'warn') : ''}"></span>{line}
    </p>
  {/if}
  <p class="faint small"><Icon name="info" size={14} /> {t("ui.orgb.note")}</p>
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  .section {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
  }

  .notes {
    margin: 0;
    padding-left: 18px;
  }

  p {
    margin: 0;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status.err {
    color: var(--err);
  }
</style>
