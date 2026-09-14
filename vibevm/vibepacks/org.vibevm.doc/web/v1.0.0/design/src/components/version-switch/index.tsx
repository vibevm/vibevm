/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-VERSION-SHOWS-CURRENT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One version of the documentation, as an address. */
export type VersionChoice = {
  /** What is printed: a version number, or the word `latest`. */
  readonly label: string;
  /** The same page at that version, already a served path. */
  readonly href: string;
  readonly current: boolean;
  /** A word under the pill: what choosing this one means. */
  readonly note: string;
};

export type VersionSwitchProps = {
  readonly label: string;
  readonly items: ReadonlyArray<VersionChoice>;
};

/**
 * The version switch: the same page, at another version's address.
 *
 * Links and not a select, because every entry is a place that must
 * survive a middle click — and because there is nothing to submit. The
 * addresses are built by the site's one address function, so the switch
 * works identically in the build the server serves and the build `vibe`
 * embeds.
 *
 * What it deliberately does not do is promise a history. An address with
 * a version number always shows the CURRENT content of that version: the
 * same number may be published ten times in a day and the site shows the
 * last publication, because the registry keeps no earlier one to show
 * (`##SITE-VERSION-SHOWS-CURRENT`, D-27). So the note under each pill
 * says what the address means, not when it was made.
 */
export const VersionSwitch = component$<VersionSwitchProps>((props) => {
  useStyles$(styles);
  return (
    <nav class="version-switch" aria-label={props.label}>
      {props.items.map((item) => (
        <a
          key={item.href}
          class={
            item.current
              ? "version-switch__item version-switch__item--current"
              : "version-switch__item"
          }
          href={item.href}
          title={item.note}
          {...(item.current ? { "aria-current": "true" as const } : {})}
        >
          {item.label}
        </a>
      ))}
    </nav>
  );
});
