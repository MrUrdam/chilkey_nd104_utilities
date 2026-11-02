<script lang="ts">
  // Keymap: change what keys, the knob and the round button do, per layer.
  // Changes are collected first and written together; the keyboard's
  // current state is saved as an automatic backup before every write.
  // Labels (what is printed on the keys) are for display only.
  import { onMount } from "svelte";
  import { api, onKeymapProgress, type AppInfo, type Keymap, type KeymapProfile, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { dateTime } from "../lib/format";
  import { BUILTIN_PRESETS, layout, resolveLegends, type CustomLegendPreset, type PhysicalKey, type Shape } from "../lib/keyboard";
  import { KC_NO, KC_TRNS, describe, setLayerLabel } from "../lib/keycodes";
  import Icon from "../components/Icon.svelte";
  import UsbRequired from "../components/UsbRequired.svelte";
  import KeyboardView, { type KeyFace } from "../components/KeyboardView.svelte";
  import KeyPicker from "../components/KeyPicker.svelte";
  import KeyFunctionsOverview from "../components/KeyFunctionsOverview.svelte";
  import KeymapBackup from "../components/KeymapBackup.svelte";
  import DisplayPreview from "../components/DisplayPreview.svelte";
  import { glowOf, type Glow } from "../lib/glow";
  import { codesFor } from "../lib/livekeys";

  let { status, info }: { status: Status; info: AppInfo } = $props();

  const LAYERS = 6;
  const KNOB_LAYERS = 4;
  const COLS = 16;
  /** Matrix slot of the knob press. */
  const KNOB_PRESS: [number, number] = [6, 9];

  // --- page settings (stored by the app in keymap-page.json) ---------------
  interface PageSettings {
    version: 1;
    layerNames: string[];
    shape: Shape;
    preset: string;
    custom: CustomLegendPreset[];
    /** Show the keyboard's lighting in the drawing. */
    showLighting: boolean;
  }

  const defaults = (): PageSettings => ({
    version: 1,
    layerNames: Array.from({ length: LAYERS }, () => ""),
    shape: "iso",
    preset: info.language === "de" ? "iso-de" : "iso-uk",
    custom: [],
    showLighting: true,
  });

  let settings = $state<PageSettings>(defaults());

  function loadSettings(raw: unknown) {
    const s = defaults();
    if (raw && typeof raw === "object") {
      const r = raw as Partial<PageSettings>;
      if (Array.isArray(r.layerNames)) s.layerNames = s.layerNames.map((d, i) => String(r.layerNames?.[i] ?? d).slice(0, 24));
      if (r.shape === "iso" || r.shape === "ansi") s.shape = r.shape;
      if (Array.isArray(r.custom)) {
        s.custom = r.custom
          .filter((c) => c && typeof c.id === "string" && typeof c.name === "string")
          .map((c) => ({ id: c.id, name: c.name.slice(0, 40), base: String(c.base), labels: { ...(c.labels ?? {}) } }));
      }
      if (typeof r.preset === "string") s.preset = r.preset;
      if (typeof r.showLighting === "boolean") s.showLighting = r.showLighting;
    }
    settings = s;
  }

  function saveSettings() {
    api.keymapPageSet($state.snapshot(settings)).catch((e) => (error = String(e)));
  }

  // --- keymap state -------------------------------------------------------
  let device = $state<Keymap | null>(null);
  let edit = $state<Keymap | null>(null);
  let loading = $state(false);
  let writing = $state(false);
  let progress = $state(0);
  let error = $state("");
  let message = $state("");

  let ready = $derived(status.device.kind === "ready");
  let layer = $state(0);
  let selected = $state<string | null>(null);
  let mode = $state<"assign" | "labels">("assign");
  let overview = $state(false);

  let keys = $derived(layout(settings.shape));
  let resolved = $derived(resolveLegends(settings.preset, settings.custom));
  let legends = $derived(resolved.preset.legends);
  let presetsForShape = $derived(BUILTIN_PRESETS.filter((p) => p.shape === settings.shape));
  let customForShape = $derived(
    settings.custom.filter((c) => BUILTIN_PRESETS.find((p) => p.id === c.base)?.shape === settings.shape),
  );

  const slot = ([r, c]: [number, number]) => r * COLS + c;

  /** Base line of a legend ("!\n1" → "1"). */
  const baseLine = (legend: string | undefined) => (legend ?? "").split("\n").pop() ?? "";

  /** Name of a basic key in the chosen labels, e.g. KC_Y → "Z" on ISO DE. */
  let basicNames = $derived.by(() => {
    const map = new Map<number, string>();
    for (const k of keys) {
      const text = baseLine(legends[k.id]);
      if (text && k.hid < 0x100 && !map.has(k.hid) && !k.mod) map.set(k.hid, text.length === 1 ? text.toUpperCase() : text);
    }
    return map;
  });
  const basicName = (code: number) => basicNames.get(code) ?? null;

  /** Default names: layer 0 is the basic layer, layer 1 the one reached with Fn. */
  function defaultLayerName(n: number): string {
    if (n === 0) return t("ui.km.layer_basic");
    if (n === 1) return t("ui.km.layer_fn");
    // reached with Fn + right Shift on the ND104
    if (n === 2) return t("ui.km.layer_fn_shift");
    // empty in the ND104's keymap and not reachable until a layer key points there
    if (n >= 3) return t("ui.km.layer_custom", { n: n - 2 });
    return t("ui.km.layer_n", { n });
  }

  function layerName(n: number): string {
    return settings.layerNames[n]?.trim() || defaultLayerName(n);
  }

  // layer functions are named after the layers ("Zu Eigene 1 wechseln")
  setLayerLabel((n) => layerName(n));

  async function read() {
    if (!ready) return;
    loading = true;
    error = "";
    progress = 0;
    try {
      const k = await api.keymapRead();
      device = k;
      edit = structuredClone(k);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    const unlisten = onKeymapProgress((p) => (progress = p));
    api
      .keymapPageGet()
      .then(loadSettings)
      .catch(() => {});
    api.keymapProfiles().then((p) => (profiles = p)).catch(() => {});
    return () => {
      unlisten.then((f) => f());
    };
  });

  // read once the keyboard is there (also after plugging it in later)
  $effect(() => {
    if (ready && !device && !loading && !error) read();
  });

  // --- keys pressed on the keyboard light up in the drawing ---------------
  // Windows reports what a key *sends*, so the key is looked up in the
  // keyboard's keymap. Fn itself sends nothing; a key that only exists on a
  // higher layer (e.g. Fn+F5 → play/pause) shows that layer while it is held.
  let pressed = $state(new Set<string>());
  /** event.code → the drawn keys it lights up */
  const held = new Map<string, string[]>();
  /** Layer shown before a combination switched the view (restored on release). */
  let layerBeforeCombo: number | null = null;
  /** A media key (or a knob turn) lights up at least this long. */
  const MIN_LIT_MS = 150;

  function typingIn(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    return !!el && (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName));
  }

  /** Drawn keys (and the knob) that send `code` on layer `l`. */
  function sendersOf(code: number, l: number): string[] {
    const map = device ?? edit;
    if (!map) return [];
    const ids = keys.filter((k) => k.pos.some((p) => map.layers[l][slot(p)] === code)).map((k) => k.id);
    const knob = map.layers[l][slot(KNOB_PRESS)] === code || (l < KNOB_LAYERS && map.knob[l].includes(code));
    if (knob) ids.push("knob");
    return ids;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (detecting || typingIn(e.target)) return;
    const codes = codesFor(e.code);
    let found: string[] = [];
    let foundLayer = 0;
    for (let l = 0; l < LAYERS && !found.length; l++) {
      found = codes.flatMap((c) => sendersOf(c, l));
      foundLayer = l;
    }
    if (!found.length) return;
    held.set(e.code, found);
    pressed = new Set([...held.values()].flat());
    // a combination: show its layer – but not while a key is being remapped
    // or a layer renamed, that would change what is being edited
    const mayFollow = mode === "assign" && selected === null && renaming === null;
    if (foundLayer !== 0 && foundLayer !== layer && mayFollow) {
      layerBeforeCombo ??= layer;
      layer = foundLayer;
    }
  }

  function onKeyUp(e: KeyboardEvent) {
    if (!held.has(e.code)) return;
    setTimeout(() => {
      held.delete(e.code);
      pressed = new Set([...held.values()].flat());
      if (held.size === 0 && layerBeforeCombo !== null) {
        layer = layerBeforeCombo;
        layerBeforeCombo = null;
      }
    }, MIN_LIT_MS);
  }

  /** The window lost focus: key releases would not arrive any more. */
  function releaseAll() {
    held.clear();
    pressed = new Set();
    if (layerBeforeCombo !== null) layer = layerBeforeCombo;
    layerBeforeCombo = null;
  }

  // --- lighting: the drawing glows like the keyboard ----------------------
  let keyLight = $state<Glow | null>(null);
  let underLight = $state<Glow | null>(null);
  let lightRead = false;

  async function readLighting() {
    lightRead = true;
    // only for the look: a failed read just leaves the drawing unlit
    const [keys, bottom] = await Promise.allSettled([api.lightRead("per_key"), api.lightRead("bottom")]);
    keyLight = keys.status === "fulfilled" ? glowOf("per_key", keys.value) : null;
    underLight = bottom.status === "fulfilled" ? glowOf("bottom", bottom.value) : null;
  }

  /** Only single-colour lighting is drawn; animated effects switch it off. */
  const shownLight = (g: Glow | null) => (settings.showLighting && g?.fx === "solid" ? g : null);
  let lightingAnimated = $derived(keyLight?.fx !== undefined && keyLight.fx !== "solid" || (underLight?.fx !== undefined && underLight.fx !== "solid"));

  $effect(() => {
    if (ready && !lightRead) readLighting();
    if (!ready) {
      lightRead = false;
      keyLight = null;
      underLight = null;
    }
  });

  // --- faces of the drawing -----------------------------------------------
  function codeAt(k: Keymap | null, l: number, key: PhysicalKey): number {
    return k ? k.layers[l][slot(key.pos[0])] : KC_NO;
  }

  function isLocked(key: PhysicalKey) {
    return layer === 0 && !!key.protectedOnBase;
  }

  function legendFace(key: PhysicalKey): Pick<KeyFace, "top" | "main"> {
    const lines = (legends[key.id] ?? "").split("\n");
    return lines.length > 1 ? { top: lines[0], main: lines[1] } : { main: lines[0] };
  }

  let faces = $derived.by(() => {
    const out: Record<string, KeyFace> = {};
    for (const key of keys) {
      const code = codeAt(edit, layer, key);
      const changed = !!device && key.pos.some((p) => device!.layers[layer][slot(p)] !== edit!.layers[layer][slot(p)]);
      const d = describe(code, info.language, basicName, layer);
      const title = `${legends[key.id]?.replace("\n", " ") || key.id} → ${d.name}${d.hint ? `\n${d.hint}` : ""}`;
      if (mode === "labels" || !edit) {
        out[key.id] = { ...legendFace(key), kind: "default", changed: false, title: legends[key.id] ?? "" };
      } else if (isLocked(key)) {
        out[key.id] = { ...legendFace(key), kind: "locked", changed, title: t("ui.km.fn_locked") };
      } else if (code === key.hid && layer === 0) {
        out[key.id] = { ...legendFace(key), kind: "default", changed, title };
      } else if (code === KC_TRNS) {
        out[key.id] = { ...legendFace(key), kind: "trns", changed, title };
      } else if (code === KC_NO && layer === 1 && key.id === "bksp") {
        // Fn + Backspace is the battery display built into the firmware
        out[key.id] = { main: t("ui.km.builtin_battery"), kind: "trns", changed, title: t("ui.km.builtin_battery_hint") };
      } else if (code === KC_NO) {
        out[key.id] = { main: "", kind: "none", changed, title };
      } else {
        // orange only where a key does something else than printed on it;
        // on the other layers an assigned function is the normal case
        const kind = code === key.hid ? "default" : layer === 0 ? "remapped" : "assigned";
        out[key.id] = { main: d.short, kind, changed, title };
      }
    }
    return out;
  });

  let selectedKey = $derived(keys.find((k) => k.id === selected) ?? null);

  // --- editing ------------------------------------------------------------
  function setKey(key: PhysicalKey, code: number) {
    if (!edit || isLocked(key)) return;
    for (const p of key.pos) edit.layers[layer][slot(p)] = code;
    message = "";
  }

  function setKnob(direction: 0 | 1, code: number) {
    if (!edit || layer >= KNOB_LAYERS) return;
    edit.knob[layer][direction] = code;
    message = "";
  }

  function setPress(code: number) {
    if (!edit) return;
    edit.layers[layer][slot(KNOB_PRESS)] = code;
    message = "";
  }

  let knobSlot = $state<"left" | "right" | "press">("right");

  let knobCodes = $derived(
    edit
      ? {
          left: layer < KNOB_LAYERS ? edit.knob[layer][0] : null,
          right: layer < KNOB_LAYERS ? edit.knob[layer][1] : null,
          press: edit.layers[layer][slot(KNOB_PRESS)],
        }
      : null,
  );

  let knobChanged = $derived(
    !!device &&
      !!edit &&
      (device.layers[layer][slot(KNOB_PRESS)] !== edit.layers[layer][slot(KNOB_PRESS)] ||
        (layer < KNOB_LAYERS && device.knob[layer].some((c, i) => c !== edit!.knob[layer][i]))),
  );

  // --- pending changes ----------------------------------------------------
  interface Change {
    id: string;
    layer: number;
    what: string;
    from: number;
    to: number;
    revert: () => void;
  }

  let changes = $derived.by(() => {
    const list: Change[] = [];
    if (!device || !edit) return list;
    const byslot = new Map<number, PhysicalKey>();
    for (const k of keys) for (const p of k.pos) byslot.set(slot(p), k);
    for (let l = 0; l < LAYERS; l++) {
      const seen = new Set<string>();
      for (let i = 0; i < device.layers[l].length; i++) {
        const from = device.layers[l][i];
        const to = edit.layers[l][i];
        if (from === to) continue;
        const key = byslot.get(i);
        const id = key ? `${l}:${key.id}` : `${l}:s${i}`;
        if (seen.has(id)) continue;
        seen.add(id);
        const what =
          i === slot(KNOB_PRESS)
            ? t("ui.km.knob_press")
            : key
              ? baseLine(legends[key.id]) || key.id
              : t("ui.km.slot");
        const slots = key ? key.pos.map(slot) : [i];
        list.push({
          id,
          layer: l,
          what,
          from,
          to,
          revert: () => slots.forEach((s) => (edit!.layers[l][s] = device!.layers[l][s])),
        });
      }
    }
    for (let l = 0; l < KNOB_LAYERS; l++) {
      for (const dir of [0, 1] as const) {
        const from = device.knob[l][dir];
        const to = edit.knob[l][dir];
        if (from === to) continue;
        list.push({
          id: `${l}:knob${dir}`,
          layer: l,
          what: t(dir === 0 ? "ui.km.knob_left" : "ui.km.knob_right"),
          from,
          to,
          revert: () => (edit!.knob[l][dir] = device!.knob[l][dir]),
        });
      }
    }
    return list;
  });

  const nameOf = (code: number, l = layer) => describe(code, info.language, basicName, l).name;
  const hintOf = (code: number) => describe(code, info.language, basicName, layer).hint;

  function discard() {
    if (device) edit = structuredClone($state.snapshot(device) as Keymap);
    message = "";
  }

  async function write() {
    if (!edit) return;
    writing = true;
    error = "";
    progress = 0;
    try {
      const target = $state.snapshot(edit) as Keymap;
      const n = await api.keymapWrite(target);
      device = structuredClone(target);
      message = t("ui.km.written", { n });
    } catch (e) {
      error = String(e);
    } finally {
      writing = false;
    }
  }

  // --- ISO / ANSI ---------------------------------------------------------
  function setShape(shape: Shape) {
    if (settings.shape === shape) return;
    settings.shape = shape;
    const current = resolveLegends(settings.preset, settings.custom).preset;
    if (current.shape !== shape) settings.preset = BUILTIN_PRESETS.find((p) => p.shape === shape)!.id;
    if (selected && !layout(shape).some((k) => k.id === selected)) selected = null;
    saveSettings();
  }

  let detecting = $state(false);
  let detectResult = $state<"" | "iso" | "ansi" | "unknown">("");

  function detect() {
    detecting = true;
    detectResult = "";
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      window.removeEventListener("keydown", handler, true);
      detecting = false;
      if (e.code === "IntlBackslash") {
        detectResult = "iso";
        setShape("iso");
      } else if (e.code === "KeyZ") {
        detectResult = "ansi";
        setShape("ansi");
      } else if (e.code === "Escape") {
        detectResult = "";
      } else {
        detectResult = "unknown";
      }
    };
    window.addEventListener("keydown", handler, true);
  }

  // --- layer names --------------------------------------------------------
  let renaming = $state<number | null>(null);
  let renameText = $state("");

  function startRename(n: number) {
    renaming = n;
    renameText = settings.layerNames[n] ?? "";
  }

  function finishRename(keep: boolean) {
    if (renaming !== null && keep) {
      settings.layerNames[renaming] = renameText.trim().slice(0, 24);
      saveSettings();
    }
    renaming = null;
  }

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  // --- labels -------------------------------------------------------------
  let copyName = $state("");

  function choosePreset(id: string) {
    settings.preset = id;
    saveSettings();
  }

  function copyPreset() {
    const base = resolved.custom ? resolved.custom.base : resolved.preset.id;
    const own: CustomLegendPreset = {
      id: `c${Date.now()}`,
      name: copyName.trim() || t("ui.km.copy_of", { name: resolved.preset.name }),
      base,
      labels: { ...(resolved.custom?.labels ?? {}) },
    };
    settings.custom.push(own);
    settings.preset = own.id;
    copyName = "";
    saveSettings();
  }

  function deletePreset() {
    if (!resolved.custom) return;
    const base = resolved.custom.base;
    settings.custom = settings.custom.filter((c) => c.id !== resolved.custom!.id);
    settings.preset = base;
    saveSettings();
  }

  function labelLines(id: string): [string, string] {
    const lines = (legends[id] ?? "").split("\n");
    return lines.length > 1 ? [lines[0], lines[1]] : ["", lines[0]];
  }

  function setLabel(id: string, top: string, main: string) {
    const own = settings.custom.find((c) => c.id === settings.preset);
    if (!own) return;
    own.labels[id] = top.trim() ? `${top.trim()}\n${main.trim()}` : main.trim();
    saveSettings();
  }

  function resetLabel(id: string) {
    const own = settings.custom.find((c) => c.id === settings.preset);
    if (!own) return;
    delete own.labels[id];
    saveSettings();
  }

  // --- profiles -----------------------------------------------------------
  let profiles = $state<KeymapProfile[]>([]);
  let profileName = $state("");
  let profileApp = $state("");
  let profileBusy = $state(false);

  async function profileAction(fn: () => Promise<void>) {
    profileBusy = true;
    error = "";
    try {
      await fn();
    } catch (e) {
      error = String(e);
    } finally {
      profileBusy = false;
    }
  }

  const saveProfile = () =>
    profileAction(async () => {
      if (!edit) return;
      await api.keymapProfileSave(profileName, profileApp, $state.snapshot(edit) as Keymap);
      profileName = "";
      profileApp = "";
      profiles = await api.keymapProfiles();
      message = t("ui.km.profile_saved");
    });

  const loadProfile = (p: KeymapProfile) =>
    profileAction(async () => {
      edit = await api.keymapProfileGet(p.id);
      message = t("ui.km.profile_loaded", { name: p.name });
    });

  const exportProfile = (p: KeymapProfile) =>
    profileAction(async () => {
      const path = await api.keymapProfileExport(p.id);
      if (path) message = t("ui.km.profile_exported", { path });
    });

  const deleteProfile = (p: KeymapProfile) =>
    profileAction(async () => {
      await api.keymapProfileDelete(p.id);
      profiles = await api.keymapProfiles();
    });

  const importProfile = () =>
    profileAction(async () => {
      const p = await api.keymapProfileImport();
      if (p) {
        profiles = await api.keymapProfiles();
        message = t("ui.km.profile_imported", { name: p.name });
      }
    });
