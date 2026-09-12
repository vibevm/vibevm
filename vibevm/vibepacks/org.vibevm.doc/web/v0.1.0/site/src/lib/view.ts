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
  resolvePage,
  sourceEdition,
  type Edition,
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
  document: string,
  at: string | null,
): LanguageChoice[] {
  return editions().map((one) => {
    const reviewed = reviewedAt(one, document);
    return {
      tag: one.tag,
      label: endonym(one.tag),
      publisher: one.publisher,
      official: one.official,
      source: one.segment === null,
      href: docHref(addressOf(one.segment, document)),
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
 * page it offers the same page in each language, and anywhere else the
 * catalogue. The fragment is added by the behaviour on the way out —
 * markup cannot know where a reader is standing.
 */
export function headerLanguageChoices(pathname: string): LanguageChoice[] {
  const segments = docSegments(pathname);
  if (segments === null) return catalogueChoices(null);
  const target = parseDocTarget(segments);
  if (target === null) return catalogueChoices(null);
  if (target.kind === "page") {
    return languageChoices(target.address.document, target.address.lang);
  }
  return catalogueChoices(
    target.kind === "catalogue" ? target.lang : target.address.lang,
  );
}

/** The catalogue in every language the library has, for the site header. */
export function catalogueChoices(at: string | null): LanguageChoice[] {
  return editions().map((one) => ({
    tag: one.tag,
    label: endonym(one.tag),
    publisher: one.publisher,
    official: one.official,
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
export function versionChoices(address: DocAddress): VersionChoice[] {
  const newest = coordinate(address.lang).version;
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
  at: string | null,
  currentDocument: string | null,
): DocsNavItem[] {
  const pages = sourceEdition().pages;
  return pages.map((page) => {
    const document = documentOf(page.path);
    return {
      label: page.title,
      href: docHref(addressOf(at, document)),
      current: document === currentDocument,
    };
  });
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

function pageView(address: DocAddress): PageView | null {
  const resolved = resolvePage(address.lang, address.document);
  if (resolved === null) return null;
  const { edition, page, fallback } = resolved;
  const text = fallback ? sourceEdition() : edition;
  const newest = coordinate(address.lang).version;
  const source = sourceEdition();

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
    canonical: docHref(addressOf(source.segment, address.document)),
    packageAt: packageHref(address),
    mount: catalogueHref(null),
    links: projectionLinks(address),
    languages: languageChoices(address.document, address.lang),
    versions: versionChoices(address),
    nav: navItems(address.lang, address.document),
  };
}

function packageView(address: PackageAddress): PackageView | null {
  const all = editions();
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
      href: packageHref(coordinate(one.segment)),
      current: one.segment === address.lang,
    })),
    nav: navItems(address.lang, null),
    llms: llmsHref(address),
    subjects: card.subjects.map((subject) => ({
      package: subject.package,
      version: subject.version,
    })),
    adaptations: all
      .filter((one) => one.segment !== null)
      .map((one) => ({
        title: one.title,
        href: packageHref(coordinate(one.segment)),
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
 * `null` is «this is not an address of this library» — which the route
 * shows as such rather than as an empty page, because a shell that
 * renders nothing when it does not understand an address is a shell that
 * cannot be debugged from the outside.
 */
export function viewOf(raw: string): DocView | null {
  const target = parseDocTarget(raw.split("/"));
  if (target === null) return null;
  if (target.kind === "catalogue") {
    const known = editions().some((one) => one.segment === target.lang);
    return known ? { kind: "catalogue", lang: target.lang } : null;
  }
  return target.kind === "page"
    ? pageView(target.address)
    : packageView(target.address);
}
