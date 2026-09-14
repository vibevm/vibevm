/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DECISION */

import { component$, useStyles$ } from "@qwik.dev/core";
import { Badge, GeneratedBadge, type DocStatus } from "../badge/index.tsx";
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
 * and a glyph rather than fetched: a shelf that reached out to someone
 * else's host for a picture would be a shelf that leaks who is reading
 * it (D-20, R-09).
 */
export const Card = component$<CardProps>((props) => {
  useStyles$(styles);
  return (
    <article
      class="card"
      {...(props.editionLang === undefined
        ? {}
        : { "data-edition-lang": props.editionLang })}
    >
      <div class="card__row">
        {props.icon === undefined ? (
          <span class="card__placeholder" aria-hidden="true">
            {props.glyph}
          </span>
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
          </h3>
          <p class="card__publisher">
            {props.publisher}
            <span class="card__coordinate">{props.coordinate}</span>
          </p>
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
