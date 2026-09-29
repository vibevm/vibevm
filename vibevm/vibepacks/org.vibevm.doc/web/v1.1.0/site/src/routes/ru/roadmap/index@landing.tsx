/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../landing/head.ts";
import { Roadmap } from "../../../roadmap/index.tsx";
import { ROADMAP_META, roadmapGraph } from "../../../roadmap/meta.ts";
import { roadmapPath } from "../../../roadmap/paths.ts";

/**
 * `https://vibevm.org/ru/roadmap/` — русская редакция дорожной карты,
 * адаптация английского текста.
 */
export default component$(() => <Roadmap locale="ru" />);

export const head: DocumentHead = pageHead({
  locale: "ru",
  path: roadmapPath(),
  ...ROADMAP_META.ru,
  graph: roadmapGraph("ru"),
});
