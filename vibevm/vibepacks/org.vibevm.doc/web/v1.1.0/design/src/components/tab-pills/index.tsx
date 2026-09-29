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
  /**
   * What this row switches, when it is not the island's `when` blocks.
   *
   * A row of pills is a row of pills and the design system has no
   * opinion about what choosing one does. The platform switch is the
   * one that has always been here, so it keeps the handle it has and a
   * caller that names something else gets a handle of its own — two
   * rows on one page would otherwise be one behaviour's two halves.
   */
  readonly name?: string;
};

/**
 * A row of pills: one choice showing, the rest a click away.
 *
 * The row this was built for is the platform switch, which decides which
 * `data-when` blocks of the island a reader sees. The blocks are all
 * there. The pipeline keeps every conditional block in the page and
 * numbers it before any of them is hidden, so that `p12` is the same
 * block on Windows and on macOS and in every translation — the switch
 * may hide a block, it may never renumber one.
 *
 * Buttons, not links: choosing does not change the page's address, and a
 * reader who copies the address must not hand someone else their own
 * operating system. `data-when-switch` is the handle the platform
 * behaviour attaches to; a named row carries `data-tab-switch` instead,
 * and whatever listens for that name owns it.
 */
export const TabPills = component$<TabPillsProps>((props) => {
  useStyles$(styles);
  return (
    <div
      class="tab-pills"
      role="group"
      aria-label={props.label}
      {...(props.name === undefined
        ? { "data-when-switch": true }
        : { "data-tab-switch": props.name })}
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
