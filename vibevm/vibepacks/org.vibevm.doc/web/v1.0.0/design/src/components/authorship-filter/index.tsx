/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-AUTHORSHIP */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One group of documents the control offers to narrow a shelf to. */
export type AuthorshipChoice = {
  /**
   * The manifest's own word for the group — `human`, `ai` — or `*` for
   * the entry that narrows nothing. The markup carries the vocabulary
   * rather than a spelling of this component's own, so the behaviour
   * that reads it and the field it is read from cannot drift apart.
   */
  readonly value: string;
  /** What the entry is called: «Human-authored», «AI-generated». */
  readonly label: string;
  /** The line under it: what asking for this group actually gets. */
  readonly note: string;
  /** What the closed pill prints while this entry is the current one. */
  readonly pill: string;
  readonly current: boolean;
};

export type AuthorshipFilterProps = {
  /** What the control is, for a reader who cannot see the pill. */
  readonly label: string;
  readonly items: ReadonlyArray<AuthorshipChoice>;
};

/**
 * Who wrote the prose: the control that narrows a shelf to the
 * documents a person wrote, or to the ones a model wrote.
 *
 * It is a statement about the DOCUMENT and never about the repository or
 * anybody's commits (`##CARD-AUTHORSHIP`), and it is the reader's own
 * question rather than the site's: a reader who wants to know whose
 * words they are about to read should not have to open every card to
 * find out.
 *
 * **Every entry is a button and none of them is an address.** All the
 * cards are already on the page — the control narrows what stands there
 * instead of moving the reader — which is the same shape the language
 * filter takes on a shelf, and for the same reason: a filter is not a
 * place. It is deliberately a second control and not a fourth entry in
 * that one, because «which language of this text» and «who wrote it» are
 * two questions with two answers, and a reader may want both narrowings
 * at once.
 *
 * The three entries are not three slices of one list either. A document
 * whose manifest says `mixed` stands in BOTH named groups, because both
 * hands are in the text; a document that says nothing stands in neither,
 * because a shelf that guessed would be making the claim the field
 * exists to stop it making. That rule lives where the field is read —
 * this component shows what it is handed and decides nothing.
 *
 * A `<details>`, like the language selector beside it, so the list opens
 * with no behaviour loaded; what a script adds later is the reader's
 * memory of which entry they chose.
 */
export const AuthorshipFilter = component$<AuthorshipFilterProps>((props) => {
  useStyles$(styles);
  const current = props.items.find((item) => item.current);
  return (
    <details class="authorship-filter" data-authorship-filter>
      <summary class="authorship-filter__pill" aria-label={props.label}>
        <span class="authorship-filter__tag">
          {current?.pill ?? props.items[0]?.pill ?? "—"}
        </span>
        <span class="authorship-filter__caret" aria-hidden="true">
          ▾
        </span>
      </summary>
      <ul class="authorship-filter__list">
        {props.items.map((item) => (
          <li key={item.value}>
            <button
              class={
                item.current
                  ? "authorship-filter__item authorship-filter__item--current"
                  : "authorship-filter__item"
              }
              type="button"
              data-authorship-choice={item.value}
              data-authorship-pill={item.pill}
              aria-pressed={item.current}
            >
              <span class="authorship-filter__name">{item.label}</span>
              <span class="authorship-filter__note">{item.note}</span>
            </button>
          </li>
        ))}
      </ul>
    </details>
  );
});
