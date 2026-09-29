/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type CitedRulesProps = {
  /** What the block is called: «Rules this page cites». */
  readonly label: string;
};

/**
 * The rules the page quotes, gathered under the end of it.
 *
 * They used to stand in the side column under the table of contents, and
 * a `spec://` address is long enough that four of them made the block
 * taller than the contents it hung beneath — a column about where you
 * are in the page, most of which was a list of addresses. The column now
 * holds the headings and nothing else, and this stands after the text,
 * folded, where a reader who wants the list goes looking for it.
 *
 * It is rendered empty and filled from the island, like the contents:
 * every rule is already in the text as a quotation with its address on
 * it, so this list is a second VIEW of the same facts and never a second
 * source. A page that quotes no rule gets no block at all rather than an
 * empty heading — which is why the element starts hidden and the reader
 * is what reveals it.
 *
 * The count is in the summary because the summary is all a reader sees
 * of a folded block, and «how many» is the question that decides whether
 * to open it. It stands in its own element so that the words beside it
 * stay one string the site's language table can move.
 */
export const CitedRules = component$<CitedRulesProps>((props) => {
  useStyles$(styles);
  return (
    <details class="cited-rules" data-page-rules hidden>
      <summary class="cited-rules__summary">
        {props.label} <span class="cited-rules__count" data-page-rules-count />
      </summary>
      <ul class="cited-rules__list" data-page-rules-list />
    </details>
  );
});
