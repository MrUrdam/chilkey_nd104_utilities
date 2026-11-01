<script lang="ts" module>
  // Line icons (shapes from lucide, ISC licence): one set, 1.5 stroke,
  // colour from the surrounding text.
  type Shape = ["path", string] | ["rect", number, number, number, number, number] | ["circle", number, number, number];

  const ICONS = {
    dashboard: [
      ["rect", 3, 3, 7, 9, 1],
      ["rect", 14, 3, 7, 5, 1],
      ["rect", 14, 12, 7, 9, 1],
      ["rect", 3, 16, 7, 5, 1],
    ],
    weather: [
      ["path", "M12 2v2"],
      ["path", "m4.93 4.93 1.41 1.41"],
      ["path", "M20 12h2"],
      ["path", "m19.07 4.93-1.41 1.41"],
      ["path", "M15.95 12.65a4 4 0 0 0-5.93-4.13"],
      ["path", "M13 22H7a5 5 0 1 1 4.9-6H13a3 3 0 0 1 0 6Z"],
    ],
    clock: [
      ["circle", 12, 12, 9],
      ["path", "M12 7v5l3 2"],
    ],
    lighting: [
      ["path", "M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5"],
      ["path", "M9 18h6"],
      ["path", "M10 22h4"],
    ],
    picture: [
      ["rect", 3, 3, 18, 18, 2],
      ["circle", 9, 9, 2],
      ["path", "m21 15-3.09-3.09a2 2 0 0 0-2.82 0L6 21"],
    ],
    device: [
      ["rect", 2, 5, 20, 14, 2],
      ["path", "M6 9h.01M10 9h.01M14 9h.01M18 9h.01M8 13h.01M12 13h.01M16 13h.01M7 16h10"],
    ],
    general: [
      ["path", "M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3"],
      ["path", "M14 2v4M8 10v4M16 18v4"],
    ],
    plug: [
      ["path", "M12 22v-5"],
      ["path", "M9 8V2M15 8V2"],
      ["path", "M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z"],
    ],
    lock: [
      ["rect", 4, 11, 16, 10, 2],
      ["path", "M8 11V7a4 4 0 0 1 8 0v4"],
    ],
    map: [
      ["path", "M14.1 5.55a2 2 0 0 0 1.8 0l3.65-1.83A1 1 0 0 1 21 4.62v12.76a1 1 0 0 1-.55.9l-4.56 2.27a2 2 0 0 1-1.78 0l-4.22-2.1a2 2 0 0 0-1.78 0l-3.66 1.82A1 1 0 0 1 3 19.38V6.62a1 1 0 0 1 .55-.9l4.56-2.27a2 2 0 0 1 1.78 0z"],
      ["path", "M15 5.76v15M9 3.24v15"],
    ],
    sync: [
      ["path", "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"],
      ["path", "M21 3v5h-5"],
      ["path", "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"],
      ["path", "M8 16H3v5"],
    ],
    pause: [
      ["rect", 6, 5, 4, 14, 1],
      ["rect", 14, 5, 4, 14, 1],
    ],
    play: [["path", "M7 4.5v15l12-7.5z"]],
    check: [["path", "M20 6 9 17l-5-5"]],
    x: [["path", "M18 6 6 18M6 6l12 12"]],
    alert: [
      ["path", "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"],
      ["path", "M12 9v4M12 17h.01"],
    ],
    info: [
      ["circle", 12, 12, 9],
      ["path", "M12 16v-4M12 8h.01"],
    ],
    minus: [["path", "M5 12h14"]],
    sidebar: [
      ["rect", 3, 3, 18, 18, 2],
      ["path", "M9 3v18"],
    ],
    layers: [
      ["path", "m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"],
      ["path", "m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"],
      ["path", "m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"],
    ],
    pencil: [
      ["path", "M21.17 6.81a1 1 0 0 0-3.99-3.99L3.84 16.17a2 2 0 0 0-.5.83l-1.32 4.35a.5.5 0 0 0 .62.62l4.35-1.32a2 2 0 0 0 .83-.5z"],
      ["path", "m15 5 4 4"],
    ],
    search: [
      ["circle", 11, 11, 7],
      ["path", "m21 21-4.3-4.3"],
    ],
    trash: [
      ["path", "M3 6h18"],
      ["path", "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"],
      ["path", "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"],
    ],
    download: [
      ["path", "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"],
      ["path", "m7 10 5 5 5-5"],
      ["path", "M12 15V3"],
    ],
    upload: [
      ["path", "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"],
      ["path", "m17 8-5-5-5 5"],
      ["path", "M12 3v12"],
    ],
  } satisfies Record<string, Shape[]>;

  export type IconName = keyof typeof ICONS;
  export { ICONS };
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
  let shapes = $derived(ICONS[name] as Shape[]);
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each shapes as s, i (i)}
    {#if s[0] === "path"}
      <path d={s[1]} />
    {:else if s[0] === "rect"}
      <rect x={s[1]} y={s[2]} width={s[3]} height={s[4]} rx={s[5]} />
    {:else}
      <circle cx={s[1]} cy={s[2]} r={s[3]} />
    {/if}
  {/each}
</svg>
