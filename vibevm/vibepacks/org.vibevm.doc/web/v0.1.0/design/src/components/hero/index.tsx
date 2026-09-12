/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One of the two calls to action under the lead. */
export type HeroAction = {
  readonly label: string;
  readonly href: string;
};

export type HeroProps = {
  /** The mono label above the headline, preceded by the accent dot. */
  readonly eyebrow: string;
  /**
   * The headline, as markup: it carries a single `<em>` around the word
   * the design sets in accented italic.
   */
  readonly headlineHtml: string;
  /** The lead, as markup: the mandated descriptor sits in `<strong>`. */
  readonly leadHtml: string;
  /** The filled button — the project's canonical source. */
  readonly primary: HeroAction;
  /** The outlined button beside it — the mirror. */
  readonly secondary: HeroAction;
  /**
   * The status pill: what stage the product is at, in two words.
   *
   * Optional, and absent rather than empty on a page that has no status
   * to report — the 404 uses this same hero for its apology, and a pill
   * saying nothing would still draw the eye to itself.
   */
  readonly badge?: string | undefined;
};

/**
 * The top of the landing: label, headline, lead, two buttons, a status
 * pill, and whatever the page hangs below and beside them.
 *
 * Two slots, because the hero owns the shape and not the contents. The
 * default slot takes the install block, which is copy and commands the
 * design system has no opinion about; the `aside` slot takes the picture
 * — today the dependency graph — which on a narrow screen moves above
 * the copy rather than shrinking beside it.
 *
 * The headline and the lead arrive as markup and are written in with
 * `dangerouslySetInnerHTML`, which is the honest name for what happens
 * and deserves a reason. Both strings carry exactly one inline element
 * — `<em>` for the accent word, `<strong>` for the descriptor that must
 * appear verbatim — and both come from the site's own compiled string
 * table, never from a request, a file on disk or a reader. Splitting
 * them into three props each would move the emphasis out of the sentence
 * and into the layout, where a translator could not see it.
 */
export const Hero = component$<HeroProps>((props) => {
  useStyles$(styles);
  return (
    <section class="hero">
      <div class="hero__copy">
        <p class="hero__eyebrow">
          <span class="hero__dot" aria-hidden="true" />
          {props.eyebrow}
        </p>
        <h1
          class="hero__headline"
          dangerouslySetInnerHTML={props.headlineHtml}
        />
        <p class="hero__lead" dangerouslySetInnerHTML={props.leadHtml} />

        <div class="hero__cta">
          <a
            class="hero__btn hero__btn--primary"
            href={props.primary.href}
            rel="noopener"
          >
            {props.primary.label}
            <svg
              class="hero__arrow"
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M3 11L11 3M11 3H5M11 3V9"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </a>
          <a
            class="hero__btn hero__btn--ghost"
            href={props.secondary.href}
            rel="noopener"
          >
            {props.secondary.label}
          </a>
        </div>

        {props.badge !== undefined && (
          <div class="hero__run">
            <p class="hero__release">
              <span class="hero__release-dot" aria-hidden="true" />
              {props.badge}
            </p>
          </div>
        )}

        <Slot />
      </div>

      <div class="hero__aside">
        <Slot name="aside" />
      </div>
    </section>
  );
});
