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

import { all, byId, island } from "./dom.ts";
import { positionKey, readLocal, writeLocal } from "./storage.ts";

/** At most one save a second, as the reference reader does it. */
const SAVE_MS = 1000;

/** How far above the top edge a block counts as «already read». */
const ABOVE = 5;

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

  const onScroll = (): void => {
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

  window.addEventListener("scroll", onScroll, { passive: true });
  for (const button of buttons) button.addEventListener("click", onReturn);
  document.addEventListener("click", onTocPick);

  return () => {
    window.removeEventListener("scroll", onScroll);
    for (const button of buttons) button.removeEventListener("click", onReturn);
    document.removeEventListener("click", onTocPick);
    if (timer !== null) window.clearTimeout(timer);
  };
}
