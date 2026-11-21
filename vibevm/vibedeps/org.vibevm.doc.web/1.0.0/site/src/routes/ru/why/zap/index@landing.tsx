/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../../landing/head.ts";
import { WHY_META } from "../../../../why/meta.ts";
import { whyPath } from "../../../../why/paths.ts";
import { WhyZap } from "../../../../why/zap/index.tsx";

/** `https://vibevm.org/ru/why/zap/` — the Russian page for Zap. */
export default component$(() => <WhyZap locale="ru" />);

export const head: DocumentHead = pageHead({
  locale: "ru",
  path: whyPath("zap"),
  ...WHY_META.zap.ru,
});
