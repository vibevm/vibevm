/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../landing/head.ts";
import { Roadmap } from "../../roadmap/index.tsx";
import { ROADMAP_META, roadmapGraph } from "../../roadmap/meta.ts";
import { roadmapPath } from "../../roadmap/paths.ts";

/**
 * `https://vibevm.org/roadmap/` — the English edition of the roadmap.
 *
 * The head is the shared builder's with one substitution: the structured
 * data is a `WebPage` whose main entity is the ordered list of stages,
 * because the order is what this page is.
 */
export default component$(() => <Roadmap locale="en" />);

export const head: DocumentHead = pageHead({
  locale: "en",
  path: roadmapPath(),
  ...ROADMAP_META.en,
  graph: roadmapGraph("en"),
});
