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
  DocsNavItem,
  LanguageChoice,
  MetaLink,
  VersionChoice,
} from "@vibe-docs/design";

import type { DocumentationStatus } from "../generated/doc-manifest.ts";

import {
  catalogueHref,
  docHref,
  docSegments,
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

function endonym(tag: string): string {
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

/**
 * The selector the site header shows, from nothing but the path the
 * browser is on.
 *
 * The header stands over the landing and over every documentation page,
 * and a selector that led to the catalogue from a page would lose the
 * reader's place on every switch. So it reads the address back: on a
 * page it offers the same page in each language OF THAT DOCUMENTATION,
 * and anywhere else the catalogue of each language the site carries. The
 * fragment is added by the behaviour on the way out — markup cannot know
 * where a reader is standing.
 */
export function headerLanguageChoices(
  libraries: readonly Library[],
  pathname: string,
): LanguageChoice[] {
  const segments = docSegments(pathname);
  if (segments === null) return catalogueChoices(libraries, null);
  const target = parseDocTarget(segments);
  if (target === null) return catalogueChoices(libraries, null);
  if (target.kind === "catalogue") {
    return catalogueChoices(libraries, target.lang);
  }
  const library = libraryAt(
    libraries,
    target.address.group,
    target.address.name,
  );
  if (library === null) return catalogueChoices(libraries, null);
  if (target.kind === "package") {
    return catalogueChoices(libraries, target.address.lang);
  }
  return languageChoices(library, target.address.document, target.address.lang);
}

/**
 * The catalogue in every language the site has, for the site header.
 *
 * One entry per language and never one per edition: the address
 * `vibevm.org/doc/ru/` is the site's Russian shelf, and three
 * documentations adapted into Russian are three cards on it rather than
 * three entries here. Where a language
 * carries exactly one documentation the entry is that edition's, star
 * and publisher and all; where it carries several, «published by» is
 * answered with how many there are, because the publisher of a shelf is
 * not a fact and the catalogue behind the entry names every one of them.
 */
export function catalogueChoices(
  libraries: readonly Library[],
  at: string | null,
): LanguageChoice[] {
  return siteLanguages(libraries).map((one) => ({
    tag: one.tag,
    label: endonym(one.tag),
    publisher:
      one.count === 1 ? one.edition.publisher : `${one.count} documentations`,
    official: one.count === 1 && one.edition.official,
    source: one.segment === null,
    href: catalogueHref(one.segment),
    current: one.segment === at,
  }));
}

function reviewedAt(one: Edition, document: string): string | undefined {
  const page = one.pages.find((each) => documentOf(each.path) === document);
  return page?.reviewed_at;
}

/**
 * The version switch for one address.
 *
 * Two entries and not a list of releases: the build carries the versions
 * it was given, and this one was given one. The number and the word
 * `latest` are two ADDRESSES of the same content, which is exactly what
 * the norm says they are — an address with a number shows the current
 * content of that number — so the switch offers both and says what each
 * of them means.
 */
export function versionChoices(
  library: Library,
  address: DocAddress,
): VersionChoice[] {
  const newest = coordinate(library, address.lang).version;
  return [
    {
      label: newest,
      href: docHref({ ...address, version: newest }),
      current: address.version === newest,
      note: "this exact version; an address with a number always shows that version's current content",
    },
    {
      label: "latest",
      href: docHref({ ...address, version: "latest" }),
      current: address.version === "latest",
      note: "whichever version is newest when the page is read",
    },
  ];
}

/**
 * The documentation's own navigation, in the manifest's order.
 *
 * It lists the SOURCE's pages in every language, not the pages the
 * chosen edition happens to have. The reason is that an adaptation in
 * progress is not a smaller manual: every page exists at every language's
 * address, and one an adaptation has not reached yet is served in the
 * source's words with a notice. A navigation built from the edition's own
 * manifest would silently hide the pages a reader most needs to be told
 * about.
 */
export function navItems(
  library: Library,
  at: string | null,
  currentDocument: string | null,
): DocsNavItem[] {
  const pages = sourceEdition(library).pages;
  return pages.map((page) => {
    const document = documentOf(page.path);
    return {
      label: page.title,
      href: docHref(addressOf(library, at, document)),
      current: document === currentDocument,
    };
  });
}

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
};

/**
 * The shelf behind the door: every edition of every library, in the
 * libraries' own order and, inside each, D-19's.
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
): readonly CatalogueEntry[] {
  return libraries.flatMap((library) =>
    editions(library).map((one) => ({
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
    })),
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
  readonly nav: readonly DocsNavItem[];
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
  readonly editions: readonly LanguageChoice[];
  readonly nav: readonly DocsNavItem[];
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
    languages: languageChoices(library, address.document, address.lang),
    versions: versionChoices(library, address),
    nav: navItems(library, address.lang, address.document),
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
    editions: all.map((one) => ({
      tag: one.tag,
      label: endonym(one.tag),
      publisher: one.publisher,
      official: one.official,
      source: one.segment === null,
      href: packageHref(coordinate(library, one.segment)),
      current: one.segment === address.lang,
    })),
    nav: navItems(library, address.lang, null),
    llms: llmsHref(address),
    subjects: card.subjects.map((subject) => ({
      package: subject.package,
      version: subject.version,
    })),
    adaptations: all
      .filter((one) => one.segment !== null)
      .map((one) => ({
        title: one.title,
        href: packageHref(coordinate(library, one.segment)),
        publisher: one.publisher,
        coordinate: `${one.card.group}/${one.card.name}@${one.card.version}`,
        ...(one.card.description === undefined
          ? {}
          : { description: one.card.description }),
        abstract: one.card.abstract,
        official: one.official,
      })),
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
