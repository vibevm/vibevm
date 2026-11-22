/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-SHELL-PARSES-NOTHING */

/**
 * What one documentation address means, worked out once and handed to
 * the route as plain values.
 *
 * The shell parses nothing and computes nothing about content: every
 * value a page shows arrives named, out of the manifest, through this
 * file (`##PIPE-SHELL-PARSES-NOTHING`). What this file does compute is
 * ADDRESSES — which page is the same page in another language, where the
 * `.md` beside it lies, what the `spec://` citation of this place is —
 * and that is the one thing the site is allowed to work out, because the
 * address map is deterministic by construction and needs no index (D-06).
 *
 * It is a separate file from the route for a plain reason: a route that
 * both decides what an address means and renders it cannot be read
 * without holding both in mind, and only one of the two can be tested
 * without a browser.
 */

import type {
  BridgeAuthorship,
  LanguageChoice,
  MetaLink,
  PackageKind,
  VersionChoice,
} from "@vibe-docs/design";

import type { Authorship } from "../generated/doc-manifest.ts";

import { bridgeOf } from "./bridge.ts";
import {
  pageCards,
  pathCards,
  type PageCard,
  type PathShelf,
} from "./cards.ts";
import {
  contentsOf,
  neighboursOf,
  type Contents,
  type PathNeighbours,
} from "./contents.ts";
import {
  catalogueHref,
  docHref,
  llmsHref,
  packageHref,
  parseDocTarget,
  projectionHref,
  specUri,
  type DocAddress,
  type PackageAddress,
} from "./href.ts";
import {
  addressOf,
  coordinate,
  documentOf,
  editions,
  kindOf,
  libraryAt,
  resolvePage,
  siteLanguages,
  sourceEdition,
  type Edition,
  type Library,
} from "./library.ts";

/**
 * What a language is called in itself.
 *
 * The manifest carries the BCP-47 tag, because a tag is identity and an
 * endonym is presentation — so the name has to come from somewhere, and
 * this table is the somewhere. It is deliberately short and deliberately
 * falls back to the tag: showing `pt-BR` is honest, and inventing a name
 * for a language nobody here reads would not be.
 */
const ENDONYM: Readonly<Record<string, string>> = {
  en: "English",
  ru: "Русский",
};

export function endonym(tag: string): string {
  return ENDONYM[tag] ?? tag;
}

/** The glyph a generated placeholder carries; a book, for documentation. */
export const DOC_GLYPH = "▤";

/** The language segment of the address the reader is at, or `null`. */
export function addressLanguage(target: PackageAddress): string | null {
  return target.lang;
}

/**
 * The language selector's entries for one page, in D-19's order: the
 * source, then the starred adaptations, then the community ones.
 *
 * Every entry points at the SAME page. That is possible because an
 * adaptation mirrors the source block for block and is served under the
 * source's coordinate with a language segment in front (D-06, §5), so
 * the destination is this address with one segment changed — and the
 * reader's fragment survives the move without anything having to store
 * where they were.
 */
export function languageChoices(
  library: Library,
  document: string,
  at: string | null,
): LanguageChoice[] {
  return editions(library).map((one) => {
    const reviewed = reviewedAt(one, document);
    return {
      tag: one.tag,
      label: endonym(one.tag),
      publisher: one.publisher,
      official: one.official,
      source: one.segment === null,
      href: docHref(addressOf(library, one.segment, document)),
      current: one.segment === at,
      ...(reviewed === undefined ? {} : { reviewedAt: reviewed }),
    };
  });
}

/** The label «Everything» wears wherever the filter is offered. */
const EVERY_LANGUAGE_LABEL = "Everything";

/**
 * The entry that hides nothing.
 *
 * It is the last of the three the owner asked for — English, Русский,
 * Everything — and it is not a language: it carries the tag `*`, which
 * is not a BCP-47 tag and cannot be mistaken for one, and it names no
 * publisher because a shelf of every edition has no single one.
 */
export function everyLanguage(current: boolean): LanguageChoice {
  return {
    tag: "*",
    label: EVERY_LANGUAGE_LABEL,
    publisher: "",
    official: false,
    source: false,
    current,
    note: "every language this build carries",
    pill: "all",
  };
}

/**
 * The documentation-language control one PAGE offers: the languages that
 * page exists in, then «Everything».
 *
 * Here the entries are addresses, because the other language of a page
 * is another page — and the reader keeps their place across the move,
 * which is the whole of `##READER-LANGUAGE-SWITCH-KEEPS-PLACE`. A
 * language the adaptation has not reached still answers: the address is
 * materialised with the source's text under it and says so on the page.
 *
 * «Everything» filters nothing here — there is one text on this page and
 * no shelf to narrow — and is offered all the same, because it is the
 * same control as on the shelves and a control that grew an entry on
 * one page and lost it on the next would be two controls.
 */
