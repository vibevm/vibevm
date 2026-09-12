/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** What the header shows on the left, whatever page it sits on. */
export type DocsHeaderProps = {
  /** The product's name, in the display face. */
  readonly brand: string;
  /** Where the brand points — the site root, not the package root. */
  readonly homeHref: string;
};

/**
 * The site's top bar: the brand, then whatever the page puts beside it —
 * a language pill, the search box, the theme switch. The slot is the
 * whole point: the header knows the shape of the bar and nothing about
 * what the documentation or the landing chooses to hang in it, so both
 * use one header instead of two that drift (D-28).
 *
 * It carries no behaviour. The theme switch, the language selector and
 * the search are wired later, over the shell this shapes.
 */
export const DocsHeader = component$<DocsHeaderProps>((props) => {
  useStyles$(styles);
  return (
    <header class="docs-header">
      <div class="docs-header__inner">
        <a class="docs-header__brand" href={props.homeHref}>
          {props.brand}
        </a>
        <div class="docs-header__actions">
          <Slot />
        </div>
      </div>
    </header>
  );
});
