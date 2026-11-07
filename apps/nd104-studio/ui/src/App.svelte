<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { api, onConfigChanged, onStatus, type AppInfo, type Config, type Status } from "./lib/api";
  import { setTexts, t } from "./lib/i18n.svelte";
  import { setPlatform, setSecureStore } from "./lib/platform.svelte";
  import { setLocale } from "./lib/format";
  import { validate } from "./lib/validate";
  import { bluetooth, watchBluetooth } from "./lib/bluetooth.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import Weather from "./pages/Weather.svelte";
  import Clock from "./pages/Clock.svelte";
  import Device from "./pages/Device.svelte";
  import General from "./pages/General.svelte";
  import Lighting from "./pages/Lighting.svelte";
  import Picture from "./pages/Picture.svelte";
  import Keymap from "./pages/Keymap.svelte";
  import Icon, { type IconName } from "./components/Icon.svelte";

  type Page = "dashboard" | "weather" | "clock" | "lighting" | "picture" | "keymap" | "device" | "general";
  const pages: { id: Page; icon: IconName }[] = [
    { id: "dashboard", icon: "dashboard" },
    { id: "keymap", icon: "layers" },
    { id: "lighting", icon: "lighting" },
    { id: "weather", icon: "weather" },
    { id: "clock", icon: "clock" },
    { id: "picture", icon: "picture" },
    { id: "device", icon: "device" },
    { id: "general", icon: "general" },
  ];

  let page = $state<Page>("dashboard");

  // Sidebar collapsed to icons (per-viewer convenience, kept in the browser storage)
  const COLLAPSED_KEY = "nd104.nav.collapsed";
  let collapsed = $state(readCollapsed());

  function readCollapsed(): boolean {
    try {
      return localStorage.getItem(COLLAPSED_KEY) === "1";
    } catch {
      return false;
    }
  }

  function toggleCollapsed() {
    collapsed = !collapsed;
    try {
      localStorage.setItem(COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch {
      /* storage unavailable: only for this session */
    }
  }
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
    setPlatform(i.platform);
    setSecureStore(i.secure_store);
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
  onDestroy(watchBluetooth());
  const bt = bluetooth();

  /** How the keyboard is connected right now (bottom of the sidebar). */
  let connection = $derived.by(() => {
    if (status?.device.kind === "ready" && status.device.connection === "bluetooth") {
      const title = t("ui.conn.bt_ready_title", { slot: bt.keyboard?.slot ?? "?" });
      return { kind: "usb", icon: "bluetooth" as const, text: t("ui.bt.badge"), title };
    }
    if (status?.device.kind === "ready" && status.device.connection === "wireless") {
      return { kind: "usb", icon: "wireless" as const, text: t("ui.conn.wireless"), title: t("ui.conn.wireless_title") };
    }
    if (status?.device.kind === "ready") {
      return { kind: "usb", icon: "plug" as const, text: t("ui.conn.usb"), title: t("ui.conn.usb_title") };
    }
    if (status?.device.kind === "no_access") {
      return { kind: "locked", icon: "lock" as const, text: t("ui.device.no_access"), title: status.device.path };
    }
    if (status?.device.kind === "locked") {
      return { kind: "locked", icon: "lock" as const, text: t("ui.device.locked"), title: status.device.reason };
    }
    if (bt.keyboard) {
      const title = t("ui.conn.bt_title", { slot: bt.keyboard.slot ?? "?" });
      return { kind: "bt", icon: "bluetooth" as const, text: t("ui.bt.badge"), title };
    }
    return { kind: "none", icon: "plug" as const, text: t("ui.conn.none"), title: t("ui.conn.none_title") };
  });

  let deviceLed = $derived.by(() => {
    switch (status?.device.kind) {
      case "ready":
        return "ok";
      case "locked":
      case "no_access":
        return "err";
      default:
        // Bluetooth works, but only for status and battery
        return bt.keyboard ? "warn" : "";
    }
  });
</script>

{#if info && draft && saved && status}
  <div class="shell" class:collapsed>
    <nav>
      <div class="brand">
        <img src="/icon.png" alt="" width="34" height="34" />
        <div class="brand-text">
          <div class="name">ND104 Studio</div>
          <div class="model"><span class="led {deviceLed}"></span>{t("ui.brand.model")}</div>
        </div>
      </div>
      {#each pages as p (p.id)}
        <button
          class="nav-item"
          aria-current={page === p.id ? "page" : undefined}
          aria-label={collapsed ? t(`ui.nav.${p.id}`) : undefined}
          onclick={() => (page = p.id)}
        >
          <Icon name={p.icon} size={18} /><span class="label">{t(`ui.nav.${p.id}`)}</span>
        </button>
      {/each}
      <div class="spacer"></div>
      <div class="wired {connection.kind}" title={connection.title}>
        <Icon name={connection.icon} size={18} /><span class="wired-text">{connection.text}</span>
      </div>
      <button
        class="nav-item collapse"
        aria-expanded={!collapsed}
        aria-label={t(collapsed ? "ui.nav.expand" : "ui.nav.collapse")}
        title={t(collapsed ? "ui.nav.expand" : "ui.nav.collapse")}
        onclick={toggleCollapsed}
      >
        <Icon name="sidebar" size={18} /><span class="label">{t("ui.nav.collapse_short")}</span>
      </button>
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
          <Lighting {status} bind:draft {errors} />
        {:else if page === "picture"}
          <Picture {status} />
        {:else if page === "keymap"}
          <Keymap {status} {info} />
        {:else if page === "device"}
          <Device {status} {info} bind:draft {errors} />
        {:else}
          <General bind:draft {info} {status} {errors} />
        {/if}
      </div>

      <footer>
        <span class="muted">ND104 Studio {info.version}</span>
        {#if status.device.kind === "ready"}
          <span class="muted">· {status.device.model_name}</span>
          {#if status.device.connection === "bluetooth"}
            <span class="badge ok">{t("ui.bt.badge")}</span>
          {:else if status.device.connection === "wireless"}
            <span class="badge ok">{t("ui.conn.wireless")}</span>
          {/if}
          <!-- also on the cable, while Bluetooth stays linked -->
          {#if bt.keyboard?.battery != null}<span class="muted">· {t("ui.bt.battery", { n: bt.keyboard.battery })}</span>{/if}
        {:else if status.device.kind === "locked"}
          <span class="badge err">{t("ui.device.locked")}</span>
        {:else if status.device.kind === "no_access"}
          <span class="badge err">{t("ui.device.no_access")}</span>
        {:else if bt.keyboard}
          <span class="badge">{t("ui.bt.badge")}</span>
          {#if bt.keyboard.battery !== null}<span class="muted">· {t("ui.bt.battery", { n: bt.keyboard.battery })}</span>{/if}
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
  <div class="loading">ND104 Studio …</div>
{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100%;
    transition: grid-template-columns 0.22s var(--ease);
  }

  .shell.collapsed {
    grid-template-columns: 64px 1fr;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 20px 12px 16px;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    /* above the page, so the labels can fly out over it */
    position: relative;
    z-index: 20;
    min-width: 0;
  }

  .brand img {
    flex: none;
  }

  .brand-text,
  .wired-text {
    white-space: nowrap;
    overflow: hidden;
    transition: opacity 0.15s var(--ease);
  }

  .label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .collapse {
    margin-top: 6px;
  }

  /* --- collapsed: icons only; the label flies out on hover/focus --------- */
  .collapsed nav {
    padding-left: 10px;
    padding-right: 10px;
  }

  .collapsed .brand {
    padding: 0 0 20px;
    justify-content: center;
  }

  .collapsed .brand-text,
  .collapsed .wired-text {
    opacity: 0;
    width: 0;
  }

  .collapsed .wired {
    justify-content: center;
    padding: 0;
    gap: 0;
  }

  .collapsed .nav-item {
    justify-content: center;
    padding: 0;
    position: relative;
  }

  .collapsed .nav-item .label {
    position: absolute;
    left: calc(100% + 12px);
    top: 50%;
    padding: 6px 12px;
    font-size: 13px;
    color: var(--text);
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.22);
    overflow: visible;
    pointer-events: none;
    opacity: 0;
    transform: translate(-8px, -50%) scale(0.96);
    transform-origin: left center;
    transition:
      opacity 0.14s var(--ease),
      transform 0.18s var(--ease);
  }

  /* small arrow pointing at the icon */
  .collapsed .nav-item .label::before {
    content: "";
    position: absolute;
    left: -5px;
    top: 50%;
    width: 8px;
    height: 8px;
    background: var(--panel);
    border-left: 1px solid var(--border-strong);
    border-bottom: 1px solid var(--border-strong);
    transform: translateY(-50%) rotate(45deg);
  }

  .collapsed .nav-item:hover .label,
  .collapsed .nav-item:focus-visible .label {
    opacity: 1;
    transform: translate(0, -50%) scale(1);
  }

  .collapsed .nav-item[aria-current="page"] .label {
    color: var(--accent-text);
  }

  @media (prefers-reduced-motion: reduce) {
    .shell,
    .collapsed .nav-item .label {
      transition: none;
    }
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

  /* same box as a menu item, so the symbol lines up with the menu icons */
  .wired {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    font-size: 12px;
    color: var(--faint);
    padding: 0 10px;
    border: 1px solid transparent;
  }

  /* the symbol takes the colour of the connection: full, limited, none */
  .wired.usb :global(svg) {
    color: var(--ok);
  }

  .wired.bt :global(svg) {
    color: var(--warn);
  }

  .wired.locked :global(svg) {
    color: var(--err);
  }

  .wired.usb,
  .wired.bt,
  .wired.locked {
    color: var(--muted);
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
