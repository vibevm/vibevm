/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-SITEMAP */

/**
 * The documentation's sitemap: an index by package and language, and one
 * url set behind each entry of it.
 *
 * `##SEO-SITEMAP` asks for exactly that shape, and the shape is not
 * decoration. A single flat file would grow with every package the site
 * renders and would have to be refetched whole whenever one page of one
 * translation moved; an index lets a crawler ask for the part that
 * changed, and lets each part carry the date its own edition was
 * published.
 *
 * Two things never enter it. Addresses with a version number, because
 * every one of them carries `rel=canonical` to `latest` and a sitemap
 * that lists a page's non-canonical address asks a crawler to index what
 * the page itself says not to index. And translation fallbacks, because
 * a fallback page IS `noindex`: a sitemap saying «index this» beside a
 * page saying «do not» spends a crawler's fetch to be turned away.
 */

import {
  docFileHref,
  doorHref,
  indexableOf,
  lastmodOf,
  sourceOf,
  type Address,
  type Edition,
} from "./editions.ts";

/** Where the index itself is served, and what robots.txt names. */
export const DOC_SITEMAP = docFileHref("sitemap.xml");

/** One member of the index: a file, the addresses in it, and its date. */
export type SitemapPart = {
  /** Root-relative address of the part, as the index names it. */
  readonly href: string;
  /** The XML of the part itself. */
  readonly xml: string;
  /** The date the index prints beside it. */
  readonly lastmod: string;
};

/** Everything the build writes for the documentation's sitemap. */
export type Sitemap = {
  /** The index itself, at the address robots.txt names. */
  readonly index: string;
  /** The parts it names, in the order it names them. */
  readonly parts: readonly SitemapPart[];
  /** How many addresses the parts hold between them. */
  readonly addresses: number;
};

function escapeXml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * A url set.
 *
 * `lastmod` and nothing else. `changefreq` and `priority` are advisory
 * fields the search engines that read sitemaps have said for years they
 * ignore, and a number nobody reads is a number that goes stale without
 * anyone noticing. The landing's own sitemap keeps its two priorities
 * because it is compared byte for byte against the file it replaced; a
 * new file inherits no such obligation.
 */
function urlset(
  origin: string,
  entries: readonly { href: string; lastmod: string }[],
): string {
  const body = entries.map((entry) =>
    [
      "  <url>",
      `    <loc>${escapeXml(origin + entry.href)}</loc>`,
      `    <lastmod>${entry.lastmod}</lastmod>`,
      "  </url>",
    ].join("\n"),
  );
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ...body,
    "</urlset>",
    "",
  ].join("\n");
}

/**
 * The part an edition's pages live in: one file per package and
 * language.
 *
 * Named after the package whose coordinate the addresses inside it are
 * served under — the SOURCE's — and not after the package that published
 * the adaptation. A translation lives under the source's coordinate with
 * a language segment in front of it (D-06), so a file named after the
 * adaptation's own coordinate would name a path that appears nowhere
 * inside it.
 */
function partHref(source: Edition, edition: Edition): string {
  const card = source.manifest.package;
  const language = edition.segment ?? edition.tag;
  return docFileHref(`sitemap/${card.group}/${card.name}/${language}.xml`);
}

/**
 * The sitemap index and its parts, from the addresses the library has.
 *
 * The catalogues are a part of their own. They belong to no package —
 * the door and its per-language twins are the entrances of the site's documentation
 * half — and putting them into one package's file would make that file
 * lie the day a second package arrives.
 */
export function sitemapOf(
  origin: string,
  editions: readonly Edition[],
  addresses: readonly Address[],
): Sitemap {
  const indexable = indexableOf(addresses);
  const parts: SitemapPart[] = [];

  const source = sourceOf(editions);
  /* Catalogues are taken from all the addresses and not from the
     indexable ones: `latest` is a version spelling and a catalogue has no
     version to spell — a catalogue address is a language and nothing else. */
  const catalogues = [
    { href: doorHref(), lastmod: lastmodOf(source) },
    ...addresses
      .filter((address) => address.kind === "catalogue" && !address.fallback)
      .map((address) => ({
        href: address.href,
        lastmod: lastmodOf(address.edition),
      })),
  ];
  parts.push({
    href: docFileHref("sitemap/catalogues.xml"),
    xml: urlset(origin, catalogues),
    lastmod: lastmodOf(source),
  });

  for (const edition of editions) {
    const mine = indexable
      .filter((address) => address.edition === edition)
      .filter((address) => address.kind !== "catalogue")
      .map((address) => ({
        href: address.href,
        lastmod: lastmodOf(edition),
      }));
    if (mine.length === 0) continue;
    parts.push({
      href: partHref(source, edition),
      xml: urlset(origin, mine),
      lastmod: lastmodOf(edition),
    });
  }

  const index = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ...parts.map((part) =>
      [
        "  <sitemap>",
        `    <loc>${escapeXml(origin + part.href)}</loc>`,
        `    <lastmod>${part.lastmod}</lastmod>`,
        "  </sitemap>",
      ].join("\n"),
    ),
    "</sitemapindex>",
    "",
  ].join("\n");

  return {
    index,
    parts,
    addresses: parts.reduce(
      (total, part) => total + (part.xml.match(/<loc>/g)?.length ?? 0),
      0,
    ),
  };
}

/**
 * The catalogue addresses a catalogue part holds, exported for the one
 * caller that needs them without the XML: the link linter, which checks
 * that every address a sitemap names is a file the site actually wrote.
 */
export function locationsOf(xml: string): readonly string[] {
  return [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map(
    (match) => match[1] ?? "",
  );
}
