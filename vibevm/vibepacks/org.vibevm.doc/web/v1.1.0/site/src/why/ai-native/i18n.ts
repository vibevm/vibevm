/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The copy of `/why/ai-native`, in both languages.
 *
 * The two editions live in files of their own — `copy-en.ts` and
 * `copy-ru.ts` — and their shape in `strings.ts`. The three were one
 * file until it crossed the discipline's six-hundred-line budget, and
 * the seam the budget pointed at was the one the content already had:
 * a translator works on one edition, and a reviewer diffs one edition.
 *
 * This file is the assembly, and it is the only thing the page imports:
 * a route asks for `STRINGS[locale]` and knows nothing about how many
 * files the table is kept in.
 */

import { COPY as EN } from "./copy-en.ts";
import { COPY as RU } from "./copy-ru.ts";
import type { Strings } from "./strings.ts";

export type {
  FamilyMember,
  Stack,
  StatusColumn,
  Step,
  Strings,
} from "./strings.ts";

export const STRINGS: Readonly<Record<"en" | "ru", Strings>> = {
  en: EN,
  ru: RU,
};
