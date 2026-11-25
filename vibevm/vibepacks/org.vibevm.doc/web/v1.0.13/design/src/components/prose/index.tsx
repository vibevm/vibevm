/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type ProseProps = {
  /** The reading measure in pixels; the reader's own setting overrides it. */
  readonly measure: number;
};

/**
 * The reading column, and the styles for everything the documentation
 * pipeline puts inside it.
 *
 * This is the one component whose CSS is written for markup it does not
 * render. The island arrives as finished HTML from the Rust pipeline —
 * `article.doc-page`, numbered blocks with their `a.p-anchor`, quoted
 * rules, derived output, examples, notes, figures, prompts — and Qwik
 * never touches it (see the island component). So the vocabulary of that
 * HTML is the vocabulary of this stylesheet, and the two are kept in
 * step by the golden the pipeline blesses, not by anyone's memory.
 *
 * The measure arrives as a custom property rather than a width so that
 * the reader's column setting can move it later without this component
 * learning that readers have settings.
 */
export const Prose = component$<ProseProps>((props) => {
  useStyles$(styles);
  return (
    <div class="prose" style={{ "--measure": `${props.measure}px` }}>
      <Slot />
    </div>
  );
});
