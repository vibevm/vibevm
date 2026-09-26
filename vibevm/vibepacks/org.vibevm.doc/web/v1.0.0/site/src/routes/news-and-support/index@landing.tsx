/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../landing/head.ts";
import { NewsAndSupport } from "../../news/index.tsx";
import { NEWS_META } from "../../news/meta.ts";
import { newsPath } from "../../news/paths.ts";

/**
 * `https://vibevm.org/news-and-support/` — the English edition of the
 * channels page.
 *
 * The head is the shared builder's with nothing substituted: the page is
 * a `WebPage` of this site, which is what it is.
 */
export default component$(() => <NewsAndSupport locale="en" />);

export const head: DocumentHead = pageHead({
  locale: "en",
  path: newsPath(),
  ...NEWS_META.en,
});
