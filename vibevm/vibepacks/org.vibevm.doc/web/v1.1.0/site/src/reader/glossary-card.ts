/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-GLOSSARY-CARD */

/**
 * A glossary term shows its definition on hover, in the desktop layout only.
 *
 * Like the rule transclusion beside it, the whole point is what this module
 * does NOT do. The definition is already in the page — the pipeline resolved
 * the entry at build time and put its term and its first paragraph into a
 * hidden block at the end of the island, keyed by the id every term link
 * carries in `data-gloss` — so the card is filled from the DOM and nothing is
 * fetched. A shell that went to the network for a definition would be a shell
 * that cannot show one offline, in the local reader, or behind a firewall.
 *
 * ## Why the desktop test is taken at the moment of the hover
 *
 * Two conditions, and neither is a fact about the page as it loaded. The
 * layout is `has-sidebar`, the class the contents column puts on the page
 * when the window has room for a column beside the text — and it moves when
 * the window is resized or the reader widens the column. The pointer is one
 * that can hover and points finely, which moves when a tablet's keyboard is
 * attached or the reader switches from a finger to a mouse. A decision taken
 * once at load would be a decision taken against a page that no longer
 * exists.
 *
 * ## What the link keeps
 *
 * Everything. The card is added beside it and the link is untouched:
 * selecting it still opens the glossary at the entry, which is the behaviour
 * a touch screen has and the only behaviour a narrow window has. And a screen
 * reader hears the definition on EVERY device, because the link's
 * `aria-describedby` points at the same hidden block the card is cloned from
 * — which is why the card itself is `aria-hidden` and nothing here changes
 * that.
 */

import { place, type Box } from "../lib/gloss-place.ts";
import { island, one, targetOf } from "./dom.ts";

/** How long a pointer has to rest on a term before the card opens. */
const OPEN_AFTER_MS = 350;

/**
 * How long the card survives the pointer leaving, so a reader can cross the
 * gap between the link and the card without it vanishing under them.
 */
const GRACE_MS = 140;

/** The pointer this behaviour is for: one that can hover, and finely. */
const DESKTOP_POINTER = "(hover: hover) and (pointer: fine)";

/** The layout this behaviour is for: the contents column beside the text. */
const SIDEBAR = ".has-sidebar";

/** Is this page, right now, a desktop page under a desktop pointer? */
function desktop(link: Element): boolean {
  if (link.closest(SIDEBAR) === null) return false;
  // Not every engine has `matchMedia`; one that has not is not a desktop
  // browser for this purpose, and guessing would open a card under a finger.
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia(DESKTOP_POINTER).matches;
}

/** The rectangle of an element, in the shape the placement wants. */
function boxOf(element: Element): Box {
  const rect = element.getBoundingClientRect();
  return {
    left: rect.left,
    top: rect.top,
    width: rect.width,
    height: rect.height,
  };
}

/**
 * Fill the card from the hidden definition the island carries for `id`.
 *
 * `false` when the page carries none: a link whose entry did not travel is a
 * link, and showing an empty card would be the shell inventing content.
 */
function fill(card: HTMLElement, region: HTMLElement, id: string): boolean {
  const source = one(`.gloss-def[data-gloss="${CSS.escape(id)}"]`, region);
  if (source === null) return false;
  const term = one(".gloss-def__term", source);
  const text = one(".gloss-def__text", source);
  if (term === null || text === null) return false;

  const intoTerm = one("[data-gloss-card-term]", card);
  const intoText = one("[data-gloss-card-text]", card);
  if (intoTerm === null || intoText === null) return false;
  intoTerm.textContent = term.textContent ?? "";
  // The definition's own markup travels with it — a code span, a link — so
  // its children are cloned rather than its text taken. The links inside it
  // are the pipeline's, resolved at build time, and they stay clickable.
  intoText.replaceChildren(
    ...Array.from(text.childNodes).map((node) => node.cloneNode(true)),
  );
  return true;
}

