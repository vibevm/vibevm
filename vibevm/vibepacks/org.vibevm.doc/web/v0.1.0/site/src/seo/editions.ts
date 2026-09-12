/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

/**
 * The library as the machine surfaces see it: editions, addresses, and
 * which page each address serves.
 *
 * It exists beside `lib/library.ts` rather than inside it because of who
 * calls it. The library is the shell's — it is imported by routes, it
 * pulls the fixture manifests in through the bundler, and nothing
 * outside a Vite build can load it. The sitemap, the resolver table, the
 * catalogue index and the copy of the agent surfaces are written by
 * `tools/build.mjs` under plain Node, after the pages are on disk, and
 * they need exactly the same answers: which editions exist, what each
 * one's addresses are, and which of those addresses is a translation
 * fallback.
 *
 * So the manifests arrive as arguments. One module, two callers, and no
 * JSON import that only a bundler can resolve. The one thing it does NOT
 * re-implement is the address map itself: every path here is built by
 * `lib/href.ts`, which is the single place a documentation address is spelled.
 */

import type { DocManifest, DocPage } from "../generated/doc-manifest.ts";
import {
  cataloguePath,
  docPath,
  href,
  packagePath,
  type DocAddress,
  type PackageAddress,
} from "../lib/href.ts";

/**
 * The version segment that always names the newest publication.
 *
 * It is an address, not a number: `spec://group/name/document` without a
 * version resolves here (D-06), the version switch offers it beside the
 * number, and `##SITE-CANONICAL-LATEST` makes it the address a versioned
 * page points its `rel=canonical` at. So the site materialises it like
 * any other address rather than leaving it as a link nothing answers.
 */
export const LATEST = "latest";

/** One language of one documentation, with the manifest behind it. */
export type Edition = {
  /** The language segment of the address; `null` for the source. */
  readonly segment: string | null;
  /** The BCP-47 tag of the text. */
  readonly tag: string;
  /** The manifest the pipeline wrote for this edition's package. */
  readonly manifest: DocManifest;
  /** Whether the edition wears the star: the source's group published it. */
  readonly official: boolean;
};

/** One address of the site's documentation half. */
export type Address = {
  /** Root-relative, with the build's base and a trailing slash. */
  readonly href: string;
  readonly kind: "catalogue" | "package" | "page";
  /** The edition this address belongs to; catalogues carry their own. */
  readonly edition: Edition;
  /** `latest` or the version number. */
  readonly version: string;
  /** The document behind the package, for a page address. */
  readonly document?: string;
  /** The page as the serving edition has it — the source's on a fallback. */
  readonly page?: DocPage;
  /**
   * True when this edition does not carry the page and the source's text
   * is served under its address. A fallback is `noindex` and canonical to
   * the source, so it never stands in a sitemap.
   */
  readonly fallback: boolean;
};

/** The extension a manifest path carries is part of the file, not of the address. */
export function documentOf(path: string): string {
  return path.replace(/\.(xml|md)$/, "");
}

/**
 * The editions in D-19's order: the source, then the starred
 * adaptations, then the community ones.
 *
 * The order is the same one the language selector draws, and for the
 * same reason: the star, the word and the place in the list have to
 * agree, and the only way to keep them agreeing is to sort by the value
 * the star is drawn from.
 */
export function editionsOf(
  manifests: readonly DocManifest[],
): readonly Edition[] {
  const one = (manifest: DocManifest, segment: string | null): Edition => ({
    segment,
    tag: manifest.package.lang,
    manifest,
    official: manifest.package.translation?.status === "official",
  });

  const source = manifests.find(
    (manifest) => manifest.package.translation === undefined,
  );
  if (source === undefined) {
    throw new Error("no source manifest: every one of them is a translation");
  }
  const adapted = manifests
    .filter((manifest) => manifest !== source)
    .map((manifest) => one(manifest, manifest.package.lang));

  return [
    one(source, null),
    ...adapted.filter((edition) => edition.official),
    ...adapted.filter((edition) => !edition.official),
  ];
}

