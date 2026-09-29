/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../landing/head.ts";
import { WHY_META } from "../../../why/meta.ts";
import { whyPath } from "../../../why/paths.ts";
import { WhyVibevm } from "../../../why/vibevm/index.tsx";

/**
 * `https://vibevm.org/why/vibevm/` — the English page for VibeVM itself.
 *
 * The route is an address and nothing else: it names the language, the
 * page, and the head that pair declares. Everything a reader sees is in
 * `why/vibevm/`, and everything a crawler reads is composed by
 * `pageHead` from the two — which is what keeps six routes from drifting
 * into six slightly different opinions about `canonical`.
 */
export default component$(() => <WhyVibevm locale="en" />);

export const head: DocumentHead = pageHead({
  locale: "en",
  path: whyPath("vibevm"),
  ...WHY_META.vibevm.en,
});
