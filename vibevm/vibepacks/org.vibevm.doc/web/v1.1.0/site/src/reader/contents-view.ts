/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * What the contents view IS, in one place: two states, where the choice
 * is kept, and what changing it does to the document.
 *
 * It is its own module for the reason the theme is: the choice is applied
 * twice, by two things that must not disagree. `theme-init.js` stamps it
 * on the root element before the first stylesheet is parsed — the server
 * writes both views of the column and the stylesheet shows one, so a
 * reader who asked for the folders never watches the path appear and go
 * away again — and this module stamps it again when the reader changes
 * it, out of the same two words and under the same key.
 *
 * Two states and the second one is the only one that is written. The
 * learning path is the default (`##NAV-CHAPTERS-READER`): where a
 * documentation declared one, the column opens on it, so the ABSENCE of
 * the attribute is the path, exactly as the absence of `data-theme` is
 * the system's colours. A reader with scripting off therefore gets the
 * default rather than a column with both views in it.
 *
 * A documentation that declared no path renders one view and no switch,
 * and the attribute means nothing on such a page — which is why nothing
 * here asks whether a path exists. The stamp is the reader's statement
 * about how they like to read; whether a given documentation can honour
 * it is the column's business and the stylesheet's.
 */

import { all } from "./dom.ts";
import { readLocal } from "./storage.ts";

/** The two views of the column the reader chooses between. */
export type ContentsView = "path" | "sections";

/** Which one a reader who has chosen nothing gets. */
export const DEFAULT_CONTENTS_VIEW: ContentsView = "path";

/** Whether a value is one of the two — asked, never asserted. */
export function isContentsView(value: unknown): value is ContentsView {
  return value === "path" || value === "sections";
}

/**
 * The view this reader is on: their own choice if they made one, and the
 * path otherwise.
 *
 * A stored value comes from a previous version of this page as often as
 * from this one, so it is checked rather than trusted; anything else is
 * the same state as never having chosen.
 */
export function storedContentsView(): ContentsView {
  const stored = readLocal("contents");
  return isContentsView(stored) ? stored : DEFAULT_CONTENTS_VIEW;
}

/**
 * Put a view on the document, and on every switch that offers one.
 *
 * The buttons are found by their data attribute rather than held as a
 * reference, for the reason the theme's are: a page may carry the column
 * more than once — the block above the text and the column beside it are
 * one element today, and nothing about this promises they always will be
 * — and a reader who changed the view must not be left looking at a
 * control still claiming the old one.
 *
 * The mark a reader SEES is the stylesheet's, out of the attribute this
 * writes on the root; what is written on the buttons here is the mark a
 * screen reader reads, which no stylesheet can state.
 */
export function applyContentsView(view: ContentsView): void {
  const root = document.documentElement;
  if (view === DEFAULT_CONTENTS_VIEW) {
    root.removeAttribute("data-contents-view");
  } else {
    root.setAttribute("data-contents-view", view);
  }

  for (const button of all("[data-contents-choice]")) {
    const current = button.dataset["contentsChoice"] === view;
    button.setAttribute("aria-pressed", current ? "true" : "false");
  }
}
