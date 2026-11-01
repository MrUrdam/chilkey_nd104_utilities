<script lang="ts" module>
  export interface KeyFace {
    /** Shifted legend (small, above) – only for printed legends. */
    top?: string;
    main: string;
    kind: "default" | "remapped" | "trns" | "none" | "locked";
    /** Differs from what is on the keyboard now. */
    changed: boolean;
    title: string;
  }
</script>

<script lang="ts">
  // Drawing of the keyboard. Scales with the available width but never
  // beyond a comfortable key size, and stays centred.
  import type { Snippet } from "svelte";
  import { DECOR, KNOB, LAYOUT_HEIGHT, LAYOUT_WIDTH, type PhysicalKey } from "../lib/keyboard";
  import type { Glow } from "../lib/glow";

  let {
    keys,
    faces,
    selected,
    knobChanged = false,
    knobTitle = "",
    onselect,
    screen,
    keyLight = null,
    underLight = null,
    pressed = new Set<string>(),
  }: {
    keys: PhysicalKey[];
    faces: Record<string, KeyFace>;
    selected: string | null;
    knobChanged?: boolean;
    knobTitle?: string;
    onselect: (id: string) => void;
    /** Content of the screen (e.g. the live preview). */
    screen?: Snippet;
    /** Light under the keys. */
    keyLight?: Glow | null;
    /** Bottom lighting: glow around the keyboard and the light bar below Del/End/PgDn. */
    underLight?: Glow | null;
    /** Keys held down right now (ids; "knob" for the knob). */
    pressed?: Set<string>;
  } = $props();

  /** Outline of the ISO Enter in px, relative to its box (wide top, notch bottom left). */
  function isoEnter(k: PhysicalKey): string {
    const w = (k.w - GAP) * unit;
    const h = (k.h - GAP) * unit;
    const notchX = 0.25 * unit;
    const notchY = (1 - GAP) * unit;
    const r = Math.max(2, unit * 0.1);
    return [
      `M ${r} 0.5 H ${w - r} Q ${w - 0.5} 0.5 ${w - 0.5} ${r}`,
      `V ${h - r} Q ${w - 0.5} ${h - 0.5} ${w - r} ${h - 0.5}`,
      `H ${notchX + r} Q ${notchX + 0.5} ${h - 0.5} ${notchX + 0.5} ${h - r}`,
      `V ${notchY + r} Q ${notchX + 0.5} ${notchY - 0.5} ${notchX - r} ${notchY - 0.5}`,
      `H ${r} Q 0.5 ${notchY - 0.5} 0.5 ${notchY - r}`,
      `V ${r} Q 0.5 0.5 ${r} 0.5 Z`,
    ].join(" ");
  }

  /** Room around the keyboard for the glow. */
  const PAD = 18;

  const glowVars = (g: Glow, prefix: string) =>
    `--${prefix}-h: ${g.h}; --${prefix}-s: ${g.s}%; --${prefix}-l: ${g.l}%; --${prefix}-a: ${g.a}; --${prefix}-d: ${g.duration}s;`;

  const MAX_UNIT = 46;
  const MIN_UNIT = 20;
  const GAP = 0.1;

  let width = $state(0);
  // 2 px slack: clientWidth is rounded and 22.5 units can end on a half pixel
  let unit = $derived(Math.max(MIN_UNIT, Math.min(MAX_UNIT, Math.floor((width - 2 - 2 * PAD) / LAYOUT_WIDTH))));
  /** Scroll only when even the smallest keys don't fit. */
  let tooNarrow = $derived(width > 0 && width - 2 - 2 * PAD < MIN_UNIT * LAYOUT_WIDTH);
  let fontMain = $derived(Math.max(9, Math.round(unit * 0.3)));
  let fontSmall = $derived(Math.max(8, Math.round(unit * 0.24)));

  const px = (v: number) => `${v * unit}px`;
</script>

