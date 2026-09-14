/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type TocProps = {
  /** What the list is called: «On this page». */
  readonly label: string;
};

/**
 * The table of contents: this page's own headings, and nothing else.
 *
 * The rules the page quotes used to hang under them here, and a
 * `spec://` address is long enough that a few of them made the block
 * taller than the contents above it. They are a list of what the page
 * cites rather than a map of where the reader is in it, and they now
 * stand folded at the end of the text (`CitedRules`), which leaves this
 * column answering the one question it is for.
 *
 * It is rendered empty and filled from the island, because the headings
 * are in HTML the pipeline produced and the shell does not parse
 * content: reading `h2` and `h3` out of a document that is already in
 * the page is a walk of the DOM, not a second renderer.
 *
 * One element serves both shapes the vision asks for. A `<details>` that
 * is forced open and loses its summary IS the sticky sidebar; the same
 * element with its summary back is the collapsible block a narrow screen
 * gets. Two elements would mean two lists to keep in step, and the one
 * that was not on screen would be the one that went stale.
 *
 * Which shape is used depends on both widths, not one: the window's, and
 * the column's. A reader who widens the reading column past 1100px has
 * taken the sidebar's room, and a sidebar squeezed against the text is
 * worse than a block above it.
 */
export const Toc = component$<TocProps>((props) => {
  useStyles$(styles);
  return (
    <details class="toc" data-toc open>
      <summary class="toc__summary">{props.label}</summary>
      <nav class="toc__nav" aria-label={props.label}>
        <ol class="toc__list" data-toc-list />
      </nav>
    </details>
  );
});
