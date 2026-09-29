/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER */

/**
 * The search index the build publishes beside the page manifest.
 *
 * A static host answers no queries, so the site publishes the corpus and
 * the browser does the asking — the same arrangement `resolve.json` and
 * its one hand-written page make for a citation (F-15). One file,
 * fetched once, the first time a reader types.
 *
 * It is its own document rather than a field of the page manifest,
 * and for a plain reason: the manifest is what a machine reads before
 * deciding what to FETCH — every page's anchors, audiences, genre and
 * both spellings of its address — and it is large for that reason. A
 * reader who typed one letter should not be waiting on the answer to a
 * different question. Nothing here is computed from a page: every value
 * is read off the manifests the pipeline wrote, exactly as the catalogue
 * and the sitemap are (`##PIPE-SHELL-PARSES-NOTHING`).
 *
 * Only the `latest` spelling, and never a fallback. The numbered address
 * shows the same content (`##SITE-VERSION-SHOWS-CURRENT`) and a search
 * that offered both would answer every question twice; a page an
 * adaptation has not reached carries the source's text, which is already
 * in the index under the source's own address.
 */

import type { SearchEntry, SearchIndex } from "../lib/search.ts";
import {
  docFileHref,
  indexableOf,
  type Address,
  type Library,
} from "./editions.ts";

/** Where the index is published, beside the manifest it is not part of. */
export const SEARCH_INDEX = docFileHref("search.json");

/** How an entry names the documentation and language it belongs to. */
function contextOf(address: Address): string {
  const card = address.edition.manifest.package;
  return `${card.title} · ${card.lang}`;
}

/** `<group>/<name>@<version>` — identity, and a thing readers type. */
function coordinateOf(address: Address): string {
  const card = address.edition.manifest.package;
  return `${card.group}/${card.name}@${card.version}`;
}

/**
 * One address as the index holds it, or `null` for an address that is
 * not a thing a reader can look for.
 *
 * A catalogue is one of those: `vibevm.org/doc/ru/` is a shelf of
 * everything in a language, which is what the language selector already
 * offers the reader, and an entry for it would match every query that
 * mentioned the language and answer none of them.
 */
function entryOf(address: Address): SearchEntry | null {
  const card = address.edition.manifest.package;
  if (address.kind === "package") {
    return {
      title: card.title,
      href: address.href,
      context: contextOf(address),
      summary: card.abstract,
      coordinate: coordinateOf(address),
      kind: "documentation",
    };
  }
  if (address.kind !== "page" || address.page === undefined) return null;
  return {
    title: address.page.title,
    href: address.href,
    context: `${card.title} · ${card.lang}`,
    summary: address.page.summary,
    coordinate: coordinateOf(address),
    kind: "page",
  };
}

/**
 * Every documentation and every page this build carries, as one index.
 *
 * The order is the libraries' own and, inside each, the addresses': a
 * source documentation before its adaptations, and a documentation's
 * page before the pages of the next one. The matcher keeps that order
 * for equal scores, so the three signals D-19 asks to agree keep
 * agreeing in a dropdown too.
 */
export function searchIndex(libraries: readonly Library[]): SearchIndex {
  const entries: SearchEntry[] = [];
  for (const library of libraries) {
    for (const address of indexableOf(library.addresses)) {
      const entry = entryOf(address);
      if (entry !== null) entries.push(entry);
    }
  }
  return { schema_version: 1, entries };
}
