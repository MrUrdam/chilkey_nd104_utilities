// UI texts come from the same locale files as the tray (locales/*.json),
// delivered by the backend for the configured language.

const state = $state<{ texts: Record<string, string> }>({ texts: {} });

export function setTexts(texts: Record<string, string>) {
  state.texts = texts;
}

/** Text for `key` with `{name}` placeholders replaced. */
export function t(key: string, args: Record<string, string | number> = {}): string {
  let text = state.texts[key] ?? key;
  for (const [name, value] of Object.entries(args)) {
    text = text.replaceAll(`{${name}}`, String(value));
  }
  return text;
}
