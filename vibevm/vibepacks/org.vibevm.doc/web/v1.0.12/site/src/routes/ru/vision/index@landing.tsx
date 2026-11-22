/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../landing/head.ts";
import { BigVision } from "../../../vision/index.tsx";
import { VISION_META, visionArticleGraph } from "../../../vision/meta.ts";
import { visionPath } from "../../../vision/paths.ts";

/**
 * `https://vibevm.org/ru/vision/` — русская редакция эссе, авторский
 * оригинал после корректорской правки.
 */
export default component$(() => <BigVision locale="ru" />);

export const head: DocumentHead = pageHead({
  locale: "ru",
  path: visionPath(),
  ...VISION_META.ru,
  graph: visionArticleGraph("ru"),
});
