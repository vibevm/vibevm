/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type SearchBoxProps = {
  /** The accessible name of the field — a placeholder is not a label. */
  readonly label: string;
  readonly placeholder: string;
  /** The shortcut printed on the right, e.g. `Ctrl K`. */
  readonly shortcut: string;
};

/**
 * The search field in the header — its shape, and nothing else. What it
 * searches, and what the shortcut opens, are wired over this shell.
 *
 * The label is visually hidden rather than absent: a field whose only
 * name is its placeholder loses that name the moment a reader types.
 */
export const SearchBox = component$<SearchBoxProps>((props) => {
  useStyles$(styles);
  return (
    <div class="search-box">
      <label class="search-box__label" for="search-box-input">
        {props.label}
      </label>
      <input
        class="search-box__input"
        id="search-box-input"
        type="search"
        placeholder={props.placeholder}
        autoComplete="off"
      />
      <kbd class="search-box__shortcut">{props.shortcut}</kbd>
    </div>
  );
});
