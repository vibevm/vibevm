/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE */

/**
 * Which language of the DOCUMENTATION a reader wants to be offered.
 *
 * It is one question asked in two places, which is why it is one control
 * and one memory. On a shelf the editions are all in front of the reader
 * already, so the answer narrows what stands there. On a page there is
 * one text and the other languages are other addresses, so the answer is
 * a move — and the entries there are links the site rendered, which work
 * with no script at all; this module only records that the reader chose.
 *
 * What it never does is touch the site's own language. A reader may read
 * a Russian manual through an English interface or the other way round,
 * and the two preferences are stored under two keys for exactly that
 * reason (`lib/site-language.ts`).
 *
 * And it never touches `hreflang`, `canonical` or a sitemap. Those say
 * which addresses carry the same document, which is a fact about the
 * documents and not about who is looking: a crawler is offered every
 * edition whatever any one reader filtered their shelf down to.
 */

import { all, one } from "./dom.ts";
import { readLocal, writeLocal } from "./storage.ts";

/** Where the choice lives, beside the theme and the site's language. */
const KEY = "doc-lang";

/** "Everything": the answer that hides nothing. */
export const EVERY_LANGUAGE = "*";

/** Whether a filter is one the page offers; anything else never chose. */
function offered(): string[] {
  const out: string[] = [];
  for (const entry of all("[data-doc-lang]")) {
    const value = entry.dataset["docLang"];
    if (value !== undefined && !out.includes(value)) out.push(value);
  }
  return out;
}

/**
 * The filter a reader is on: their own choice if the page offers it,
 * then the language of the ADDRESS they walked in through, then
 * everything.
 *
 * The address is second and not first because arriving at
 * `vibevm.org/doc/ru/` is a statement about this visit and a stored
 * choice is a statement about the reader; but a reader who has never
 * chosen and asked for the Russian shelf by name should get the Russian
 * shelf rather than all of them.
 */
function chosen(addressLanguage: string | null): string {
  const available = offered();
  const stored = readLocal(KEY);
  if (stored !== null && available.includes(stored)) return stored;
  if (addressLanguage !== null && available.includes(addressLanguage)) {
    return addressLanguage;
  }
  return EVERY_LANGUAGE;
}

/** Mark the entry the reader is on, and the pill that summarises them. */
function markEntries(filter: string): void {
  for (const entry of all("[data-doc-lang]")) {
    const current = entry.dataset["docLang"] === filter;
    entry.classList.toggle("language-selector__item--current", current);
    if (entry instanceof HTMLButtonElement) {
      entry.setAttribute("aria-pressed", current ? "true" : "false");
    } else if (current) entry.setAttribute("aria-current", "true");
    else entry.removeAttribute("aria-current");
  }

  const pill = one("[data-language-selector] summary .language-selector__tag");
  const entry = one(`[data-doc-lang="${filter}"]`);
  if (pill !== null) {
    pill.textContent = entry?.dataset["docPill"] ?? filter;
  }
}

/**
 * Show the editions in the chosen language and hide the rest, then tell
 * a shelf that has nothing left that it has nothing left.
 *
 * A card with no language on it is not an edition of anything — a page
 * of the documentation already open — and is never hidden: a filter over
 * languages has no opinion about those.
 */
function filterCards(filter: string): void {
  for (const card of all("[data-edition-lang]")) {
    const tag = card.dataset["editionLang"];
    card.hidden = filter !== EVERY_LANGUAGE && tag !== filter;
  }

  for (const shelf of all("[data-shelf]")) {
    const cards = all("[data-edition-lang]", shelf);
    if (cards.length === 0) continue;
    const left = cards.some((card) => !card.hidden);
    const items = one(".shelf__items", shelf);
    const empty = one("[data-shelf-empty]", shelf);
    if (items !== null) items.hidden = !left;
    if (empty !== null) empty.hidden = left;
  }
}

/** Put a filter on the page. Everything visual happens here. */
export function applyDocLanguage(filter: string): void {
  markEntries(filter);
  filterCards(filter);
}

export function startDocLanguage(addressLanguage: string | null): () => void {
  /* A page with editions on it is a shelf, and a filter is what the
     control means there. A page with one text on it is not: the entry
     marked is the edition the reader is actually reading — a fact about
     the address and not a preference — and «Everything» there narrows
     nothing, exactly as it says. The page tells the two apart by what is
     on it rather than by being told, so the rule cannot get out of step
     with the markup. */
  const shelved = all("[data-edition-lang]").length > 0;
  let filter = chosen(addressLanguage);
  if (shelved) applyDocLanguage(filter);

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const entry = target.closest("[data-doc-lang]");
    if (!(entry instanceof HTMLElement)) return;
    const value = entry.dataset["docLang"];
    if (value === undefined) return;
    /* Recorded before anything else, because an entry that is a link is
       about to take the page away: the choice has to be in storage by
       the time the next page reads it. */
    writeLocal(KEY, value);
    if (entry instanceof HTMLAnchorElement) return;
    filter = value;
    if (shelved) applyDocLanguage(filter);
    /* The list is a `<details>`; an answer given is an answer, and
       leaving it open over the shelf it has just narrowed hides the
       result of the click. */
    entry.closest("details")?.removeAttribute("open");
  };

  document.addEventListener("click", onClick);
  return () => document.removeEventListener("click", onClick);
}