export function pageLanguageChoices(
  library: Library,
  document: string,
  at: string | null,
): LanguageChoice[] {
  return [...languageChoices(library, document, at), everyLanguage(false)];
}

function reviewedAt(one: Edition, document: string): string | undefined {
  const page = one.pages.find((each) => documentOf(each.path) === document);
  return page?.reviewed_at;
}

/** The address that always names the newest publication (D-06). */
const LATEST = "latest";

/**
 * Every spelling of the version, for whatever address the caller is on.
 *
 * `latest` stands first because it is the one to keep: an address with a
 * number shows whatever that number currently holds, so it is not a
 * permanent link to anything, and `##SITE-CANONICAL-LATEST` makes the
 * word the address every numbered page points its `rel=canonical` at,
 * the sitemap lists and a citation without a version resolves to. The
 * numbers follow, newest first, and each says what choosing it means.
 *
 * The numbers are the versions this BUILD carries, which today is one
 * per documentation: a deployment renders a package at a version and the
 * site is given that tree. One version is therefore one numbered entry
 * and the switch is shown all the same — a control that vanished when
 * there was a single answer would leave a reader unable to see which
 * version they are reading, which is the question it exists to answer.
 *
 * The destination is composed by the caller, because the same two
 * spellings name a page and a package's own front page, and those are
 * two shapes of address rather than two switches.
 */
function versionsOf(
  library: Library,
  lang: string | null,
  at: string,
  address: (version: string) => string,
): VersionChoice[] {
  const newest = coordinate(library, lang).version;
  return [
    {
      label: LATEST,
      href: address(LATEST),
      current: at === LATEST,
      note: "whichever version is newest when the page is read",
    },
    {
      label: newest,
      href: address(newest),
      current: at === newest,
      note: "this exact version; an address with a number always shows that version's current content",
    },
  ];
}

/**
 * The version switch on one page of a documentation.
 *
 * Every entry is the SAME document at another spelling of the version,
 * which is possible without a lookup because every version of a
 * documentation materialises every page of its source — the address map
 * is the coordinate with one segment changed, exactly as it is for
 * language. A page that a version did not carry would have to land on
 * that version's first page and say so; no build carries two numbered
 * versions of one documentation yet, so that case has no data to be
 * written against and would be a branch nothing could ever take.
 */
export function versionChoices(
  library: Library,
  address: DocAddress,
): VersionChoice[] {
  return versionsOf(library, address.lang, address.version, (version) =>
    docHref({ ...address, version }),
  );
}

/** The same switch on a documentation's own front page. */
export function packageVersionChoices(
  library: Library,
  address: PackageAddress,
): VersionChoice[] {
  return versionsOf(library, address.lang, address.version, (version) =>
    packageHref({ ...address, version }),
  );
}

/** The machine surfaces that lie beside a page, as the meta row shows them. */
export function projectionLinks(address: DocAddress): MetaLink[] {
  return [
    { label: ".md", href: projectionHref(address, "md") },
    { label: ".xml", href: projectionHref(address, "xml") },
    { label: "llms.txt", href: llmsHref(address) },
  ];
}

/** Everything one documentation page needs, as values. */
export type PageView = {
  readonly kind: "page";
  readonly address: DocAddress;
  readonly title: string;
  readonly summary: string;
  /** The language of the TEXT — the source's when this is a fallback. */
  readonly textLanguage: string;
  /** True when the edition does not carry this page and the source's does. */
  readonly fallback: boolean;
  readonly publisher: string;
  readonly version: string;
  readonly latest: boolean;
  readonly renderedAt: string;
  readonly reviewedAt?: string;
  readonly adapts?: string;
  readonly audiences: readonly string[];
  readonly readingMinutes: number;
  /** The citation an agent is handed — with the version, never `latest`. */
  readonly uri: string;
  /** The source page's own address, which a fallback is canonical to. */
  readonly canonical: string;
  readonly packageAt: string;
  /** Where the documentation is mounted, in this build's own base. */
  readonly mount: string;
  readonly links: readonly MetaLink[];
  readonly languages: readonly LanguageChoice[];
  readonly versions: readonly VersionChoice[];
  /** The manual's own pages, grouped as the column beside the text shows them. */
  readonly contents: Contents;
  /**
   * Where the learning path leads from this page, when the documentation
   * declared one and this page stands on it (`##NAV-CHAPTERS-READER`).
   * `null` is «there is no path to walk from here», which is every page
   * of every documentation that declared none.
   */
  readonly path: PathNeighbours | null;
};

