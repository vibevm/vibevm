/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import {
  $,
  component$,
  useOnDocument,
  useSignal,
  useStyles$,
  type QRL,
} from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One answer the box offers: what it is, where it is, what it belongs to. */
export type SearchHit = {
  readonly title: string;
  /** Where it leads, already a served path. */
  readonly href: string;
  /** The line under the title: which documentation, in which language. */
  readonly context: string;
};

export type SearchBoxProps = {
  /** The accessible name of the field — a placeholder is not a label. */
  readonly label: string;
  readonly placeholder: string;
  /** The shortcut printed on the right, e.g. `Ctrl K`. */
  readonly shortcut: string;
  /** What to say when a reader's words match nothing. */
  readonly emptyLabel: string;
  /**
   * The corpus, as a question this box may ask.
   *
   * The box knows how to ask — when, how many, and what to do with the
   * answers — and knows nothing about what is being searched. That is
   * the seam: a design system with an opinion about a documentation
   * library would be a design system with one user.
   */
  readonly find$: QRL<(query: string) => Promise<readonly SearchHit[]>>;
};

/** The id of the field, and the stem every answer's id is built on. */
const FIELD = "search-box-input";
const LIST = "search-box-list";

/** How long a query has to be before it is worth answering. */
const MIN = 2;

/**
 * The search field in the header, and the answers it offers under it.
 *
 * It was a shape with nothing behind it: a bare `<input>` in no form,
 * with no route to submit to and no listener anywhere — so typing did
 * nothing, Enter did nothing, and the `Ctrl K` pill beside it was a
 * promise the page could not keep. All three are kept now.
 *
 * The behaviour is Qwik's own and not a module in `reader/`, which is
 * the opposite of how the documentation page's furniture works — and
 * the difference is the island. A reader's page is finished HTML the
 * framework never rendered, so nothing inside it can carry a handler;
 * this header IS the framework's, on both halves of the site, so the
 * ordinary way is available and one component can serve both.
 *
 * The label is visually hidden rather than absent: a field whose only
 * name is its placeholder loses that name the moment a reader types.
 * The answers are real links, so a reader may open one in a new tab,
 * and `role="option"` over them is what lets the keyboard walk the list
 * without the focus ever leaving the field they are typing in.
 */
export const SearchBox = component$<SearchBoxProps>((props) => {
  useStyles$(styles);

  const field = useSignal<HTMLInputElement>();
  const hits = useSignal<readonly SearchHit[]>([]);
  const open = useSignal(false);
  /** Which answer Enter would take; `-1` while there are none. */
  const at = useSignal(-1);
  /** True once a query was asked and answered with nothing. */
  const empty = useSignal(false);

  const close = $(() => {
    open.value = false;
    at.value = -1;
  });

  /* The shortcut the pill has always printed. It is caught on the
     document because a reader who wants the search box is by definition
     not standing in it. */
  useOnDocument(
    "keydown",
    $((event: KeyboardEvent) => {
      if (event.key !== "k" && event.key !== "K") return;
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      field.value?.focus();
      field.value?.select();
    }),
  );

  /* A click anywhere else puts the list away. Blur would be the obvious
     hook and is the wrong one: a click on an answer blurs the field
     before the link is followed, and the list would be gone by then. */
  useOnDocument(
    "pointerdown",
    $((event: PointerEvent) => {
      if (!open.value) return;
      const target = event.target;
      if (target instanceof Element && target.closest(".search-box") !== null) {
        return;
      }
      open.value = false;
      at.value = -1;
    }),
  );

  return (
    <div class="search-box">
      <label class="search-box__label" for={FIELD}>
        {props.label}
      </label>
      <input
        ref={field}
        class="search-box__input"
        id={FIELD}
        type="search"
        placeholder={props.placeholder}
        autoComplete="off"
        role="combobox"
        aria-expanded={open.value}
        aria-controls={LIST}
        aria-autocomplete="list"
        {...(at.value >= 0
          ? { "aria-activedescendant": `${LIST}-${at.value}` }
          : {})}
        onInput$={async (_event, element) => {
          const query = element.value;
          if (query.trim().length < MIN) {
            hits.value = [];
            empty.value = false;
            open.value = false;
            at.value = -1;
            return;
          }
          const found = await props.find$(query);
          hits.value = found;
          empty.value = found.length === 0;
          open.value = true;
          at.value = found.length === 0 ? -1 : 0;
        }}
        onKeyDown$={(event) => {
          if (event.key === "Escape") {
            open.value = false;
            at.value = -1;
            return;
          }
          if (!open.value || hits.value.length === 0) return;
          if (event.key === "ArrowDown") {
            event.preventDefault();
            at.value = (at.value + 1) % hits.value.length;
            return;
          }
          if (event.key === "ArrowUp") {
            event.preventDefault();
            at.value = (at.value - 1 + hits.value.length) % hits.value.length;
            return;
          }
          if (event.key === "Enter") {
            const hit = hits.value[at.value];
            if (hit === undefined) return;
            event.preventDefault();
            window.location.assign(hit.href);
          }
        }}
        onFocus$={() => {
          if (hits.value.length > 0 || empty.value) open.value = true;
        }}
      />
      <kbd class="search-box__shortcut">{props.shortcut}</kbd>

      <div
        class="search-box__results"
        id={LIST}
        role="listbox"
        aria-label={props.label}
        hidden={!open.value}
      >
        {hits.value.length === 0 ? (
          <p class="search-box__empty">{props.emptyLabel}</p>
        ) : (
          hits.value.map((hit, index) => (
            <a
              key={hit.href}
              class={
                index === at.value
                  ? "search-box__hit search-box__hit--at"
                  : "search-box__hit"
              }
              id={`${LIST}-${index}`}
              role="option"
              aria-selected={index === at.value}
              href={hit.href}
              onClick$={close}
            >
              <span class="search-box__hit-title">{hit.title}</span>
              <span class="search-box__hit-context">{hit.context}</span>
            </a>
          ))
        )}
      </div>
    </div>
  );
});
