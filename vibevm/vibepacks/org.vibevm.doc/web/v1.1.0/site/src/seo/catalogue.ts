/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LLMS-FILES */

/**
 * The two documents the documentation half of the domain publishes about
 * itself: `vibevm.org/doc/llms.txt` and its `manifest.json`.
 *
 * Both are catalogues, and neither is a copy of anything the pipeline
 * wrote. A package's own `llms.txt` is an index of ITS pages and travels
 * from its `vibe doc build` output byte for byte; this one is an index of
 * the documentations the SITE carries — «title, officiality star,
 * publisher, language, audiences, abstract, link», the arXiv shape the
 * owner asked for (`##SEO-LLMS-FILES`). Nothing here is recomputed from
 * a page: every value is read off the manifests the pipeline emitted.
 *
 * The manifest is the same catalogue for a machine that will act on it
 * rather than read it: every edition, every page, its address in both
 * spellings of the version, its anchors, its audiences and its genre —
 * enough to decide what to fetch without fetching anything first
 * (`##SEO-MANIFEST-AND-RESOLVER`).
 */

import type { DocPage } from "../generated/doc-manifest.ts";
import {
  LATEST,
  docFileHref,
  documentOf,
  type Address,
  type Edition,
  type Library,
} from "./editions.ts";
import { RESOLVER_PAGE } from "./resolve.ts";

/** The star an edition wears, or the word that says it is the source. */
function standing(edition: Edition): string {
  if (edition.segment === null) return "source";
  return edition.official ? "★ official" : "community";
}

function addressOf(
  addresses: readonly Address[],
  edition: Edition,
  kind: Address["kind"],
  document?: string,
): string {
  const found = addresses.find(
    (address) =>
      address.edition === edition &&
      address.kind === kind &&
      address.version === LATEST &&
      address.document === document,
  );
  return found?.href ?? "";
}

/**
 * The catalogue an agent reads first.
 *
 * One entry per edition rather than one per package: a reader looking
 * for Russian wants to be told that Russian exists and who published it,
 * and a catalogue that folded the languages together would answer the
 * question with a shrug. The abstract is the package's own — four
 * questions answered in the package's own words (D-20) — and is quoted,
 * never summarised.
 */
export function catalogueLlmsTxt(
  origin: string,
  libraries: readonly Library[],
): string {
  const lines: string[] = [
    "# Documentation on vibevm.org",
    "",
    "> Every documentation this site carries, in every language it has been adapted into. A star marks an edition the documentation's own author published; the rest are the community's. Each entry links the edition's own index, which lists its pages.",
    "",
    "## Documentation",
    "",
  ];

  for (const library of libraries) {
    const whole = library.source.manifest.pages.length;
    for (const edition of library.editions) {
      const card = edition.manifest.package;
      const at = addressOf(library.addresses, edition, "package");
      const mine = edition.manifest.pages.length;
      lines.push(
        `### ${card.title} — ${standing(edition)}`,
        "",
        `- Address: [${origin}${at}](${origin}${at})`,
        `- Publisher: ${card.publisher}`,
        `- Language: ${card.lang}`,
        `- Documents: ${card.subjects.map((subject) => `${subject.package} ${subject.version}`).join(", ")}`,
        `- Audiences: ${card.audiences.join(", ")}`,
        /* An adaptation in progress is not a smaller manual: every page of
           the source answers at its address, in the source's words where
           the adaptation has not reached it yet. The count says which of
           the two a reader is choosing between. */
        edition.segment === null
          ? `- Pages: ${mine}`
          : `- Pages: ${mine} adapted of ${whole}; the rest are served in the source language`,
        `- Rendered: ${card.rendered_at.slice(0, 10)}`,
        `- Agent index: [${origin}${at}llms.txt](${origin}${at}llms.txt)`,
        "",
        card.abstract.trim(),
        "",
      );
    }
  }

  const file = (name: string): string => `${origin}${docFileHref(name)}`;
  lines.push(
    "## Machine-readable",
    "",
    `- [Page manifest](${file("manifest.json")}): every page, its address, anchors, audiences and genre.`,
    `- [Full text](${file("llms-full.txt")}): every page of every documentation, as one file.`,
    `- [Resolver](${origin}${RESOLVER_PAGE}): a spec:// citation turned into an address on this site — ask it with \`?uri=\`.`,
    `- [Sitemap](${file("sitemap.xml")})`,
    "",
    "Every page is also served as Markdown at its address with a `.md` suffix and as the dialect XML at `.xml`.",
  );

  return `${lines.join("\n")}\n`;
}

/** One page as the site's manifest describes it. */
type ManifestPage = {
  readonly document: string;
  readonly href: string;
  readonly latest_href: string;
  readonly markdown: string;
  readonly xml: string;
  readonly fallback: boolean;
  readonly title: string;
  readonly summary: string;
  readonly genre: DocPage["genre"];
  readonly audiences: DocPage["audiences"];
  readonly anchors: readonly string[];
  readonly reading_time_min: number;
};

/**
 * The site's page manifest: what it carries, where each page is, and in
 * which languages.
 *
 * It repeats each edition's package card rather than linking to it,
 * because the one question this document exists to answer — «what is
 * here and should I fetch it» — must be answerable from this document
 * alone. It carries no date of its own and no revision of any kind: a
 * version number is a contract and the site shows nothing behind it
 * (D-27).
 */
export function siteManifest(libraries: readonly Library[]): unknown {
  return {
    schema_version: 1,
    documentations: libraries.flatMap((library) =>
      library.editions.map((edition) => one(library.addresses, edition)),
    ),
  };
}

/** One edition of one library, as the site's manifest describes it. */
function one(addresses: readonly Address[], edition: Edition): unknown {
  const pages: ManifestPage[] = addresses
    .filter(
      (address) =>
        address.edition === edition &&
        address.kind === "page" &&
        address.version !== LATEST,
    )
    .map((address) => {
      const document = address.document ?? "";
      const latest = addressOf(addresses, edition, "page", document);
      const page = address.page;
      return {
        document,
        href: address.href,
        latest_href: latest,
        markdown: `${address.href.slice(0, -1)}.md`,
        xml: `${address.href.slice(0, -1)}.xml`,
        fallback: address.fallback,
        title: page?.title ?? document,
        summary: page?.summary ?? "",
        genre: page?.genre ?? "concept",
        audiences: page?.audiences ?? [],
        anchors: page?.anchors ?? [],
        reading_time_min: page?.reading_time_min ?? 1,
      };
    });

  return {
    language: edition.tag,
    language_segment: edition.segment,
    source: edition.segment === null,
    official: edition.official,
    href: addressOf(addresses, edition, "package"),
    package: edition.manifest.package,
    pages,
  };
}

/** Every document path the source declares, for the surfaces that copy them. */
export function documentsOf(source: Edition): readonly string[] {
  return source.manifest.pages.map((page) => documentOf(page.path));
}
