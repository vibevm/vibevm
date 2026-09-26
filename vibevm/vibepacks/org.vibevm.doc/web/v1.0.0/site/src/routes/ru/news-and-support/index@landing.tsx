/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../landing/head.ts";
import { NewsAndSupport } from "../../../news/index.tsx";
import { NEWS_META } from "../../../news/meta.ts";
import { newsPath } from "../../../news/paths.ts";

/**
 * `https://vibevm.org/ru/news-and-support/` — русская редакция страницы
 * каналов: те же пять адресов, подписанные по-русски.
 */
export default component$(() => <NewsAndSupport locale="ru" />);

export const head: DocumentHead = pageHead({
  locale: "ru",
  path: newsPath(),
  ...NEWS_META.ru,
});
