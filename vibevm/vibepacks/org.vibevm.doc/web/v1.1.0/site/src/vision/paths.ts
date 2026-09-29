/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The essay's address, as an address rather than as a string.
 *
 * `/vision/` and `/ru/vision/` are the marketing half's fourth pair of
 * long-form pages beside the three Why pages, and they are named here
 * for the reason `why/paths.ts` exists: `lib/href.ts` owns the manual's
 * address map, and a marketing route inside it would be one more meaning
 * for `docSegments`.
 *
 * The slug is the identity, and it is one word. The essay's title —
 * «Большой Вижен» / "The Big Vision" — belongs to the page, not to the
 * URL: a title may be playful, an address has to be typed, remembered
 * and shared, and `vision` is the same seven letters in both languages'
 * addresses. One slug, no aliases, no redirects.
 */

import { type Locale, localePath } from "../landing/i18n.ts";
import { href } from "../lib/href.ts";

/**
 * The path of the essay inside its language, root-relative, without the
 * language in front and with the trailing slash every page address on
 * this site carries (`##SITE-TRAILING-SLASH`).
 */
export function visionPath(): string {
  return "vision/";
}

/** The address a link to the essay carries, in one language. */
export function visionHref(locale: Locale): string {
  return href(`${localePath(locale)}${visionPath()}`);
}

/**
 * Whether an address is the essay, read off the served path the way
 * `whyPageOf` reads the Why pages: the chrome is one component standing
 * over every landing address, and the address is the only thing they
 * all agree on. The language is stripped first by `pathWithinLocale`,
 * so `/ru/vision/` and `/vision/` answer the same page.
 */
export function isVisionPath(pathWithinLocale: string): boolean {
  return pathWithinLocale === visionPath();
}
