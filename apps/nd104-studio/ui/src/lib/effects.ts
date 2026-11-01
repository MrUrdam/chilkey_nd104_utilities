// Lighting effects built into the ND104 keyboard firmware (QMK), in the order
// and with the names of Chilkey's web driver (docs/protocol.md § 5.1).
// Names: locale keys `fx.<zone>.<index>`.
import type { LightZone } from "./api";

export interface Effect {
  value: number;
  /** Uses the chosen colour (hue/saturation). */
  color: boolean;
  /** Animated: the speed setting changes something. */
  animated: boolean;
}

function range(from: number, to: number): number[] {
  return Array.from({ length: to - from + 1 }, (_, i) => from + i);
}

function catalogue(count: number, color: number[], still: number[]): Effect[] {
  return range(0, count - 1).map((value) => ({
    value,
    color: color.includes(value),
    animated: !still.includes(value),
  }));
}

export const EFFECTS: Record<LightZone, Effect[]> = {
  // QMK rgb_matrix (per key)
  per_key: catalogue(
    45,
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 23, 25, 26, 27, ...range(33, 40), 43, 44],
    [0, 1, 2, 3, 4],
  ),
  // QMK rgblight (around the keyboard + light bar below Del/End/PgDn)
  bottom: catalogue(43, [...range(1, 5), ...range(15, 23), ...range(25, 34), ...range(36, 42)], [0, 1, ...range(25, 34)]),
};

export const EFFECT_OFF = 0;
export const EFFECT_SOLID = 1;

export function effect(zone: LightZone, value: number): Effect {
  return EFFECTS[zone][value] ?? { value, color: true, animated: true };
}
