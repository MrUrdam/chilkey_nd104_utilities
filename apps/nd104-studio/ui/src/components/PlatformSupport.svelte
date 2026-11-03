<script lang="ts">
  // What works on which operating system – only the features that differ;
  // the current system is highlighted. Keep in step with README → Platform
  // support (the one place the docs describe this).
  import { t } from "../lib/i18n.svelte";
  import { platform, type Platform } from "../lib/platform.svelte";
  import Icon from "./Icon.svelte";

  type Support = "yes" | "no" | "limited";
  // per feature: Windows, Linux
  const FEATURES: [string, Support, Support][] = [
    ["openrgb", "yes", "yes"],
    ["signalrgb", "yes", "no"],
    ["tray", "yes", "limited"],
    ["autostart", "yes", "yes"],
    ["keys", "yes", "yes"],
    ["access", "yes", "limited"],
    ["bt_battery", "yes", "yes"],
    ["updates", "yes", "limited"],
  ];
  const SYSTEMS: Platform[] = ["windows", "linux"];
  const icon = { yes: "check", no: "x", limited: "info" } as const;
</script>

<div class="card stack">
  <h2>{t("ui.os.title")}</h2>
  <p class="muted small">{t("ui.os.intro")}</p>
  <table>
    <thead>
      <tr>
        <th>{t("ui.modes.feature")}</th>
        {#each SYSTEMS as os}
          <th class="mode" class:current={platform() === os}>{t(`ui.os.${os}`)}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each FEATURES as [feature, ...support]}
        <tr>
          <td>{t(`ui.os.f.${feature}`)}</td>
          {#each support as s, i}
            <td class="mode {s}" class:current={platform() === SYSTEMS[i]} title={t(`ui.os.${s}`)}>
              <Icon name={icon[s]} size={16} /><span class="sr">{t(`ui.os.${s}`)}</span>
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
  <ul class="muted small notes">
    <li>{t("ui.os.note_same")}</li>
    <li>{t("ui.os.note_tray")}</li>
    <li>{t("ui.os.note_access")}</li>
    <li>{t("ui.os.note_updates")}</li>
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
    width: 110px;
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

  td.limited {
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
