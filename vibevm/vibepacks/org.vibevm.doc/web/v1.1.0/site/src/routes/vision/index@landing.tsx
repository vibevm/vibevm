/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../landing/head.ts";
import { BigVision } from "../../vision/index.tsx";
import { VISION_META, visionArticleGraph } from "../../vision/meta.ts";
import { visionPath } from "../../vision/paths.ts";

/**
 * `https://vibevm.org/vision/` — the English edition of the essay.
 *
 * The head is the shared builder's with one substitution: the
 * structured data is an `Article`, because that is what this page is —
 * an authored essay, not a product surface.
 */
export default component$(() => <BigVision locale="en" />);

export const head: DocumentHead = pageHead({
  locale: "en",
  path: visionPath(),
  ...VISION_META.en,
  graph: visionArticleGraph("en"),
});
