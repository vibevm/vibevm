/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../../landing/head.ts";
import { WhyAiNative } from "../../../../why/ai-native/index.tsx";
import { WHY_META } from "../../../../why/meta.ts";
import { whyPath } from "../../../../why/paths.ts";

/**
 * `https://vibevm.org/ru/why/ai-native/` — the Russian page for the
 * AI-Native Code Discipline.
 */
export default component$(() => <WhyAiNative locale="ru" />);

export const head: DocumentHead = pageHead({
  locale: "ru",
  path: whyPath("ai-native"),
  ...WHY_META["ai-native"].ru,
});
