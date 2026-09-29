/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../../landing/head.ts";
import { WHY_META } from "../../../../why/meta.ts";
import { whyPath } from "../../../../why/paths.ts";
import { WhyVibevm } from "../../../../why/vibevm/index.tsx";

/**
 * `https://vibevm.org/ru/why/vibevm/` — the Russian page for VibeVM.
 *
 * The whole page differs from its English twin by one argument, which is
 * the point of keeping the copy in a string table rather than in the
 * markup: a translator adds a language by adding a column, and nothing
 * about the frieze, the architecture diagram or the inverted band has an
 * opinion about which one is being rendered.
 */
export default component$(() => <WhyVibevm locale="ru" />);

export const head: DocumentHead = pageHead({
  locale: "ru",
  path: whyPath("vibevm"),
  ...WHY_META.vibevm.ru,
});
