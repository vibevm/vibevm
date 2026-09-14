/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * The library a shell is showing: one source documentation and every
 * adaptation of it, as manifests turned into addresses — and, since a
 * site carries as many libraries as it carries source documentations,
 * the grouping that decides which manifests make up which library.
 *
 * Two rules shape this file and nothing else does.
 *
 * The first is that a translation is served under the SOURCE package's
 * coordinate with a language segment in front of it (D-06). Not under
 * its own name — and that is not a cosmetic choice about pretty URLs. It
 * is what makes «the same page in another language with the same
 * fragment» an address the site can compute rather than a lookup it has
 * to keep: a page's address and its adaptation's differ in exactly one
 * segment, so the language selector is a string operation and the reader
 * never loses their place (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`).
 *
 * The second is that officiality is never read from a flag. It arrives
 * computed, on the manifest, from the convergence of the `documents` and
 * `translates` edges at render time (`##REL-NO-OFFICIAL-FLAG`), and the
 * shell's whole part in it is to print the value it was handed beside
 * the word and the order that agree with it (D-19).
 *
 * Nothing here reaches for a library of its own. Every function takes
 * the one it is answering about, because there are two and they arrive
 * by different roads: the public site's is baked in at build time out of
 * the documentation trees the deployment rendered, and the local
 * reader's is fetched from `<base>manifest.json` while the page is being
 * read. A module-level library would have made the second impossible to
 * express and the first impossible to test.
 *
 * And a build is given a LIST of them. One library is one source
 * documentation and its adaptations; a registry render hands the site
 * every package it published, which is forty-eight documentations that
 * adapt nothing of each other's. So the manifests are grouped by the
 * coordinate an adaptation names in `translates`, and each group becomes
 * a library of its own with its own addresses, its own `latest` and its
 * own languages.
 */

import type {
  DocManifest,
  DocPage,
  DocPackage,
} from "../generated/doc-manifest.ts";
import { parseDocManifest } from "./manifest.ts";
import type { DocAddress, PackageAddress } from "./href.ts";

/** One language this documentation can be read in. */
export type Edition = {
  /**
   * The language segment of the address, or `null` for the source —
   * whose pages carry no prefix at all.
   */
  readonly segment: string | null;
  /** The BCP-47 tag of the text itself. */
  readonly tag: string;
  /** The edition's own display title, in its own language. */
  readonly title: string;
  /** The group that published this edition, printed wherever it is. */
  readonly publisher: string;
  /**
   * Whether this edition wears the star: the source's own group
   * published it as `<name>-<tag>`. The source edition is not
   * «official» — it is the thing officiality is measured against — so
   * it carries `false` and the selector marks it as the source instead.
   */
  readonly official: boolean;
  /** The package this edition's pages came from. */
  readonly card: DocPackage;
  /** The pages this edition actually carries, by manifest path. */
  readonly pages: readonly DocPage[];
};

/**
 * One documentation, in every language a shell was given it in.
 *
 * It is a value and not a module, so the same code answers for the site
 * and for the reader `vibe` serves — which is the whole reason the local
 * reader can show the package it was pointed at rather than the fixture
 * its shell was built from.
 */
export type Library = {
  /** The source edition, then the starred adaptations, then the rest. */
  readonly editions: readonly Edition[];
  /** The source — the coordinate every address in this library is built on. */
  readonly source: Edition;
};

/** The extension a manifest path carries is part of the file, not of the address. */
export function documentOf(path: string): string {
  return path.replace(/\.(xml|md)$/, "");
}

/**
 * Whether a card is a level-zero RENDERING of a package rather than a
 * documentation package about one.
 *
 * The two are different things wearing the same shape. A `doc` package
 * is prose somebody wrote about a subject; a level-zero rendering is the
 * pipeline printing a package's own bytes as pages, so that every
 * package on the registry has something to read even where nobody has
 * written a word. A reader deciding which of the two they are looking at
 * is the whole reason the catalogue has a shelf for each.
 *
 * **This is the one place that decides it, and it decides it twice.**
 * `projection` is the field the manifest is gaining, and a card that
 * carries it is believed: the builder is the only thing that knows
 * which of its two modes produced a tree, so the answer belongs there
 * and arrives named like every other value (`##PIPE-SHELL-PARSES-
 * NOTHING`). Until every deployment's manifests carry it there is one
 * signal a manifest already has, and it is not a guess: a rendering of a
 * package documents THAT package, so its own coordinate stands among its
 * subjects, while a `doc` package names something other than itself
 * (`##REL-DOCUMENTS-REQUIRED`). The day the field is everywhere, the
 * second half of this function goes and the first stays.
 */
export function isProjection(card: DocPackage): boolean {
  const declared: unknown = Reflect.get(card, "projection");
  if (typeof declared === "boolean") return declared;
  const own = `${card.group}/${card.name}`;
  return card.subjects.some((subject) => subject.package === own);
}

function edition(manifest: DocManifest, segment: string | null): Edition {
  const card = manifest.package;
  return {
    segment,
    tag: card.lang,
    title: card.title,
    publisher: card.publisher,
    official: card.translation?.status === "official",
    card,
    pages: manifest.pages,
  };
}

/** One documentation and the adaptations of it, still as manifests. */
export type SourceGroup = {
  readonly source: DocManifest;
  readonly adaptations: readonly DocManifest[];
};

/** The `<group>/<name>` an adaptation names its source by (`translates`). */
function coordinateOf(manifest: DocManifest): string {
  return `${manifest.package.group}/${manifest.package.name}`;
}

/**
 * The manifests a build was given, grouped into the libraries they are.
 *
 * An adaptation belongs to the documentation it names in `translates`
 * and to no other — never to whichever source happens to be first, and
 * never to a language it shares with an unrelated package. A source that
 * nobody adapted is a library of one edition, which is what every
 * package of a registry render is.
 *
 * An adaptation whose SOURCE this build does not carry becomes the
 * source of a library of its own, under its own coordinate. The
 * alternative was to refuse it, and a registry that publishes a
 * volunteer's translation of something published elsewhere would then
 * stop the whole render over one package — the same failure a render
 * refuses to make of a broken package. Nothing collides: the coordinate
 * an adaptation is served under here is its own, and the source it names
 * has no address on this site to be confused with.
 *
 * The order is the order the manifests were named, taken at each
 * library's first mention, so a deployment's `VIBE_DOC_OUT` decides it
 * and two runs over the same trees agree.
 */
export function groupBySource(
  manifests: readonly DocManifest[],
): readonly SourceGroup[] {
  const sources = new Map<string, DocManifest>();
  for (const manifest of manifests) {
    if (manifest.package.translation !== undefined) continue;
    const at = coordinateOf(manifest);
    if (!sources.has(at)) sources.set(at, manifest);
  }

  const groups = new Map<
    string,
    { source: DocManifest; adapted: DocManifest[] }
  >();
  const open = (at: string, source: DocManifest) => {
    const group = { source, adapted: [] };
    groups.set(at, group);
    return group;
  };

  for (const manifest of manifests) {
    const adapts = manifest.package.translation?.package;
    const source = adapts === undefined ? undefined : sources.get(adapts);
    if (adapts !== undefined && source !== undefined) {
      (groups.get(adapts) ?? open(adapts, source)).adapted.push(manifest);
      continue;
    }
    const at = coordinateOf(manifest);
    if (!groups.has(at)) open(at, manifest);
  }

  return [...groups.values()].map((group) => ({
    source: group.source,
    adaptations: group.adapted,
  }));
}

/**
 * The library one group describes, in D-19's order: the source, then the
 * starred adaptations, then the community ones.
 *
 * The order is not decoration. Three signals have to agree — the star,
 * the word, and the place in the list — and the only way to keep them
 * agreeing is to sort by the same value the star is drawn from.
 */
export function libraryOf(group: SourceGroup): Library {
  const source = edition(group.source, null);
  const adapted = group.adaptations.map((manifest) =>
    edition(manifest, manifest.package.lang),
  );
  return {
    source,
    editions: [
      source,
      ...adapted.filter((one) => one.official),
      ...adapted.filter((one) => !one.official),
    ],
  };
}

/** Every library a set of manifests describes, one per source documentation. */
export function librariesOf(
  manifests: readonly DocManifest[],
): readonly Library[] {
  return groupBySource(manifests).map(libraryOf);
}

/**
 * The libraries out of manifests that are still bytes.
 *
 * Failing here fails with the name of the manifest that failed, which is
 * the only useful moment to learn that a manifest is not one — at build
 * time it stops the build, and in the local reader it is the difference
 * between «the store holds something this shell cannot read» and a page
 * that renders half a shelf.
 */
export function parseLibraries(
  inputs: readonly { readonly name: string; readonly value: unknown }[],
): readonly Library[] {
  return librariesOf(
    inputs.map((one) => {
      const parsed = parseDocManifest(one.value);
      if (!parsed.ok) {
        throw new Error(
          `${one.name} is not a page manifest: ${parsed.error.path} — ${parsed.error.reason}`,
        );
      }
      return parsed.value;
    }),
  );
}

/**
 * The library one documentation address belongs to, found by the
 * coordinate the address is built on.
 *
 * That coordinate is always the SOURCE's — an adaptation is served under
 * it with a language segment in front (D-06) — so one lookup answers for
 * every language of every library, and an address of a documentation
 * this build does not carry answers `null` rather than the first library
 * that happens to be there.
 */
export function libraryAt(
  libraries: readonly Library[],
  group: string,
  name: string,
): Library | null {
  return (
    libraries.find(
      (one) => one.source.card.group === group && one.source.card.name === name,
    ) ?? null
  );
}

/** Every edition, source first. */
export function editions(library: Library): readonly Edition[] {
  return library.editions;
}

/** The source documentation — the coordinate every address is built on. */
export function sourceEdition(library: Library): Edition {
  return library.source;
}

/** The coordinate of the documentation, in one language or another. */
export function coordinate(
  library: Library,
  lang: string | null,
): PackageAddress {
  const card = library.source.card;
  return {
    lang,
    group: card.group,
    name: card.name,
    version: card.version,
  };
}

/** The address of one page of the source documentation, in one language. */
export function addressOf(
  library: Library,
  lang: string | null,
  document: string,
): DocAddress {
  return { ...coordinate(library, lang), document };
}

/** A page as one edition has it — or as the source has it, when it does not. */
export type ResolvedPage = {
  readonly edition: Edition;
  /** The page's own manifest entry in the edition that serves it. */
  readonly page: DocPage;
  /**
   * True when the edition does not carry this page and the source's own
   * text is being served under the edition's address.
   */
  readonly fallback: boolean;
};

function findEdition(library: Library, segment: string | null): Edition | null {
  return library.editions.find((one) => one.segment === segment) ?? null;
}

function findPage(within: Edition, document: string): DocPage | null {
  return (
    within.pages.find((page) => documentOf(page.path) === document) ?? null
  );
}

/**
 * Which page an address names, and in whose language it will be read.
 *
 * `null` only for an address the library does not have at all — a
 * language nobody published, or a document no edition carries. A page
 * the chosen language is merely missing is NOT one of those: it resolves
 * to the source's text under the chosen language's address, which is the
 * whole of the fallback rule and the reason a translation never 404s.
 */
export function resolvePage(
  library: Library,
  segment: string | null,
  document: string,
): ResolvedPage | null {
  const chosen = findEdition(library, segment);
  if (chosen === null) return null;

  const own = findPage(chosen, document);
  if (own !== null) return { edition: chosen, page: own, fallback: false };

  const fallback = findPage(library.source, document);
  if (fallback === null) return null;
  return { edition: chosen, page: fallback, fallback: true };
}

/** One address this build writes to disk. */
export type SiteAddress = {
  /** The catch-all route's `path` parameter — no leading or trailing slash. */
  readonly path: string;
  readonly kind: "page" | "package" | "catalogue";
  /** True when the page is the source's text under another language's address. */
  readonly fallback: boolean;
};

function prefix(library: Library, segment: string | null): string {
  const card = library.source.card;
  const lang = segment === null ? "" : `${segment}/`;
  return `${lang}${card.group}/${card.name}/${card.version}`;
}

/**
 * Every documentation address the site has, as the generator's
 * parameters — the pages of every edition plus each edition's own
 * package page.
 *
 * This is the site's half of the page-count gate. The build counts what
 * the generator reported against what this list says exists, so the two
 * numbers have to be arrived at independently: the build driver reads
 * the manifests itself rather than importing this file
 * (`##STACK-PAGE-COUNT-GATE`).
 */
export function siteAddresses(library: Library): readonly SiteAddress[] {
  const out: SiteAddress[] = [];
  for (const one of library.editions) {
    const at = prefix(library, one.segment);
    // Every language has a catalogue of its own; the source's is the
    // door itself, which is a route file rather than an address of
    // this list.
    if (one.segment !== null) {
      out.push({ path: one.segment, kind: "catalogue", fallback: false });
    }
    out.push({ path: at, kind: "package", fallback: false });
    // Every edition materialises every page of the SOURCE, not only the
    // ones it has. A page an adaptation is missing is written anyway,
    // with the source's text under the adaptation's address: that is the
    // fallback, and it is what makes a language never 404
    // (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`). A missing page is a fact
    // about the adaptation's progress, not a hole in the site.
    for (const page of library.source.pages) {
      const document = documentOf(page.path);
      out.push({
        path: `${at}/${document}`,
        kind: "page",
        fallback: findPage(one, document) === null,
      });
    }
  }
  return out;
}

/**
 * Every documentation address the whole site has — every library's, with
 * one catalogue per language rather than one per adaptation.
 *
 * A catalogue is a LANGUAGE and not an edition: `vibevm.org/doc/ru/` is the site's
 * Russian shelf, and three documentations adapted into Russian are three
 * cards on it, not three pages at one address. The static generator is
 * handed this list, so a duplicate here would be an address written
 * twice and a page-count gate that disagreed with itself.
 */
export function siteAddressesOf(
  libraries: readonly Library[],
): readonly SiteAddress[] {
  const out: SiteAddress[] = [];
  const catalogues = new Set<string>();
  for (const library of libraries) {
    for (const address of siteAddresses(library)) {
      if (address.kind === "catalogue") {
        if (catalogues.has(address.path)) continue;
        catalogues.add(address.path);
      }
      out.push(address);
    }
  }
  return out;
}

/**
 * Every language the site carries, once each, in the order the libraries
 * put them: the source languages first, then the starred adaptations,
 * then the community ones.
 *
 * The entry is an EDITION because that is what the language selector is
 * built to show — a publisher and a star, which belong to a
 * documentation and not to a language. Where a language carries more
 * than one documentation, the count of them is the honest answer to
 * «published by whom», and the catalogue the entry leads to names each
 * one with its own publisher and its own star.
 */
export type SiteLanguage = {
  readonly segment: string | null;
  readonly tag: string;
  /** The one edition in this language, when there is only one. */
  readonly edition: Edition;
  /** How many documentations the site carries in this language. */
  readonly count: number;
};

export function siteLanguages(
  libraries: readonly Library[],
): readonly SiteLanguage[] {
  const found = new Map<string, { edition: Edition; count: number }>();
  for (const library of libraries) {
    for (const one of library.editions) {
      const key = one.segment ?? "";
      const seen = found.get(key);
      if (seen === undefined) found.set(key, { edition: one, count: 1 });
      else seen.count += 1;
    }
  }
  return [...found.values()].map((one) => ({
    segment: one.edition.segment,
    tag: one.edition.tag,
    edition: one.edition,
    count: one.count,
  }));
}
