/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The roadmap's address, as an address rather than as a string.
 *
 * `/roadmap/` and `/ru/roadmap/` are the marketing half's sixth pair of
 * pages, beside the three Why pages, the essay and the channels page,
 * and they are named here for the reason the other five have a
 * `paths.ts`: `lib/href.ts` owns the manual's address map, and a
 * marketing route inside it would be one more meaning for `docSegments`.
 *
 * The slug is the identity, and it is the one word every language spells
 * the same way in an address bar. The page's title — «From a preview to
 * an operating system» / «От превью до операционной системы» — belongs to
 * the page; the address has to be typed, remembered and shared, and
 * `roadmap` is what a reader looking for this page will type.
 */

import { type Locale, localePath } from "../landing/i18n.ts";
import { href } from "../lib/href.ts";

/**
 * The path of the roadmap inside its language, root-relative, without the
 * language in front and with the trailing slash every page address on
 * this site carries (`##SITE-TRAILING-SLASH`).
 */
export function roadmapPath(): string {
  return "roadmap/";
}

/** The address a link to the roadmap carries, in one language. */
export function roadmapHref(locale: Locale): string {
  return href(`${localePath(locale)}${roadmapPath()}`);
}

/**
 * Whether an address is the roadmap, read off the served path the way
 * `isVisionPath` and `whyPageOf` are: the chrome is one component
 * standing over every landing address, and the address is the only thing
 * they all agree on. The language is stripped first by
 * `pathWithinLocale`, so `/ru/roadmap/` and `/roadmap/` answer the same
 * page — which is what lets the entry mark itself current in either
 * language and the language switch keep a reader on it.
 */
export function isRoadmapPath(pathWithinLocale: string): boolean {
  return pathWithinLocale === roadmapPath();
}

/**
 * The future home of Spec-Driven Linux, the fifth stage (owner,
 * 2026-09-26). The domain is the owner's and the page links to it as it
 * links to every other place off this domain — with `rel="noopener"`
 * — and it is one constant so that the stage's text, the structured data
 * and the link linter's allow-list name one address rather than three
 * spellings of it. Nothing on this site fetches from it.
 */
export const SPEC_DRIVEN_LINUX_URL = "https://specdrivenlinux.org/";

/** The address as the page prints it: the host, without the scheme. */
export const SPEC_DRIVEN_LINUX_HOST = "specdrivenlinux.org";
