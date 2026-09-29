/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-CHARSET-AND-REDIRECTS */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { href } from "../../lib/href.ts";
import { redirectHead } from "../../landing/head.ts";
import { STRINGS } from "../../landing/i18n.ts";

/**
 * `/en/` — the older form of the English address, kept alive.
 *
 * The Astro deployment answered it with `return 301 /` in nginx, and
 * links to it exist: the 404 page's own «go home» button is one. A
 * static build cannot return a status code, so the address becomes a
 * page whose only job is to leave — a meta refresh for the browser, a
 * `canonical` on the root and `noindex` for the crawler, and a plain
 * link for anyone whose browser honours neither.
 *
 * A server-side 301 is still the better answer and the deployment atom
 * may restore it. This page is what makes the address work without one,
 * which is the difference between a redirect that is configured and a
 * redirect that is built.
 */
export default component$(() => (
  <section class="landing-redirect">
    <p>
      <a href={href("")}>{STRINGS.en.metaTitle}</a>
    </p>
  </section>
));

export const head: DocumentHead = redirectHead();
