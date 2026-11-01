<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import DisplayScreen from "../../ui/src/components/DisplayScreen.svelte";
  import * as api from "./api";
  import type { SetupInfo, Step, Texts } from "./api";

  type Phase = "loading" | "welcome" | "confirm" | "working" | "done" | "error";

  let t = $state<Texts>({});
  let info = $state<SetupInfo | null>(null);
  let phase = $state<Phase>("loading");
  let autostart = $state(true);
  let desktop = $state(false);
  let keepSettings = $state(true);
  let step = $state<Step>("preparing");
  let progress = $state(0);
  let error = $state("");
  let showDetails = $state(false);
  let keyboard = $state<boolean | null>(null);
  let now = $state(new Date());

  const win = getCurrentWindow();

  function tr(key: string, args: Record<string, string> = {}): string {
    let text = t[key] ?? key;
    for (const [k, v] of Object.entries(args)) text = text.replaceAll(`{${k}}`, v);
    return text;
  }

  let uninstalling = $derived(info?.mode === "uninstall");
  let busy = $derived(phase === "working" || phase === "loading");
  // sample values for the display replica: a sunny summer day
  const summer = { condition: "clear", is_day: true, current_c: 26.4, min_c: 17.2, max_c: 28.9 } as const;
  let disclaimer = $derived(tr("disclaimer").split("{license}"));

  let primaryLabel = $derived.by(() => {
    switch (info?.relation) {
      case "upgrade":
        return tr("button.update");
      case "same":
        return tr("button.reinstall");
      case "downgrade":
        return tr("button.downgrade");
      default:
        return tr("button.install");
    }
  });

  onMount(() => {
    const timer = setInterval(() => (now = new Date()), 1000);
    const unlisten = api.onProgress((p) => {
      step = p.step;
      // never jump backwards
      progress = Math.max(progress, p.value);
    });
    (async () => {
      t = await api.texts();
      document.documentElement.lang = t.lang ?? "en";
      info = await api.setupInfo();
      autostart = info.autostart;
      desktop = info.desktop_shortcut;
      if (info.mode === "update") install();
      else phase = info.mode === "uninstall" ? "confirm" : "welcome";
    })();
    return () => {
      clearInterval(timer);
      unlisten.then((f) => f());
    };
  });

  async function install() {
    phase = "working";
    progress = 0;
    step = "preparing";
    try {
      await api.runInstall(autostart, desktop);
      progress = 1;
      phase = "done";
      if (info?.mode !== "update") keyboard = await api.keyboardConnected();
    } catch (e) {
      error = String(e);
      phase = "error";
    }
  }

  async function uninstall() {
    phase = "working";
    progress = 0;
    step = "closing";
    try {
      await api.runUninstall(keepSettings);
      progress = 1;
      phase = "done";
    } catch (e) {
      error = String(e);
      phase = "error";
    }
  }

  async function startApp() {
    try {
      await api.launchApp();
    } finally {
      await win.close();
    }
  }

  const close = () => win.close();
  const minimize = () => win.minimize();
</script>