/** Everything one package page needs, as values. */
export type PackageView = {
  readonly kind: "package";
  readonly address: PackageAddress;
  readonly title: string;
  readonly publisher: string;
  readonly coordinate: string;
  readonly description?: string;
  readonly abstract: string;
  readonly textLanguage: string;
  readonly status: "primary" | "official" | "community";
  /**
   * What kind of package this page is about, where the manifest lets it
   * be known (VIBEVM-SPEC §4.1). It is spelled `packageKind` and not
   * `kind` because this type already answers to that word: `kind` says
   * which of the three views a route is rendering, and two meanings of
   * one name on one object is how a page ends up drawing the mark of a
   * documentation on a catalogue.
   *
   * The same value travels to the head of the page, to the card of this
   * documentation and to every page card below it, so one package wears
   * one mark wherever this route puts it.
   */
  readonly packageKind?: PackageKind;
  /**
   * Who wrote this documentation's prose, when it says so
   * (`##CARD-AUTHORSHIP`). Absent is a card with no mark, here as on the
   * door: a documentation that declared nothing is not a documentation a
   * page may guess about.
   */
  readonly authorship?: Authorship;
  /**
   * The two authorships this documentation keeps apart, when it is a
   * bridge (PROP-023 `##AUTHORSHIP-SEPARATION`). The head of the page
   * shows both names or neither; nothing about them is worked out from
   * the publisher, and a package that is not a bridge is unchanged.
   */
  readonly bridge?: BridgeAuthorship;
  /**
   * The documentation-language filter this page offers: the languages
   * this documentation has, then «Everything». A filter and not a set of
   * addresses, because every edition is already on the shelves below.
   */
  readonly languages: readonly LanguageChoice[];
  /**
   * The version this front page is at, and the other spelling of it.
   * A documentation's own page carries the switch for the same reason
   * every page of it does: a reader must be able to see which version
   * they are being shown without reading the address bar.
   */
  readonly versions: readonly VersionChoice[];
  /** The manual's own pages, as the column beside the shelves shows them. */
  readonly contents: Contents;
  /** The same pages as `contents`, with what each one is about on them. */
  readonly pages: readonly PageCard[];
  /**
   * The same cards again, grouped into the chapters of the declared
   * learning path, or `null` when the documentation declared none — in
   * which case the shelf is `pages` in the manifest's order, as it was.
   */
  readonly chapters: readonly PathShelf[] | null;
  /**
   * Where a reader is asked to begin: the first page of the declared
   * path, or the first page of the manifest when there is no path
   * (`##NAV-CHAPTERS-READER`). Absent only for a documentation with no
   * pages at all, which has nowhere to be opened at.
   */
  readonly start?: string;
  readonly llms: string;
  readonly subjects: readonly { package: string; version: string }[];
  readonly adaptations: readonly {
    title: string;
    href: string;
    publisher: string;
    coordinate: string;
    description?: string;
    abstract: string;
    official: boolean;
    /** The BCP-47 tag of this adaptation, for the language filter. */
    tag: string;
    /** Its own kind, which for an adaptation of a manual is the manual's. */
    packageKind?: PackageKind;
    /** Who wrote this adaptation's prose, when it says so. */
    authorship?: Authorship;
    /** Its own two authorships, when the adaptation is itself a bridge. */
    bridge?: BridgeAuthorship;
  }[];
};

/** The catalogue of one language — the same shelf, at its own address. */
export type LanguageCatalogueView = {
  readonly kind: "catalogue";
  readonly lang: string;
};

export type DocView = PageView | PackageView | LanguageCatalogueView;

function pageView(library: Library, address: DocAddress): PageView | null {
  const resolved = resolvePage(library, address.lang, address.document);
  if (resolved === null) return null;
  const { edition, page, fallback } = resolved;
  const text = fallback ? sourceEdition(library) : edition;
  const newest = coordinate(library, address.lang).version;
  const source = sourceEdition(library);

  return {
    kind: "page",
    address,
    title: page.title,
    summary: page.summary,
    textLanguage: text.tag,
    fallback,
    publisher: text.publisher,
    version: address.version === "latest" ? newest : address.version,
    latest: address.version === "latest" || address.version === newest,
    renderedAt: text.card.rendered_at,
    ...(page.reviewed_at === undefined ? {} : { reviewedAt: page.reviewed_at }),
    ...(text.card.translation === undefined
      ? {}
      : { adapts: text.card.translation.package }),
    audiences: page.audiences,
    readingMinutes: page.reading_time_min,
    uri: specUri({ ...address, version: newest }),
    canonical: docHref(addressOf(library, source.segment, address.document)),
    packageAt: packageHref(address),
    mount: catalogueHref(null),
    links: projectionLinks(address),
    languages: pageLanguageChoices(library, address.document, address.lang),
    versions: versionChoices(library, address),
    contents: contentsOf(library, address.lang, address.document),
    path: neighboursOf(library, address.lang, address.document),
  };
}

