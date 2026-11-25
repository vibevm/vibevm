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
 * The site's top bar: the mark, the brand, then whatever the page puts
 * beside it — a language pill, the search box, the theme switch. The slot
 * is the whole point: the header knows the shape of the bar and nothing
 * about what the documentation or the landing chooses to hang in it, so
 * both use one header instead of two that drift (D-28).
 *
 * The mark stands here rather than in either half of the site for the
 * same reason: it is the same drawing at the same size beside the same
 * word on the landing and on every page of the manual, and a copy in the
 * other header would be the pair that drifts.
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
          <Mark />
          <b class="docs-header__name">{props.brand}</b>
        </a>
        <div class="docs-header__actions">
          <Slot />
        </div>
      </div>
    </header>
  );
});

/**
 * The mark: a hub with four nodes around it, drawn and never fetched.
 *
 * It is the drawing the first vibevm.org carried beside the name, at the
 * size it carried it — twenty-six pixels on a thirty-two-unit grid — and
 * it is written out here as paths instead of pointing at the favicon
 * file it was also published as. A header that went to an address for
 * its own name would be a header that can be half-drawn, and a picture
 * on the page is a request a policy has to account for (R-09, D-20);
 * inline, it is part of the first byte the reader receives.
 *
 * Every stroke is `currentColor`, so the mark is the one colour the
 * stylesheet gives it and it follows the theme the reader chose rather
 * than carrying a hex of its own. The four outer nodes are filled with
 * the page's background instead of a colour, which is what makes each
 * spoke stop at its node in both maps of the palette — in the original
 * that fill was the dark page's own ink, written out as a literal.
 */
const Mark = component$(() => (
  <svg
    class="docs-header__mark"
    viewBox="0 0 32 32"
    width="26"
    height="26"
    fill="none"
    aria-hidden="true"
  >
    <g stroke="currentColor" stroke-width="2" opacity="0.55">
      <line x1="16" y1="16" x2="7" y2="7" />
      <line x1="16" y1="16" x2="26" y2="9" />
      <line x1="16" y1="16" x2="9" y2="25" />
      <line x1="16" y1="16" x2="25" y2="24" />
    </g>
    <circle cx="16" cy="16" r="5" fill="currentColor" />
    <g fill="var(--bg)" stroke="currentColor" stroke-width="2">
      <circle cx="7" cy="7" r="3" />
      <circle cx="26" cy="9" r="3" />
      <circle cx="9" cy="25" r="3" />
      <circle cx="25" cy="24" r="3" />
    </g>
  </svg>
));
