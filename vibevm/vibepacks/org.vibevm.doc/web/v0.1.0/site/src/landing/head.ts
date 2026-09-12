/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

import type { DocumentHeadValue } from "@qwik.dev/router";

import { SITE } from "../config.ts";
import { href } from "../lib/href.ts";
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

/** The absolute address of a locale's landing — `canonical` and `og:url`. */
function absolute(locale: Locale): string {
  return `${SITE.origin}${href(localePath(locale))}`;
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
function structuredData(): string {
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

/** The head of a landing page in one language. */
export function landingHead(locale: Locale): DocumentHeadValue {
  const t = STRINGS[locale];
  const canonical = absolute(locale);
  const image = `${SITE.origin}${href("og.png")}`;

  return {
    title: t.metaTitle,
    meta: [
      { name: "description", content: t.metaDescription },
      { name: "theme-color", content: THEME_COLOR },
      { property: "og:type", content: "website" },
      { property: "og:site_name", content: "VibeVM" },
      { property: "og:locale", content: t.ogLocale },
      { property: "og:url", content: canonical },
      { property: "og:title", content: t.metaTitle },
      { property: "og:description", content: t.metaDescription },
      { property: "og:image", content: image },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: t.metaTitle },
      { name: "twitter:description", content: t.metaDescription },
      { name: "twitter:image", content: image },
    ],
    links: [
      { rel: "canonical", href: canonical },
      { rel: "alternate", hreflang: "en", href: absolute("en") },
      { rel: "alternate", hreflang: "ru", href: absolute("ru") },
      { rel: "alternate", hreflang: "x-default", href: absolute("en") },
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
        dangerouslySetInnerHTML: structuredData(),
      },
      ...analytics(),
    ],
  };
}

/**
 * The analytics tag, or nothing at all.
 *
 * First-party, self-hosted, cookie-less and served by the domain itself
 * (D-24): the site emits one `<script>` and never touches `/u/s.js` or
 * `/u/e`, which the domain answers. It is absent from the embedded build
 * without a condition, because the landing routes are not in that
 * build's route tree at all.
 */
function analytics(): NonNullable<DocumentHeadValue["scripts"]> {
  if (SITE.umamiWebsiteId.length === 0) return [];
  /* Qwik types a head script by the HTML attributes it knows about, and
     `data-*` is not among them: inside JSX TypeScript waives its check
     for any hyphenated attribute, but a plain object gets no waiver.
     Composing the tag out of its two halves states what it carries
     without asserting a type over it, which an `as` would. */
  const tag = Object.assign(
    { defer: true, src: href("u/s.js") },
    {
      "data-website-id": SITE.umamiWebsiteId,
      "data-host-url": SITE.origin,
    },
  );
  return [tag];
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
