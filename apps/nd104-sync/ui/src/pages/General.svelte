<script lang="ts">
  import { api, type AppInfo, type Config, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { dateTime } from "../lib/format";

  let {
    draft = $bindable(),
    info,
    status,
    errors,
  }: { draft: Config; info: AppInfo; status: Status; errors: Record<string, string> } = $props();

  let checking = $state(false);
  const themes = ["auto", "dark", "light"] as const;

  async function checkUpdates() {
    checking = true;
    try {
      await api.engine("check_updates");
    } finally {
      setTimeout(() => (checking = false), 1500);
    }
  }

  let update = $derived(status.app_update);
</script>

<div class="stack">
  <h1>{t("ui.nav.general")}</h1>

  <div class="card stack">
    <h2>{t("ui.general.app")}</h2>
    <label class="switch"><input type="checkbox" bind:checked={draft.general.autostart} />{t("menu.autostart")}</label>

    <div class="grid-2">
      <label class="field"
        >{t("menu.language")}
        <select bind:value={draft.general.language}>
          <option value="auto">{t("menu.language_auto")}</option>
          {#each info.languages as [code, name] (code)}<option value={code}>{name}</option>{/each}
        </select>
      </label>
      <div class="field">
        <span class="label">{t("ui.general.theme")}</span>
        <div class="segmented">
          {#each themes as th (th)}
            <button aria-pressed={draft.general.theme === th} onclick={() => (draft.general.theme = th)}
              >{t(`ui.general.theme_${th}`)}</button
            >
          {/each}
        </div>
      </div>
    </div>
  </div>

  <div class="card stack">
    <h2>{t("ui.general.updates")}</h2>
    <div class="row">
      <button class="key primary" onclick={checkUpdates} disabled={checking}>
        {checking ? "…" : t("menu.check_updates")}
      </button>
      {#if update}
        {#if "Err" in update}
          <span class="error-text selectable">{update.Err}</span>
        {:else if update.Ok.kind === "available"}
          <span class="badge warn">{t("ui.general.update_available", { version: update.Ok.version })}</span>
          {@const url = update.Ok.url}
          <button type="button" class="link" onclick={() => api.open(url)}>{t("ui.general.open_release")}</button>
        {:else}
          <span class="badge ok">{t("ui.general.up_to_date")}</span>
        {/if}
        <span class="muted small">{t("ui.general.last_check", { time: dateTime(status.app_update_checked) })}</span>
      {/if}
    </div>
    <label class="switch"><input type="checkbox" bind:checked={draft.updates.app_check} />{t("ui.general.auto_check")}</label>
    <label class="field narrow"
      >{t("ui.general.check_interval")}
      <input type="number" min="1" step="1" bind:value={draft.updates.check_interval_h} aria-invalid={!!errors["updates.interval"]} />
    </label>
  </div>

  <div class="card stack">
    <h2>{t("ui.general.about")}</h2>
    <div class="about">
      <img src="/icon.png" alt="" width="64" height="64" />
      <div>
        <div class="title">nd104-sync <span class="version">{info.version}</span></div>
        <div class="muted">{t("ui.general.tagline")}</div>
        <div class="row links">
          <button type="button" class="link" onclick={() => api.open(info.project_url)}>{t("ui.general.project")}</button>
          <button type="button" class="link" onclick={() => api.open(info.changelog_url)}>{t("ui.general.changelog")}</button>
          <button type="button" class="link" onclick={() => api.open(info.releases_url)}>{t("ui.general.releases")}</button>
        </div>
      </div>
    </div>
    <dl class="kv">
      <dt>{t("ui.general.config_file")}</dt>
      <dd class="mono">{info.config_path} <button type="button" class="link" onclick={() => api.open("config")}>{t("ui.open")}</button></dd>
      <dt>{t("ui.general.logs")}</dt>
      <dd class="mono">{info.log_dir} <button type="button" class="link" onclick={() => api.open("logs")}>{t("ui.open")}</button></dd>
    </dl>
    <p class="muted small">{t("ui.general.disclaimer")}</p>
  </div>
</div>

<style>
  .label {
    font-size: 13px;
    color: var(--muted);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .narrow {
    max-width: 260px;
  }

  .small {
    font-size: 12px;
    margin: 0;
  }

  .about {
    display: flex;
    gap: 16px;
    align-items: center;
  }

  .title {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 700;
  }

  .version {
    color: var(--accent);
    text-shadow: var(--glow);
  }

  .links {
    margin-top: 6px;
    font-size: 13px;
  }
</style>
