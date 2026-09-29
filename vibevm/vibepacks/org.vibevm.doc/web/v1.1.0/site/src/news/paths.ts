/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The address of the channels page, as an address rather than as a
 * string.
 *
 * `/news-and-support/` and `/ru/news-and-support/` are the marketing
 * half's fifth pair, beside the three Why pages and the essay, and they
 * are named here for the reason `why/paths.ts` and `vision/paths.ts`
 * exist: `lib/href.ts` owns the manual's address map, and a marketing
 * route inside it would be one more meaning for `docSegments`.
 *
 * The slug spells out what the page is rather than what it is called. A
 * reader who wants the news channel and a reader who wants to report a
 * bug arrive at the same page, and an address that named only one of
 * them would send the other away — so both words are in it, in the order
 * the page puts them. It is the same slug in both languages' addresses,
 * as every address on this site is: a title may be translated, an
 * address has to be typed, remembered and shared.
 */

import { type Locale, localePath } from "../landing/i18n.ts";
import { href } from "../lib/href.ts";

/**
 * The path of the page inside its language, root-relative, without the
 * language in front and with the trailing slash every page address on
 * this site carries (`##SITE-TRAILING-SLASH`).
 */
export function newsPath(): string {
  return "news-and-support/";
}

/** The address a link to the page carries, in one language. */
export function newsHref(locale: Locale): string {
  return href(`${localePath(locale)}${newsPath()}`);
}

/**
 * Whether an address is this page, read off the served path the way
 * `isVisionPath` and `whyPageOf` are: the chrome is one component
 * standing over every landing address, and the address is the only thing
 * they all agree on. The language is stripped first by
 * `pathWithinLocale`, so `/ru/news-and-support/` and
 * `/news-and-support/` answer the same page — which is what lets the
 * entry mark itself current in either language.
 */
export function isNewsPath(pathWithinLocale: string): boolean {
  return pathWithinLocale === newsPath();
}
