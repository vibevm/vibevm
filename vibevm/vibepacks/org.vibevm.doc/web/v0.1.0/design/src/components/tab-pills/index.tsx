/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One choice of the switch: the value it selects and what it is called. */
export type TabPill = {
  readonly label: string;
  /** The `data-when` value this pill shows, e.g. `windows`, `macos`. */
  readonly value: string;
  readonly current: boolean;
};

export type TabPillsProps = {
  readonly label: string;
  readonly items: ReadonlyArray<TabPill>;
};

/**
 * The platform switch: the row of pills that decides which `data-when`
 * blocks of the island a reader sees.
 *
 * The blocks are all there. The pipeline keeps every conditional block in
 * the page and numbers it before any of them is hidden, so that `p12` is
 * the same block on Windows and on macOS and in every translation — the
 * switch may hide a block, it may never renumber one.
 *
 * Buttons, not links: choosing a platform does not change the page's
 * address, and a reader who copies the address must not hand someone
 * else their own operating system. The `data-when-switch` attribute is
 * the handle the behaviour attaches to later.
 */
export const TabPills = component$<TabPillsProps>((props) => {
  useStyles$(styles);
  return (
    <div
      class="tab-pills"
      role="group"
      aria-label={props.label}
      data-when-switch
    >
      {props.items.map((item) => (
        <button
          key={item.value}
          class={
            item.current
              ? "tab-pills__pill tab-pills__pill--current"
              : "tab-pills__pill"
          }
          type="button"
          value={item.value}
          aria-pressed={item.current}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
});
