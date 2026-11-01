import type { Condition } from "./api";
import { t } from "./i18n.svelte";

let locale = "en";

export function setLocale(language: string) {
  locale = language === "de" ? "de-DE" : "en-GB";
}

/** Timestamp (UTC, RFC 3339) → local time, e.g. `14:05`. */
export function time(ts: string | null | undefined, seconds = false): string {
  if (!ts) return "–";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return "–";
  return d.toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
    second: seconds ? "2-digit" : undefined,
  });
}

/** Timestamp → local date + time. */
export function dateTime(ts: string | null | undefined): string {
  if (!ts) return "–";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return "–";
  return d.toLocaleString(locale, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Wall-clock time without zone (`2026-10-31T14:05:09.12`) → parts. */
export function civil(dt: string | null | undefined): { time: string; date: string; weekday: string } | null {
  if (!dt) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(dt);
  if (!m) return null;
  const [, y, mo, d, h, mi] = m;
  const date = new Date(Number(y), Number(mo) - 1, Number(d));
  return {
    time: `${h}:${mi}`,
    date: date.toLocaleDateString(locale, { day: "2-digit", month: "2-digit", year: "numeric" }),
    weekday: date.toLocaleDateString(locale, { weekday: "short" }),
  };
}

export function temperature(celsius: number | null | undefined, unit: "celsius" | "fahrenheit"): string {
  if (celsius == null) return "–";
  if (unit === "fahrenheit") return `${(celsius * 1.8 + 32).toFixed(1)} °F`;
  return `${celsius.toFixed(1)} °C`;
}

export function condition(c: Condition): string {
  return t(`weather.${c}`);
}
