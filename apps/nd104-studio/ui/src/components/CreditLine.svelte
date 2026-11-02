<script lang="ts">
  // One data credit: "Weather data by Open-Meteo.com (CC BY 4.0)", both parts
  // linked.
  import { api } from "../lib/api";
  import { LICENSE_URLS, type Credit } from "../lib/credits";

  let { credit }: { credit: Credit } = $props();
</script>

<span class="muted small credit">
  <button type="button" class="link" onclick={() => api.open(credit.url)}>{credit.text}</button>
  {#if credit.license}
    (<button type="button" class="link" onclick={() => api.open(LICENSE_URLS[credit.license!])}>{credit.license}</button>)
  {/if}
</span>

<style>
  .credit .link {
    font-size: inherit;
    color: inherit;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    text-underline-offset: 2px;
  }

  .credit .link:hover {
    color: var(--text);
  }
</style>