/** Show the card beside `link`, measured after it has its content. */
function show(card: HTMLElement, link: Element): void {
  card.hidden = false;
  const at = place(boxOf(link), boxOf(card), {
    width: window.innerWidth,
    height: window.innerHeight,
    scrollX: window.scrollX,
    scrollY: window.scrollY,
  });
  card.style.left = `${at.left}px`;
  card.style.top = `${at.top}px`;
  card.dataset["glossSide"] = at.side;
  // Opacity is a frame later than the layout, so the fade starts from where
  // the card actually is rather than from where the last one was.
  window.requestAnimationFrame(() => card.setAttribute("data-open", ""));
}

/** Wire the cards over the island. One card per page, reused. */
export function startGlossaryCard(): () => void {
  const region = island();
  const card = one("[data-gloss-card]");
  if (region === null || card === null) return () => undefined;

  let openFor: Element | null = null;
  let opening: number | undefined;
  let closing: number | undefined;

  const cancel = (): void => {
    if (opening !== undefined) window.clearTimeout(opening);
    if (closing !== undefined) window.clearTimeout(closing);
    opening = undefined;
    closing = undefined;
  };

  const close = (): void => {
    cancel();
    openFor = null;
    card.removeAttribute("data-open");
    card.hidden = true;
    card.removeAttribute("style");
    delete card.dataset["glossSide"];
  };

  /** Open for a term link, after the pause and only if still on a desktop. */
  const open = (link: Element, delay: number): void => {
    if (openFor === link) {
      cancel();
      return;
    }
    cancel();
    opening = window.setTimeout(() => {
      opening = undefined;
      const id = link.getAttribute("data-gloss");
      // Re-asked here and not at the hover: 350ms is long enough for a
      // window to be resized or a page to be left.
      if (id === null || !desktop(link)) return;
      if (!fill(card, region, id)) return;
      openFor = link;
      show(card, link);
    }, delay);
  };

  /** The term link an event happened on, when it happened on one. */
  const termOf = (event: Event): Element | null => {
    const target = targetOf(event);
    return target === null ? null : target.closest("a[data-gloss]");
  };

  const onOver = (event: Event): void => {
    const link = termOf(event);
    if (link === null) return;
    if (!desktop(link)) return;
    open(link, OPEN_AFTER_MS);
  };

  const onOut = (event: Event): void => {
    if (termOf(event) === null) return;
    cancel();
    // A grace period rather than a close: the pointer is on its way to the
    // card as often as it is on its way out of both.
    closing = window.setTimeout(close, GRACE_MS);
  };

  /* The card keeps itself open while the pointer is on it, so the links
     inside a definition can be read and followed. */
  const onCardOver = (): void => cancel();
  const onCardOut = (): void => {
    cancel();
    closing = window.setTimeout(close, GRACE_MS);
  };

  /* Keyboard focus opens it too, and without the pause: a reader who tabbed
     to a term asked for it, and a wait would look like nothing happening. */
  const onFocusIn = (event: Event): void => {
    const link = termOf(event);
    if (link === null || !desktop(link)) return;
    open(link, 0);
  };

  const onFocusOut = (event: Event): void => {
    if (termOf(event) === null) return;
    close();
  };

  const onKey = (event: KeyboardEvent): void => {
    if (event.key === "Escape") close();
  };

  /* Scrolling closes it. The card is placed in document coordinates, so it
     would travel with the text correctly — but a reader who scrolls has
     stopped reading that word, and a card that follows them down the page is
     a card in the way. */
  const onScroll = (): void => {
    if (card.hidden) return;
    close();
  };

  region.addEventListener("pointerover", onOver);
  region.addEventListener("pointerout", onOut);
  region.addEventListener("focusin", onFocusIn);
  region.addEventListener("focusout", onFocusOut);
  card.addEventListener("pointerover", onCardOver);
  card.addEventListener("pointerout", onCardOut);
  document.addEventListener("keydown", onKey);
  window.addEventListener("scroll", onScroll, { passive: true });

  return () => {
    cancel();
    region.removeEventListener("pointerover", onOver);
    region.removeEventListener("pointerout", onOut);
    region.removeEventListener("focusin", onFocusIn);
    region.removeEventListener("focusout", onFocusOut);
    card.removeEventListener("pointerover", onCardOver);
    card.removeEventListener("pointerout", onCardOut);
    document.removeEventListener("keydown", onKey);
    window.removeEventListener("scroll", onScroll);
  };
}
