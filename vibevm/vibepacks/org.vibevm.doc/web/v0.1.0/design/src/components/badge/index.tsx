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

/**
 * The mark a level-zero rendering carries: the pipeline printed this out
 * of a package's own bytes, and nobody wrote it.
 *
 * It is a SECOND badge and not a fourth standing, because it answers a
 * different question. Where a documentation stands for its subject is
 * about the relation between two packages; how a document came to exist
 * is about the document. A rendering can be primary for its subject —
 * it usually is, being the subject itself — and a reader needs both
 * facts, so both marks travel.
 *
 * The word is not softened and is not an apology. A reader deciding
 * whether to trust a page is owed the plainest available statement of
 * what it is.
 */
export const GeneratedBadge = component$(() => {
  useStyles$(styles);
  return <span class="badge badge--generated">GENERATED</span>;
});
