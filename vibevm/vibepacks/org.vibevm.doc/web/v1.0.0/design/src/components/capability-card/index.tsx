/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type CapabilityCardProps = {
  /** The mono kicker: one word for what part of the product this is. */
  readonly label: string;
  /** The claim, in the display face. */
  readonly heading: string;
  /** The claim made concrete, in one sentence. */
  readonly body: string;
};

/**
 * One of the three cards under the hero: a label, a claim, a sentence.
 *
 * It is not a link and has no hover state, which is the decision worth
 * recording: these are the answer to «what is this», read once on the
 * way down the page, and giving them the affordances of a card that
 * leads somewhere would promise a destination that does not exist.
 */
export const CapabilityCard = component$<CapabilityCardProps>((props) => {
  useStyles$(styles);
  return (
    <div class="capability">
      <div class="capability__label">{props.label}</div>
      <div class="capability__heading">{props.heading}</div>
      <p class="capability__body">{props.body}</p>
    </div>
  );
});

export type CapabilityRowProps = {
  /** What the row is, for a reader who cannot see that it is one. */
  readonly label: string;
};

/**
 * The row the cards sit in: three columns that become one on a narrow
 * screen, separated by a hairline the grid draws with its own gap.
 *
 * The separator is the row's background showing through a one-pixel gap
 * rather than a border on each card, because borders on adjacent cards
 * double up and a rule drawn twice is a rule drawn wrong.
 */
export const CapabilityRow = component$<CapabilityRowProps>((props) => {
  useStyles$(styles);
  return (
    <section class="capability-row" aria-label={props.label}>
      <Slot />
    </section>
  );
});