function packageView(
  library: Library,
  address: PackageAddress,
): PackageView | null {
  const all = editions(library);
  const here = all.find((one) => one.segment === address.lang);
  if (here === undefined) return null;
  const card = here.card;
  const bridge = bridgeOf(card);
  const kind = kindOf(card);
  const cards = pageCards(library, address.lang);
  const chapters = pathCards(library, address.lang);
  /* Where the reader is asked to begin. A declared path says so itself,
     and its first page is rarely the manifest's first: the manifest is
     ordered by the layer law, which on the official manual opens with the
     architecture and reaches installing the product thirty pages later
     (`##NAV-CHAPTERS`). Without a path the manifest's first page is still
     the best answer there is. */
  const start = chapters?.flatMap((one) => one.pages)[0] ?? cards[0];

  return {
    kind: "package",
    address,
    title: card.title,
    publisher: card.publisher,
    coordinate: `${card.group}/${card.name}@${card.version}`,
    ...(card.description === undefined
      ? {}
      : { description: card.description }),
    abstract: card.abstract,
    textLanguage: card.lang,
    status: card.status,
    ...(kind === undefined ? {} : { packageKind: kind }),
    ...(card.authorship === undefined ? {} : { authorship: card.authorship }),
    ...(bridge === undefined ? {} : { bridge }),
    languages: [
      ...all.map((one) => ({
        tag: one.tag,
        label: endonym(one.tag),
        publisher: one.publisher,
        official: one.official,
        source: one.segment === null,
        current: one.segment === address.lang && address.lang !== null,
      })),
      everyLanguage(address.lang === null),
    ],
    versions: packageVersionChoices(library, address),
    contents: contentsOf(library, address.lang, null),
    pages: cards,
    chapters,
    ...(start === undefined ? {} : { start: start.href }),
    llms: llmsHref(address),
    subjects: card.subjects.map((subject) => ({
      package: subject.package,
      version: subject.version,
    })),
    adaptations: all
      .filter((one) => one.segment !== null)
      .map((one) => {
        const adapted = bridgeOf(one.card);
        const adaptedKind = kindOf(one.card);
        return {
          title: one.title,
          href: packageHref(coordinate(library, one.segment)),
          publisher: one.publisher,
          coordinate: `${one.card.group}/${one.card.name}@${one.card.version}`,
          ...(one.card.description === undefined
            ? {}
            : { description: one.card.description }),
          abstract: one.card.abstract,
          official: one.official,
          tag: one.tag,
          ...(adaptedKind === undefined ? {} : { packageKind: adaptedKind }),
          ...(one.card.authorship === undefined
            ? {}
            : { authorship: one.card.authorship }),
          ...(adapted === undefined ? {} : { bridge: adapted }),
        };
      }),
  };
}

/**
 * Read the catch-all route's path into everything the page will show.
 *
 * The address says which library answers for it: every address is built
 * on a source documentation's coordinate, so the group and the name in
 * it name one library out of however many this build carries, and no
 * search of the others follows.
 *
 * `null` is «this is not an address this build carries» — which the
 * route shows as such rather than as an empty page, because a shell that
 * renders nothing when it does not understand an address is a shell that
 * cannot be debugged from the outside.
 */
export function viewOf(
  libraries: readonly Library[],
  raw: string,
): DocView | null {
  const target = parseDocTarget(raw.split("/"));
  if (target === null) return null;
  if (target.kind === "catalogue") {
    /* A catalogue is a language of the SITE, so it exists when any
       library was adapted into it — the shelf it shows is every
       documentation, not one. */
    const known = siteLanguages(libraries).some(
      (one) => one.segment === target.lang,
    );
    return known ? { kind: "catalogue", lang: target.lang } : null;
  }
  const library = libraryAt(
    libraries,
    target.address.group,
    target.address.name,
  );
  if (library === null) return null;
  return target.kind === "page"
    ? pageView(library, target.address)
    : packageView(library, target.address);
}
