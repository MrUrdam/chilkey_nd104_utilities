<script lang="ts" module>
  export type UsbFeature = "keymap" | "backup" | "lighting" | "picture" | "settings";
</script>

<script lang="ts">
  // Shown where a feature needs the USB cable: names what needs it, and says
  // so more precisely when the keyboard is connected over Bluetooth instead.
  import { bluetooth } from "../lib/bluetooth.svelte";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let { feature }: { feature: UsbFeature } = $props();

  const bt = bluetooth();

  // continues the sentence after "Über Bluetooth verbunden –"
  const lower = (text: string) => text.charAt(0).toLowerCase() + text.slice(1);
</script>

<div class="notice warn">
  <Icon name="plug" />
  <span>
    {#if feature === "picture" && bt.keyboard}
      {t("ui.usb.via_bt", { what: lower(t("ui.usb.picture")) })}
    {:else}
      {t(`ui.usb.${feature}`)}
    {/if}
  </span>
</div>
