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
  /**
   * One quieter paragraph directly under the lead, as markup — what the
   * thing being introduced is properly called, or whatever else a reader
   * should have before the buttons and not instead of the lead.
   *
   * Optional and omitted rather than empty: the 404 shares this hero and
   * has nothing to add under its apology, and a note set in the muted
   * role with nothing in it would still cost the lead its spacing.
   */
  readonly noteHtml?: string | undefined;
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
  /**
   * Where the pill leads, when the stage it names has somewhere to be
   * read about: the plan this release is a point on.
   *
   * Optional, because a pill is a statement before it is a control — the
   * 404's hero has one and nothing to say about it — and a badge that led
   * nowhere would still take a reader's click.
   */
  readonly badgeHref?: string | undefined;
  /**
   * The one link beside the pill: where a reader who has just read what
   * stage this is goes to find out what the thing is.
   *
   * Beside the pill and not among the two buttons above, because it is
   * not a third call to action — those two lead to the source, this one
   * leads to a page of the manual — and the row the pill stands in has
   * the room. Optional, for the pages that have a pill and nowhere in
   * particular to send anybody.
   */
  readonly badgeAction?: HeroAction | undefined;
};

/**
 * The top of the landing: label, headline, lead, an optional quieter note
 * under it, two buttons, a status pill with one link beside it, and
 * whatever the page hangs below and beside them.
 *
 * Two slots, because the hero owns the shape and not the contents. The
 * default slot takes the install block, which is copy and commands the
 * design system has no opinion about; the `aside` slot takes the picture
 * — today the dependency graph — which on a narrow screen moves above
 * the copy rather than shrinking beside it.
 *
 * The headline, the lead and the note arrive as markup and are written in
 * with `dangerouslySetInnerHTML`, which is the honest name for what
 * happens and deserves a reason. Each string carries a few inline
 * elements and nothing else — `<em>` for the accent word, `<strong>` for
 * the descriptor that must appear verbatim, an `<a>` around a word that
 * is also a place — and all of them come from the site's own compiled
 * string table, never from a request, a file on disk or a reader.
 * Splitting them into three props each would move the emphasis out of the
 * sentence and into the layout, where a translator could not see it.
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
        {props.noteHtml === undefined ? null : (
          <p class="hero__note" dangerouslySetInnerHTML={props.noteHtml} />
        )}

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
            {/* The pill is a link when the release has a plan behind it,
                and the same pill either way: what changes is that it can
                be pressed, which it says with its own hover and focus
                rather than by looking like a second button. */}
            {props.badgeHref === undefined ? (
              <p class="hero__release">
                <span class="hero__release-dot" aria-hidden="true" />
                {props.badge}
              </p>
            ) : (
              <a
                class="hero__release hero__release--link"
                href={props.badgeHref}
                data-hero-release
              >
                <span class="hero__release-dot" aria-hidden="true" />
                {props.badge}
              </a>
            )}
            {props.badgeAction === undefined ? null : (
              <a
                class="hero__btn hero__btn--ghost hero__btn--compact"
                href={props.badgeAction.href}
                data-hero-guide
              >
                {props.badgeAction.label}
                <svg
                  class="hero__arrow"
                  width="14"
                  height="14"
                  viewBox="0 0 14 14"
                  fill="none"
                  aria-hidden="true"
                >
                  <path
                    d="M2.5 7H11.5M11.5 7L8 3.5M11.5 7L8 10.5"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </a>
            )}
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
