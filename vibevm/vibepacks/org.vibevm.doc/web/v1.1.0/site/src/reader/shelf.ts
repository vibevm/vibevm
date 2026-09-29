/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * What is left standing on a shelf after every control a reader has
 * touched has had its say.
 *
 * Two of them narrow the same cards — which language of a documentation
 * to be offered, and who wrote its prose — and a card stands when BOTH
 * admit it. That is the whole reason neither control hides a card
 * itself. Two modules each writing `hidden` would make the last click
 * win: choosing a language would silently widen the authorship the
 * reader had chosen a moment earlier, and the shelf would be answering a
 * question nobody asked. So the decision is taken once, out of every
 * narrowing currently registered, and a control's whole part is to say
 * which cards IT admits and ask for the shelves to be drawn again.
 *
 * The cards it governs are the EDITIONS — the ones carrying
 * `data-edition-lang` — and nothing else. A card with no language on it
 * is a page of the documentation already open rather than an edition of
 * one, and neither question has any business hiding it.
 *
 * The empty line belongs here for the same reason the decision does:
 * whether a shelf has anything left on it is a fact about the result,
 * and no single control can work it out. The line is already in the
 * document — the build writes it hidden — so a shelf a reader has
 * emptied says so instead of looking broken.
 */

import { all, one } from "./dom.ts";

/** The cards a control may narrow: one edition of one documentation. */
const CARDS = "[data-edition-lang]";

/** Whether one control admits one card. */
export type Narrowing = (card: HTMLElement) => boolean;

/**
 * The narrowings in force, by the name of the control that asked for
 * them.
 *
 * By name and not by insertion, so a control that re-applies — which is
 * what every click does — replaces its own answer rather than stacking a
 * second one on top of it.
 */
const narrowings = new Map<string, Narrowing>();

/** Put one control's narrowing in force and draw the shelves again. */
export function narrowShelves(name: string, admits: Narrowing): void {
  narrowings.set(name, admits);
  applyShelves();
}

/** Take one control's narrowing away — a page being left — and redraw. */
export function widenShelves(name: string): void {
  narrowings.delete(name);
  applyShelves();
}

/** Show every card every narrowing admits, and hide the rest. */
export function applyShelves(): void {
  const admitting = [...narrowings.values()];
  for (const card of all(CARDS)) {
    card.hidden = admitting.some((admits) => !admits(card));
  }

  for (const shelf of all("[data-shelf]")) {
    const cards = all(CARDS, shelf);
    if (cards.length === 0) continue;
    const left = cards.some((card) => !card.hidden);
    const items = one(".shelf__items", shelf);
    const empty = one("[data-shelf-empty]", shelf);
    if (items !== null) items.hidden = !left;
    if (empty !== null) empty.hidden = left;
  }
}
