<script lang="ts">
  // Overview of the keyboard's own functions (connection, lighting, display,
  // layers …) with the keys they are on in the current keymap.
  import type { Keymap } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { Legends, PhysicalKey } from "../lib/keyboard";
  import { describe, type Category } from "../lib/keycodes";
  import Icon from "./Icon.svelte";

  let {
    keymap,
    keys,
    legends,
    lang,
    basicName,
    layerName,
    onclose,
  }: {
    keymap: Keymap;
    keys: PhysicalKey[];
    legends: Legends;
    lang: string;
    basicName: (code: number) => string | null;
    layerName: (n: number) => string;
    onclose: () => void;
  } = $props();

  const COLS = 16;
  const KNOB_LAYERS = 4;
  const KNOB_PRESS = 6 * COLS + 9;
  /** Categories that are about the keyboard itself (not ordinary keys). */
  const SHOWN: (Category | null)[] = ["special", "light", "layer", null];

  let dialog: HTMLDialogElement;

  function open(node: HTMLDialogElement) {
    node.showModal();
  }

  /** How a layer is reached, as a key prefix. */
  function prefix(l: number): string {
    if (l === 0) return "";
    if (l === 1) return "Fn + ";
    if (l === 2) return "Fn + Shift + ";
    return `${layerName(l)}: `;
  }

  function keyLabel(k: PhysicalKey): string {
    const text = (legends[k.id] ?? k.id).split("\n").pop() || k.id;
    return k.id === "rshift" || k.id === "rctrl" ? t("ui.km.right_key", { key: text }) : text;
  }

  interface Row {
    name: string;
    hint?: string;
    keys: string[];
    order: number;
    category: Category | null;
  }

  let rows = $derived.by(() => {
    const byName = new Map<string, Row>();
    const add = (code: number, l: number, where: string) => {
      if (code <= 1) return;
      const d = describe(code, lang, basicName, l);
      if (!SHOWN.includes(d.category)) return;
      const row = byName.get(d.name) ?? {
        name: d.name,
        // the Fn + Shift meaning has its own row here
        hint: d.hint?.replace(/\s*(With|Mit) Fn \+ Shift: [^.]*\./, "").trim() || undefined,
        keys: [],
        order: SHOWN.indexOf(d.category),
        category: d.category,
      };
      const text = prefix(l) + where;
      if (!row.keys.includes(text)) row.keys.push(text);
      byName.set(d.name, row);
    };
    keymap.layers.forEach((layer, l) => {
      for (const k of keys) add(layer[k.pos[0][0] * COLS + k.pos[0][1]], l, keyLabel(k));
      add(layer[KNOB_PRESS], l, t("ui.km.ov_knob_press"));
      if (l < KNOB_LAYERS) {
        add(keymap.knob[l][0], l, t("ui.km.ov_knob_left"));
        add(keymap.knob[l][1], l, t("ui.km.ov_knob_right"));
      }
    });
    return [...byName.values()].sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
  });

  const builtIn = $derived([
    { name: t("ui.km.ov_battery"), keys: [t("ui.km.ov_battery_keys")] },
    { name: t("ui.km.ov_reset"), keys: [t("ui.km.ov_reset_keys")] },
  ]);
</script>

<dialog bind:this={dialog} use:open onclose={onclose} onclick={(e) => e.target === dialog && dialog.close()}>
  <div class="box">
    <div class="head">
      <h2>{t("ui.km.overview")}</h2>
      <button class="icon-btn" aria-label={t("ui.keymap.close")} onclick={() => dialog.close()}><Icon name="x" /></button>
    </div>
    <p class="muted small">{t("ui.km.ov_intro")}</p>

    <div class="scroll">
      <table>
        <thead>
          <tr><th>{t("ui.km.ov_function")}</th><th>{t("ui.km.ov_keys")}</th></tr>
        </thead>
        <tbody>
          {#each rows as r, i (r.name)}
            {#if i === 0 || rows[i - 1].category !== r.category}
              <tr class="section">
                <td colspan="2">{r.category ? t(`ui.km.cat.${r.category}`) : t("ui.km.ov_other")}</td>
              </tr>
            {/if}
            <tr>
              <td>
                <strong>{r.name}</strong>
                {#if r.hint}<div class="muted small">{r.hint}</div>{/if}
              </td>
              <td>{#each r.keys as k (k)}<span class="combo">{k}</span>{/each}</td>
            </tr>
          {/each}
          <tr class="section"><td colspan="2">{t("ui.km.ov_builtin")}</td></tr>
          {#each builtIn as r (r.name)}
            <tr>
              <td><strong>{r.name}</strong></td>
              <td>{#each r.keys as k (k)}<span class="combo">{k}</span>{/each}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--text);
    width: min(820px, calc(100vw - 48px));
    max-height: calc(100vh - 64px);
    box-shadow: 0 18px 48px rgb(0 0 0 / 0.35);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }

  .box {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 20px 24px;
    max-height: calc(100vh - 66px);
    box-sizing: border-box;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .head h2 {
    margin: 0;
  }

  .small {
    font-size: 12px;
  }

  p.small {
    margin: 0;
  }

  .scroll {
    overflow-y: auto;
    min-height: 0;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    position: sticky;
    top: 0;
    background: var(--panel);
    text-align: left;
    color: var(--muted);
    font-weight: 600;
    border-bottom: 1px solid var(--border);
    padding: 6px 8px;
  }

  td {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  td:first-child {
    width: 55%;
  }

  .section td {
    color: var(--muted);
    font-weight: 600;
    padding-top: 16px;
  }

  .combo {
    display: inline-block;
    margin: 0 6px 4px 0;
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    background: var(--panel-2);
    font-size: 12px;
    white-space: nowrap;
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
  }

  .icon-btn:hover {
    color: var(--text);
    background: var(--panel-2);
  }
</style>
