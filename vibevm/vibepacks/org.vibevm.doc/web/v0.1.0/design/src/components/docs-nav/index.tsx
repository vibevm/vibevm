/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One entry of the section row. */
export type DocsNavItem = {
  readonly label: string;
  /** Already a served path: the caller builds it with `href()`. */
  readonly href: string;
  /** The page the reader is on, marked for the eye and for a screen reader. */
  readonly current: boolean;
};

export type DocsNavProps = {
  /** What the row is, said once for whoever cannot see it. */
  readonly label: string;
  readonly items: ReadonlyArray<DocsNavItem>;
};

/**
 * The row of section tabs under the header — the manual's top level, or
 * the landing's. Links, never buttons: every entry is a place with an
 * address, and a reader who middle-clicks one must get a tab.
 *
 * `aria-current="page"` and not only a class, because the mark is
 * information and a colour is not readable aloud.
 */
export const DocsNav = component$<DocsNavProps>((props) => {
  useStyles$(styles);
  return (
    <nav class="docs-nav" aria-label={props.label}>
      <ul class="docs-nav__list">
        {props.items.map((item) => (
          <li key={item.href} class="docs-nav__item">
            <a
              class={
                item.current
                  ? "docs-nav__link docs-nav__link--current"
                  : "docs-nav__link"
              }
              href={item.href}
              {...(item.current ? { "aria-current": "page" as const } : {})}
            >
              {item.label}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
});
