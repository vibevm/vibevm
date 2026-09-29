/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type FootnotesProps = {
  /** The heading the section is announced by, e.g. «Notes». */
  readonly title: string;
};

/**
 * The notes at the foot of a page, set smaller and quieter than the text
 * they belong to, with the back-reference arrow the reader returns by.
 *
 * A labelled `<section>` and not a bare `<div>`: the notes are a landmark
 * a reader jumps to and back from, and the heading is what names it in a
 * document outline.
 */
export const Footnotes = component$<FootnotesProps>((props) => {
  useStyles$(styles);
  return (
    <section class="footnotes" aria-label={props.title}>
      <h2 class="footnotes__title">{props.title}</h2>
      <Slot />
    </section>
  );
});
