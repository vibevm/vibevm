/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-CANONICAL-HREFLANG */

/**
 * What a documentation address says in its `<head>`.
 *
 * The landing has had this since it was ported, tag for tag
 * (`landing/head.ts`); this is the same contract for the half of the
 * domain that has thousands of addresses rather than three, and the
 * difference is that here nothing can be written by hand. Every value is
 * computed from the address and the manifest behind it.
 *
 * Three rules decide the shape, and each one is a rule about which of
 * several addresses is THE address.
 *
 * A page has two version spellings — the number and `latest` — and one
 * of them is canonical. `##SITE-CANONICAL-LATEST` picks `latest`: the
 * numbered address shows whatever that number currently holds
 * (`##SITE-VERSION-SHOWS-CURRENT`), so it is not a permanent link to
 * anything and has no claim to be the one a search engine keeps.
 *
 * A page exists in every language, and `hreflang` is how the set says so.
 * Every language points at every other AT THE SAME VERSION SPELLING, so
 * the annotations are reciprocal within their own cluster — an
 * annotation that crossed spellings would name a page that names a
 * different one back, which search engines discard silently.
 *
 * A translation fallback is the source's text at another language's
 * address. It carries `noindex`, and its `canonical` names the source's
 * page at the same version: one axis changes at a time, and the page a
 * crawler is sent to is the one whose words these actually are.
 *
 * None of it is built for the reader `vibe` serves from a machine's own
 * store: `##SEO-LOCAL-EXEMPT` says the local mode publishes none of this,
 * and `local.ts` writes the short head it gets instead. The flag is an
 * argument rather than a module constant so that this file stays free of
 * the build-time island — which is what lets the head be read by a test
 * at all.
 */

import type { DocumentHeadValue } from "@qwik.dev/router";

import { SITE } from "../config.ts";
import {
  catalogueHref,
  docHref,
  href,
  llmsHref,
  packageHref,
  projectionHref,
  type DocAddress,
} from "../lib/href.ts";
import {
  editions,
  libraryAt,
  resolvePage,
  siteLanguages,
  sourceEdition,
  type Library,
} from "../lib/library.ts";
import { BUILT } from "../lib/library-source.ts";
import type { DocView, PackageView, PageView } from "../lib/view.ts";
import { analytics } from "./analytics.ts";
import { docFileHref, LATEST } from "./editions.ts";
import { localHead } from "./local.ts";
import { previewOf } from "./media.ts";
import { articleData, collectionData } from "./structured-data.ts";

/**
 * An address a crawler outside this document has to resolve — `hreflang`
 * and Open Graph both require a fully qualified URL, and a scraper that
 * fetched the page over some other origin has nothing to resolve a
 * relative one against.
 *
 * `canonical` is deliberately NOT one of these. The same routes are
 * built twice: once for the domain and once for the reader `vibe` serves
 * from a machine's own store, where the origin is not this one
 * (`##SITE-CANONICAL-LATEST`). A relative canonical is correct in both
 * and resolves against the document that carries it; an absolute one
 * baked at build time would have every local page claim to be a copy of
 * a page on the public site.
 */
function absolute(path: string): string {
  return `${SITE.origin}${path}`;
}

/**
 * The card a share of this page shows.
 *
 * The package's composed preview when the build was given the tree that
 * holds it, and the site's own brand card otherwise — never nothing. Four
 * meta tags name an image on every page, and an address that answers with
 * a 404 is worse than a generic picture: it is a broken promise on every
 * share, and nobody sees it until somebody shares.
 *
 * The coordinate is always the source's, because that is the coordinate
 * every address on this site is built on, and the version is always the
 * number: `latest` is an address, not a publication, and the card belongs
 * to the publication.
 */
function cardOf(library: Library): string {
  const card = sourceEdition(library).card;
  const preview = previewOf(card.group, card.name, card.version);
  return absolute(preview ?? href("og.png"));
}

