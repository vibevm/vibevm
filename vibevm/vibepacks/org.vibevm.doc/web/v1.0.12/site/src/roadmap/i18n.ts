/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The roadmap's copy, in both editions.
 *
 * The assembly and nothing else, after the essay's pattern: the route
 * asks for `STRINGS[locale]` and does not know how many files the table
 * is kept in. The English edition is the source; the Russian edition is
 * its adaptation.
 */

import { COPY as EN } from "./copy-en.ts";
import { COPY as RU } from "./copy-ru.ts";
import type { RoadmapStrings } from "./strings.ts";

export type { RoadmapStrings, Stage, StageId } from "./strings.ts";
export { STAGE_IDS } from "./strings.ts";

export const STRINGS: Readonly<Record<"en" | "ru", RoadmapStrings>> = {
  en: EN,
  ru: RU,
};
