<script lang="ts" module>
  export type UsbFeature = "keymap" | "backup" | "lighting" | "picture" | "settings";
</script>

<script lang="ts">
  // Shown where a feature needs a connection (the picture: the USB cable):
  // names what needs it, and how the keyboard is connected instead.
  import { bluetooth } from "../lib/bluetooth.svelte";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let { feature, via = null }: { feature: UsbFeature; via?: "usb" | "bluetooth" | "wireless" | null } =
    $props();

  const bt = bluetooth();

  // continues the sentence after "Über Bluetooth verbunden –"
  const lower = (text: string) => text.charAt(0).toLowerCase() + text.slice(1);
</script>

<div class="notice warn">
  <Icon name="plug" />
  <span>
    {#if feature === "picture" && via === "wireless"}
      {t("ui.usb.via_wireless", { what: lower(t("ui.usb.picture")) })}
    {:else if feature === "picture" && (via === "bluetooth" || bt.keyboard)}
      {t("ui.usb.via_bt", { what: lower(t("ui.usb.picture")) })}
    {:else}
      {t(`ui.usb.${feature}`)}
    {/if}
  </span>
</div>
