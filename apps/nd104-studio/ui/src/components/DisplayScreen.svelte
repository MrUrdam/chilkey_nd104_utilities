<script lang="ts">
  // The 320×172 keyboard display, laid out like the real weather page: time
  // top left, battery top right, weather icon left, temperature right with
  // low ~ high below. Light like the device in every app theme. Sizes follow
  // the screen width (container units), so it works large and small.
  // The battery is a fixed placeholder – nd104-studio cannot read battery level
  // or charging state.
  import type { WeatherReport } from "../lib/api";
  import WeatherIcon from "./WeatherIcon.svelte";

  type ScreenWeather = Pick<WeatherReport, "condition" | "is_day" | "current_c" | "min_c" | "max_c">;

  let {
    time,
    weather,
    dim = false,
    overlay = null,
    noWeather = "",
  }: {
    time: Date | null;
    weather: ScreenWeather | null;
    dim?: boolean;
    overlay?: string | null;
    noWeather?: string;
  } = $props();

  const pad = (n: number) => String(n).padStart(2, "0");
  const deg = (c: number) => `${c.toFixed(1)}°C`;
</script>

<div class="bezel">
  <div class="screen" class:dim>
    <div class="content">
      <span class="time">{time ? `${pad(time.getHours())}:${pad(time.getMinutes())}` : "--:--"}</span>

      <svg class="battery" viewBox="0 0 34 16" aria-hidden="true">
        <rect x="1" y="1" width="28" height="14" rx="4" fill="none" stroke="#8a8f98" stroke-width="1.6" />
        <rect x="30" y="5" width="3" height="6" rx="1" fill="#8a8f98" />
        <rect x="3.6" y="3.6" width="15" height="8.8" rx="2.4" fill="#5bbf6a" />
      </svg>

      {#if weather}
        <div class="icon"><WeatherIcon condition={weather.condition} isDay={weather.is_day} size="100%" onLight /></div>
        <div class="temps">
          <span class="current">{deg(weather.current_c)}</span>
          <span class="range">{deg(weather.min_c)} ~ {deg(weather.max_c)}</span>
        </div>
      {:else}
        <div class="none">{noWeather}</div>
      {/if}

      <div class="dots" aria-hidden="true"><i></i><i class="on"></i><i></i><i></i></div>
    </div>

    {#if overlay}
      <div class="overlay">{overlay}</div>
    {/if}
  </div>
</div>

<style>
  .bezel {
    background: linear-gradient(180deg, #2b313b, #14171c);
    border: 1px solid #3b4350;
    border-radius: 7% / 13%;
    padding: 3.2%;
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.35),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
    width: 100%;
    box-sizing: border-box;
  }

  .screen {
    container-type: inline-size;
    position: relative;
    aspect-ratio: 320 / 172;
    /* like the real screen: about 4.5 % of the width, same curve vertically */
    border-radius: 4.5% / 8.4%;
    overflow: hidden;
    background:
      radial-gradient(60% 80% at 90% 85%, rgba(196, 181, 253, 0.35), transparent 70%),
      radial-gradient(50% 70% at 75% 15%, rgba(244, 199, 222, 0.3), transparent 70%),
      linear-gradient(135deg, #eef3f8 0%, #fbfbfd 45%, #f7f3f8 100%);
    color: #474b53;
    font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  }

  .content {
    position: absolute;
    inset: 0;
  }

  .screen.dim .content {
    opacity: 0.35;
  }

  .time {
    position: absolute;
    left: 6.5%;
    top: 6.5%;
    font-size: 6cqw;
    line-height: 1;
  }

  .battery {
    position: absolute;
    right: 6%;
    top: 7%;
    width: 11cqw;
  }

  .icon {
    position: absolute;
    left: 5%;
    top: 26%;
    width: 27cqw;
    aspect-ratio: 1;
  }

  .temps {
    position: absolute;
    left: 38%;
    right: 3%;
    top: 30%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2cqw;
  }

  .current {
    font-size: 15cqw;
    font-weight: 300;
    line-height: 1;
    color: #3f434a;
    letter-spacing: -0.02em;
  }

  .range {
    font-size: 4.6cqw;
    color: #6b7078;
  }

  .none {
    position: absolute;
    inset: 25% 5% 15%;
    display: grid;
    place-items: center;
    font-size: 5cqw;
    color: #8a8f98;
    text-align: center;
  }

  .dots {
    position: absolute;
    right: 5%;
    bottom: 6%;
    display: flex;
    gap: 1.3cqw;
  }

  .dots i {
    width: 1.7cqw;
    aspect-ratio: 1;
    border-radius: 50%;
    background: #cfc9ee;
  }

  .dots i.on {
    background: #8b80d6;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-family: var(--font-ui);
    font-size: 4.4cqw;
    font-weight: 600;
    color: #1f2937;
    background: rgba(255, 255, 255, 0.35);
  }
</style>
