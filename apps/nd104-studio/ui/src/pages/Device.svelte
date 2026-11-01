<script lang="ts">
  import { api, type AppInfo, type Config, type FirmwareInfo, type FirmwarePart, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { dateTime } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import KeyboardSettingsCard from "../components/KeyboardSettingsCard.svelte";

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

  let report = $derived(status.firmware_update && "Ok" in status.firmware_update ? status.firmware_update.Ok : null);
  let reportError = $derived(status.firmware_update && "Err" in status.firmware_update ? status.firmware_update.Err : null);
  let checking = $state(false);

  function part(role: string): FirmwarePart | undefined {
    return report?.parts.find((p) => p.role === role);
  }

  const stateBadge = { up_to_date: "ok", update_available: "warn", newer: "", unknown: "" } as const;

  async function checkFirmware() {
    checking = true;
    try {
      await api.engine("check_updates");
    } finally {
      setTimeout(() => (checking = false), 1500);
    }
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

  <div class="notice"><Icon name="plug" />{t("ui.device.wired_notice")}</div>

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
          <th>{t("ui.fw.latest")}</th>
          <th>{t("ui.fw.state")}</th>
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
            {#if part(role)}
              {@const p = part(role)!}
              <td class="mono small-cell" title={p.latest?.file ?? ""}>
                {p.latest ? p.latest.build_date : "–"}
                {#if p.label}<div class="muted">{p.label}</div>{/if}
                {#if p.source}
                  <div class="source {p.source}" title={p.official_error ?? ""}>
                    {t(p.source === "official" ? "ui.fw.source_official" : "ui.fw.source_repository")}
                  </div>
                {/if}
              </td>
              <td>
                <span class="badge {stateBadge[p.state]}">{t(`ui.fw.${p.state}`)}</span>
                {#if p.state === "update_available" && p.download_url}
                  {@const url = p.download_url}
                  <div><button type="button" class="link" onclick={() => api.open(url)}>{t("ui.fw.download")}</button></div>
                {/if}
              </td>
            {:else}
              <td>–</td>
              <td></td>
            {/if}
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="row fw-actions">
      <button class="key" onclick={checkFirmware} disabled={checking}>{checking ? "…" : t("ui.fw.check")}</button>
      {#if report}
        {@const url = report.instructions_url}
        <span class="muted small">{t("ui.fw.checked", { time: dateTime(status.firmware_update_checked) })}</span>
        <button type="button" class="link" onclick={() => api.open(url)}>{t("ui.fw.instructions")}</button>
      {:else if reportError}
        <span class="error-text selectable">{reportError}</span>
      {:else}
        <span class="muted small">{t("ui.fw.not_checked")}</span>
      {/if}
    </div>
    {#if report?.parts.some((p) => p.source === "repository")}
      <p class="muted small">
        {t("ui.fw.fallback_note", { date: report.list_updated ?? "–" })}
      </p>
    {/if}
    {#if report?.parts.some((p) => p.state === "unknown" && p.role === "screen")}
      <p class="muted small">{t("ui.fw.unknown_hint")}</p>
    {/if}
    <p class="muted small">{t("ui.device.firmware_update_hint")}</p>
    <div class="notice">{t("ui.fw.never_flash")}</div>
  </div>

  <KeyboardSettingsCard {status} />

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

  .fw-actions {
    margin-top: 12px;
  }

  .small-cell {
    font-size: 12px;
  }

  .source {
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 600;
  }

  .source.official {
    color: var(--ok);
  }

  .source.repository {
    color: var(--warn);
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
