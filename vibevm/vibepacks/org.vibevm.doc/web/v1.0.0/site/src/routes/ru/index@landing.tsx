/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { landingHead } from "../../landing/head.ts";
import { Landing } from "../../landing/landing.tsx";

/**
 * `https://vibevm.org/ru/` — the Russian landing.
 *
 * The whole page differs from its English twin by one argument, which
 * is the point of keeping the copy in a string table rather than in the
 * markup: a translator adds a language by adding a column, and nothing
 * about the layout, the graph or the install panel has an opinion about
 * which one is being rendered.
 *
 * Russian is the tree submitted to Yandex, so it is fully translated
 * and fully addressed — `hreflang`, `canonical` and the sitemap all name
 * it, and the language switch in the header points at its twin.
 */
export default component$(() => <Landing locale="ru" />);

export const head: DocumentHead = landingHead("ru");
