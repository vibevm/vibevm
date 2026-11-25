/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NO-AUTOSCROLL */

/**
 * Where the reader was, and the button that offers it back.
 *
 * The page never scrolls itself. That is the decision, taken from the
 * experience the vision cites: an automatic jump to a remembered place
 * breaks every deep link — a reader who followed `#p12` lands somewhere
 * else — and it takes the page out from under someone who only wanted to
 * re-read the opening. So the place is remembered and OFFERED, and the
 * offer withdraws itself as soon as the reader has scrolled past the
 * first heading by themselves, because by then they have chosen.
 *
 * `history.scrollRestoration` is switched to manual for the same reason:
 * the browser's own restoration is the very jump this exists to avoid,
 * and leaving it on would mean two mechanisms moving one page.
 */

import { all, byId, island, one } from "./dom.ts";
import { positionKey, readLocal, writeLocal } from "./storage.ts";

/** At most one save a second, as the reference reader does it. */
const SAVE_MS = 1000;

/** How far above the top edge a block counts as «already read». */
const ABOVE = 5;

/** The mark the page's own entry wears for a moment when it is pointed at. */
const CUED = "contents__link--cued";

/** And how long it wears it: long enough to be seen, short enough to go. */
const CUE_MS = 1600;

/** How long after the last scroll event a movement counts as finished. */
const STILL_MS = 200;

/** The nearest block above the top of the window, by id. */
function nearestAbove(region: HTMLElement): string | null {
  let best: string | null = null;
  for (const node of Array.from(region.querySelectorAll("[id]"))) {
    if (!(node instanceof HTMLElement)) continue;
    if (node.getBoundingClientRect().top > ABOVE) break;
    best = node.getAttribute("id");
  }
  return best;
}

