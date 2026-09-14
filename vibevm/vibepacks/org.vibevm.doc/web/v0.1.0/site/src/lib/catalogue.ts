/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * What the door shows: the three shelves behind it, and the control that
 * narrows them by language.
 *
 * It is a file of its own beside `view.ts`, which answers what one
 * documentation ADDRESS means. The door has no address to read. It is
 * the site's own page, made of everything the build carries at once,
 * and the questions it answers are about the collection rather than
 * about any member of it: which of these did the deployment recommend,
 * which of them did somebody write, and which did the pipeline print.
 *
 * Nothing here reaches for a library or for the configuration. The
 * libraries and the featured list both arrive as arguments, because a
 * shelf is a pure function of what a build was given — which is what
 * lets the whole of it be measured without a browser or a build.
 */

import type { LanguageChoice } from "@vibe-docs/design";

import type {
  Authorship,
  DocumentationStatus,
} from "../generated/doc-manifest.ts";

import { catalogueHref, packageHref } from "./href.ts";
import {
  coordinate,
  editions,
  isProjection,
  siteLanguages,
  type Library,
} from "./library.ts";
import { endonym, everyLanguage } from "./view.ts";

/** One card of the catalogue: one edition of one library the site carries. */
export type CatalogueEntry = {
  readonly tag: string;
  readonly segment: string | null;
  readonly title: string;
  readonly href: string;
  readonly publisher: string;
  readonly coordinate: string;
  readonly description?: string;
  readonly abstract: string;
  readonly status: DocumentationStatus;
  /** True when the pipeline printed this out of a package's own bytes. */
  readonly projection: boolean;
  /** True when the deployment named this documentation on the first shelf. */
  readonly featured: boolean;
  /**
   * Who wrote this edition's prose, when it says so (`##CARD-
   * AUTHORSHIP`). Absent where the documentation declared nothing, which
   * is a card with no mark and a card no named group admits — never a
   * card quietly counted as one of them.
   */
  readonly authorship?: Authorship;
};

/** Which of the door's three shelves an entry stands on. */
export type CatalogueTab = "featured" | "documents" | "projections";

/** One shelf of the door: what it is called, and what stands on it. */
export type CatalogueShelf = {
  readonly tab: CatalogueTab;
  readonly entries: readonly CatalogueEntry[];
};

/**
 * Every edition of every library, in the libraries' own order and,
 * inside each, D-19's.
 *
 * One card per EDITION and not per documentation, because a reader
 * looking for a language is looking for a text they can read. The
 * standing on a card is the standing of the thing it names: a source
 * card carries the documentation's own — primary, official or community
 * for its subject — and an adaptation's carries whether the source's
 * author named it, which is a different question with the same three
 * words (`##REL-OFFICIAL-IS-CONVERGENCE`, `##LOC-OFFICIAL-TRANSLATION`).
 */
export function catalogueEntries(
  libraries: readonly Library[],
  featured: readonly string[],
): readonly CatalogueEntry[] {
  return libraries.flatMap((library) => {
    /* Featured names a DOCUMENTATION and not an edition of one, so it is
       matched on the coordinate every address in this library is built
       on — the source's. An adaptation of a featured manual is the same
       manual in another language and stands on the same shelf. */
    const source = library.source.card;
    const named = featured.includes(`${source.group}/${source.name}`);
    return editions(library).map((one) => ({
      tag: one.tag,
      segment: one.segment,
      title: one.title,
      href: packageHref(coordinate(library, one.segment)),
      publisher: one.publisher,
      coordinate: `${one.card.group}/${one.card.name}@${one.card.version}`,
      ...(one.card.description === undefined
        ? {}
        : { description: one.card.description }),
      abstract: one.card.abstract,
      status:
        one.segment === null
          ? one.card.status
          : one.official
            ? "official"
            : "community",
      projection: isProjection(one.card),
      featured: named,
      /* The edition's own answer and never the source's: a translation
         is prose somebody wrote, and which hand wrote it is a fact about
         that text rather than about the one it adapts. */
      ...(one.card.authorship === undefined
        ? {}
        : { authorship: one.card.authorship }),
    }));
  });
}

/**
 * The door's three shelves, in the order they are offered.
 *
 * They answer three different questions and are deliberately not three
 * slices of one list. **Featured** is the deployment's answer to «where
 * do I start», and it is the only one a person chose. **Documents** is
 * every documentation somebody wrote — the featured ones included,
 * because a shelf that hid what it had just recommended would make a
 * reader who wanted the full list hunt for half of it. **Projections**
 * is what the pipeline printed from packages' own bytes, which is a
 * different kind of thing and says so on every card.
 */
export function catalogueShelves(
  libraries: readonly Library[],
  featured: readonly string[],
): readonly CatalogueShelf[] {
  const all = catalogueEntries(libraries, featured);
  return [
    { tab: "featured", entries: all.filter((one) => one.featured) },
    { tab: "documents", entries: all.filter((one) => !one.projection) },
    { tab: "projections", entries: all.filter((one) => one.projection) },
  ];
}

/**
 * The documentation-language filter a SHELF offers: every language the
 * site carries, then «Everything».
 *
 * One entry per language and never one per edition: the address
 * `vibevm.org/doc/ru/` is the site's Russian shelf, and three
 * documentations adapted into Russian are three cards on it rather than
 * three entries here. Where a language carries exactly one documentation
 * the entry is that edition's, star and publisher and all; where it
 * carries several, «published by» is answered with how many there are,
 * because the publisher of a shelf is not a fact and the shelf behind
 * the entry names every one of them.
 *
 * No entry carries an address, and that is the change the owner asked
 * for: every edition is already on the page, so choosing a language
 * narrows what stands there rather than moving the reader to a second
 * shelf showing the same cards.
 */
export function shelfLanguageChoices(
  libraries: readonly Library[],
  at: string | null,
): LanguageChoice[] {
  return [
    ...siteLanguages(libraries).map((one) => ({
      tag: one.tag,
      label: endonym(one.tag),
      publisher:
        one.count === 1 ? one.edition.publisher : `${one.count} documentations`,
      official: one.count === 1 && one.edition.official,
      source: one.segment === null,
      current: one.segment === at && at !== null,
    })),
    everyLanguage(at === null),
  ];
}

/** The addresses of the site's shelves, one per language it carries. */
export function catalogueDoors(
  libraries: readonly Library[],
): { tag: string; href: string; source: boolean }[] {
  return siteLanguages(libraries).map((one) => ({
    tag: one.tag,
    href: catalogueHref(one.segment),
    source: one.segment === null,
  }));
}
