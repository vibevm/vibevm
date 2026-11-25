/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-GLOSSARY-CARD */

/**
 * Where a glossary card stands beside the term that opened it.
 *
 * The norm asks for three things at once and they can disagree: the card is
 * BESIDE the link, it does not cover the link, and it fits in the window.
 * Below the link satisfies all three most of the time; when the window ends
 * before the card does, above the link satisfies them instead. When neither
 * has room the card goes below anyway and the page scrolls to it, because a
 * card clamped into the middle of the text would be a card over the term.
 *
 * Sideways it is the link's own left edge, pulled back only far enough to
 * keep the card inside the window. A card that started at the link and ran
 * off the right of a narrow column would give the page a horizontal
 * scrollbar, which is the one thing a hover must never do.
 *
 * It is a pure function over rectangles on purpose: it is the half of the
 * behaviour a browser is not needed for, and the half most likely to be
 * wrong.
 */

/** A rectangle as a browser reports one, in viewport coordinates. */
export type Box = {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
};

/** The window the card has to fit in, and the page's scroll offsets. */
export type Viewport = {
  readonly width: number;
  readonly height: number;
  readonly scrollX: number;
  readonly scrollY: number;
};

/** Where the card goes, in DOCUMENT coordinates, and on which side. */
export type Placement = {
  readonly left: number;
  readonly top: number;
  readonly side: "below" | "above";
};

/** The gap between the link and the card, in pixels. */
export const GAP = 8;

/** How close to the window's edge the card may come, in pixels. */
export const MARGIN = 12;

/**
 * Place `card` beside `link`.
 *
 * Both boxes are in viewport coordinates, as `getBoundingClientRect` gives
 * them; the answer is in document coordinates, which is what an absolutely
 * positioned element wants.
 */
export function place(link: Box, card: Box, view: Viewport): Placement {
  const below = link.top + link.height + GAP;
  const above = link.top - card.height - GAP;
  const fitsBelow = below + card.height + MARGIN <= view.height;
  const fitsAbove = above >= MARGIN;
  const side = fitsBelow || !fitsAbove ? "below" : "above";
  const top = side === "below" ? below : above;

  // The right edge first, then the left: on a window narrower than the card
  // the left margin wins, because a card that starts off-screen cannot be
  // read at all.
  const widest = view.width - MARGIN - card.width;
  const left = Math.max(MARGIN, Math.min(link.left, widest));

  return {
    left: left + view.scrollX,
    top: top + view.scrollY,
    side,
  };
}