/** The same address in every language the library has, at this version. */
function alternates(
  library: Library,
  address: DocAddress,
): NonNullable<DocumentHeadValue["links"]> {
  const links = editions(library).map((edition) => ({
    rel: "alternate",
    hreflang: edition.tag,
    href: absolute(docHref({ ...address, lang: edition.segment })),
  }));
  /* `x-default` is the address a reader of any other language should be
     sent to, and for documentation that is the language it was written
     in: an adaptation is a reading of the source, and the source is the
     text the others are measured against (D-25). */
  links.push({
    rel: "alternate",
    hreflang: "x-default",
    href: absolute(docHref({ ...address, lang: null })),
  });
  return links;
}

/** The machine projections that lie beside a page, named for an agent. */
function projections(
  address: DocAddress,
): NonNullable<DocumentHeadValue["links"]> {
  return [
    {
      rel: "alternate",
      type: "text/markdown",
      href: projectionHref(address, "md"),
      title: "This page as Markdown",
    },
    {
      rel: "alternate",
      type: "application/xml",
      href: projectionHref(address, "xml"),
      title: "This page as the dialect XML",
    },
    {
      rel: "alternate",
      type: "text/plain",
      href: llmsHref(address),
      title: "llms.txt of this documentation",
    },
  ];
}

/** Open Graph and the Twitter card, which are one set of values twice. */
function social(
  url: string,
  title: string,
  description: string,
  card: string,
  locale: string,
): NonNullable<DocumentHeadValue["meta"]> {
  return [
    { property: "og:type", content: "article" },
    { property: "og:site_name", content: "VibeVM" },
    { property: "og:locale", content: locale },
    { property: "og:url", content: url },
    { property: "og:title", content: title },
    { property: "og:description", content: description },
    { property: "og:image", content: card },
    { name: "twitter:card", content: "summary_large_image" },
    { name: "twitter:title", content: title },
    { name: "twitter:description", content: description },
    { name: "twitter:image", content: card },
  ];
}

/**
 * The Open Graph spelling of a language.
 *
 * `og:locale` wants a language and a territory joined by an underscore,
 * and a BCP-47 tag carries the same information with a hyphen and
 * sometimes without the territory. The landing writes `en_US` and
 * `ru_RU` because those are what it wrote before the port; a
 * documentation edition declares only a language, so the tag is spelled
 * in Open Graph's punctuation and left at that — a territory nobody
 * declared would be an invention.
 */
function ogLocale(tag: string): string {
  return tag.replace(/-/g, "_");
}

/** The head of one documentation page. */
function pageHead(
  library: Library,
  view: PageView,
  local: boolean,
): DocumentHeadValue {
  const address = view.address;
  if (local) return localHead();
  const latest: DocAddress = { ...address, version: LATEST };
  const source = resolvePage(library, address.lang, address.document);
  const canonical = view.fallback
    ? docHref({ ...address, lang: null })
    : docHref(latest);
  const card = cardOf(library);

  return {
    title: view.title,
    meta: [
      { name: "description", content: view.summary },
      ...(view.fallback ? [{ name: "robots", content: "noindex" }] : []),
      ...social(
        absolute(canonical),
        view.title,
        view.summary,
        card,
        ogLocale(view.textLanguage),
      ),
    ],
    links: [
      { rel: "canonical", href: canonical },
      ...alternates(library, address),
      ...projections(address),
    ],
    scripts: [
      {
        type: "application/ld+json",
        dangerouslySetInnerHTML: articleData({
          absolute: absolute(canonical),
          title: view.title,
          summary: view.summary,
          language: view.textLanguage,
          publisher: view.publisher,
          renderedAt: view.renderedAt,
          audiences: view.audiences,
          ...(source === null ? {} : { genre: source.page.genre }),
          image: card,
          packageTitle: sourceEdition(library).card.title,
          packageAbsolute: absolute(packageHref(latest)),
          doorAbsolute: absolute(catalogueHref(null)),
        }),
      },
      ...analytics(),
    ],
  };
}

