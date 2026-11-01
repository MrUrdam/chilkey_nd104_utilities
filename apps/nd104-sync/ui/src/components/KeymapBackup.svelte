<script lang="ts">
  // Keymap backup and restore (all layers and the knob, as a JSON file).
  // Restore first shows what would change; only then it writes.
  import { onDestroy } from "svelte";
  import { api, onKeymapProgress, type KeymapRestoreCheck, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { dateTime } from "../lib/format";
  import Icon from "./Icon.svelte";

  let { status, onrestored }: { status: Status; onrestored?: () => void } = $props();

  type Phase = "idle" | "reading" | "saved" | "checked" | "writing" | "restored" | "error";
  let phase = $state<Phase>("idle");
  let progress = $state(0);
  let saved = $state<{ path: string; layers: number; assigned_keys: number } | null>(null);
  let check = $state<KeymapRestoreCheck | null>(null);
  let restored = $state(0);
  let error = $state("");

  let ready = $derived(status.device.kind === "ready");
  let busy = $derived(phase === "reading" || phase === "writing");
  let sameFirmware = $derived(!check || check.backup_firmware === check.keyboard_firmware);

  const unlisten = onKeymapProgress((p) => (progress = p));
  onDestroy(() => {
    unlisten.then((f) => f());
  });

  async function backup() {
    error = "";
    progress = 0;
    phase = "reading";
    try {
      const result = await api.keymapBackup();
      if (result) {
        saved = result;
        phase = "saved";
      } else {
        phase = "idle";
      }
    } catch (e) {
      error = String(e);
      phase = "error";
    }
  }

  async function chooseRestore() {
    error = "";
    progress = 0;
    phase = "reading";
    try {
      const result = await api.keymapRestoreCheck();
      check = result;
      phase = result ? "checked" : "idle";
    } catch (e) {
      error = String(e);
      phase = "error";
    }
  }

  async function restore() {
    progress = 0;
    phase = "writing";
    try {
      restored = await api.keymapRestoreApply();
      phase = "restored";
      onrestored?.();
    } catch (e) {
      error = String(e);
      phase = "error";
    }
  }

  async function cancel() {
    await api.keymapRestoreCancel();
    check = null;
    phase = "idle";
  }
</script>

<div class="card stack">
  <h2>{t("ui.keymap.title")}</h2>
  <p class="muted small">{t("ui.keymap.intro")}</p>

  {#if !ready}
    <div class="notice warn"><Icon name="plug" />{t("ui.light.not_connected")}</div>
  {/if}

  <div class="row">
    <button class="key" onclick={backup} disabled={!ready || busy || phase === "checked"}>
      {t("ui.keymap.backup")}
    </button>
    <button class="key" onclick={chooseRestore} disabled={!ready || busy || phase === "checked"}>
      {t("ui.keymap.restore")}
    </button>
  </div>

  {#if busy}
    <div class="progress">
      <span class="muted small">{t(phase === "reading" ? "ui.keymap.reading" : "ui.keymap.writing")}</span>
      <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progress * 100)}>
        <div class="fill" style="width: {Math.max(progress, 0.02) * 100}%"></div>
      </div>
    </div>
  {:else if phase === "saved" && saved}
    <div class="notice">
      <Icon name="check" />
      <span>
        {t("ui.keymap.saved", { layers: saved.layers, keys: saved.assigned_keys })}<br />
        <span class="mono small selectable">{saved.path}</span>
      </span>
    </div>
  {:else if phase === "checked" && check}
    <div class="review">
      <dl class="kv">
        <dt>{t("ui.keymap.file")}</dt>
        <dd class="selectable">{check.name}</dd>
        <dt>{t("ui.keymap.created")}</dt>
        <dd>{dateTime(check.created)}</dd>
        <dt>{t("ui.keymap.changes")}</dt>
        <dd>{t("ui.keymap.changes_value", { keys: check.keys, knob: check.knob })}</dd>
      </dl>
      {#if !sameFirmware}
        <div class="notice warn">
          <Icon name="alert" />
          <span>{t("ui.keymap.other_firmware", { backup: check.backup_firmware, now: check.keyboard_firmware })}</span>
        </div>
      {/if}
      {#if check.keys + check.knob === 0}
        <div class="notice"><Icon name="info" />{t("ui.keymap.identical")}</div>
        <div class="row"><button class="key" onclick={cancel}>{t("ui.keymap.close")}</button></div>
      {:else}
        <p class="muted small">{t("ui.keymap.restore_hint")}</p>
        <div class="row">
          <button class="key" onclick={cancel}>{t("ui.cancel")}</button>
          <button class="key primary" onclick={restore} disabled={!ready}>
            {t("ui.keymap.restore_now", { n: check.keys + check.knob })}
          </button>
        </div>
      {/if}
    </div>
  {:else if phase === "restored"}
    <div class="notice">
      <Icon name="check" />
      {restored === 0 ? t("ui.keymap.identical") : t("ui.keymap.restored", { n: restored })}
    </div>
  {:else if phase === "error"}
    <div class="notice err selectable"><Icon name="alert" />{error}</div>
  {/if}
</div>

<style>
  .small {
    font-size: 12px;
  }

  p.small {
    margin: 0;
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .bar {
    height: 8px;
    border-radius: 999px;
    background: var(--panel-3);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s var(--ease);
  }

  .review {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
  }
</style>