export function startPosition(): () => void {
  const region = island();
  if (region === null) return () => undefined;

  if ("scrollRestoration" in window.history) {
    window.history.scrollRestoration = "manual";
  }

  const key = positionKey(window.location.pathname);
  const buttons = all("[data-return]");
  const saved = readLocal(key);

  let target: string | null = null;
  let tracking = false;
  let timer: number | null = null;
  let cue: number | null = null;

  const show = (): void => {
    for (const button of buttons) button.hidden = false;
  };
  const hide = (): void => {
    for (const button of buttons) button.hidden = true;
  };

  if (saved !== null && byId(saved) !== null) {
    target = saved;
    show();
  }

  const firstHeading = region.querySelector("h2, h3");

  const save = (): void => {
    if (timer !== null) return;
    timer = window.setTimeout(() => {
      timer = null;
      const at = nearestAbove(region);
      if (at !== null) writeLocal(key, at);
    }, SAVE_MS);
  };

  /**
   * Whose movement this is.
   *
   * The page scrolls smoothly, so one jump arrives as a stream of scroll
   * events — and the rule below reads a stream of scroll events as «the
   * reader has gone past the opening by themselves, withdraw the offer».
   * The offer that ≡ makes is followed by exactly such a jump, and it
   * withdrew itself in the moment it was made. So a movement this module
   * asked for says so, until the events stop: how long a smooth scroll
   * takes is the browser's business and not a number to write down.
   */
  let moving = false;
  let still: number | null = null;

  const rearm = (): void => {
    if (still !== null) window.clearTimeout(still);
    still = window.setTimeout(() => {
      still = null;
      moving = false;
    }, STILL_MS);
  };

  const carrying = (): void => {
    moving = true;
    rearm();
  };

  const onScroll = (): void => {
    if (moving) {
      rearm();
      return;
    }
    if (firstHeading === null) return;
    const top = firstHeading.getBoundingClientRect().top;
    if (!tracking && top < 0) {
      tracking = true;
      target = null;
      hide();
    }
    if (tracking) save();
  };

  const onReturn = (): void => {
    if (target === null) return;
    const element = byId(target);
    if (element !== null) element.scrollIntoView({ block: "start" });
    target = null;
    hide();
  };

  /**
   * Choosing a place from the table of contents is choosing a place: the
   * old offer is void, and tracking starts again from where the reader
   * landed.
   */
  const onTocPick = (event: Event): void => {
    const node = event.target;
    if (!(node instanceof Element)) return;
    if (node.closest("[data-toc] a") === null) return;
    tracking = true;
    target = null;
    hide();
    save();
  };

  /**
   * The page's own entry in the column, brought into the column and
   * marked — without the document moving a pixel.
   *
   * `scrollIntoView` is not what does it: it scrolls every scrollable
   * ancestor, the document included, which is the jump this is here to
   * avoid. The column's `scrollTop` is set directly instead, and only
   * when the entry is actually out of its frame.
   *
   * The entry is looked for among the VISIBLE ones: the column carries
   * both views of the list — the path and the folders — and the one that
   * is not shown has an entry for this page too, with no box and nothing
   * to focus.
   */
  const reveal = (contents: HTMLElement): void => {
    const shown = all(".contents__link--current", contents).filter(
      (link) => link.offsetParent !== null,
    );
    const current = shown[0];
    if (current === undefined) return;
    const frame = contents.getBoundingClientRect();
    const box = current.getBoundingClientRect();
    if (box.top < frame.top || box.bottom > frame.bottom) {
      contents.scrollTop +=
        box.top - frame.top - (frame.height - box.height) / 2;
    }
    current.focus({ preventScroll: true });
    current.classList.add(CUED);
    if (cue !== null) window.clearTimeout(cue);
    cue = window.setTimeout(() => {
      cue = null;
      current.classList.remove(CUED);
    }, CUE_MS);
  };

  /**
   * ≡ — the way to the MANUAL's pages, in the two shapes that list has.
   *
   * The button asked for `[data-toc]` before, which is the other list
   * entirely: the headings of the page the reader is already on. So the
   * contents never opened, and on a wide screen the page jumped to a
   * STICKY column, which is a scroll to wherever that column happened to
   * be standing — the two halves of the owner's report, one cause.
   *
   * The two shapes are answered differently because the reader's question
   * has two different answers. Folded, the list is a disclosure above the
   * text: it is opened, and the page moves to it — which is LEAVING a
   * place, so the offer to come back is made there and then rather than on
   * the next visit, and it is the reader's own click that asks for it.
   * In the column the list is already on the screen and the page must not
   * move at all; what moves is the column's own scroller, far enough to
   * bring the page's own entry into it, and the entry says so for a
   * moment because nothing else changed.
   *
   * Focus goes with the eye in both: the control that was just reached is
   * the next thing a keyboard will act on.
   */
  const onTocButton = (event: Event): void => {
    const node = event.target;
    if (!(node instanceof Element)) return;
    if (node.closest("[data-quick-toc]") === null) return;
    const contents = one("[data-contents]");
    if (contents === null) return;

    if (contents.closest(".has-sidebar") !== null) {
      reveal(contents);
      return;
    }

    const at = nearestAbove(region);
    if (at !== null) {
      target = at;
      writeLocal(key, at);
      show();
    }
    tracking = false;
    if (contents instanceof HTMLDetailsElement) contents.open = true;
    const summary = one("summary", contents) ?? contents;
    /* The stylesheet holds the gap: the control stops under the sticky
       header rather than behind it (`scroll-margin-top`). The movement is
       this module's, and says so, or the offer just made would read as
       the reader scrolling away from their place. */
    carrying();
    summary.scrollIntoView({ block: "start" });
    summary.focus({ preventScroll: true });
  };

  window.addEventListener("scroll", onScroll, { passive: true });
  for (const button of buttons) button.addEventListener("click", onReturn);
  document.addEventListener("click", onTocPick);
  document.addEventListener("click", onTocButton);

  return () => {
    window.removeEventListener("scroll", onScroll);
    for (const button of buttons) button.removeEventListener("click", onReturn);
    document.removeEventListener("click", onTocPick);
    document.removeEventListener("click", onTocButton);
    if (timer !== null) window.clearTimeout(timer);
    if (cue !== null) window.clearTimeout(cue);
    if (still !== null) window.clearTimeout(still);
  };
}
