<script lang="ts">
  import { api, CONDITIONS, osmUrl, type AppInfo, type Config, type Place, type WeatherReport } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { condition, temperature } from "../lib/format";
  import WeatherIcon from "../components/WeatherIcon.svelte";
  import ApiKeyField from "../components/ApiKeyField.svelte";
  import Icon from "../components/Icon.svelte";
  import { onMount } from "svelte";

  let { draft = $bindable(), info, errors }: { draft: Config; info: AppInfo; errors: Record<string, string> } =
    $props();

  let query = $state("");
  let places = $state<Place[] | null>(null);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let testing = $state(false);
  let testResult = $state<WeatherReport | null>(null);
  let testError = $state<string | null>(null);

  let w = $derived(draft.weather);

  // API keys: status per provider, keys typed but not saved yet (for Test).
  let keyStatus = $state<Record<string, boolean>>({});
  let typedKeys = $state<Record<string, string>>({});
  async function refreshKeys() {
    keyStatus = await api.apiKeyStatus();
  }
  onMount(refreshKeys);

  let primary = $derived(info.providers.find((p) => p.id === w.provider));
  let fallback = $derived(info.providers.find((p) => p.id === w.fallback_provider));
  let keyProviders = $derived(
    [primary, fallback].filter((p, i, all) => p && p.requires_api_key && all.indexOf(p) === i) as typeof info.providers,
  );
  let missingKey = $derived(keyProviders.some((p) => !keyStatus[p.id] && !typedKeys[p.id]?.trim()));

  async function search() {
    if (!query.trim()) return;
    searching = true;
    searchError = null;
    try {
      places = await api.searchPlaces(query.trim(), info.language);
    } catch (e) {
      searchError = String(e);
      places = null;
    } finally {
      searching = false;
    }
  }

  function choose(p: Place) {
    const label = [p.name, p.admin1, p.country].filter(Boolean).join(", ");
    draft.weather.location = { mode: "search", name: label, lat: p.latitude, lon: p.longitude };
    places = null;
    query = "";
  }

  async function test() {
    testing = true;
    testError = null;
    testResult = null;
    try {
      testResult = await api.testWeather(
        $state.snapshot(draft.weather) as Config["weather"],
        typedKeys[draft.weather.provider]?.trim() || null,
      );
    } catch (e) {
      testError = String(e);
    } finally {
      testing = false;
    }
  }

  function num(value: string): number | null {
    const n = parseFloat(value.replace(",", "."));
    return Number.isFinite(n) ? n : null;
  }
</script>

