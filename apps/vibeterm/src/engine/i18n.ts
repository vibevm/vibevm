// spec://vibeterm/PROP-046#i18n-catalogue
// Address-keyed message catalogue. Msg { key, default_en }; ResolvedLabel { value, original_en }.
// A Catalogue resolves through a parent chain terminating in inline English — a release lookup never
// misses. English is the default, mandatory-complete locale; Russian ships from the start.

export interface Msg {
  readonly key: string;
  readonly default_en: string;
}

export function msg(default_en: string, key?: string): Msg {
  return { key: key ?? default_en, default_en };
}

export interface ResolvedLabel {
  readonly value: string;
  readonly original_en: string;
}

export type Locale = "en" | "ru";
export type CatalogueEntries = ReadonlyMap<string, string>;

export class Catalogue {
  constructor(
    readonly locale: Locale,
    readonly entries: CatalogueEntries,
    readonly parent: Catalogue | null,
  ) {}

  resolve(key: string, default_en: string): ResolvedLabel {
    const value = this.entries.get(key) ?? this.parent?.entries.get(key) ?? default_en;
    return { value, original_en: default_en };
  }
}

// The inline-English catalogue: every key resolves to its default_en (the terminating fallback).
export const EN_INLINE: Catalogue = new Catalogue("en", new Map(), null);

export function resolveMsg(cat: Catalogue, m: Msg): ResolvedLabel {
  return cat.resolve(m.key, m.default_en);
}
