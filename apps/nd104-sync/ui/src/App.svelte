<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { api, onConfigChanged, onStatus, type AppInfo, type Config, type Status } from "./lib/api";
  import { setTexts, t } from "./lib/i18n.svelte";
  import { setLocale } from "./lib/format";
  import { validate } from "./lib/validate";
  import Dashboard from "./pages/Dashboard.svelte";
  import Weather from "./pages/Weather.svelte";
  import Clock from "./pages/Clock.svelte";
  import Device from "./pages/Device.svelte";
  import General from "./pages/General.svelte";

  type Page = "dashboard" | "weather" | "clock" | "device" | "general";
  const pages: { id: Page; icon: string }[] = [
    { id: "dashboard", icon: "◉" },
    { id: "weather", icon: "☁" },
    { id: "clock", icon: "◷" },
    { id: "device", icon: "⌨" },
    { id: "general", icon: "⚙" },
  ];

  let page = $state<Page>("dashboard");
  let info = $state<AppInfo | null>(null);
  let status = $state<Status | null>(null);
  let saved = $state<Config | null>(null);
  let draft = $state<Config | null>(null);
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  let savedFlash = $state(false);

  let dirty = $derived(draft !== null && saved !== null && JSON.stringify(draft) !== JSON.stringify(saved));
  let errors = $derived(draft && info ? validate(draft, info) : {});
  let valid = $derived(Object.keys(errors).length === 0);

  // Theme: preview the draft immediately; "auto" follows Windows.
  $effect(() => {
    const theme = draft?.general.theme ?? "auto";
    if (theme === "dark" || theme === "light") document.documentElement.dataset.theme = theme;
    else delete document.documentElement.dataset.theme;
  });

  async function loadTexts() {
    const [i, texts] = await Promise.all([api.appInfo(), api.texts()]);
    info = i;
    setTexts(texts);
    setLocale(i.language);
    document.documentElement.lang = i.language;
  }

  async function loadConfig() {
    const config = await api.config();
    saved = structuredClone(config);
    draft = structuredClone(config);
  }

  async function apply() {
    if (!draft || !valid) return;
    saving = true;
    saveError = null;
    try {
      const languageChanged = draft.general.language !== saved?.general.language;
      await api.saveConfig($state.snapshot(draft) as Config);
      saved = structuredClone($state.snapshot(draft) as Config);
      if (languageChanged) await loadTexts();
      savedFlash = true;
      setTimeout(() => (savedFlash = false), 1800);
    } catch (e) {
      saveError = String(e);
    } finally {
      saving = false;
    }
  }

  function cancel() {
    if (saved) draft = structuredClone($state.snapshot(saved) as Config);
    saveError = null;
  }

  const unlisten: (() => void)[] = [];
  onMount(async () => {
    await loadTexts();
    await loadConfig();
    status = await api.status();
    unlisten.push(await onStatus((s) => (status = s)));
    unlisten.push(
      await onConfigChanged(async () => {
        // Changed outside the window (tray or file): take it over unless the user is editing.
        const fresh = await api.config();
        if (!dirty) {
          saved = structuredClone(fresh);
          draft = structuredClone(fresh);
        } else {
          saved = structuredClone(fresh);
        }
        await loadTexts();
      }),
    );
  });
  onDestroy(() => unlisten.forEach((u) => u()));

  let deviceLed = $derived.by(() => {
    switch (status?.device.kind) {
      case "ready":
        return status.weather_stale ? "err" : "accent";
      case "locked":
        return "warn";
      default:
        return "";
    }
  });
</script>

{#if info && draft && saved && status}
  <div class="shell">
    <nav>
      <div class="brand">
        <img src="/icon.png" alt="" width="34" height="34" />
        <div>
          <div class="name">nd104-sync</div>
          <div class="model"><span class="led {deviceLed}"></span>{t("ui.brand.model")}</div>
        </div>
      </div>
      {#each pages as p (p.id)}
        <button class="nav-item" aria-current={page === p.id ? "page" : undefined} onclick={() => (page = p.id)}>
          <span class="glyph">{p.icon}</span>{t(`ui.nav.${p.id}`)}
        </button>
      {/each}
      <div class="spacer"></div>
      <div class="wired">🔌 {t("ui.wired_only")}</div>
    </nav>

    <main>
      <div class="content">
        {#if page === "dashboard"}
          <Dashboard {status} config={saved} />
        {:else if page === "weather"}
          <Weather bind:draft {info} {errors} />
        {:else if page === "clock"}
          <Clock bind:draft {errors} />
        {:else if page === "device"}
          <Device {status} {info} bind:draft {errors} />
        {:else}
          <General bind:draft {info} {status} {errors} />
        {/if}
      </div>

      <footer>
        <span class="muted">nd104-sync {info.version}</span>
        {#if status.device.kind === "ready"}
          <span class="muted">· {status.device.model_name}</span>
          <span class="badge {status.device.trust === 'verified' ? 'ok' : 'warn'}">
            {t(status.device.trust === "verified" ? "status.verified" : "status.compatible")}
          </span>
        {:else if status.device.kind === "locked"}
          <span class="badge warn">{t("ui.device.locked")}</span>
        {:else}
          <span class="badge">{t("status.not_connected")}</span>
        {/if}
        <div class="spacer"></div>
        {#if saveError}<span class="error-text">{saveError}</span>{/if}
        {#if savedFlash}<span class="saved">✔ {t("ui.saved")}</span>{/if}
        {#if dirty}
          {#if !valid}<span class="error-text">{t("ui.fix_errors")}</span>{/if}
          <button class="key" onclick={cancel} disabled={saving}>{t("ui.cancel")}</button>
          <button class="key primary" onclick={apply} disabled={saving || !valid}>{t("ui.apply")}</button>
        {/if}
      </footer>
    </main>
  </div>
{:else}
  <div class="loading">nd104-sync …</div>
{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100%;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 18px 12px;
    background: var(--panel);
    border-right: 1px solid var(--border);
  }

  .brand {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 0 8px 18px;
  }

  .name {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .model {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font: 600 14px var(--font-ui);
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 9px 12px;
    text-align: left;
    cursor: pointer;
  }

  .nav-item:hover {
    color: var(--text);
    background: var(--panel-2);
  }

  .nav-item[aria-current="page"] {
    color: var(--text);
    background: var(--panel-3);
    border-color: var(--border-strong);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .glyph {
    width: 18px;
    text-align: center;
    color: var(--accent);
  }

  .spacer {
    flex: 1;
  }

  .wired {
    font-size: 12px;
    color: var(--muted);
    padding: 0 8px;
  }

  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .content {
    flex: 1;
    overflow: auto;
    padding: 22px 26px;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 20px;
    border-top: 1px solid var(--border);
    background: var(--panel);
    font-size: 13px;
    min-height: 54px;
  }

  .saved {
    color: var(--ok);
    font-weight: 600;
  }

  .loading {
    display: grid;
    place-items: center;
    height: 100%;
    font-family: var(--font-display);
    font-size: 22px;
    color: var(--muted);
  }
</style>