/** The head of a package's own page — the shelf and the card. */
function packageHead(
  library: Library,
  view: PackageView,
  local: boolean,
): DocumentHeadValue {
  const address = view.address;
  const description = view.description ?? view.abstract;
  const agentIndex = {
    rel: "alternate",
    type: "text/plain",
    href: llmsHref(address),
    title: "llms.txt of this documentation",
  };
  if (local) return localHead();

  const canonical = packageHref({ ...address, version: LATEST });
  const card = cardOf(library);

  return {
    title: view.title,
    meta: [
      { name: "description", content: description },
      ...social(
        absolute(canonical),
        view.title,
        description,
        card,
        ogLocale(view.textLanguage),
      ),
    ],
    links: [
      { rel: "canonical", href: canonical },
      /* At the version this address is spelled with, exactly as a page's
         are: an annotation that crossed spellings would name a page that
         names a different one back. */
      ...editions(library).map((edition) => ({
        rel: "alternate",
        hreflang: edition.tag,
        href: absolute(packageHref({ ...address, lang: edition.segment })),
      })),
      {
        rel: "alternate",
        hreflang: "x-default",
        href: absolute(packageHref({ ...address, lang: null })),
      },
      agentIndex,
    ],
    scripts: [
      {
        type: "application/ld+json",
        dangerouslySetInnerHTML: collectionData({
          absolute: absolute(canonical),
          title: view.title,
          abstract: view.abstract,
          language: view.textLanguage,
          publisher: view.publisher,
          image: card,
          doorAbsolute: absolute(catalogueHref(null)),
        }),
      },
      ...analytics(),
    ],
  };
}

/**
 * The head of a language's catalogue, and of the door.
 *
 * The door is the source language's catalogue under another address, so
 * the two share everything except which of them is canonical: the door
 * is, and a language's catalogue points at itself — it is a different
 * list, not the same one at another address.
 *
 * Its `hreflang` set is the site's languages and not one library's: a
 * catalogue lists every documentation, so the address that answers for
 * it in another language is that language's catalogue, whichever
 * libraries happen to stand on it.
 */
export function catalogueHead(
  lang: string | null,
  local: boolean,
): DocumentHeadValue {
  const description =
    "Every documentation this site carries, in every language it has been adapted into.";
  const agentIndex = {
    rel: "alternate",
    type: "text/plain",
    href: docFileHref("llms.txt"),
    title: "The catalogue for an agent",
  };
  if (local) return localHead();

  const canonical = catalogueHref(lang);
  return {
    title: "Documentation",
    meta: [
      { name: "description", content: description },
      ...social(
        absolute(canonical),
        "Documentation — VibeVM",
        description,
        absolute(href("og.png")),
        ogLocale(lang ?? siteLanguages(BUILT)[0]?.tag ?? "en"),
      ),
    ],
    links: [
      { rel: "canonical", href: canonical },
      ...siteLanguages(BUILT).map((language) => ({
        rel: "alternate",
        hreflang: language.tag,
        href: absolute(catalogueHref(language.segment)),
      })),
      {
        rel: "alternate",
        hreflang: "x-default",
        href: absolute(catalogueHref(null)),
      },
      agentIndex,
    ],
    scripts: [...analytics()],
  };
}

/**
 * The head of whichever address the catch-all route matched.
 *
 * `local` is passed in and never read from a module of its own: the flag
 * is derived from the island this build baked in (`mode.ts`), and this
 * file must stay importable by a test that has no build behind it.
 */
export function documentationHead(
  view: DocView,
  local: boolean,
): DocumentHeadValue {
  if (view.kind === "catalogue") return catalogueHead(view.lang, local);
  /* The address names its library, exactly as it does for the page
     itself: every address is built on a source documentation's
     coordinate, and a view the route rendered has one behind it. */
  const library = libraryAt(BUILT, view.address.group, view.address.name);
  if (library === null) return { title: view.title };
  return view.kind === "page"
    ? pageHead(library, view, local)
    : packageHead(library, view, local);
}