/** The source edition — the coordinate every address is built on. */
export function sourceOf(editions: readonly Edition[]): Edition {
  const source = editions.find((edition) => edition.segment === null);
  if (source === undefined) {
    throw new Error("no source edition among the manifests");
  }
  return source;
}

/** The package coordinate of an address: always the source's, in one language. */
export function coordinateOf(
  source: Edition,
  segment: string | null,
  version: string,
): PackageAddress {
  const card = source.manifest.package;
  return { lang: segment, group: card.group, name: card.name, version };
}

/** The page one edition has under a document path, or nothing. */
function pageOf(edition: Edition, document: string): DocPage | undefined {
  return edition.manifest.pages.find(
    (page) => documentOf(page.path) === document,
  );
}

/**
 * Every address the documentation half of the site has, in both
 * spellings of the version.
 *
 * Two rules produce the list, and neither is negotiable.
 *
 * Every LANGUAGE materialises every page of the SOURCE. A page an
 * adaptation has not reached is written anyway, in the source's words
 * and under the adaptation's address, which is what makes a language
 * never a 404 (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`).
 *
 * And every page has two addresses: the version number and `latest`.
 * They are the same content — an address with a number shows that
 * number's current content, and there is no permanent link to a past
 * publication (`##SITE-VERSION-SHOWS-CURRENT`) — so the numbered one
 * carries `rel=canonical` to `latest` and the sitemap lists `latest`
 * alone.
 */
export function addressesOf(editions: readonly Edition[]): readonly Address[] {
  const source = sourceOf(editions);
  const number = source.manifest.package.version;
  const out: Address[] = [];

  for (const edition of editions) {
    if (edition.segment !== null) {
      out.push({
        href: href(cataloguePath(edition.segment)),
        kind: "catalogue",
        edition,
        version: number,
        fallback: false,
      });
    }
    for (const version of [number, LATEST]) {
      const at = coordinateOf(source, edition.segment, version);
      out.push({
        href: href(packagePath(at)),
        kind: "package",
        edition,
        version,
        fallback: false,
      });
      for (const declared of source.manifest.pages) {
        const document = documentOf(declared.path);
        const own = pageOf(edition, document);
        const address: DocAddress = { ...at, document };
        out.push({
          href: href(docPath(address)),
          kind: "page",
          edition,
          version,
          document,
          page: own ?? declared,
          fallback: own === undefined,
        });
      }
    }
  }
  return out;
}

/** The door of the documentation half: the one address with no edition. */
export function doorHref(): string {
  return href(cataloguePath(null));
}

/**
 * A machine file the documentation half publishes beside its pages —
 * `sitemap.xml`, `llms.txt`, `manifest.json`, `resolve.json`, and the
 * parts of the sitemap index (`##SITE-ONE-SITE`).
 *
 * It goes through the address map like everything else. Writing
 * the address of the sitemap into a module would be right in the build that
 * serves a domain and wrong in the one `vibe` embeds, and the test beside
 * `href.ts` catches exactly that literal.
 */
export function docFileHref(name: string): string {
  return href(`${cataloguePath(null)}${name}`);
}

/** The addresses a crawler should be offered: `latest`, and never a fallback. */
export function indexableOf(addresses: readonly Address[]): readonly Address[] {
  return addresses.filter(
    (address) => address.version === LATEST && !address.fallback,
  );
}

/**
 * The date a sitemap dates an edition by.
 *
 * `published_at` when the pipeline recorded one, because that is what
 * `##SEO-SITEMAP` asks for; `rendered_at` otherwise, which is the only
 * other date a manifest carries. Neither is shown to a reader: a page
 * carries exactly two dates and this is not one of them (D-27).
 */
export function lastmodOf(edition: Edition): string {
  const card = edition.manifest.package;
  return (card.published_at ?? card.rendered_at).slice(0, 10);
}
