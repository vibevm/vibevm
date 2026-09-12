/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { Catalogue } from "../../components/catalogue/index.tsx";
import { catalogueHead } from "../../seo/head.ts";

/**
 * The door: every documentation this build carries, with no language
 * segment in front of it.
 *
 * It has none deliberately. Which language a reader walks through it
 * into is their choice — remembered, then asked of the browser, then
 * decided by the documentation's own language — and the catalogue
 * component takes that decision once per session. What this page shows
 * meanwhile is every door at once, so a reader who lands on the wrong
 * one is never stuck.
 */
export default component$(() => {
  return <Catalogue lang={null} />;
});

/**
 * The door's head is the catalogue's head, and the catalogue of a
 * language is this page at that language's address: one function
 * answers for both, so the `hreflang` set they publish about each other
 * cannot disagree.
 */
export const head: DocumentHead = catalogueHead(null);
