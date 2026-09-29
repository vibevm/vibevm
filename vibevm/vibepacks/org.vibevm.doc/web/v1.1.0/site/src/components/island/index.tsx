/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-ONE-CONTENT-PATH */

import { component$, useStyles$ } from "@qwik.dev/core";
import { fromElement, islandTarget } from "../../lib/island-target.ts";
import styles from "./styles.css?inline";

export type IslandProps = {
  /**
   * What stands where the rendered page goes.
   *
   * It is the marker in both builds, and what replaces it is the only
   * difference between them: the build driver fills every page's hole
   * once, out of the `<document>/index.html` the pipeline wrote, and
   * `vibe doc serve` fills one per request out of the machine store.
   * Passing the bytes through the framework instead would put a page of
   * the manual into the serialised state of every page that shows it,
   * and would make the two fillers two mechanisms.
   */
  readonly html: string;
};

/**
 * The region a rendered page is put into, and the one listener over it.
 *
 * The island is the seam between two worlds. On one side a Rust pipeline
 * has already turned the pivot into HTML — numbered blocks, quoted
 * rules, executed examples, generated output — and the web and the local
 * reader share that one content path precisely because neither of them
 * re-renders it. On this side Qwik must therefore do nothing at all: the
 * HTML is inserted as a string and never becomes a component tree, never
 * takes props, and is never re-rendered, because a framework that
 * re-rendered it would be a second renderer disagreeing with the first.
 *
 * That leaves events. Nothing inside the island can carry a Qwik handler
 * — Qwik did not render any of it — so the region carries exactly one,
 * and the node that was clicked is turned into an intent by a pure
 * function beside this file. Today the intent is only recorded on the
 * region; the reader's behaviour that acts on it is the next atom's, and
 * it will find the delegation already wired rather than have to invent
 * it over markup it cannot touch.
 */
export const Island = component$<IslandProps>((props) => {
  useStyles$(styles);
  return (
    <div
      class="island"
      data-island
      onClick$={(event, element) => {
        const target = event.target;
        const intent = islandTarget(
          target instanceof Element ? fromElement(target) : null,
        );
        element.dataset["islandIntent"] = intent.kind;
      }}
      dangerouslySetInnerHTML={props.html}
    />
  );
});
