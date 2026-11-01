// Turns a lighting zone's state (as read from the keyboard) into the colour
// the keymap drawing glows in. Exact for single-colour effects; animated
// effects are approximated (pulsing colour, or a slow colour cycle for
// effects that ignore the chosen colour).
import type { LightZone, ZoneState } from "./api";
import { EFFECT_OFF, effect } from "./effects";

export interface Glow {
  /** HSL colour; hue in degrees. */
  h: number;
  s: number;
  l: number;
  /** Strength 0–1 from the brightness. */
  a: number;
  fx: "solid" | "pulse" | "cycle";
  /** Seconds per animation cycle (from the speed). */
  duration: number;
}

export function glowOf(zone: LightZone, state: ZoneState | null): Glow | null {
  if (!state || state.effect === null || state.effect === EFFECT_OFF) return null;
  const brightness = (state.brightness ?? 255) / 255;
  if (brightness <= 0.02) return null;
  const info = effect(zone, state.effect);
  // keyboard colour is HSV with full value; as HSL that is 100 % saturation
  // and a lightness from 100 % (white) down to 50 % (full colour)
  const sat = (state.sat ?? 255) / 255;
  const speed = (state.speed ?? 128) / 255;
  const cycle = !info.color;
  return {
    h: Math.round(((state.hue ?? 0) * 360) / 255),
    // a colour cycle runs through fully saturated colours
    s: cycle || sat > 0 ? 100 : 0,
    l: cycle ? 58 : Math.round(100 - sat * 50),
    // kept subtle: a hint of the lighting, not a light show
    a: 0.15 + 0.35 * brightness,
    fx: cycle ? "cycle" : info.animated ? "pulse" : "solid",
    duration: 12 - 10 * speed,
  };
}