<div class="fit" class:scroll={tooNarrow} style="padding: {PAD}px" bind:clientWidth={width}>
  <div
    class="board"
    class:lit={keyLight}
    class:lit-pulse={keyLight?.fx === "pulse"}
    class:lit-cycle={keyLight?.fx === "cycle"}
    class:bar-lit={underLight}
    style="width: {px(LAYOUT_WIDTH)}; height: {px(LAYOUT_HEIGHT)}; --fm: {fontMain}px; --fs: {fontSmall}px; --u: {unit}px; {keyLight
      ? glowVars(keyLight, 'lit')
      : ''} {underLight ? glowVars(underLight, 'glow') : ''}"
  >
    {#if underLight}
      <div
        class="under"
        class:glow-pulse={underLight.fx === "pulse"}
        class:glow-cycle={underLight.fx === "cycle"}
        style={glowVars(underLight, "glow")}
      ></div>
    {/if}

    {#each DECOR as d (d.kind)}
      <div
        class="decor {d.kind}"
        class:live={d.kind === "screen" && screen}
        style="left: {px(d.x + GAP / 2)}; top: {px(d.y + GAP / 2)}; width: {px(d.w - GAP)}; height: {px(d.h - GAP)}"
      >
        {#if d.kind === "screen" && screen}{@render screen()}{/if}
      </div>
    {/each}

    {#each keys as k (k.id)}
      {@const f = faces[k.id]}
      <button
        class="cap {f?.kind ?? 'default'}"
        class:mod={k.mod}
        class:round={k.round}
        class:changed={f?.changed}
        class:shaped={k.shape}
        class:down={pressed.has(k.id)}
        aria-pressed={selected === k.id}
        title={f?.title}
        style="left: {px(k.x + GAP / 2)}; top: {px(k.y + GAP / 2)}; width: {px(k.w - GAP)}; height: {px(k.h - GAP)}"
        onclick={() => onselect(k.id)}
      >
        {#if k.shape === "iso-enter"}
          <svg class="outline" aria-hidden="true"><path d={isoEnter(k)} /></svg>
        {/if}
        {#if f?.top}<span class="top">{f.top}</span>{/if}
        <span class="main" class:long={(f?.main.length ?? 0) > 3}>{f?.main ?? ""}</span>
      </button>
    {/each}

    <button
      class="knob"
      class:changed={knobChanged}
      class:down={pressed.has("knob")}
      aria-pressed={selected === "knob"}
      title={knobTitle}
      style="left: {px(KNOB.x + GAP / 2)}; top: {px(KNOB.y + GAP / 2)}; width: {px(KNOB.w - GAP)}; height: {px(KNOB.h - GAP)}"
      onclick={() => onselect("knob")}
    >
      <span class="grip"></span>
    </button>
  </div>
</div>

<style>
  .fit {
    width: 100%;
    box-sizing: border-box;
    overflow: hidden;
    display: flex;
  }

  .fit.scroll {
    overflow-x: auto;
  }

  .board {
    position: relative;
    margin: 0 auto;
    flex: none;
  }

  .cap,
  .knob {
    position: absolute;
    box-sizing: border-box;
    padding: 0 2px;
    font-family: var(--font-ui);
    color: var(--text);
    background: var(--panel-2);
    border: 1px solid var(--border-strong);
    border-bottom-width: 3px;
    border-bottom-color: var(--key-edge);
    border-radius: calc(var(--radius-sm) * 0.8);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0;
    line-height: 1.1;
    overflow: hidden;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .cap.mod {
    background: var(--panel-3);
  }

  .cap:hover,
  .knob:hover {
    border-color: var(--accent-edge);
  }

  /* a key held down on the keyboard right now */
  .cap.down,
  .knob.down {
    background: var(--accent-soft);
    border-color: var(--accent);
    border-bottom-width: 1px;
    transform: translateY(1px);
    transition: none;
  }

  .cap.down .main,
  .cap.down .top {
    color: var(--accent-text);
  }

  /* ISO Enter: the outline is drawn, the button itself is only its box */
  .cap.shaped {
    background: none !important;
    border: 0 !important;
    box-shadow: none !important;
    outline: 0 !important;
    /* own stacking context: the outline sits behind the label, not behind the page */
    isolation: isolate;
    pointer-events: none;
    justify-content: flex-start;
    padding-top: calc(var(--u) * 0.22);
    padding-left: calc(var(--u) * 0.25);
  }

  .outline {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    z-index: -1;
  }

  .outline path {
    pointer-events: visiblePainted;
    fill: var(--panel-3);
    stroke: var(--border-strong);
    stroke-width: 1;
    cursor: pointer;
    transition:
      fill 0.12s var(--ease),
      stroke 0.12s var(--ease);
  }

  .cap.shaped:hover .outline path {
    stroke: var(--accent-edge);
  }

  .cap.shaped.remapped .outline path {
    fill: var(--accent-soft);
    stroke: var(--accent-edge);
  }

  .cap.shaped[aria-pressed="true"] .outline path {
    stroke: var(--accent);
    stroke-width: 2;
  }

  .cap.shaped.down .outline path {
    fill: var(--accent-soft);
    stroke: var(--accent);
  }

  .board.lit .cap.shaped .outline {
    filter: drop-shadow(0 0 calc(var(--u) * 0.12) var(--lit-soft));
  }

  .cap[aria-pressed="true"],
  .knob[aria-pressed="true"] {
    border-color: var(--accent);
    outline: 2px solid var(--accent-soft);
    outline-offset: 1px;
  }

  /* --- lighting read from the keyboard ---------------------------------- */
  .board {
    --lit: hsl(var(--lit-h) var(--lit-s) var(--lit-l) / var(--lit-a));
    --lit-soft: hsl(var(--lit-h) var(--lit-s) var(--lit-l) / calc(var(--lit-a) * 0.45));
  }

  .board.lit .cap,
  .board.lit .knob {
    box-shadow: 0 0 calc(var(--u) * 0.12) var(--lit-soft);
    border-bottom-color: var(--lit);
  }

  .board.lit .cap .main,
  .board.lit .cap .top {
    color: color-mix(in srgb, var(--text) 85%, hsl(var(--lit-h) var(--lit-s) var(--lit-l)));
  }

  .board.lit .cap.remapped .main {
    color: var(--accent-text);
  }

  .board.lit-pulse {
    animation: lit-pulse var(--lit-d) ease-in-out infinite;
  }

  .board.lit-cycle {
    animation: lit-cycle var(--lit-d) linear infinite;
  }

  .under {
    position: absolute;
    inset: calc(var(--u) * 0.2);
    border-radius: calc(var(--u) * 0.4);
    box-shadow: 0 0 calc(var(--u) * 0.5) 0 hsl(var(--glow-h) var(--glow-s) var(--glow-l) / calc(var(--glow-a) * 0.6));
    pointer-events: none;
  }

  .under.glow-pulse {
    animation: glow-pulse var(--glow-d) ease-in-out infinite;
  }

  .under.glow-cycle {
    animation: glow-cycle var(--glow-d) linear infinite;
  }

  @keyframes lit-pulse {
    50% {
      --lit-a: 0.06;
    }
  }

  @keyframes lit-cycle {
    from {
      --lit-h: 0;
    }
    to {
      --lit-h: 360;
    }
  }

  @keyframes glow-pulse {
    50% {
      --glow-a: 0.05;
    }
  }

  @keyframes glow-cycle {
    from {
      --glow-h: 0;
    }
    to {
      --glow-h: 360;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .board,
    .under {
      animation: none !important;
    }
  }

  .round,
  .knob {
    border-radius: 50%;
  }

  .knob {
    background: radial-gradient(circle at 50% 40%, var(--panel-3), var(--panel-2));
  }

  .grip {
    width: 2px;
    height: 32%;
    margin-bottom: 30%;
    border-radius: 2px;
    background: var(--muted);
  }

  .main {
    font-size: var(--fm);
    font-weight: 500;
    white-space: nowrap;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .main.long {
    font-size: var(--fs);
  }

  .top {
    font-size: var(--fs);
    color: var(--muted);
  }

  .remapped {
    border-color: var(--accent-edge);
    background: var(--accent-soft);
  }

  .cap.remapped .main {
    color: var(--accent-text);
  }

  .trns .main,
  .trns .top {
    color: var(--faint);
    font-weight: 400;
  }

  .none {
    background: transparent !important;
    border-style: dashed;
  }

  .locked {
    cursor: not-allowed;
  }

  .locked .main {
    color: var(--muted);
  }

  .changed::after {
    content: "";
    position: absolute;
    top: 3px;
    right: 3px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn);
  }

  .decor {
    position: absolute;
    box-sizing: border-box;
    border-radius: calc(var(--radius-sm) * 0.8);
  }

  .decor.screen {
    background: #0b0d10;
    border: 1px solid var(--border-strong);
  }

  /* live preview: the preview brings its own bezel */
  .decor.screen.live {
    background: none;
    border: 0;
    display: flex;
    align-items: center;
    pointer-events: none;
  }

  .decor.screen.live :global(.bezel) {
    box-shadow: none;
  }

  /* light bar below Del / End / PgDn (part of the background lighting) */
  .decor.bar {
    border-radius: 999px;
    background: var(--panel-3);
    border: 1px solid var(--border);
  }

  /* the bar belongs to the bottom lighting; it is a light source itself, so
     it shows the solid colour, not the soft glow */
  .board.bar-lit .decor.bar {
    background: hsl(var(--glow-h) var(--glow-s) var(--glow-l));
    border-color: transparent;
    box-shadow: 0 0 calc(var(--u) * 0.2) hsl(var(--glow-h) var(--glow-s) var(--glow-l) / calc(var(--glow-a) * 0.6));
  }
</style>
