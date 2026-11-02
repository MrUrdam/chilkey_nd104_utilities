<script lang="ts">
  // Keyboard lighting: on/off, effect, brightness, speed and colour for both
  // zones. Like Chilkey's web driver, every change is sent and stored in the
  // keyboard right away (slider moves are bundled first).
  import { onDestroy } from "svelte";
  import { api, type LightChange, type LightZone, type Status } from "../lib/api";
  import { EFFECTS, EFFECT_OFF, EFFECT_SOLID, effect as effectInfo } from "../lib/effects";
  import { t } from "../lib/i18n.svelte";
  import Icon from "../components/Icon.svelte";
  import UsbRequired from "../components/UsbRequired.svelte";

  let { status }: { status: Status } = $props();

  const zones: LightZone[] = ["per_key", "bottom"];
  const SEND_DELAY = 200;

  interface ZoneUi {
    loading: boolean;
    error: string | null;
    on: boolean;
    /** Current effect, or the one to restore when switching on. */
    effect: number;
    brightness: number;
    speed: number;
    color: string;
  }

  const blank = (): ZoneUi => ({
    loading: true,
    error: null,
    on: true,
    effect: EFFECT_SOLID,
    brightness: 128,
    speed: 128,
    color: "#22d3ee",
  });
  let ui = $state<Record<LightZone, ZoneUi>>({ per_key: blank(), bottom: blank() });
  let ready = $derived(status.device.kind === "ready");

  // --- colour helpers (same scale as the keyboard: hue/sat 0–255) ---------
  function hexToHueSat(hex: string): [number, number] {
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
    const max = Math.max(r, g, b);
    const d = max - Math.min(r, g, b);
    let h = 0;
    if (d !== 0) {
      if (max === r) h = (60 * ((((g - b) / d) % 6) + 6)) % 360;
      else if (max === g) h = 60 * ((b - r) / d + 2);
      else h = 60 * ((r - g) / d + 4);
    }
    const s = max === 0 ? 0 : d / max;
    return [Math.min(255, Math.round((h / 360) * 255)), Math.round(s * 255)];
  }

  function hueSatToHex(hue: number, sat: number): string {
    const h = (hue * 360) / 255;
    const s = sat / 255;
    const x = s * (1 - Math.abs(((h / 60) % 2) - 1));
    const [r, g, b] =
      h < 60 ? [s, x, 0] : h < 120 ? [x, s, 0] : h < 180 ? [0, s, x] : h < 240 ? [0, x, s] : h < 300 ? [x, 0, s] : [s, 0, x];
    const m = 1 - s;
    return "#" + [r, g, b].map((v) => Math.round((v + m) * 255).toString(16).padStart(2, "0")).join("");
  }

  // --- remembered effect (per-viewer convenience) -------------------------
  function rememberEffect(zone: LightZone, value: number) {
    try {
      localStorage.setItem(`nd104.light.${zone}.effect`, String(value));
    } catch {
      /* storage unavailable: switching on uses the solid colour */
    }
  }

  function rememberedEffect(zone: LightZone): number {
    try {
      const v = Number(localStorage.getItem(`nd104.light.${zone}.effect`));
      return Number.isInteger(v) && v > 0 && v < EFFECTS[zone].length ? v : EFFECT_SOLID;
    } catch {
      return EFFECT_SOLID;
    }
  }

  // --- reading -------------------------------------------------------------
  async function load(zone: LightZone) {
    const z = ui[zone];
    z.loading = true;
    z.error = null;
    try {
      const s = await api.lightRead(zone);
      if (s.effect !== null) {
        z.on = s.effect !== EFFECT_OFF;
        z.effect = z.on ? s.effect : rememberedEffect(zone);
        if (z.on) rememberEffect(zone, s.effect);
      } else {
        z.effect = rememberedEffect(zone);
      }
      if (s.brightness !== null) z.brightness = s.brightness;
      if (s.speed !== null) z.speed = s.speed;
      if (s.hue !== null && s.sat !== null) z.color = hueSatToHex(s.hue, s.sat);
    } catch (e) {
      z.error = String(e);
    } finally {
      z.loading = false;
    }
  }

  let loadedFor = "";
  $effect(() => {
    // read again whenever the keyboard (re)connects
    const key = ready ? JSON.stringify(status.device) : "";
    if (key && key !== loadedFor) {
      loadedFor = key;
      zones.forEach(load);
    }
  });

  // --- sending (bundled, then stored like the web driver does) -----------
  const pending: Record<LightZone, Map<string, LightChange>> = { per_key: new Map(), bottom: new Map() };
  const timers: Partial<Record<LightZone, ReturnType<typeof setTimeout>>> = {};
  let busy = $state<Record<LightZone, boolean>>({ per_key: false, bottom: false });

  function queue(zone: LightZone, ...changes: LightChange[]) {
    for (const c of changes) pending[zone].set(c.field, c);
    clearTimeout(timers[zone]);
    timers[zone] = setTimeout(() => flush(zone), SEND_DELAY);
  }

  async function flush(zone: LightZone) {
    timers[zone] = undefined;
    // effect first: a new effect may reset the other values
    const order = ["effect", "brightness", "speed", "color"];
    const changes = [...pending[zone].values()].sort((a, b) => order.indexOf(a.field) - order.indexOf(b.field));
    pending[zone].clear();
    if (!changes.length) return;
    busy[zone] = true;
    try {
      await api.lightApply(zone, changes, true);
      ui[zone].error = null;
    } catch (e) {
      ui[zone].error = String(e);
    } finally {
      busy[zone] = false;
    }
  }

  function setPower(zone: LightZone, on: boolean) {
    const z = ui[zone];
    z.on = on;
    if (!on) rememberEffect(zone, z.effect);
    queue(zone, { field: "effect", value: on ? z.effect : EFFECT_OFF });
  }

  function setEffect(zone: LightZone, value: number) {
    const z = ui[zone];
    z.effect = value;
    rememberEffect(zone, value);
    // values the user sees are sent along, so the new effect looks as shown
    const [hue, sat] = hexToHueSat(z.color);
    queue(
      zone,
      { field: "effect", value },
      { field: "brightness", value: z.brightness },
      { field: "color", hue, sat },
    );
  }

  function setBrightness(zone: LightZone, value: number) {
    ui[zone].brightness = value;
    queue(zone, { field: "brightness", value });
  }

  function setSpeed(zone: LightZone, value: number) {
    ui[zone].speed = value;
    queue(zone, { field: "speed", value });
  }

  function setColor(zone: LightZone, hex: string) {
    ui[zone].color = hex;
    const [hue, sat] = hexToHueSat(hex);
    queue(zone, { field: "color", hue, sat });
  }

  onDestroy(() => {
    // send what is still pending when the page is left
    for (const zone of zones) {
      if (timers[zone]) {
        clearTimeout(timers[zone]);
        flush(zone);
      }
    }
  });

  const presets = ["#ffffff", "#ff0000", "#ff8800", "#ffee00", "#00ff44", "#22d3ee", "#0055ff", "#aa00ff", "#ff00aa"];
  const pct = (v: number) => Math.round((v / 255) * 100);
