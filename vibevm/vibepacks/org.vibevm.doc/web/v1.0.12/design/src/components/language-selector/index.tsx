/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One language a reader can move to, as the selector shows it. */
export type LanguageChoice = {
  /** The BCP-47 tag, shown small beside the name. */
  readonly tag: string;
  /** The name of the language in that language: «Русский», «English». */
  readonly label: string;
  /** The group that published this edition. Always shown, never hidden. */
  readonly publisher: string;
  /** Whether this edition wears the star — «the source's own group made it». */
  readonly official: boolean;
  /** The source documentation itself, which officiality is measured against. */
  readonly source: boolean;
  /**
   * The same page in this language, already a served path — present only
   * where choosing IS a move. On a shelf the same control narrows what
   * stands on it and the reader does not leave the address they are on,
   * so the entry is a button: a filter is not a place.
   */
  readonly href?: string;
  readonly current: boolean;
  /** When a human last read this edition aloud; absent when never. */
  readonly reviewedAt?: string;
  /**
   * What this entry means, when the three words below cannot say it.
   * «Everything» is not an edition and has neither a standing nor a
   * publisher; every other entry leaves this absent and is described by
   * what it is.
   */
  readonly note?: string;
  /**
   * What the closed pill prints while this entry is the current one.
   * A language prints its tag and needs none of this; «Everything» is
   * not a tag and `*` is a filter's spelling rather than a word.
   */
  readonly pill?: string;
};

export type LanguageSelectorProps = {
  /** What the control is, for a reader who cannot see the pill. */
  readonly label: string;
  /**
   * The choices in the order D-19 puts them: the source, then the
   * starred editions, then the community ones. The component does not
   * sort — the order is one of the three signals that must agree with
   * the star and the word, and they can only agree if one place decides.
   */
  readonly items: ReadonlyArray<LanguageChoice>;
};

/**
 * The language of the DOCUMENTATION: which edition of a text a reader is
 * reading, and which others exist.
 *
 * Three things travel with every entry, and none of them is optional:
 * the star for an edition the source's own group published, the word
 * that says the same thing for a reader who has never been told what a
 * star means, and the publisher's group — because the defence against a
 * misleading adaptation is not moderation, it is the publisher being
 * visible (D-19).
 *
 * On a page an entry is a plain link with the destination already in it,
 * so the selector works before any script does. What a script adds later
 * is the reader's PLACE: the fragment is appended on the way out,
 * because an adaptation mirrors the source block for block, so `#p12`
 * means the same block in both and the reader keeps their line. The
 * behaviour finds the control by `data-language-selector` and has no
 * markup of its own to invent.
 *
 * On a shelf there is nowhere to go — every edition is already on the
 * page — so the same control narrows what stands there instead, and an
 * entry with no address is a button. One control for one question, in
 * the two places the question has different answers; `data-doc-lang` is
 * what the filter listens for either way.
 *
 * It is NOT the switch beside it in the header. That one says what the
 * furniture is written in and changes no text a package wrote.
 */
export const LanguageSelector = component$<LanguageSelectorProps>((props) => {
  useStyles$(styles);
  const current = props.items.find((item) => item.current);
  return (
    <details class="language-selector" data-language-selector>
      <summary class="language-selector__pill" aria-label={props.label}>
        <span class="language-selector__tag">
          {current?.pill ?? current?.tag ?? "—"}
        </span>
        <span class="language-selector__caret" aria-hidden="true">
          ▾
        </span>
      </summary>
      <ul class="language-selector__list">
        {props.items.map((item) => (
          <li key={item.tag}>
            {item.href === undefined ? (
              <button
                class={
                  item.current
                    ? "language-selector__item language-selector__item--current"
                    : "language-selector__item"
                }
                type="button"
                data-doc-lang={item.tag}
                data-doc-pill={item.pill ?? item.tag}
                aria-pressed={item.current}
              >
                <Entry item={item} />
              </button>
            ) : (
              <a
                class={
                  item.current
                    ? "language-selector__item language-selector__item--current"
                    : "language-selector__item"
                }
                href={item.href}
                hreflang={item.tag}
                data-lang-choice={item.tag}
                data-doc-lang={item.tag}
                data-doc-pill={item.pill ?? item.tag}
                {...(item.current ? { "aria-current": "true" as const } : {})}
              >
                <Entry item={item} />
              </a>
            )}
          </li>
        ))}
      </ul>
    </details>
  );
});

/**
 * What one entry says about itself, written once for the two shapes it
 * is worn in. A link and a button differ in what clicking does and in
 * nothing a reader reads, so the reading is one component.
 */
const Entry = component$<{ item: LanguageChoice }>((props) => {
  const item = props.item;
  return (
    <>
      <span class="language-selector__name">
        {item.official ? (
          <span class="language-selector__star" aria-hidden="true">
            ★
          </span>
        ) : null}
        {item.label}
        <span class="language-selector__tag">{item.tag}</span>
      </span>
      <span class="language-selector__note">
        {item.note ??
          `${
            item.source
              ? "source"
              : item.official
                ? "official adaptation"
                : "community adaptation"
          } · ${item.publisher}`}
      </span>
      {item.reviewedAt === undefined ? null : (
        <span class="language-selector__note">
          read aloud {item.reviewedAt.slice(0, 10)}
        </span>
      )}
    </>
  );
});
