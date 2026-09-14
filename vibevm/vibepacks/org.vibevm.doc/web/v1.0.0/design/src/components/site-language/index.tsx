/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One language the SITE can be read through, as the switch offers it. */
export type SiteLanguageChoice = {
  /** The BCP-47 tag the choice records: `en`, `ru`. */
  readonly language: string;
  /** The two letters printed, in that language's own alphabet. */
  readonly label: string;
  /**
   * Where choosing it leads — present only where the language IS a
   * place. The landing is written out in each language at its own
   * address, so there the choice is a move; in the manual it changes
   * the words on the furniture and nothing about which page you are on.
   */
  readonly href?: string;
  readonly current: boolean;
};

export type SiteLanguageSwitchProps = {
  /** What the pair is, for a reader who cannot see two letters. */
  readonly label: string;
  readonly items: ReadonlyArray<SiteLanguageChoice>;
};

/**
 * The language of the SITE, in the corner of every page.
 *
 * It is not the language selector beside it. That one says which edition
 * of a documentation you are reading and leads to another document; this
 * one says which words the buttons carry and leads nowhere — which is
 * why an entry here is a link only where the language is an address, and
 * a button everywhere else. A reader who copies a documentation address
 * must not hand someone else their own choice of interface, for the same
 * reason the platform pills are buttons.
 *
 * It carries no behaviour, like everything else here: each entry says
 * what it is with `data-site-lang-choice`, and `reader/site-language.ts`
 * is what records the choice and puts the chrome into it.
 */
export const SiteLanguageSwitch = component$<SiteLanguageSwitchProps>(
  (props) => {
    useStyles$(styles);
    return (
      <div
        class="site-language"
        role="group"
        aria-label={props.label}
        data-site-language
      >
        {props.items.map((item) =>
          item.href === undefined ? (
            <button
              key={item.language}
              class={
                item.current
                  ? "site-language__choice is-current"
                  : "site-language__choice"
              }
              type="button"
              data-site-lang-choice={item.language}
              aria-pressed={item.current}
            >
              {item.label}
            </button>
          ) : (
            <a
              key={item.language}
              class={
                item.current
                  ? "site-language__choice is-current"
                  : "site-language__choice"
              }
              href={item.href}
              hreflang={item.language}
              data-site-lang-choice={item.language}
              {...(item.current ? { "aria-current": "page" as const } : {})}
            >
              {item.label}
            </a>
          ),
        )}
      </div>
    );
  },
);
