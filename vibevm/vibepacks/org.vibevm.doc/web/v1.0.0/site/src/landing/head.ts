/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

import type { DocumentHeadValue } from "@qwik.dev/router";

import { SITE } from "../config.ts";
import { href } from "../lib/href.ts";
import { analytics } from "../seo/analytics.ts";
import {
  GITHUB_URL,
  GITVERSE_URL,
  type Locale,
  STRINGS,
  localePath,
} from "./i18n.ts";
import { THEME_COLOR } from "./theme-color.ts";

/**
 * Everything the landing pages put in `<head>`, moved across tag for tag.
 *
 * This is the half of the port a reader never sees and a crawler sees
 * first, and it is the half the parity test exists for: an address that
 * keeps its words but loses its `canonical`, its `hreflang` pair or its
 * structured data has not been moved, it has been replaced by something
 * that looks like it. Every tag below is the one `BaseLayout.astro`
 * emitted, with the same value and the same conditions.
 *
 * Two of them are computed rather than written. The origin comes from
 * the build environment (`config.ts`), because the same source builds
 * for the live domain and for a preview of it. The analytics tag appears
 * only when that environment named a website id: an id-less Umami tag
 * loads a script that reports to nobody, which is worse than no tag at
 * all (`##SITE-ANALYTICS`).
 */

/**
 * The absolute address of a page in one language — `canonical`, `og:url`
 * and one half of the `hreflang` pair.
 *
 * The path is the page's address inside its language, so `""` is the
 * landing and `"why/zap/"` is a Why page. One function for both because
 * they are the same address with a different tail, and a second one
 * would be a second opinion about where the language sits.
 */
function absolute(locale: Locale, path = ""): string {
  return `${SITE.origin}${href(`${localePath(locale)}${path}`)}`;
}

/**
 * The structured data, identical on both pages — including the English
 * `description`, which the Astro site did not localise.
 *
 * That is a fact about the source and not an oversight of the port: the
 * same landing gives Yandex a Russian `meta description` and a
 * `SoftwareApplication` described in English. It is copied as it stands
 * so the parity test can say so, and so the decision to change it is
 * taken deliberately rather than absorbed into a move.
 */
function siteGraph(): string {
  return JSON.stringify({
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "SoftwareApplication",
        name: "VibeVM",
        applicationCategory: "DeveloperApplication",
        operatingSystem: "Windows, macOS, Linux",
        url: SITE.origin,
        description:
          "An ultimate prompt library, package manager, and agentic system for Spec-Driven Development.",
        offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
        sameAs: [GITHUB_URL, GITVERSE_URL],
      },
      {
        "@type": "WebSite",
        name: "VibeVM",
        url: SITE.origin,
        inLanguage: ["en", "ru"],
      },
    ],
  });
}

/**
 * The structured data of a page that is not the front door.
 *
 * The site graph belongs to the root and is published there once: a
 * `SoftwareApplication` repeated on every page of the domain would be
 * the same claim asserted six more times, and a crawler that read it
 * that way would have six answers to «what is this product» instead of
 * one. A subpage says the smaller true thing — this is a page, it is
 * called this, it is in this language, and it is part of that site.
 *
 * That split is the Astro layout's own (`BaseLayout.astro:24`), kept
 * rather than improved: the address map and what each address claims
 * about itself are the half of the move a reader never sees and a
 * crawler sees first.
 */
function pageGraph(
  canonical: string,
  title: string,
  description: string,
  language: string,
): string {
  return JSON.stringify({
    "@context": "https://schema.org",
    "@type": "WebPage",
    name: title,
    url: canonical,
    description,
    inLanguage: language,
    isPartOf: { "@type": "WebSite", name: "VibeVM", url: SITE.origin },
  });
}

/**
 * The faces the page will certainly set, preloaded before the stylesheet
 * asks for them.
 *
 * Two for English, four for Russian — the Cyrillic subsets are a
 * separate download and an English page must never pay for them
 * (`fonts.css` splits every family by `unicode-range` for exactly this
 * reason). The list is the display weight and the body face: what the
 * first screen is made of.
 *
 * The address is the font's public one, `/fonts/<face>.woff2`, kept from
 * the old site. The build rewrites it to the content-hashed asset the
 * stylesheet actually fetches, so that the preload and the `@font-face`
 * name one file rather than two copies of it; the public path keeps
 * serving the same bytes beside it.
 */
function fontPreloads(locale: Locale): readonly string[] {
  const shared = ["Spectral-latin-600.woff2", "Inter-latin.woff2"];
  if (locale !== "ru") return shared;
  return [...shared, "Spectral-cyrillic-600.woff2", "Inter-cyrillic.woff2"];
}

