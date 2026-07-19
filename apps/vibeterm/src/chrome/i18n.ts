// spec://vibeterm/PROP-046#i18n-fallback
// A reactive catalogue signal: live locale swap with no reload. English is the default,
// mandatory-complete locale; Russian ships from the start.

import { createSignal } from "solid-js";
import { en } from "./locales/en";
import { ru } from "./locales/ru";

export type Locale = "en" | "ru";

const TABLES: Record<Locale, Record<string, string>> = { en, ru };

const [locale, setLocaleSig] = createSignal<Locale>("en");

export { locale };

export function setLocale(l: Locale): void {
  setLocaleSig(l);
}

export function toggleLocale(): void {
  setLocaleSig(locale() === "en" ? "ru" : "en");
}

// resolve a key; fall back to the English entry, then to the key itself.
export function t(key: string): string {
  const table = TABLES[locale()];
  return table[key] ?? en[key] ?? key;
}
