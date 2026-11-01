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
  import Lighting from "./pages/Lighting.svelte";
  import Picture from "./pages/Picture.svelte";
  import Icon, { type IconName } from "./components/Icon.svelte";

  type Page = "dashboard" | "weather" | "clock" | "lighting" | "picture" | "device" | "general";
  const pages: { id: Page; icon: IconName }[] = [
    { id: "dashboard", icon: "dashboard" },
    { id: "weather", icon: "weather" },
    { id: "clock", icon: "clock" },
    { id: "lighting", icon: "lighting" },
    { id: "picture", icon: "picture" },
    { id: "device", icon: "device" },
    { id: "general", icon: "general" },
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
    document.documentElement.dataset.style = draft?.general.style === "gamer" ? "gamer" : "calm";
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
        return status.weather_stale ? "err" : "ok";
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
          <Icon name={p.icon} size={18} />{t(`ui.nav.${p.id}`)}
        </button>
      {/each}
      <div class="spacer"></div>
      <div class="wired"><Icon name="plug" />{t("ui.wired_only")}</div>
    </nav>

    <main>
      <div class="content">
        {#if page === "dashboard"}
          <Dashboard {status} config={saved} />
        {:else if page === "weather"}
          <Weather bind:draft {info} {errors} />
        {:else if page === "clock"}
          <Clock bind:draft {errors} />
        {:else if page === "lighting"}
          <Lighting {status} />
        {:else if page === "picture"}
          <Picture {status} />
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
        {:else if status.device.kind === "locked"}
          <span class="badge warn">{t("ui.device.locked")}</span>
        {:else}
          <span class="badge">{t("status.not_connected")}</span>
        {/if}
        <div class="spacer"></div>
        {#if saveError}<span class="error-text">{saveError}</span>{/if}
        {#if savedFlash}<span class="saved"><Icon name="check" />{t("ui.saved")}</span>{/if}
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
    gap: 2px;
    padding: 20px 12px 16px;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
  }

  .brand {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 0 8px 20px;
  }

  .name {
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 600;
  }

  :global(:root[data-style="gamer"]) .name {
    font-size: 21px;
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
    height: 36px;
    font: 500 14px var(--font-ui);
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 0 10px;
    text-align: left;
    cursor: pointer;
    transition:
      color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .nav-item :global(svg) {
    color: var(--faint);
  }

  .nav-item:hover {
    color: var(--text);
    background: var(--panel-2);
  }

  .nav-item[aria-current="page"] {
    color: var(--text);
    background: var(--panel-2);
    border-color: var(--border);
  }

  .nav-item[aria-current="page"] :global(svg) {
    color: var(--accent-text);
  }

  :global(:root[data-style="gamer"]) .nav-item[aria-current="page"] {
    box-shadow: inset 2px 0 0 var(--accent);
    border-radius: 2px var(--radius-sm) var(--radius-sm) 2px;
  }

  .spacer {
    flex: 1;
  }

  .wired {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--faint);
    padding: 0 10px;
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
    padding: 24px 32px 32px;
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
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ok);
    font-weight: 500;
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
