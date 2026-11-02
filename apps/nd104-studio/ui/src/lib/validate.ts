import type { AppInfo, Config } from "./api";

/** Field path → translation key of the problem. Empty = valid. */
export function validate(c: Config, info: AppInfo): Record<string, string> {
  const e: Record<string, string> = {};
  const loc = c.weather.location;
  const needsLocation = c.weather.provider !== "manual";

  if (needsLocation && (loc.lat == null || loc.lon == null)) e["weather.location"] = "ui.err.location_missing";
  if (loc.lat != null && (Number.isNaN(loc.lat) || loc.lat < -90 || loc.lat > 90)) e["weather.lat"] = "ui.err.lat";
  if (loc.lon != null && (Number.isNaN(loc.lon) || loc.lon < -180 || loc.lon > 180)) e["weather.lon"] = "ui.err.lon";
  if (!Number.isInteger(c.weather.interval_min) || c.weather.interval_min < info.min_weather_interval)
    e["weather.interval"] = "ui.err.interval";
  if (c.weather.fallback_provider && c.weather.fallback_provider === c.weather.provider)
    e["weather.fallback"] = "ui.err.fallback_same";

  for (const k of ["current_c", "max_c", "min_c"] as const) {
    const v = c.weather.manual[k];
    if (Number.isNaN(v) || v < -99.9 || v > 99.9) e[`weather.manual.${k}`] = "ui.err.temperature";
  }

  if ((c.clock.source === "timezone" || c.clock.source === "ntp") && !c.clock.timezone.trim())
    e["clock.timezone"] = "ui.err.timezone";
  if (c.clock.source === "ntp" && !c.clock.ntp_server.trim()) e["clock.ntp_server"] = "ui.err.ntp_server";
  if (!Number.isInteger(c.clock.manual_offset_min) || Math.abs(c.clock.manual_offset_min) > 24 * 60)
    e["clock.offset"] = "ui.err.offset";
  if (!Number.isInteger(c.clock.sync_interval_min) || c.clock.sync_interval_min < 1)
    e["clock.interval"] = "ui.err.interval_positive";

  if (!Number.isInteger(c.device.poll_interval_s) || c.device.poll_interval_s < 1 || c.device.poll_interval_s > 60)
    e["device.poll"] = "ui.err.poll";
  if (!Number.isInteger(c.device.openrgb_port) || c.device.openrgb_port < 1024 || c.device.openrgb_port > 65535)
    e["device.openrgb_port"] = "ui.err.port";
  if (!Number.isInteger(c.updates.check_interval_h) || c.updates.check_interval_h < 1)
    e["updates.interval"] = "ui.err.interval_positive";
  return e;
}
