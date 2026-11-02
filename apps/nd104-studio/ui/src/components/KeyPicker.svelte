<script lang="ts">
  // Chooses a function for a key: categories and a search over all names.
  import { CATEGORIES, KC_NO, KC_TRNS, KEYS, describe, keysIn, type Category } from "../lib/keycodes";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let {
    code,
    lang,
    basicName,
    allowTrns,
    onpick,
    layer = 0,
  }: {
    code: number;
    lang: string;
    basicName: (code: number) => string | null;
    allowTrns: boolean;
    onpick: (code: number) => void;
    /** Layer being edited (some codes mean something else on layer 2). */
    layer?: number;
  } = $props();

  let category = $state<Category>("basic");
  let query = $state("");

  const nameOf = (c: number) => describe(c, lang, basicName, layer).name;
  function tooltip(c: number): string {
    const d = describe(c, lang, basicName, layer);
    return d.hint ? `${d.name}\n${d.hint}` : d.name;
  }

  let list = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return keysIn(category);
    return KEYS.filter((k) => {
      if (k.code === KC_NO || k.code === KC_TRNS) return false;
      const name = nameOf(k.code).toLowerCase();
      return name.includes(q) || k.en.toLowerCase().includes(q) || (k.short ?? "").toLowerCase().includes(q);
    });
  });
</script>

<div class="picker">
  <div class="tools">
    <label class="search">
      <Icon name="search" />
      <input type="search" placeholder={t("ui.km.search")} bind:value={query} />
    </label>
    <div class="row quick">
      <button class="key small" aria-pressed={code === KC_NO} onclick={() => onpick(KC_NO)}>{t("ui.km.none")}</button>
      {#if allowTrns}
        <button class="key small" aria-pressed={code === KC_TRNS} onclick={() => onpick(KC_TRNS)}>
          ▽ {t("ui.km.trns")}
        </button>
      {/if}
    </div>
  </div>

  {#if !query.trim()}
    <div class="cats" role="tablist">
      {#each CATEGORIES as c (c)}
        <button role="tab" aria-selected={category === c} onclick={() => (category = c)}>{t(`ui.km.cat.${c}`)}</button>
      {/each}
    </div>
  {/if}

  <div class="grid">
    {#each list as k (k.code)}
      <button class="fn" aria-pressed={k.code === code} title={tooltip(k.code)} onclick={() => onpick(k.code)}>
        {nameOf(k.code)}
      </button>
    {:else}
      <span class="muted small">{t("ui.km.no_match")}</span>
    {/each}
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  .tools {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }

  .search {
    flex: 1 1 200px;
    display: flex;
    align-items: center;
    gap: 8px;
    position: relative;
    color: var(--faint);
  }

  .search :global(svg) {
    position: absolute;
    left: 10px;
  }

  .search input {
    width: 100%;
    padding-left: 32px;
  }

  .quick {
    gap: 8px;
  }

  .key.small {
    height: 32px;
    padding: 0 10px;
    font-size: 13px;
  }

  .key.small[aria-pressed="true"] {
    border-color: var(--accent);
  }

  .cats {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 8px;
  }

  .cats button {
    font: 500 13px var(--font-ui);
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 999px;
    padding: 4px 12px;
    cursor: pointer;
  }

  .cats button:hover {
    color: var(--text);
  }

  .cats button[aria-selected="true"] {
    color: var(--text);
    background: var(--panel-2);
    border-color: var(--border-strong);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
    /* fixed row height: buttons clip their text, so rows would otherwise shrink to fit max-height */
    grid-auto-rows: 34px;
    align-content: start;
    gap: 6px;
    max-height: 260px;
    overflow-y: auto;
    padding: 2px;
  }

  .fn {
    font: 13px var(--font-ui);
    color: var(--text);
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0 8px;
    line-height: 1.2;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .fn:hover {
    border-color: var(--accent-edge);
  }

  .fn[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .small {
    font-size: 12px;
  }
</style>
