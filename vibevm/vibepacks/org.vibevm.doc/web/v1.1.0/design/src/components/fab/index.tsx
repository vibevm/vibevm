/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#DISC-MACHINE-MIRROR */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type FabProps = {
  /** The button's accessible name — «For an agent», and what it opens. */
  readonly label: string;
  /** The glyph on the button. One character; the name carries the meaning. */
  readonly glyph: string;
};

/**
 * The floating button in the corner and the panel it owns.
 *
 * On this site it is the agent surface: the page's `spec://` address with
 * its version, the `.md` and `.xml` projections beside it, the package's
 * `llms.txt` — everything a reader hands to an agent instead of pasting
 * a screenshot. It is deliberately not a chat.
 *
 * The shell is all that is here. Opening, closing and copying arrive with
 * the reader's behaviour; the markup already carries `aria-expanded` so
 * that wiring has one attribute to move and no structure to invent.
 */
export const Fab = component$<FabProps>((props) => {
  useStyles$(styles);
  return (
    <div class="fab">
      <button
        class="fab__button"
        type="button"
        aria-label={props.label}
        aria-expanded={false}
        aria-controls="fab-panel"
      >
        <span aria-hidden="true">{props.glyph}</span>
      </button>
      <div class="fab__panel" id="fab-panel" hidden>
        <Slot />
      </div>
    </div>
  );
});