<div class="stack">
  <h1>{t("ui.nav.weather")}</h1>

  <div class="card stack">
    <h2>{t("ui.weather.location")}</h2>
    <div class="current">
      <span class="led {w.location.lat != null ? 'accent' : ''}"></span>
      <strong class="selectable">{w.location.name || t("ui.weather.no_location")}</strong>
      {#if w.location.lat != null && w.location.lon != null}
        {@const lat = w.location.lat}
        {@const lon = w.location.lon}
        <span class="mono muted">{lat.toFixed(4)}, {lon.toFixed(4)}</span>
        <button type="button" class="link" title={t("ui.weather.show_map_long")} onclick={() => api.open(osmUrl(lat, lon))}
          ><Icon name="map" size={14} /> {t("ui.weather.show_map")}</button
        >
      {/if}
    </div>
    {#if errors["weather.location"]}<span class="error-text">{t(errors["weather.location"])}</span>{/if}

    <div class="segmented">
      <button
        aria-pressed={w.location.mode === "search"}
        onclick={() => (draft.weather.location.mode = "search")}>{t("ui.weather.mode_search")}</button
      >
      <button
        aria-pressed={w.location.mode === "coordinates"}
        onclick={() => (draft.weather.location.mode = "coordinates")}>{t("ui.weather.mode_coordinates")}</button
      >
    </div>

    {#if w.location.mode === "search"}
      <form
        class="row"
        onsubmit={(e) => {
          e.preventDefault();
          search();
        }}
      >
        <input class="grow" bind:value={query} placeholder={t("ui.weather.search_placeholder")} />
        <button class="key" type="submit" disabled={searching || !query.trim()}>
          {searching ? "…" : t("ui.weather.search")}
        </button>
      </form>
      {#if searchError}<span class="error-text">{searchError}</span>{/if}
      {#if places}
        {#if places.length === 0}
          <p class="muted">{t("ui.weather.no_results")}</p>
        {:else}
          <ul class="results">
            {#each places as p, i (i)}
              <li>
                <button class="pick" onclick={() => choose(p)}>
                  <strong>{p.name}</strong>
                  <span class="muted">{[p.admin1, p.country].filter(Boolean).join(", ")}</span>
                  <span class="mono muted coords">{p.latitude.toFixed(3)}, {p.longitude.toFixed(3)}</span>
                </button>
                <button
                  class="map"
                  title={t("ui.weather.show_map_long")}
                  onclick={() => api.open(osmUrl(p.latitude, p.longitude))}><Icon name="map" size={14} /> {t("ui.weather.show_map")}</button
                >
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    {:else}
      <div class="grid-3">
        <label class="field"
          >{t("ui.weather.name")}<input bind:value={draft.weather.location.name} /></label
        >
        <label class="field"
          >{t("ui.weather.lat")}<input
            value={w.location.lat ?? ""}
            aria-invalid={!!errors["weather.lat"]}
            oninput={(e) => (draft.weather.location.lat = num(e.currentTarget.value))}
          />
          {#if errors["weather.lat"]}<span class="error-text">{t(errors["weather.lat"])}</span>{/if}
        </label>
        <label class="field"
          >{t("ui.weather.lon")}<input
            value={w.location.lon ?? ""}
            aria-invalid={!!errors["weather.lon"]}
            oninput={(e) => (draft.weather.location.lon = num(e.currentTarget.value))}
          />
          {#if errors["weather.lon"]}<span class="error-text">{t(errors["weather.lon"])}</span>{/if}
        </label>
      </div>
    {/if}
    <p class="muted small with-icon"><Icon name="lock" size={14} />{t("ui.weather.privacy")}</p>
  </div>

  <div class="card stack">
    <h2>{t("ui.weather.source")}</h2>
    <div class="grid-2">
      <label class="field"
        >{t("ui.weather.provider")}
        <select bind:value={draft.weather.provider}>
          {#each info.providers as p (p.id)}<option value={p.id}
              >{p.name}{p.requires_api_key ? ` – ${t("ui.key.needs_key")}` : ""}</option
            >{/each}
        </select>
        {#if primary && primary.coverage !== "none"}
          <span class="muted small">{t(`ui.coverage.${primary.coverage}`)}</span>
        {/if}
      </label>
      <label class="field"
        >{t("ui.weather.fallback")}
        <select bind:value={draft.weather.fallback_provider} aria-invalid={!!errors["weather.fallback"]}>
          <option value="">{t("ui.none")}</option>
          {#each info.providers as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
        </select>
        {#if errors["weather.fallback"]}<span class="error-text">{t(errors["weather.fallback"])}</span>{/if}
      </label>
      <label class="field"
        >{t("ui.weather.interval", { min: info.min_weather_interval })}
        <input
          type="number"
          min={info.min_weather_interval}
          step="1"
          bind:value={draft.weather.interval_min}
          aria-invalid={!!errors["weather.interval"]}
        />
        {#if errors["weather.interval"]}<span class="error-text">{t(errors["weather.interval"], { min: info.min_weather_interval })}</span>{/if}
      </label>
      <div class="field">
        <span class="label">{t("ui.weather.unit")}</span>
        <div class="segmented">
          <button aria-pressed={w.ui_unit === "celsius"} onclick={() => (draft.weather.ui_unit = "celsius")}>°C</button>
          <button aria-pressed={w.ui_unit === "fahrenheit"} onclick={() => (draft.weather.ui_unit = "fahrenheit")}
            >°F</button
          >
        </div>
        <span class="muted small">{t("ui.weather.unit_hint")}</span>
      </div>
    </div>

    {#each keyProviders as p (p.id)}
      <ApiKeyField
        provider={p}
        stored={!!keyStatus[p.id]}
        bind:typed={() => typedKeys[p.id] ?? "", (v) => (typedKeys[p.id] = v)}
        onchange={refreshKeys}
      />
    {/each}
    {#if missingKey}<div class="notice warn">{t("ui.key.missing_notice")}</div>{/if}

    {#if w.provider === "manual"}
      <div class="notice">{t("ui.weather.manual_hint")}</div>
      <div class="grid-3">
        <label class="field"
          >{t("ui.dash.condition")}
          <select bind:value={draft.weather.manual.condition}>
            {#each CONDITIONS as c (c)}<option value={c}>{condition(c)}</option>{/each}
          </select>
        </label>
        <label class="switch"><input type="checkbox" bind:checked={draft.weather.manual.is_day} />{t("ui.day")}</label>
        <span></span>
        {#each ["current_c", "max_c", "min_c"] as const as k (k)}
          <label class="field"
            >{t(`ui.weather.manual_${k}`)}
            <input type="number" step="0.1" bind:value={draft.weather.manual[k]} aria-invalid={!!errors[`weather.manual.${k}`]} />
          </label>
        {/each}
      </div>
    {/if}

    <div class="row">
      <button class="key" onclick={test} disabled={testing}>{testing ? "…" : t("ui.weather.test")}</button>
      <span class="muted small">{t("ui.weather.test_hint")}</span>
    </div>
    {#if testError}<div class="notice err selectable">{testError}</div>{/if}
    {#if testResult}
      <div class="notice">
        <WeatherIcon condition={testResult.condition} isDay={testResult.is_day} size={40} />
        <div>
          <strong>{condition(testResult.condition)}</strong> · {temperature(testResult.current_c, w.ui_unit)}<br />
          <span class="muted"
            >↑ {temperature(testResult.max_c, w.ui_unit)} ↓ {temperature(testResult.min_c, w.ui_unit)} · {testResult.provider}</span
          >
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .current {
    display: flex;
    gap: 10px;
    align-items: center;
    font-size: 15px;
  }

  .grow {
    flex: 1;
    min-width: 200px;
  }

  .grid-3 {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    align-items: end;
  }

  .results {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .results li {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--panel-2);
  }

  .results li:last-child {
    border-bottom: 0;
  }

  .results button {
    font: 14px var(--font-ui);
    color: var(--text);
    background: transparent;
    border: 0;
    padding: 9px 12px;
    cursor: pointer;
  }

  .results .pick {
    flex: 1;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
    text-align: left;
  }

  .results .map {
    color: var(--accent);
    font-size: 13px;
    border-left: 1px solid var(--border);
    white-space: nowrap;
  }

  .results button:hover {
    background: var(--accent-soft);
  }

  .coords {
    font-size: 12px;
  }

  .small {
    font-size: 12px;
    margin: 0;
  }

  .label {
    font-size: 13px;
    color: var(--muted);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
</style>
