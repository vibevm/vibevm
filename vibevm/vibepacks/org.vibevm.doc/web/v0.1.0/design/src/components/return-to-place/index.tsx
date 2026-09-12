/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NO-AUTOSCROLL */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type ReturnToPlaceProps = {
  readonly label: string;
};

/**
 * «Return to where you were» — a button, and deliberately not a jump.
 *
 * The page remembers the nearest block above the top of the window while
 * a reader scrolls, and on the next visit it does NOT move them: it
 * offers. Auto-scrolling breaks a deep link — a reader who followed
 * `#p12` lands somewhere else entirely — and it takes the page away from
 * under someone who only wanted the first paragraph again. The offer
 * disappears the moment they scroll past the first heading by themselves,
 * because by then they have chosen where they are.
 *
 * It renders hidden. Nothing to return to is the ordinary case — a first
 * visit — and a button that promises a place it does not have is worse
 * than no button.
 */
export const ReturnToPlace = component$<ReturnToPlaceProps>((props) => {
  useStyles$(styles);
  return (
    <button class="return-to-place" type="button" data-return hidden>
      <span class="return-to-place__mark" aria-hidden="true">
        ↩
      </span>
      {props.label}
    </button>
  );
});