<div class="frame">
  <header class="titlebar" data-tauri-drag-region>
    <img src="/icon.png" alt="" width="18" height="18" data-tauri-drag-region />
    <span data-tauri-drag-region>{tr("window_title")}</span>
    <div class="spacer" data-tauri-drag-region></div>
    <button class="wbtn" onclick={minimize} aria-label="Minimize">
      <svg viewBox="0 0 10 10" width="10" height="10"><path d="M1 5h8" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button class="wbtn close" onclick={close} disabled={busy} aria-label={tr("button.close")}>
      <svg viewBox="0 0 10 10" width="10" height="10">
        <path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
  </header>

  <section class="hero" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <h1 data-tauri-drag-region>nd104<span>-sync</span></h1>
      <p class="muted" data-tauri-drag-region>{tr("tagline")}</p>
      {#if info}
        <span class="version" data-tauri-drag-region>v{info.version}</span>
      {/if}
    </div>
    <div class="display" class:off={uninstalling && phase === "done"} aria-hidden="true">
      <DisplayScreen time={now} weather={summer} />
    </div>
  </section>

  <main>
    {#if phase === "welcome" && info}
      <div class="stack content">
        {#if info.relation === "upgrade"}
          <h3>{tr("welcome.update", { from: info.installed ?? "", to: info.version })}</h3>
          <p class="muted">{tr("welcome.update_text")}</p>
        {:else if info.relation === "same"}
          <p>{tr("welcome.reinstall", { version: info.version })}</p>
        {:else if info.relation === "downgrade"}
          <div class="notice warn">{tr("welcome.downgrade", { from: info.installed ?? "", to: info.version })}</div>
        {:else}
          <h3>{tr("welcome.install", { version: info.version })}</h3>
        {/if}
        {#if info.msi}
          <div class="notice warn">{tr("welcome.msi")}</div>
        {/if}
        {#if !info.payload}
          <div class="notice err">{tr("welcome.no_payload")}</div>
        {/if}
        <div class="options">
          <label class="switch"><input type="checkbox" bind:checked={autostart} />{tr("option.autostart")}</label>
          <label class="switch"><input type="checkbox" bind:checked={desktop} />{tr("option.desktop")}</label>
        </div>
      </div>
    {:else if phase === "confirm" && info}
      <div class="stack content">
        <h3>{tr("uninstall.title", { version: info.installed ?? info.version })}</h3>
        <p class="muted">{tr("uninstall.text")}</p>
        <div>
          <label class="switch"><input type="checkbox" bind:checked={keepSettings} />{tr("uninstall.keep")}</label>
          {#if !keepSettings}
            <p class="hint">{tr("uninstall.keep_hint")}</p>
          {/if}
        </div>
      </div>
    {:else if phase === "working" || phase === "loading"}
      <div class="content working">
        <p class="status">{phase === "loading" ? "" : tr(`step.${step}`)}</p>
        <div
          class="bar"
          role="progressbar"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={Math.round(progress * 100)}
        >
          <div class="fill" style="width: {Math.max(progress, 0.02) * 100}%"></div>
        </div>
        <p class="percent mono">{Math.round(progress * 100)} %</p>
      </div>
    {:else if phase === "done" && info}
      <div class="stack content">
        {#if uninstalling}
          <h3 class="result"><span class="led ok"></span>{tr("done.uninstalled")}</h3>
          {#if keepSettings}<p class="muted">{tr("done.uninstalled_kept")}</p>{/if}
        {:else if info.mode === "update"}
          <h3 class="result"><span class="led ok"></span>{tr("done.updated")}</h3>
          <p class="muted">{tr("done.updated_text", { version: info.version })}</p>
        {:else}
          <h3 class="result"><span class="led ok"></span>{tr("done.installed")}</h3>
          <p class="muted">{tr("done.tray")}</p>
          {#if keyboard === true}
            <div class="notice"><span class="led ok"></span>{tr("hint.keyboard_found")}</div>
          {:else if keyboard === false}
            <div class="notice">
              <svg class="plug" viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
                <path
                  d="M9 2v5M15 2v5M6 7h12v4a6 6 0 0 1-12 0zM12 17v5"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                />
              </svg>
              {tr("hint.keyboard")}
            </div>
          {/if}
        {/if}
      </div>
    {:else if phase === "error"}
      <div class="stack content">
        <h3 class="result"><span class="led err"></span>{tr("error.title")}</h3>
        <p>{tr(uninstalling ? "error.uninstall" : "error.install")}</p>
        <button class="link details" onclick={() => (showDetails = !showDetails)}>{tr("error.details")}</button>
        {#if showDetails}<p class="mono selectable small">{error}</p>{/if}
      </div>
    {/if}
  </main>

  <footer>
    <div class="legal">
      {#if phase === "welcome"}
        {disclaimer[0]}<button class="link" onclick={() => api.openLink("license")}>{tr("license")}</button
        >{disclaimer[1] ?? ""}
      {/if}
    </div>
    <div class="row buttons">
      {#if phase === "welcome" && info}
        <button class="key" onclick={close}>{tr("button.cancel")}</button>
        <button class="key primary" onclick={install} disabled={!info.payload}>{primaryLabel}</button>
      {:else if phase === "confirm"}
        <button class="key" onclick={close}>{tr("button.cancel")}</button>
        <button class="key primary danger" onclick={uninstall}>{tr("button.uninstall")}</button>
      {:else if phase === "done"}
        {#if uninstalling || info?.mode === "update"}
          <button class="key primary" onclick={close}>{tr("button.close")}</button>
        {:else}
          <button class="key" onclick={close}>{tr("button.close")}</button>
          <button class="key primary" onclick={startApp}>{tr("button.start")}</button>
        {/if}
      {:else if phase === "error"}
        <button class="key" onclick={close}>{tr("button.close")}</button>
        <button class="key primary" onclick={uninstalling ? uninstall : install}>{tr("button.retry")}</button>
      {/if}
    </div>
  </footer>
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }

  .frame {
    height: 100vh;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-strong);
    box-sizing: border-box;
    background:
      radial-gradient(120% 70% at 100% 0%, var(--accent-soft), transparent 60%),
      var(--bg);
  }

  .titlebar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding-left: 12px;
    font-size: 12px;
    color: var(--muted);
    flex: none;
  }

  .spacer {
    flex: 1;
    align-self: stretch;
  }

  .wbtn {
    width: 46px;
    height: 34px;
    display: grid;
    place-items: center;
    background: none;
    border: 0;
    color: var(--muted);
    cursor: pointer;
  }

  .wbtn:hover:not(:disabled) {
    background: var(--panel-3);
    color: var(--text);
  }

  .wbtn.close:hover:not(:disabled) {
    background: #c42b1c;
    color: #ffffff;
  }

  .wbtn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 6px 32px 18px;
    flex: none;
  }

  .brand h1 {
    font-size: 40px;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.03em;
  }

  .brand h1 span {
    color: var(--accent);
    text-shadow: var(--glow);
  }

  .brand p {
    margin: 6px 0 10px;
  }

  .version {
    font: 600 12px var(--font-mono);
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: 999px;
    padding: 2px 10px;
  }

  /* small replica of the keyboard's display */
  .display {
    width: 200px;
    flex: none;
    transition: opacity 0.6s;
  }

  .display.off {
    opacity: 0.4;
  }

  main {
    flex: 1;
    padding: 0 32px;
    min-height: 0;
  }

  .content {
    height: 100%;
    gap: 12px;
  }

  .content p,
  .content h3 {
    margin: 0;
  }

  .options {
    display: flex;
    gap: 28px;
    margin-top: 4px;
  }

  .hint {
    margin: 8px 0 0 52px !important;
    font-size: 12px;
    color: var(--warn);
  }

  .working {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
  }

  .status {
    font: 600 17px var(--font-display);
    letter-spacing: 0.04em;
    min-height: 1.4em;
  }

  .bar {
    height: 10px;
    border-radius: 999px;
    background: var(--panel-3);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background:
      repeating-linear-gradient(-45deg, rgba(255, 255, 255, 0.18) 0 8px, transparent 8px 16px),
      linear-gradient(90deg, var(--accent-strong), var(--accent));
    background-size:
      22.6px 22.6px,
      100% 100%;
    box-shadow: var(--glow);
    transition: width 0.35s ease-out;
    animation: stripes 0.8s linear infinite;
  }

  @keyframes stripes {
    to {
      background-position:
        22.6px 0,
        0 0;
    }
  }

  .percent {
    text-align: right;
    font-size: 12px;
    color: var(--muted);
  }

  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 20px;
  }

  .notice .led {
    margin-top: 5px;
  }

  .plug {
    flex: none;
    color: var(--accent);
  }

  .details {
    align-self: flex-start;
    font-size: 12px;
  }

  .small {
    font-size: 11px;
    max-height: 70px;
    overflow: auto;
    margin: 0;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 20px 18px 32px;
    flex: none;
  }

  .legal {
    font-size: 11px;
    color: var(--muted);
    max-width: 330px;
  }

  .legal .link {
    font-size: 11px;
  }

  .buttons {
    flex-wrap: nowrap;
    margin-left: auto;
  }

  .buttons .key {
    min-width: 96px;
    padding: 9px 18px;
  }

  .key.danger {
    background: #b91c1c;
    border-color: var(--err);
    box-shadow: var(--key-shadow);
  }
</style>
