<script lang="ts">
  // Same ten icon types as the keyboard display (docs/protocol.md § 4.2).
  import type { Condition } from "../lib/api";

  let {
    condition,
    isDay = true,
    size = 64,
  }: { condition: Condition; isDay?: boolean; size?: number | string } = $props();

  const cloud = "M17 47h30a10 10 0 0 0 1-20 15 15 0 0 0-28-4 11 11 0 0 0-3 24z";
  const backCloud = "M30 36h20a8 8 0 0 0 1-16 12 12 0 0 0-22-3 9 9 0 0 0 1 19z";

  let celestial = $derived(condition === "clear" || condition === "partly_cloudy");
  let small = $derived(condition === "partly_cloudy");
</script>

<svg viewBox="0 0 64 64" width={size} height={size} aria-hidden="true">
  {#if celestial && isDay}
    <g transform={small ? "translate(-8 -9) scale(0.75)" : "translate(0 0)"}>
      <circle cx="32" cy="32" r="12" fill="#fbbf24" />
      {#each Array.from({ length: 8 }, (_, i) => i * 45) as angle (angle)}
        <line
          x1="32"
          y1="12"
          x2="32"
          y2="6"
          stroke="#fbbf24"
          stroke-width="3.5"
          stroke-linecap="round"
          transform="rotate({angle} 32 32)"
        />
      {/each}
    </g>
  {:else if celestial}
    <g transform={small ? "translate(-6 -8) scale(0.75)" : "translate(0 0)"}>
      <path d="M38 12a20 20 0 1 0 14 30A16 16 0 0 1 38 12z" fill="#a78bfa" />
      <circle cx="50" cy="14" r="2" fill="#c4b5fd" />
      <circle cx="56" cy="26" r="1.5" fill="#c4b5fd" />
    </g>
  {/if}

  {#if condition === "overcast"}
    <path d={backCloud} fill="#94a3b8" />
  {/if}

  {#if condition !== "clear"}
    <path d={cloud} fill={condition === "thunderstorm" || condition === "overcast" ? "#cbd5e1" : "#f1f5f9"} />
  {/if}

  {#if condition === "rain" || condition === "drizzle"}
    {#each [22, 32, 42] as x (x)}
      <line x1={x} y1="51" x2={x - 3} y2="59" stroke="#38bdf8" stroke-width="3" stroke-linecap="round" />
    {/each}
  {:else if condition === "snow"}
    {#each [[22, 54], [32, 58], [42, 54]] as [x, y] (x)}
      <circle cx={x} cy={y} r="2.6" fill="#e0f2fe" stroke="#38bdf8" stroke-width="1" />
    {/each}
  {:else if condition === "thunderstorm"}
    <path d="M33 46l-6 10h6l-3 8 10-12h-6l4-6z" fill="#facc15" />
  {:else if condition === "fog"}
    {#each [51, 57] as y (y)}
      <rect x="14" {y} width="36" height="3.5" rx="1.75" fill="#94a3b8" />
    {/each}
  {/if}
</svg>
