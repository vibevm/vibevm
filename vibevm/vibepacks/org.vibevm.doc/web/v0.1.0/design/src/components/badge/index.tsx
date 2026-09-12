/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#ROW-DISC-DOCS-STATUS */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/**
 * Where one documentation stands for one subject. The three names are
 * the manifest's, closed by construction: the value is computed from the
 * convergence of two edges at every render, and a fourth name would mean
 * a fourth way for edges to meet.
 */
export type DocStatus = "primary" | "official" | "community";

export type BadgeProps = {
  readonly status: DocStatus;
};

/** The word each standing is shown with, next to its mark. */
const WORDS = {
  primary: "primary",
  official: "official",
  community: "community",
} as const satisfies Record<DocStatus, string>;

/**
 * The star and the word, in one place.
 *
 * The star is never the whole message. A mark a reader has to have been
 * told about says nothing on first sight, so the word travels with it —
 * and the word is the one the manifest uses, not a synonym invented for
 * the page.
 */
export const Badge = component$<BadgeProps>((props) => {
  useStyles$(styles);
  return (
    <span class={`badge badge--${props.status}`}>
      {props.status === "primary" ? <span class="badge__star">★</span> : null}
      {WORDS[props.status]}
    </span>
  );
});
