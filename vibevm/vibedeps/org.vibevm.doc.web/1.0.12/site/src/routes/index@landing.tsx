/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { landingHead } from "../landing/head.ts";
import { Landing } from "../landing/landing.tsx";

/**
 * `https://vibevm.org/` — the English landing, and the site's root.
 *
 * English lives at the root without a prefix and Russian under `/ru/`.
 * That is the strongest URL serving content rather than redirecting to a
 * prefixed copy of itself, and it is the address the domain has had:
 * the port keeps it, `hreflang` declares it, and `/en/` still resolves
 * for anyone holding the older form.
 */
export default component$(() => <Landing locale="en" />);

export const head: DocumentHead = landingHead("en");
