<script lang="ts">
  // API key of one provider. The stored key never comes back to the window –
  // only "stored yes/no". Keys live in the Windows Credential Manager.
  import { api, type ProviderInfo } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let {
    provider,
    stored,
    typed = $bindable(""),
    onchange,
  }: { provider: ProviderInfo; stored: boolean; typed?: string; onchange: () => void } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);

  async function save() {
    busy = true;
    error = null;
    message = null;
    try {
      await api.setApiKey(provider.id, typed);
      typed = "";
      message = t("ui.key.saved");
      onchange();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    busy = true;
    error = null;
    message = null;
    try {
      await api.deleteApiKey(provider.id);
      message = t("ui.key.deleted");
      onchange();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="key-box">
  <div class="row head">
    <strong>{t("ui.key.title", { provider: provider.name })}</strong>
    {#if stored}
      <span class="badge ok"><span class="led ok"></span>{t("ui.key.stored")}</span>
    {:else}
      <span class="badge warn">{t("ui.key.missing")}</span>
    {/if}
    {#if provider.signup_url}
      {@const url = provider.signup_url}
      <button type="button" class="link" onclick={() => api.open(url)}>{t("ui.key.get")}</button>
    {/if}
  </div>
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      save();
    }}
  >
    <input
      class="grow mono"
      type="password"
      autocomplete="off"
      spellcheck="false"
      bind:value={typed}
      placeholder={t(stored ? "ui.key.placeholder_replace" : "ui.key.placeholder")}
    />
    <button class="key primary" type="submit" disabled={busy || !typed.trim()}>{t("ui.key.save")}</button>
    {#if stored}
      <button class="key" type="button" disabled={busy} onclick={remove}>{t("ui.key.delete")}</button>
    {/if}
  </form>
  {#if error}<span class="error-text selectable">{error}</span>{/if}
  {#if message}<span class="ok-text">{message}</span>{/if}
  <span class="muted small with-icon"><Icon name="lock" size={14} />{t("ui.key.hint")}</span>
</div>

<style>
  .key-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
  }

  .head {
    gap: 10px;
  }

  .grow {
    flex: 1;
    min-width: 220px;
  }

  .small {
    font-size: 12px;
  }

  .ok-text {
    color: var(--ok);
    font-size: 12px;
    font-weight: 600;
  }
</style>