</script>

<div class="stack">
  <h1>{t("ui.nav.lighting")}</h1>
  <p class="muted intro">{t("ui.light.intro")}</p>

  {#if !ready}
    <UsbRequired feature="lighting" />
  {/if}

  {#each zones as zone (zone)}
    {@const z = ui[zone]}
    {@const fx = effectInfo(zone, z.effect)}
    {@const usable = ready && !z.loading && z.on}
    <div class="card stack" class:disabled={!ready}>
      <div class="head">
        <div>
          <h2>{t(`ui.light.${zone}`)}</h2>
          <span class="muted small">{t(`ui.light.${zone}_hint`)}</span>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            checked={z.on}
            disabled={!ready || z.loading}
            onchange={(e) => setPower(zone, e.currentTarget.checked)}
          />{z.on ? t("ui.light.on") : t("ui.light.off")}
        </label>
      </div>

      <div class="field-row" class:off={!z.on}>
        <span class="label">{t("ui.light.effect")}</span>
        <select
          value={z.effect}
          disabled={!usable}
          onchange={(e) => setEffect(zone, Number(e.currentTarget.value))}
        >
          {#each EFFECTS[zone].filter((f) => f.value !== EFFECT_OFF) as f (f.value)}
            <option value={f.value}>{t(`fx.${zone}.${f.value}`)}</option>
          {/each}
        </select>
      </div>

      <div class="field-row" class:off={!z.on}>
        <span class="label">{t("ui.light.brightness")}</span>
        <input
          type="range"
          min="0"
          max="255"
          step="1"
          value={z.brightness}
          disabled={!usable}
          oninput={(e) => setBrightness(zone, Number(e.currentTarget.value))}
          style="--p: {z.brightness / 255}"
        />
        <span class="mono value">{pct(z.brightness)} %</span>
      </div>

      <div class="field-row" class:off={!z.on || !fx.animated}>
        <span class="label">{t("ui.light.speed")}</span>
        <input
          type="range"
          min="0"
          max="255"
          step="1"
          value={z.speed}
          disabled={!usable || !fx.animated}
          title={fx.animated ? undefined : t("ui.light.speed_static")}
          oninput={(e) => setSpeed(zone, Number(e.currentTarget.value))}
          style="--p: {z.speed / 255}"
        />
        <span class="mono value">{pct(z.speed)} %</span>
      </div>

      <div class="field-row" class:off={!z.on || !fx.color}>
        <span class="label">{t("ui.light.color")}</span>
        <div class="colors">
          <input
            type="color"
            value={z.color}
            disabled={!usable || !fx.color}
            oninput={(e) => setColor(zone, e.currentTarget.value)}
            aria-label={t("ui.light.color")}
          />
          {#each presets as p (p)}
            <button
              class="swatch"
              style="background: {p}"
              aria-label={p}
              aria-pressed={z.color.toLowerCase() === p}
              disabled={!usable || !fx.color}
              onclick={() => setColor(zone, p)}
            ></button>
          {/each}
        </div>
      </div>
      {#if z.on && !fx.color}
        <div class="own-colors muted small">
          {t("ui.light.own_colors")}
          <button class="link" disabled={!usable} onclick={() => setEffect(zone, EFFECT_SOLID)}>
            {t("ui.light.use_solid")}
          </button>
        </div>
      {/if}

      <div class="foot">
        {#if z.loading && ready}
          <span class="muted small">{t("ui.light.reading")}</span>
        {:else if z.error}
          <span class="error-text selectable">{z.error}</span>
        {:else if busy[zone]}
          <span class="muted small">{t("ui.light.sending")}</span>
        {/if}
      </div>
    </div>
  {/each}

  <p class="muted small">{t("ui.light.effects_hint")}</p>
</div>

<style>
  .intro {
    margin: -6px 0 0;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
  }

  .head h2 {
    margin-bottom: 4px;
  }

  .small {
    font-size: 12px;
  }

  p.small {
    margin: 0;
  }

  .card.disabled {
    opacity: 0.6;
  }

  .field-row {
    display: grid;
    grid-template-columns: 130px 1fr 56px;
    align-items: center;
    gap: 14px;
    transition: opacity 0.2s;
  }

  .field-row.off {
    opacity: 0.45;
  }

  .field-row select {
    grid-column: 2 / 4;
    max-width: 360px;
  }

  .label {
    color: var(--muted);
    font-size: 13px;
  }

  .value {
    text-align: right;
    font-size: 13px;
  }

  /* track and thumb styled explicitly: the thumb (18 px, border included)
     reaches both ends, and the filled part ends at its centre */
  input[type="range"] {
    appearance: none;
    width: 100%;
    height: 18px;
    margin: 0;
    padding: 0;
    border: 0;
    box-shadow: none;
    background: transparent;
    cursor: pointer;
  }

  input[type="range"]::-webkit-slider-runnable-track {
    height: 6px;
    border-radius: 999px;
    background: linear-gradient(
      90deg,
      var(--accent) calc(9px + (100% - 18px) * var(--p)),
      var(--panel-3) calc(9px + (100% - 18px) * var(--p))
    );
  }

  input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    box-sizing: border-box;
    width: 18px;
    height: 18px;
    margin-top: -6px;
    border-radius: 50%;
    background: #ffffff;
    border: 2px solid var(--accent);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .colors {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    grid-column: 2 / 4;
  }

  input[type="color"] {
    width: 44px;
    height: 30px;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--panel-3);
    cursor: pointer;
  }

  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    cursor: pointer;
    padding: 0;
  }

  .swatch[aria-pressed="true"] {
    border-color: var(--text);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .swatch:disabled,
  input:disabled,
  select:disabled {
    cursor: default;
  }

  .own-colors {
    margin: -6px 0 0 144px;
  }

  .foot {
    min-height: 18px;
  }
</style>
