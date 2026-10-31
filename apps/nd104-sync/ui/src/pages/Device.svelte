<script lang="ts">
  import { api, type AppInfo, type Config, type FirmwareInfo, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";

  let {
    status,
    info,
    draft = $bindable(),
    errors,
  }: { status: Status; info: AppInfo; draft: Config; errors: Record<string, string> } = $props();

  let busy = $state(false);
  let model = $derived(info.models[0]);
  let firmware = $derived<FirmwareInfo[]>(status.device.kind === "not_connected" ? [] : status.device.firmware);

  function verified(f: FirmwareInfo): boolean {
    return model.verified_firmware.some((v) => v.role === f.role && v.version === f.version);
  }

  async function recheck() {
    busy = true;
    try {
      await api.engine("recheck");
    } finally {
      setTimeout(() => (busy = false), 800);
    }
  }
</script>

<div class="stack">
  <h1>{t("ui.nav.device")}</h1>

  <div class="notice">🔌 {t("ui.device.wired_notice")}</div>

  <div class="card stack">
    <div class="head">
      <h2>{t("ui.dash.keyboard")}</h2>
      <button class="key" onclick={recheck} disabled={busy}>{t("menu.recheck")}</button>
    </div>

    {#if status.device.kind === "ready"}
      <div class="state">
        <span class="led ok"></span><strong>{status.device.model_name}</strong>
        <span class="badge {status.device.trust === 'verified' ? 'ok' : 'warn'}">
          {t(status.device.trust === "verified" ? "status.verified" : "status.compatible")}
        </span>
      </div>
      <p class="muted">
        {t(status.device.trust === "verified" ? "ui.device.verified_text" : "ui.device.compatible_text")}
      </p>
      <dl class="kv">
        <dt>{t("ui.device.variant")}</dt>
        <dd>{t(`ui.variant.${status.device.screen_variant}`)}</dd>
      </dl>
    {:else if status.device.kind === "locked"}
      <div class="state"><span class="led warn"></span><strong>{t("ui.device.locked")}</strong></div>
      <div class="notice warn selectable">{status.device.reason}</div>
      <p class="muted">{t("ui.device.locked_text")}</p>
    {:else}
      <div class="state"><span class="led"></span><strong>{t("status.not_connected")}</strong></div>
      <p class="muted">{t("ui.dash.connect_hint")}</p>
    {/if}
  </div>

  <div class="card">
    <h2>{t("ui.device.firmware")}</h2>
    <table>
      <thead>
        <tr>
          <th>{t("ui.device.part")}</th>
          <th>{t("ui.device.installed")}</th>
          <th>{t("ui.device.verified_versions")}</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each ["keyboard", "screen"] as role (role)}
          {@const f = firmware.find((x) => x.role === role)}
          <tr>
            <td>{t(`ui.role.${role}`)}</td>
            <td class="mono">{f?.raw ?? "–"}</td>
            <td class="mono">
              {model.verified_firmware
                .filter((v) => v.role === role)
                .map((v) => "V" + v.version)
                .join(", ")}
            </td>
            <td>
              {#if f}
                <span class="badge {verified(f) ? 'ok' : 'warn'}">{t(verified(f) ? "ui.device.ok" : "ui.device.unverified")}</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="muted small">{t("ui.device.firmware_update_hint")}</p>
  </div>

  <div class="card stack">
    <h2>{t("ui.device.advanced")}</h2>
    <label class="field narrow"
      >{t("ui.device.poll")}
      <input type="number" min="1" max="60" step="1" bind:value={draft.device.poll_interval_s} aria-invalid={!!errors["device.poll"]} />
      {#if errors["device.poll"]}<span class="error-text">{t(errors["device.poll"])}</span>{/if}
    </label>
  </div>
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .head h2 {
    margin: 0;
  }

  .state {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 15px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    text-align: left;
    color: var(--muted);
    font-weight: 600;
    border-bottom: 1px solid var(--border);
    padding: 6px 8px;
  }

  td {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    user-select: text;
  }

  .narrow {
    max-width: 260px;
  }

  .small {
    font-size: 12px;
    margin: 10px 0 0;
  }

  p {
    margin: 0;
  }
</style>
