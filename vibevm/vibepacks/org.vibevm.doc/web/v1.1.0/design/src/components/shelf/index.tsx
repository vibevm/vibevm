/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#DISC-THREE-SIGNALS */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type ShelfProps = {
  /** What stands on this shelf: «Documentation», «Adaptations». */
  readonly title: string;
  /**
   * The sentence under the heading that says what the shelf's order and
   * marks mean — written once, here, rather than assumed known.
   */
  readonly caption: string;
  /** What to say when the shelf is empty; an empty shelf still speaks. */
  readonly emptyLabel: string;
  readonly empty: boolean;
};

/**
 * A shelf: a heading, the sentence that explains its marks, and the
 * cards in the order the marks agree with.
 *
 * The caption is not decoration and not a tooltip. A star means «named
 * by the author of the thing above it», which no reader can be expected
 * to guess, and D-19 asks for the three signals — mark, word, order — to
 * say one thing. Two of them are on the cards; the third is the order,
 * and the only place it can be explained is here.
 *
 * An empty shelf renders as an empty shelf, with a line saying so. The
 * alternative — hiding it — answers the reader's question («is there a
 * translation?») by making the question disappear.
 *
 * The line is therefore always in the document and merely hidden, rather
 * than rendered only when the shelf is empty at build time: a reader's
 * own filter can empty a shelf that was full, and a shelf that answered
 * a filter with nothing at all would look broken rather than filtered.
 * Which is why `data-shelf` and `data-shelf-empty` are here — the filter
 * shows the line it did not have to invent.
 */
export const Shelf = component$<ShelfProps>((props) => {
  useStyles$(styles);
  return (
    <section class="shelf" data-shelf>
      <h2 class="shelf__title">{props.title}</h2>
      <p class="shelf__caption">{props.caption}</p>
      <p class="shelf__empty" data-shelf-empty hidden={!props.empty}>
        {props.emptyLabel}
      </p>
      <div class="shelf__items" hidden={props.empty}>
        <Slot />
      </div>
    </section>
  );
});
