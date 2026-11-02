<script lang="ts">
  // The third-party licences ND104 Studio ships with (THIRD-PARTY-NOTICES.txt,
  // embedded in the app), plus a search to find one package quickly.
  import { api } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let dialog: HTMLDialogElement;
  let text = $state("");
  let error = $state("");
  let query = $state("");

  function open(node: HTMLDialogElement) {
    node.showModal();
    api.thirdPartyNotices().then(
      (t) => (text = t),
      (e) => (error = String(e)),
    );
  }

  // the package list (before the licence texts), filtered by the search
  let list = $derived.by(() => {
    const head = text.split("\nLicence texts\n")[0];
    // "name version – licence – https://…"
    const lines = head.split("\n").filter((l) => / – https?:\/\//.test(l));
    const q = query.trim().toLowerCase();
    return q ? lines.filter((l) => l.toLowerCase().includes(q)) : lines;
  });
</script>

<dialog bind:this={dialog} use:open {onclose} onclick={(e) => e.target === dialog && dialog.close()}>
  <div class="box">
    <div class="head">
      <h2>{t("ui.lic.title")}</h2>
      <button class="icon-btn" aria-label={t("ui.keymap.close")} onclick={() => dialog.close()}><Icon name="x" /></button>
    </div>
    <p class="muted small">{t("ui.lic.intro", { n: list.length })}</p>
    <input bind:value={query} placeholder={t("ui.lic.search")} />
    {#if error}<div class="notice err selectable">{error}</div>{/if}
    <div class="scroll selectable">
      {#if query.trim()}
        <pre>{list.join("\n")}</pre>
      {:else}
        <pre>{text}</pre>
      {/if}
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
    width: min(860px, calc(100vw - 48px));
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
    overflow: auto;
    min-height: 0;
    flex: 1;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--panel-2);
  }

  pre {
    margin: 0;
    padding: 12px 14px;
    font: 12px/1.5 var(--font-mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
