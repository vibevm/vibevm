/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";
import { Hero } from "@vibe-docs/design";

import { href } from "../lib/href.ts";
import { notFoundHead } from "../landing/head.ts";
import { GITHUB_URL, NOT_FOUND } from "../landing/i18n.ts";

/**
 * `/404.html` — the page the server hands back for an address that is
 * not one.
 *
 * Qwik Router builds a route named `404` into exactly that file, which
 * is what `error_page 404 /404.html` has always served. One page for the
 * whole domain and in English, as before: a reader who mistypes a
 * Russian address gets the same apology they got yesterday.
 *
 * It is the hero component with different words in it — the joke about
 * the lockfile is the landing's voice, and a second set of styles for
 * one page would be a second place to change the button.
 */
export default component$(() => (
  <Hero
    eyebrow={NOT_FOUND.eyebrow}
    headlineHtml={NOT_FOUND.headlineHtml}
    leadHtml={NOT_FOUND.lead}
    primary={{ label: NOT_FOUND.ctaPrimary, href: href("en/") }}
    secondary={{ label: NOT_FOUND.ctaSecondary, href: GITHUB_URL }}
  />
));

export const head: DocumentHead = notFoundHead();
