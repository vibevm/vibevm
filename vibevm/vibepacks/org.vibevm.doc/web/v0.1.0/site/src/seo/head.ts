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
import { editions, resolvePage, sourceEdition } from "../lib/library.ts";
import type { DocView, PackageView, PageView } from "../lib/view.ts";
import { analytics } from "./analytics.ts";
import { docFileHref, LATEST } from "./editions.ts";
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
function cardOf(): string {
  const card = sourceEdition().card;
  const preview = previewOf(card.group, card.name, card.version);
  return absolute(preview ?? href("og.png"));
}

/** The same address in every language the library has, at this version. */
function alternates(
  address: DocAddress,
): NonNullable<DocumentHeadValue["links"]> {
  const links = editions().map((edition) => ({
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
function pageHead(view: PageView): DocumentHeadValue {
  const address = view.address;
  const latest: DocAddress = { ...address, version: LATEST };
  const source = resolvePage(address.lang, address.document);
  const canonical = view.fallback
    ? docHref({ ...address, lang: null })
    : docHref(latest);
  const card = cardOf();

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
      ...alternates(address),
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
          packageTitle: sourceEdition().card.title,
          packageAbsolute: absolute(packageHref(latest)),
          doorAbsolute: absolute(catalogueHref(null)),
        }),
      },
      ...analytics(),
    ],
  };
}

/** The head of a package's own page — the shelf and the card. */
function packageHead(view: PackageView): DocumentHeadValue {
  const address = view.address;
  const canonical = packageHref({ ...address, version: LATEST });
  const description = view.description ?? view.abstract;
  const card = cardOf();

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
      ...editions().map((edition) => ({
        rel: "alternate",
        hreflang: edition.tag,
        href: absolute(packageHref({ ...address, lang: edition.segment })),
      })),
      {
        rel: "alternate",
        hreflang: "x-default",
        href: absolute(packageHref({ ...address, lang: null })),
      },
      {
        rel: "alternate",
        type: "text/plain",
        href: llmsHref(address),
        title: "llms.txt of this documentation",
      },
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
 */
export function catalogueHead(lang: string | null): DocumentHeadValue {
  const canonical = catalogueHref(lang);
  const description =
    "Every documentation this site carries, in every language it has been adapted into.";
  return {
    title: "Documentation",
    meta: [
      { name: "description", content: description },
      ...social(
        absolute(canonical),
        "Documentation — VibeVM",
        description,
        absolute(href("og.png")),
        ogLocale(lang ?? sourceEdition().tag),
      ),
    ],
    links: [
      { rel: "canonical", href: canonical },
      ...editions().map((edition) => ({
        rel: "alternate",
        hreflang: edition.tag,
        href: absolute(catalogueHref(edition.segment)),
      })),
      {
        rel: "alternate",
        hreflang: "x-default",
        href: absolute(catalogueHref(null)),
      },
      {
        rel: "alternate",
        type: "text/plain",
        href: docFileHref("llms.txt"),
        title: "The catalogue for an agent",
      },
    ],
    scripts: [...analytics()],
  };
}

/** The head of whichever address the catch-all route matched. */
export function documentationHead(view: DocView): DocumentHeadValue {
  if (view.kind === "catalogue") return catalogueHead(view.lang);
  return view.kind === "page" ? pageHead(view) : packageHead(view);
}
