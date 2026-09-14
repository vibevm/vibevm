/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import { Badge, type DocStatus } from "../badge/index.tsx";
import {
  KindGlyph,
  kindLabel,
  type PackageKind,
} from "../kind-glyph/index.tsx";
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
  /**
   * What kind of package the card names, when the caller knows it
   * (VIBEVM-SPEC §4.1). This card carries no tile to stand a mark in, so
   * the mark stands on the title's line — where the other card puts it
   * from the width a shelf holds two. A caller that names no kind gets
   * the row it had before, with nothing in front of the title.
   */
  readonly kind?: PackageKind;
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
 *
 * A fourth thing joins them where the caller knows it: the mark of the
 * package's kind, in front of the title. It is drawn by the same
 * component the other card's tile holds, so the two cards cannot end up
 * telling a reader that a `flow` looks like one thing on one shelf and
 * another on the next.
 */
export const DocCard = component$<DocCardProps>((props) => {
  useStyles$(styles);
  const kind = props.kind;
  return (
    <a class="doc-card" href={props.href}>
      <span class="doc-card__head">
        {kind === undefined ? null : (
          <span
            class="doc-card__mark"
            role="img"
            title={kindLabel(kind)}
            aria-label={kindLabel(kind)}
          >
            <KindGlyph kind={kind} />
          </span>
        )}
        <span class="doc-card__title">{props.title}</span>
        <Badge status={props.status} />
      </span>
      <span class="doc-card__publisher">{props.publisher}</span>
      <span class="doc-card__summary">{props.summary}</span>
    </a>
  );
});
