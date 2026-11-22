/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$ } from "@qwik.dev/core";
import { useLocation } from "@qwik.dev/router";

import { LandingChrome } from "../landing/chrome.tsx";
import { localeFromPath, pathWithinLocale } from "../landing/i18n.ts";

/**
 * The chrome the landing addresses wear — a NAMED layout, and the name
 * is the mechanism.
 *
 * Qwik Router gives a route the chain of `layout.tsx` files above it
 * unless the route asks for a named one: a file called `index@landing`
 * takes `layout-landing` instead and stops there. That is how `/`,
 * `/ru/`, `/en/` and `/404.html` sit in the same route tree as the
 * documentation's addresses and wear a different frame, without a flag
 * inside either frame asking which page it is on.
 *
 * It matters that this is a separate layout and not a condition in the
 * shared one. The documentation's chrome carries a search box over a
 * corpus; the landing's carries the way in to that corpus, the two
 * source mirrors and the language switch. Two fillings of the same two
 * components (D-28) — and each half of the site can change its own
 * without reading the other's conditions.
 *
 * The language comes from the address rather than from a prop, because
 * the layout is one file serving ten routes. `/ru/…` is Russian; every
 * other landing address is English, which is the address map the Astro
 * site chose — the root serves content instead of redirecting to a
 * prefixed copy of itself.
 *
 * So does the page, and for the same reason: the header marks which of
 * the three Why entries a reader is standing on, and the two letters in
 * the corner offer the other language's spelling of that same page. Both
 * are facts about the address, and the address is what a layout has.
 */
export default component$(() => {
  const location = useLocation();
  const pathname = location.url.pathname;
  return (
    <LandingChrome
      locale={localeFromPath(pathname)}
      path={pathWithinLocale(pathname)}
    >
      <Slot />
    </LandingChrome>
  );
});
