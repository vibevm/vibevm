/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The three Why pages, as addresses rather than as strings.
 *
 * They are the marketing half of the domain beside the landing: one page
 * per product of the family, each written out in both languages at its
 * own address, all six wearing the landing's chrome. Nothing about them
 * is the documentation's, which is why they are named here and not in
 * `lib/href.ts` — that file owns the address map of the manual, and a
 * marketing route in it would be a fourth meaning for `docSegments`.
 *
 * The slug is the identity. `ai-native` and not `ai-native-language`:
 * the prototype that authored these pages wrote the longer form while
 * the family was still being named, the owner settled on the short one,
 * and the port carries exactly one of them. There is no second address
 * and no redirect from the longer form, because the longer form was
 * never served from this domain — it existed only in a review worktree.
 */

import { href } from "../lib/href.ts";
import { type Locale, localePath } from "../landing/i18n.ts";

/** Which of the three pages an address is. */
export const WHY_PAGES = ["vibevm", "zap", "ai-native"] as const;

export type WhyPage = (typeof WHY_PAGES)[number];

/**
 * The path of a Why page inside its language, root-relative, without the
 * language in front and with the trailing slash every page address on
 * this site carries (`##SITE-TRAILING-SLASH`).
 */
export function whyPath(page: WhyPage): string {
  return `why/${page}/`;
}

/** The address a link to a Why page carries, in one language. */
export function whyHref(page: WhyPage, locale: Locale): string {
  return href(`${localePath(locale)}${whyPath(page)}`);
}

/**
 * Which Why page an address is, or `null` when it is not one.
 *
 * Read off the served path rather than passed down as a prop, because
 * the chrome is one component standing over eight addresses and the
 * address is the only thing all eight agree on. The language is stripped
 * first by `pathWithinLocale`, so `/ru/why/zap/` and `/why/zap/` answer
 * the same page — which is the whole reason the language switch can keep
 * a reader where they are.
 */
export function whyPageOf(pathWithinLocale: string): WhyPage | null {
  const parts = pathWithinLocale.split("/").filter((part) => part.length > 0);
  if (parts[0] !== "why" || parts.length !== 2) return null;
  return WHY_PAGES.find((page) => page === parts[1]) ?? null;
}