</script>

<svelte:window onkeydown={onKeyDown} onkeyup={onKeyUp} onblur={releaseAll} />

{#if overview && (device ?? edit)}
  <KeyFunctionsOverview
    keymap={(device ?? edit)!}
    {keys}
    {legends}
    lang={info.language}
    {basicName}
    {layerName}
    onclose={() => (overview = false)}
  />
{/if}

<div class="stack page">
  <div class="title">
    <h1>{t("ui.nav.keymap")}</h1>
  </div>
  <p class="muted intro">{t("ui.km.intro")}</p>

  {#if !ready}
    <UsbRequired feature="keymap" />
  {/if}

  <div class="card stack">
    <!-- layers and view options -->
    <div class="bar">
      <div class="layers" role="tablist" aria-label={t("ui.km.layers")}>
        {#each Array.from({ length: LAYERS }, (_, n) => n) as n (n)}
          {#if renaming === n}
            <input
              class="rename"
              maxlength="24"
              placeholder={defaultLayerName(n)}
              bind:value={renameText}
              use:focus
              onblur={() => finishRename(true)}
              onkeydown={(e) => {
                if (e.key === "Enter") finishRename(true);
                if (e.key === "Escape") finishRename(false);
              }}
            />
          {:else}
            <button
              role="tab"
              aria-selected={layer === n}
              onclick={() => (layer = n)}
              ondblclick={() => startRename(n)}
              title={t("ui.km.rename_hint")}
            >
              {#if settings.layerNames[n]?.trim()}<span class="num">{n}</span>{/if}{layerName(n)}
            </button>
          {/if}
        {/each}
        <button class="icon-btn" title={t("ui.km.rename")} aria-label={t("ui.km.rename")} onclick={() => startRename(layer)}>
          <Icon name="pencil" size={14} />
        </button>
      </div>

      <div class="view">
        <div class="pill" role="radiogroup" aria-label={t("ui.km.shape")}>
          <span class="thumb" class:right={settings.shape === "ansi"}></span>
          <button role="radio" aria-checked={settings.shape === "iso"} onclick={() => setShape("iso")}>ISO</button>
          <button role="radio" aria-checked={settings.shape === "ansi"} onclick={() => setShape("ansi")}>ANSI</button>
        </div>
        <button class="link" onclick={detect} disabled={detecting}>{t("ui.km.detect")}</button>
        <select
          class="labels"
          aria-label={t("ui.km.labels")}
          value={settings.preset}
          onchange={(e) => choosePreset(e.currentTarget.value)}
        >
          <optgroup label={t("ui.km.labels_builtin")}>
            {#each presetsForShape as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
          </optgroup>
          {#if customForShape.length}
            <optgroup label={t("ui.km.labels_own")}>
              {#each customForShape as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
            </optgroup>
          {/if}
        </select>
      </div>
    </div>

    {#if detecting}
      <div class="notice"><Icon name="info" />{t("ui.km.detect_prompt")}</div>
    {:else if detectResult}
      <div class="notice" class:warn={detectResult === "unknown"}>
        <Icon name={detectResult === "unknown" ? "alert" : "check"} />
        {t(`ui.km.detect_${detectResult}`)}
      </div>
    {/if}

    <div class="modebar">
      <div class="segmented" role="group">
        <button aria-pressed={mode === "assign"} onclick={() => (mode = "assign")}>{t("ui.km.mode_assign")}</button>
        <button aria-pressed={mode === "labels"} onclick={() => (mode = "labels")}>{t("ui.km.mode_labels")}</button>
      </div>
      {#if mode === "assign"}
        <span class="legend muted small">
          {#if layer === 0}<span class="sw remapped"></span>{t("ui.km.legend_remapped")}{/if}
          <span class="sw changed"></span>{t("ui.km.legend_changed")}
          {#if layer > 0}<span class="sw trns">▽</span>{t("ui.km.legend_trns")}{/if}
        </span>
      {/if}
      <button class="key small-key" onclick={() => (overview = true)} disabled={!edit}>
        <Icon name="info" />{t("ui.km.overview")}
      </button>
      <label class="switch light-switch" title={lightingAnimated ? t("ui.km.lighting_animated") : ""}>
        <input
          type="checkbox"
          checked={settings.showLighting}
          onchange={(e) => {
            settings.showLighting = e.currentTarget.checked;
            saveSettings();
          }}
        />
        <span class="small">{t("ui.km.show_lighting")}</span>
      </label>
    </div>
    {#if settings.showLighting && lightingAnimated}
      <p class="muted small">{t("ui.km.lighting_animated")}</p>
    {/if}

    {#if loading}
      <div class="progress">
        <span class="muted small">{t("ui.keymap.reading")}</span>
        <div class="pbar"><div class="fill" style="width: {Math.max(progress, 0.03) * 100}%"></div></div>
      </div>
    {/if}

    <KeyboardView
      {keys}
      {faces}
      {selected}
      {knobChanged}
      keyLight={shownLight(keyLight)}
      underLight={shownLight(underLight)}
      {pressed}
      knobTitle={t("ui.km.knob")}
      onselect={(id) => {
        selected = selected === id ? null : id;
        if (id === "knob" && layer >= KNOB_LAYERS) knobSlot = "press";
      }}
    >
      {#snippet screen()}<DisplayPreview {status} />{/snippet}
    </KeyboardView>
  </div>

  <div class="split">
    <!-- editor for the selected key -->
    <div class="card stack editor">
      {#if mode === "labels"}
        <h2>{t("ui.km.labels_title")}</h2>
        {#if !resolved.custom}
          <p class="muted small">{t("ui.km.labels_builtin_hint", { name: resolved.preset.name })}</p>
          <div class="row">
            <input class="grow" placeholder={t("ui.km.copy_of", { name: resolved.preset.name })} bind:value={copyName} maxlength="40" />
            <button class="key primary" onclick={copyPreset}>{t("ui.km.copy_preset")}</button>
          </div>
        {:else}
          <div class="row">
            <span class="muted small grow">{t("ui.km.labels_own_hint", { name: resolved.preset.name })}</span>
            <button class="key" onclick={deletePreset}><Icon name="trash" />{t("ui.km.delete_preset")}</button>
          </div>
          {#if selectedKey}
            {@const key = selectedKey}
            {@const [top, main] = labelLines(key.id)}
            <div class="label-edit">
              <label>
                {t("ui.km.label_top")}
                <input value={top} maxlength="8" onchange={(e) => setLabel(key.id, e.currentTarget.value, main)} />
              </label>
              <label>
                {t("ui.km.label_main")}
                <input value={main} maxlength="10" onchange={(e) => setLabel(key.id, top, e.currentTarget.value)} />
              </label>
              <button class="key" onclick={() => resetLabel(key.id)}>{t("ui.km.label_reset")}</button>
            </div>
          {:else}
            <p class="muted small">{t("ui.km.select_key_label")}</p>
          {/if}
        {/if}
      {:else if !edit}
        <h2>{t("ui.km.assign_title")}</h2>
        {#if ready}<p class="muted small">{t("ui.keymap.reading")}</p>{/if}
      {:else if selected === "knob" && knobCodes}
        <h2>{t("ui.km.knob")} · {layerName(layer)}</h2>
        <div class="segmented" role="group">
          <button aria-pressed={knobSlot === "left"} disabled={knobCodes.left === null} onclick={() => (knobSlot = "left")}>
            ↺ {t("ui.km.knob_left")}
          </button>
          <button aria-pressed={knobSlot === "right"} disabled={knobCodes.right === null} onclick={() => (knobSlot = "right")}>
            ↻ {t("ui.km.knob_right")}
          </button>
          <button aria-pressed={knobSlot === "press"} onclick={() => (knobSlot = "press")}>● {t("ui.km.knob_press")}</button>
        </div>
        {#if layer >= KNOB_LAYERS}
          <p class="muted small">{t("ui.km.knob_layers", { n: KNOB_LAYERS })}</p>
        {/if}
        {@const code = knobSlot === "press" ? knobCodes.press : (knobSlot === "left" ? knobCodes.left : knobCodes.right)}
        {#if code !== null}
          <div class="current">{t("ui.km.now")}: <strong>{nameOf(code)}</strong></div>
          {#if hintOf(code)}<p class="muted small hint">{hintOf(code)}</p>{/if}
          <KeyPicker
            {code}
            lang={info.language}
            {basicName}
            allowTrns={layer > 0}
            {layer}
            onpick={(c) => (knobSlot === "press" ? setPress(c) : setKnob(knobSlot === "left" ? 0 : 1, c))}
          />
        {/if}
      {:else if selectedKey}
        {@const key = selectedKey}
        {@const code = codeAt(edit, layer, key)}
        <h2>{baseLine(legends[selectedKey.id]) || selectedKey.id} · {layerName(layer)}</h2>
        {#if isLocked(selectedKey)}
          <div class="notice"><Icon name="lock" />{t("ui.km.fn_locked")}</div>
        {:else}
          <div class="current">
            {t("ui.km.now")}: <strong>{nameOf(code)}</strong>
            {#if layer === 0 && code !== selectedKey.hid}
              <button class="link" onclick={() => setKey(key, key.hid)}>{t("ui.km.back_to_default")}</button>
            {/if}
          </div>
          {#if hintOf(code)}<p class="muted small hint">{hintOf(code)}</p>{/if}
          <KeyPicker {code} lang={info.language} {basicName} allowTrns={layer > 0} {layer} onpick={(c) => setKey(key, c)} />
        {/if}
      {:else}
        <h2>{t("ui.km.assign_title")}</h2>
        <p class="muted small">{t("ui.km.select_key")}</p>
      {/if}
    </div>

    <!-- pending changes and write -->
    <div class="card stack side">
      <h2>{t("ui.km.changes_title")}</h2>
      {#if changes.length === 0}
        <p class="muted small">{t("ui.km.no_changes")}</p>
      {:else}
        <ul class="changes">
          {#each changes as c (c.id)}
            <li>
              <span class="badge">{layerName(c.layer)}</span>
              <span class="what">{c.what}</span>
              <span class="muted small arrow">{nameOf(c.from, c.layer)} → <strong>{nameOf(c.to, c.layer)}</strong></span>
              <button class="icon-btn" title={t("ui.km.revert")} aria-label={t("ui.km.revert")} onclick={c.revert}>
                <Icon name="x" size={14} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if writing}
        <div class="progress">
          <span class="muted small">{t("ui.keymap.writing")}</span>
          <div class="pbar"><div class="fill" style="width: {Math.max(progress, 0.03) * 100}%"></div></div>
        </div>
      {/if}
      {#if message}<div class="notice"><Icon name="check" /><span>{message}</span></div>{/if}
      {#if error}<div class="notice err selectable"><Icon name="alert" /><span>{error}</span></div>{/if}

      <div class="row">
        <button class="key" onclick={discard} disabled={!changes.length || writing}>{t("ui.km.discard")}</button>
        <button class="key primary" onclick={write} disabled={!ready || !changes.length || writing}>
          {t("ui.km.write", { n: changes.length })}
        </button>
      </div>
      <p class="muted small">
        {t("ui.km.auto_backup")}
        <button class="link" onclick={() => api.open("keymap_backups")}>{t("ui.km.open_backups")}</button>
      </p>
    </div>
  </div>

  <div class="split">
    <!-- profiles -->
    <div class="card stack">
      <div class="head">
        <h2>{t("ui.km.profiles")}</h2>
        <button class="key" onclick={importProfile} disabled={!ready || profileBusy}><Icon name="upload" />{t("ui.km.import")}</button>
      </div>
      <p class="muted small">{t("ui.km.profiles_hint")}</p>
      {#if profiles.length}
        <ul class="profiles">
          {#each profiles as p (p.id)}
            <li>
              <div class="pname">
                <strong>{p.name}</strong>
                <span class="muted small">{p.app ? `${p.app} · ` : ""}{dateTime(p.created)}</span>
              </div>
              <button class="key" onclick={() => loadProfile(p)} disabled={!edit || profileBusy}>{t("ui.km.load")}</button>
              <button class="icon-btn" title={t("ui.km.export")} aria-label={t("ui.km.export")} onclick={() => exportProfile(p)} disabled={profileBusy}>
                <Icon name="download" size={15} />
              </button>
              <button class="icon-btn" title={t("ui.km.delete")} aria-label={t("ui.km.delete")} onclick={() => deleteProfile(p)} disabled={profileBusy}>
                <Icon name="trash" size={15} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="save-profile">
        <input placeholder={t("ui.km.profile_name")} bind:value={profileName} maxlength="60" />
        <input placeholder={t("ui.km.profile_app")} bind:value={profileApp} maxlength="80" />
        <button class="key" onclick={saveProfile} disabled={!edit || !profileName.trim() || profileBusy}>
          {t("ui.km.save_profile")}
        </button>
      </div>
    </div>

    <KeymapBackup {status} onrestored={read} />
  </div>
</div>

<style>
  .page {
    container-type: inline-size;
  }

  .intro {
    margin: -8px 0 0;
    max-width: 72ch;
  }

  .small {
    font-size: 12px;
  }

  p.small {
    margin: 0;
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 16px;
    align-items: center;
    justify-content: space-between;
  }

  .layers {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }

  .layers button[role="tab"] {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font: 500 13px var(--font-ui);
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 5px 10px;
    cursor: pointer;
    max-width: 180px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .layers button[role="tab"]:hover {
    color: var(--text);
  }

  .layers button[aria-selected="true"] {
    color: var(--text);
    background: var(--panel-2);
    border-color: var(--border-strong);
  }

  .num {
    font: 600 11px var(--font-mono);
    color: var(--faint);
  }

  .layers button[aria-selected="true"] .num {
    color: var(--accent-text);
  }

  .rename {
    width: 140px;
    padding: 4px 8px;
    font-size: 13px;
  }

  .icon-btn {
    display: inline-grid;
    place-items: center;
    width: 28px;
    height: 28px;
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    flex: none;
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--panel-2);
  }

  .view {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  /* ISO / ANSI pill with a sliding thumb */
  .pill {
    position: relative;
    display: inline-grid;
    grid-template-columns: 1fr 1fr;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px;
  }

  .pill button {
    position: relative;
    z-index: 1;
    font: 600 12px var(--font-ui);
    letter-spacing: 0.04em;
    color: var(--muted);
    background: transparent;
    border: 0;
    padding: 5px 14px;
    border-radius: 999px;
    cursor: pointer;
    transition: color 0.16s var(--ease);
  }

  .pill button[aria-checked="true"] {
    color: var(--accent-ink);
  }

  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc(50% - 2px);
    border-radius: 999px;
    background: var(--accent);
    transition: transform 0.18s var(--ease);
  }

  .thumb.right {
    transform: translateX(100%);
  }

  .labels {
    min-width: 150px;
    max-width: 220px;
    padding-top: 5px;
    padding-bottom: 5px;
    font-size: 13px;
  }

  .link {
    font: 500 13px var(--font-ui);
    color: var(--accent-text);
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }

  .link:hover:not(:disabled) {
    text-decoration: underline;
  }

  .link:disabled {
    color: var(--faint);
    cursor: default;
  }

  .modebar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
    align-items: center;
  }

  .small-key {
    height: 32px;
    padding: 0 12px;
    font-size: 13px;
    margin-left: auto;
  }

  .light-switch {
    gap: 8px;
  }

  .legend {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .sw {
    display: inline-grid;
    place-items: center;
    width: 14px;
    height: 14px;
    border-radius: 3px;
    border: 1px solid var(--border-strong);
    font-size: 9px;
    margin-left: 8px;
  }

  .sw.remapped {
    background: var(--accent-soft);
    border-color: var(--accent-edge);
  }

  .sw.changed {
    border: 0;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warn);
  }

  .sw.trns {
    color: var(--faint);
  }

  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 380px);
    gap: 16px;
    align-items: start;
  }

  @container (max-width: 820px) {
    .split {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .editor {
    min-width: 0;
  }

  .hint {
    margin: -6px 0 0;
  }

  .current {
    display: flex;
    gap: 10px;
    align-items: baseline;
    flex-wrap: wrap;
    font-size: 14px;
  }

  .changes,
  .profiles {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 280px;
    overflow-y: auto;
  }

  .changes li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    grid-template-areas:
      "badge what x"
      "arrow arrow arrow";
    gap: 2px 8px;
    align-items: center;
    padding: 8px 10px;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .changes .badge {
    grid-area: badge;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .changes .what {
    grid-area: what;
    font-weight: 500;
  }

  .changes .arrow {
    grid-area: arrow;
  }

  .changes .icon-btn {
    grid-area: x;
  }

  .profiles li {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px 10px;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .pname {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .pname > * {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profiles .key {
    height: 30px;
    padding: 0 12px;
    font-size: 13px;
  }

  .save-profile {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
    gap: 8px;
  }

  @container (max-width: 560px) {
    .save-profile {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .grow {
    flex: 1;
    min-width: 0;
  }

  .label-edit {
    display: flex;
    gap: 12px;
    align-items: end;
    flex-wrap: wrap;
  }

  .label-edit input {
    width: 110px;
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .pbar {
    height: 6px;
    border-radius: 999px;
    background: var(--panel-3);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s var(--ease);
  }
</style>
