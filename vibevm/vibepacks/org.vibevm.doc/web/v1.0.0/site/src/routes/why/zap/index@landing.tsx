/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../landing/head.ts";
import { WHY_META } from "../../../why/meta.ts";
import { whyPath } from "../../../why/paths.ts";
import { WhyZap } from "../../../why/zap/index.tsx";

/** `https://vibevm.org/why/zap/` — the English page for Zap. */
export default component$(() => <WhyZap locale="en" />);

export const head: DocumentHead = pageHead({
  locale: "en",
  path: whyPath("zap"),
  ...WHY_META.zap.en,
});
