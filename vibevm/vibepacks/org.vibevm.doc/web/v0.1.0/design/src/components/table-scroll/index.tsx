/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type TableScrollProps = {
  /**
   * Let the container leave the reading column and use the window's
   * width. For a table the measure cannot hold, this is better than
   * scrolling it inside a narrow box; for one that fits, it is worse.
   */
  readonly breakout: boolean;
  /** What the scrollable region is called for a reader who tabs into it. */
  readonly label: string;
};

/**
 * A wide table inside a narrow column, without the page scrolling
 * sideways with it.
 *
 * `tabindex="0"` is the part that is easy to forget and impossible to do
 * without: a region that scrolls must be reachable from the keyboard, or
 * the columns past the fold exist only for a mouse.
 */
export const TableScroll = component$<TableScrollProps>((props) => {
  useStyles$(styles);
  return (
    <div
      class={props.breakout ? "table-scroll breakout" : "table-scroll"}
      role="region"
      aria-label={props.label}
      tabIndex={0}
    >
      <Slot />
    </div>
  );
});
