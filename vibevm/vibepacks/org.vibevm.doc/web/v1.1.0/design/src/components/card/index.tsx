/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DECISION */

import { component$, useStyles$ } from "@qwik.dev/core";
import {
  AuthorshipBadge,
  Badge,
  GeneratedBadge,
  type AuthorshipMark,
  type DocStatus,
} from "../badge/index.tsx";
import {
  BridgeSignatures,
  type BridgeAuthorship,
} from "../bridge-signatures/index.tsx";
import {
  KindGlyph,
  kindLabel,
  type PackageKind,
} from "../kind-glyph/index.tsx";
import styles from "./styles.css?inline";

export type CardProps = {
  /** The display title from the manifest's card, never the coordinate. */
  readonly title: string;
  /** Where the card leads, already a served path. */
  readonly href: string;
  /** The group that published it — printed beside the title, always. */
  readonly publisher: string;
  /** The coordinate, shown small: identity, as opposed to the name. */
  readonly coordinate: string;
  /** The one-line subtitle; absent when the package declares none. */
  readonly description?: string;
  /**
   * What it covers, for whom, what it assumes known, what it leaves out.
   * Shown on click rather than on the shelf: a shelf of six abstracts is
   * a wall of text, and the question a shelf answers is «which one».
   *
   * Absent when the thing on the card carries none of its own — in which
   * case the disclosure is not rendered at all. Borrowing the abstract
   * of whatever the card happens to sit under would put four answers
   * about a documentation under the name of one page of it.
   */
  readonly abstract?: string;
  /** Where this documentation stands for its subject. */
  readonly status: DocStatus;
  /** The address of the package's icon, when it has one. */
  readonly icon?: string;
  /**
   * What kind of package the card names, when the caller knows it
   * (VIBEVM-SPEC §4.1). With it the placeholder carries that kind's own
   * mark, so a flow reads as a flow and a manual as a manual before the
   * title is read; without it the card draws the glyph below instead,
   * because a mark is a statement and a card may not guess one.
   */
  readonly kind?: PackageKind;
  /** One character for the generated placeholder when it has no icon. */
  readonly glyph: string;
  /** What the abstract's disclosure is called. */
  readonly abstractLabel: string;
  /**
   * The BCP-47 tag of the edition this card names, for the shelf's
   * language filter to act on. Absent on a card that is not an edition
   * of anything — a page of the one documentation already being read —
   * which is the same as saying the filter has no business hiding it.
   */
  readonly editionLang?: string;
  /**
   * True when the thing on the card is a level-zero rendering: the
   * pipeline printed it out of a package's own bytes and nobody wrote
   * it. It travels beside the standing rather than instead of it,
   * because the two answer different questions.
   */
  readonly generated?: boolean;
  /**
   * Who wrote the prose, when the documentation says so
   * (`##CARD-AUTHORSHIP`). Absent is not a third state to draw: a
   * document that declared nothing wears no mark, and the shelf's filter
   * leaves it out of both named groups. The value is written onto the
   * card as data as well as shown, because the filter narrows a shelf
   * the build already wrote and has nothing else to read.
   */
  readonly authorship?: AuthorshipMark;
  /**
   * The two authorships of a bridge, when the thing on the card is one
   * (PROP-023 `##AUTHORSHIP-SEPARATION`). Absent for everything else,
   * which leaves the card exactly as it was: the publisher, and no claim
   * about who wrote what it points at.
   */
  readonly bridge?: BridgeAuthorship;
};

/**
 * One entry on a shelf: the arXiv row of the vision, shaped as a card.
 *
 * Four things are always present and in this order — the mark, the
 * title, the publisher and the coordinate — because the reader's
 * question on a shelf is «whose is this, and is it the one the subject
 * points at». The mark is drawn by the badge, which owns what the three
 * standings look like, so a card cannot disagree with a catalogue row
 * about what `primary` means.
 *
 * The abstract is a `<details>` and not a hover card or a script. It
 * opens on click, it prints when the page prints, it survives a reader
 * with no pointer, and it costs no behaviour at all. A card whose
 * subject has no abstract of its own shows no disclosure — an empty
 * promise and a borrowed answer are both worse than nothing.
 *
 * When a package declares no icon, the placeholder is drawn from tokens
 * rather than fetched: a shelf that reached out to someone else's host
 * for a picture would be a shelf that leaks who is reading it (D-20,
 * R-09). What stands in it is the package's KIND where the caller knew
 * it — the one mark that tells a flow from a manual before either is
 * opened — and the plain glyph where it did not, because a shelf of
 * eight kinds is only readable if a card that knows no kind says so by
 * showing none.
 */
export const Card = component$<CardProps>((props) => {
  useStyles$(styles);
  return (
    <article
      class="card"
      {...(props.editionLang === undefined
        ? {}
        : { "data-edition-lang": props.editionLang })}
      {...(props.authorship === undefined
        ? {}
        : { "data-authorship": props.authorship })}
    >
      <div class="card__row">
        {props.icon === undefined ? (
          props.kind === undefined ? (
            <span class="card__placeholder" aria-hidden="true">
              {props.glyph}
            </span>
          ) : (
            /* Named, because with a kind in it the tile carries a fact
               that is nowhere else on the card. The mark inside stays
               hidden: one announcement, not two. */
            <span
              class="card__placeholder"
              role="img"
              title={kindLabel(props.kind)}
              aria-label={kindLabel(props.kind)}
            >
              <KindGlyph kind={props.kind} />
            </span>
          )
        ) : (
          <img
            class="card__icon"
            src={props.icon}
            alt=""
            width="48"
            height="48"
          />
        )}
        <div class="card__body">
          <h3 class="card__title">
            <a class="card__link" href={props.href}>
              {props.title}
            </a>
            <Badge status={props.status} />
            {props.generated === true ? <GeneratedBadge /> : null}
            {props.authorship === undefined ? null : (
              <AuthorshipBadge authorship={props.authorship} />
            )}
          </h3>
          <p class="card__publisher">
            {props.publisher}
            <span class="card__coordinate">{props.coordinate}</span>
          </p>
          {props.bridge === undefined ? null : (
            <BridgeSignatures bridge={props.bridge} />
          )}
          {props.description === undefined ? null : (
            <p class="card__description">{props.description}</p>
          )}
        </div>
      </div>
      {props.abstract === undefined ? null : (
        <details class="card__abstract">
          <summary>{props.abstractLabel}</summary>
          <p>{props.abstract}</p>
        </details>
      )}
    </article>
  );
});
