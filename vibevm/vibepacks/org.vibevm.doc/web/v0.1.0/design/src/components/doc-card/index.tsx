/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import { Badge, type DocStatus } from "../badge/index.tsx";
import styles from "./styles.css?inline";

export type DocCardProps = {
  /** The display name from the manifest's card, never the coordinate. */
  readonly title: string;
  /** Where the card leads, already a served path. */
  readonly href: string;
  /** The group that published it, printed beside the title, always. */
  readonly publisher: string;
  /** The one-line subtitle; the full abstract belongs to the page. */
  readonly summary: string;
  /** Where this documentation stands for its subject. */
  readonly status: DocStatus;
};

/**
 * One entry on a shelf: the arXiv row of the vision, shaped as a card.
 *
 * Three things are always present and in this order — a star for primary
 * standing, the title, and the publisher — because the reader's question
 * on a shelf is «whose is this, and is it the one the subject points
 * at». The star is rendered by the badge, which owns what the three
 * standings look like, so a card cannot disagree with a catalogue row
 * about what `primary` means.
 */
export const DocCard = component$<DocCardProps>((props) => {
  useStyles$(styles);
  return (
    <a class="doc-card" href={props.href}>
      <span class="doc-card__head">
        <span class="doc-card__title">{props.title}</span>
        <Badge status={props.status} />
      </span>
      <span class="doc-card__publisher">{props.publisher}</span>
      <span class="doc-card__summary">{props.summary}</span>
    </a>
  );
});
