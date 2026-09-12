/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type TocProps = {
  /** What the list is called: «Contents». */
  readonly label: string;
  /** The heading over the rules the page cites. */
  readonly rulesLabel: string;
};

/**
 * The table of contents, and the rules the page quotes under it.
 *
 * It is rendered empty and filled from the island, because the headings
 * are in HTML the pipeline produced and the shell does not parse
 * content: reading `h2` and `h3` out of a document that is already in
 * the page is a walk of the DOM, not a second renderer. The same is true
 * of the rules — every one of them is a `blockquote` already standing in
 * the text, with its `spec://` address on it.
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
      <section class="toc__rules" data-page-rules hidden>
        <h2 class="toc__rules-title">{props.rulesLabel}</h2>
        <ul class="toc__rules-list" data-page-rules-list />
      </section>
    </details>
  );
});