/** What a page of the marketing half declares about itself. */
export type PageHeadProps = {
  /** Which language's spelling of the page this is. */
  readonly locale: Locale;
  /**
   * The page's address inside its language, slash-ended: `""` for the
   * landing, `"why/zap/"` for a Why page. It is what makes `canonical`
   * and the `hreflang` pair name this page rather than the front door.
   */
  readonly path: string;
  /** The `<title>`, already in the page's language. */
  readonly title: string;
  readonly description: string;
  /**
   * The page's structured data, serialised, when `WebPage` is not what
   * the page IS. The essay passes an `Article`; everything else omits
   * this and gets the default split — the site graph on the front door,
   * a `WebPage` everywhere else. One optional field rather than a
   * second builder, so a page cannot end up with two graphs.
   */
  readonly graph?: string;
};

/**
 * The head of one page of the marketing half of the domain, in one
 * language.
 *
 * Every page declares the same set of tags and differs in four values,
 * which is why there is one builder and not one per page: a `canonical`
 * that a route forgot, or an `hreflang` pair naming the landing from a
 * Why page, is a defect no reader can see and no test would catch if
 * each route composed its own head by hand.
 */
export function pageHead(props: PageHeadProps): DocumentHeadValue {
  const { locale, path, title, description } = props;
  const t = STRINGS[locale];
  const canonical = absolute(locale, path);
  const image = `${SITE.origin}${href("og.png")}`;

  return {
    title,
    meta: [
      { name: "description", content: description },
      { name: "theme-color", content: THEME_COLOR },
      { property: "og:type", content: "website" },
      { property: "og:site_name", content: "VibeVM" },
      { property: "og:locale", content: t.ogLocale },
      { property: "og:url", content: canonical },
      { property: "og:title", content: title },
      { property: "og:description", content: description },
      { property: "og:image", content: image },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: title },
      { name: "twitter:description", content: description },
      { name: "twitter:image", content: image },
    ],
    links: [
      { rel: "canonical", href: canonical },
      { rel: "alternate", hreflang: "en", href: absolute("en", path) },
      { rel: "alternate", hreflang: "ru", href: absolute("ru", path) },
      { rel: "alternate", hreflang: "x-default", href: absolute("en", path) },
      { rel: "icon", href: href("favicon.svg"), type: "image/svg+xml" },
      {
        rel: "alternate",
        type: "text/plain",
        href: href("llms.txt"),
        title: "llms.txt",
      },
      ...fontPreloads(locale).map((face) => ({
        rel: "preload",
        href: href(`fonts/${face}`),
        as: "font",
        type: "font/woff2",
        crossorigin: "",
      })),
    ],
    scripts: [
      /* `dangerouslySetInnerHTML` and not `script`: both put the JSON
         inside the element, but the latter also emits it a second time
         as an attribute of the same tag — a kilobyte of duplicated
         structured data on every page, and a non-standard attribute for
         a crawler to wonder about. */
      {
        type: "application/ld+json",
        dangerouslySetInnerHTML:
          props.graph ??
          (path.length === 0
            ? siteGraph()
            : pageGraph(canonical, title, description, t.htmlLang)),
      },
      ...analytics(),
    ],
  };
}

/** The head of a landing page in one language. */
export function landingHead(locale: Locale): DocumentHeadValue {
  const t = STRINGS[locale];
  return pageHead({
    locale,
    path: "",
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

/**
 * The head of the 404 page: the English landing's, as it was.
 *
 * Including its `canonical`, which points at the site root rather than
 * at `/404.html`. That is what the Astro page emitted, and it is not
 * wrong: the page has no address of its own worth indexing, and naming
 * the root is how it says so.
 */
export function notFoundHead(): DocumentHeadValue {
  return landingHead("en");
}

/**
 * The head of `/en/`: a page that exists only to send a reader to `/`.
 *
 * The old deployment answered `/en/` with a 301 in nginx. A static build
 * cannot return a status code, so the address becomes a real page that
 * redirects itself — and says clearly that it is not content: `noindex`
 * for the crawler, `canonical` on the root for the one that ignores it.
 */
export function redirectHead(): DocumentHeadValue {
  const target = absolute("en");
  return {
    title: STRINGS.en.metaTitle,
    meta: [
      { name: "robots", content: "noindex, follow" },
      { httpEquiv: "refresh", content: `0; url=${href("")}` },
    ],
    links: [{ rel: "canonical", href: target }],
  };
}
