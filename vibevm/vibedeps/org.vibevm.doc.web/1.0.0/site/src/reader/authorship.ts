/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-AUTHORSHIP */

/**
 * Which documents a reader wants on the shelf: the ones a person wrote,
 * the ones a model wrote, or all of them.
 *
 * It is the reader's own question and so it lives in the reader's own
 * browser, beside the language they want offered and the theme they read
 * in. Nothing about it reaches a server, and nothing about it changes an
 * address: every card the door carries is already on the page, and this
 * narrows what stands there.
 *
 * Which cards a group holds is not decided here — `lib/authorship.ts`
 * holds that rule, so the door that renders the control and the
 * behaviour that acts on it cannot come to disagree about what «human-
 * authored» means. What is decided here is only what a reader last
 * asked for, and it is asked of the page first: a stored answer the
 * page does not offer is no answer at all.
 */

import { admitsAuthorship, EVERY_AUTHORSHIP } from "../lib/authorship.ts";
import { all, one } from "./dom.ts";
import { narrowShelves, widenShelves } from "./shelf.ts";
import { readLocal, writeLocal } from "./storage.ts";

/** Where the choice lives, beside the two languages and the theme. */
const KEY = "authorship";

/** The name this control's narrowing is held under on a shelf. */
const NARROWING = "authorship";

/** The groups this page offers; anything else was never chosen here. */
function offered(): string[] {
  const out: string[] = [];
  for (const entry of all("[data-authorship-choice]")) {
    const value = entry.dataset["authorshipChoice"];
    if (value !== undefined && !out.includes(value)) out.push(value);
  }
  return out;
}

/**
 * The group a reader is on: their own answer when this page offers it,
 * and everything otherwise.
 *
 * There is no address to fall back to, unlike the language filter: a
 * shelf of one authorship has no address of its own and is not going to
 * get one — which group a reader wants is a preference about how they
 * read, not a place they can send somebody.
 */
function chosen(): string {
  const available = offered();
  const stored = readLocal(KEY);
  return stored !== null && available.includes(stored)
    ? stored
    : EVERY_AUTHORSHIP;
}

/** Mark the entry the reader is on, and the pill that summarises them. */
function markEntries(choice: string): void {
  for (const entry of all("[data-authorship-choice]")) {
    const current = entry.dataset["authorshipChoice"] === choice;
    entry.classList.toggle("authorship-filter__item--current", current);
    entry.setAttribute("aria-pressed", current ? "true" : "false");
  }

  const pill = one("[data-authorship-filter] summary .authorship-filter__tag");
  const entry = one(`[data-authorship-choice="${choice}"]`);
  if (pill !== null) {
    pill.textContent = entry?.dataset["authorshipPill"] ?? choice;
  }
}

/** Put a choice on the page. Everything visual happens here. */
export function applyAuthorship(choice: string): void {
  markEntries(choice);
  narrowShelves(NARROWING, (card) =>
    admitsAuthorship(choice, card.dataset["authorship"]),
  );
}

export function startAuthorship(): () => void {
  /* A page that does not offer the control is a page this behaviour has
     nothing to do on — the documentation's own page and every page of a
     manual. Registering a narrowing there would hide cards by a rule the
     reader was never shown a way to change. */
  const control = one("[data-authorship-filter]");
  if (control === null) return () => undefined;

  applyAuthorship(chosen());

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const entry = target.closest("[data-authorship-choice]");
    if (!(entry instanceof HTMLElement)) return;
    const value = entry.dataset["authorshipChoice"];
    if (value === undefined) return;
    writeLocal(KEY, value);
    applyAuthorship(value);
    /* The list is a `<details>`; leaving it open over the shelf it has
       just narrowed hides the result of the click. */
    entry.closest("details")?.removeAttribute("open");
  };

  document.addEventListener("click", onClick);
  return () => {
    document.removeEventListener("click", onClick);
    widenShelves(NARROWING);
  };
}
