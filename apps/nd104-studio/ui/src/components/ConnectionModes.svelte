<script lang="ts">
  import type { Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  let { status }: { status: Status } = $props();

  type Support = "yes" | "no" | "last_usb" | "bt_too";
  // per feature: USB cable, Bluetooth, 2.4 GHz – keep in step with the matrix
  // in README → Connection modes (the one place the docs describe this)
  const FEATURES: [string, Support, Support, Support][] = [
    ["time_weather", "yes", "yes", "no"],
    ["picture", "yes", "no", "no"],
    ["lighting", "yes", "yes", "no"],
    ["keymap", "yes", "yes", "no"],
    ["settings", "yes", "yes", "no"],
    ["keyboard_fw", "yes", "yes", "no"],
    ["screen_fw", "yes", "last_usb", "no"],
    ["battery", "bt_too", "yes", "no"],
    ["app", "yes", "yes", "yes"],
  ];
  const MODES = ["usb", "bluetooth", "wireless"] as const;

  // the column of the current connection is highlighted
  let current = $derived(status.device.kind === "ready" ? status.device.connection : null);

  const icon = { yes: "check", no: "x", last_usb: "info", bt_too: "info" } as const;
</script>

<div class="card stack">
  <h2>{t("ui.modes.title")}</h2>
  <p class="muted small">{t("ui.modes.intro")}</p>
  <table>
    <thead>
      <tr>
        <th>{t("ui.modes.feature")}</th>
        {#each MODES as mode}
          <th class="mode" class:current={current === mode}>{t(`ui.modes.${mode}`)}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each FEATURES as [feature, ...support]}
        <tr>
          <td>{t(`ui.modes.f.${feature}`)}</td>
          {#each support as s, i}
            <td class="mode {s}" class:current={current === MODES[i]} title={t(`ui.modes.${s}`)}>
              <Icon name={icon[s]} size={16} /><span class="sr">{t(`ui.modes.${s}`)}</span>
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
  <ul class="muted small notes">
    <li>{t("ui.modes.note_last_usb")}</li>
    <li>{t("ui.modes.note_battery")}</li>
    <li>{t("ui.modes.note_cable_wins")}</li>
    <li>{t("ui.modes.note_wireless")}</li>
  </ul>
</div>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    text-align: left;
    color: var(--muted);
    font-weight: 600;
    border-bottom: 1px solid var(--border);
    padding: 6px 8px;
  }

  td {
    padding: 7px 8px;
    border-bottom: 1px solid var(--border);
  }

  .mode {
    text-align: center;
    width: 92px;
    /* keeps the hidden label inside the cell (otherwise it widens the page) */
    position: relative;
  }

  .current {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  th.current {
    color: var(--text);
  }

  td.yes {
    color: var(--ok);
  }

  td.no {
    color: var(--faint);
  }

  td.last_usb,
  td.bt_too {
    color: var(--warn);
  }

  .notes {
    margin: 0;
    padding-left: 18px;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
