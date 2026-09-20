/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The essay's copy, in both editions.
 *
 * The assembly and nothing else, after the AI-Native page's pattern: the
 * route asks for `STRINGS[locale]` and does not know how many files the
 * table is kept in. The Russian edition is the author's original after
 * proofreading; the English edition is its adaptation.
 */

import { COPY as EN } from "./copy-en.ts";
import { COPY as RU } from "./copy-ru.ts";
import type { VisionStrings } from "./strings.ts";

export type { VisionStrings } from "./strings.ts";

export const STRINGS: Readonly<Record<"en" | "ru", VisionStrings>> = {
  en: EN,
  ru: RU,
};
