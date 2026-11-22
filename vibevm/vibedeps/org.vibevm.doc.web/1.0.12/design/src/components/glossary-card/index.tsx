/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-GLOSSARY-CARD */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/**
 * The card a glossary term shows beside itself on hover.
 *
 * Rendered empty, once per page, and filled from the hidden definitions the
 * island already carries — the pipeline resolved the entry at build time and
 * put its term and its first paragraph there, so showing one costs a read of
 * the DOM and no request at all. That is what makes the same page work in
 * the local reader, offline and behind a firewall.
 *
 * `aria-hidden`, and deliberately so: a screen reader already hears the
 * definition as the link's description, because every term link carries
 * `aria-describedby` pointing at that same hidden block on every device.
 * A card that announced itself would read the definition twice.
 *
 * The markup carries the handles (`data-gloss-card*`) and no behaviour. A
 * card the behaviour had to create would be a card with its styles in a
 * string, unreviewable and invisible to the contrast audit.
 */
export const GlossaryCard = component$(() => {
  useStyles$(styles);
  return (
    <aside class="gloss-card" data-gloss-card aria-hidden="true" hidden>
      <p class="gloss-card__term" data-gloss-card-term />
      <p class="gloss-card__text" data-gloss-card-text />
    </aside>
  );
});
