/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * Reading mode: the moment the page decides a reader is reading.
 *
 * One threshold, taken from the reference reader and unchanged — the
 * first heading above 40 % of the window's height. Before it, the gear
 * alone stands in the corner; after it, the quick row appears beside the
 * gear, so changing the text size costs one click instead of three.
 *
 * The threshold is a position rather than a timer or a scroll distance,
 * because what it is actually measuring is «the reader has left the head
 * of the page», and the head of the page is a different length on every
 * page and on every screen.
 */

import { island, one } from "./dom.ts";

/** The share of the window below which the first heading means «reading». */
const THRESHOLD = 0.4;

export function startReadingMode(): () => void {
  const region = island();
  const panel = one("[data-settings]");
  if (region === null || panel === null) return () => undefined;

  const trigger = region.querySelector("h2, h3") ?? region;

  const onScroll = (): void => {
    const top = trigger.getBoundingClientRect().top;
    panel.classList.toggle(
      "settings--reading",
      top < window.innerHeight * THRESHOLD,
    );
  };

  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  return () => window.removeEventListener("scroll", onScroll);
}
