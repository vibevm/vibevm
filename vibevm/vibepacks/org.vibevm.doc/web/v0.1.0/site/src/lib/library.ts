/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * The library a shell is showing: one source documentation and every
 * adaptation of it, as manifests turned into addresses.
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

/**
 * The library a set of manifests describes, in D-19's order: the source,
 * then the starred adaptations, then the community ones.
 *
 * The order is not decoration. Three signals have to agree — the star,
 * the word, and the place in the list — and the only way to keep them
 * agreeing is to sort by the same value the star is drawn from.
 *
 * A set with no source is refused rather than guessed at: every address
 * of every edition is built on the source's coordinate (D-06), so a
 * library of translations alone has nothing to be served under.
 */
export function libraryOf(manifests: readonly DocManifest[]): Library {
  const sources = manifests.filter(
    (manifest) => manifest.package.translation === undefined,
  );
  const found = sources[0];
  if (found === undefined) {
    throw new Error("no source manifest: every one of them is a translation");
  }
  /* One documentation and its adaptations, and not two documentations.
     Every address of every edition is built on the SOURCE's coordinate
     with a language segment in front of it (D-06), so a second source
     has nothing to be served under: it would be read as an adaptation
     of the first and would collide with any real one in its language.
     The site that carries many documentations is the registry build,
     which gives each its own library. */
  if (sources.length > 1) {
    const named = sources
      .map((one) => `${one.package.group}/${one.package.name}`)
      .join(", ");
    throw new Error(
      `${sources.length} source manifests in one library (${named}): this build renders one documentation and the adaptations of it`,
    );
  }
  const source = edition(found, null);
  const adapted = manifests
    .filter((manifest) => manifest !== found)
    .map((manifest) => edition(manifest, manifest.package.lang));
  return {
    source,
    editions: [
      source,
      ...adapted.filter((one) => one.official),
      ...adapted.filter((one) => !one.official),
    ],
  };
}

/**
 * A library out of manifests that are still bytes.
 *
 * Failing here fails with the name of the manifest that failed, which is
 * the only useful moment to learn that a manifest is not one — at build
 * time it stops the build, and in the local reader it is the difference
 * between «the store holds something this shell cannot read» and a page
 * that renders half a shelf.
 */
export function parseLibrary(
  inputs: readonly { readonly name: string; readonly value: unknown }[],
): Library {
  return libraryOf(
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
