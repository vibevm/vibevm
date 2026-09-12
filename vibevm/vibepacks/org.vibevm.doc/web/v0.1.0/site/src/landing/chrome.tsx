/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import { DocsHeader, Footer } from "@vibe-docs/design";

import { href } from "../lib/href.ts";
import styles from "./chrome.css?inline";
import {
  GITHUB_URL,
  GITVERSE_URL,
  type Locale,
  STRINGS,
  localePath,
  otherLocale,
} from "./i18n.ts";

export type LandingChromeProps = {
  /** Which language's page this is; the chrome speaks it too. */
  readonly locale: Locale;
};

/**
 * The frame around every landing page: the site's header, the page, the
 * site's footer.
 *
 * The header and the footer are the documentation's — the same two
 * components from `design/`, not copies of them (D-28). What differs is
 * what hangs in them, and it differs because the two halves of the site
 * ask different things of a reader: the documentation offers a search
 * box, the landing offers the way in to the documentation, the two
 * source mirrors and the language switch. One shape, two fillings.
 *
 * The chrome is per-locale rather than per-site, which is why it is a
 * component taking a locale and not a layout reading the URL. The
 * Russian page's footer says «в пакетах» and «Олег Чирухин»; those are
 * the owner's words and a chrome that rendered the English ones under a
 * Russian page would quietly drop half the translation.
 */
export const LandingChrome = component$<LandingChromeProps>((props) => {
  useStyles$(styles);
  const t = STRINGS[props.locale];
  const other = otherLocale(props.locale);
  const isEnglish = props.locale === "en";

  return (
    <div class="landing-page">
      <DocsHeader brand="VibeVM" homeHref={href(localePath(props.locale))}>
        <nav class="landing-nav">
          <a class="landing-nav__link" href={href("doc/")}>
            {t.documentation}
          </a>
          <a
            class="landing-nav__link landing-nav__link--wide"
            href={GITHUB_URL}
            rel="noopener"
          >
            GitHub
          </a>
          <a
            class="landing-nav__link landing-nav__link--wide"
            href={GITVERSE_URL}
            rel="noopener"
          >
            GitVerse
          </a>
          <span class="landing-nav__lang">
            <a
              class={
                isEnglish
                  ? "landing-nav__lang-link landing-nav__lang-link--current"
                  : "landing-nav__lang-link"
              }
              href={href(localePath("en"))}
              {...(isEnglish ? { "aria-current": "page" as const } : {})}
            >
              EN
            </a>
            <a
              class={
                isEnglish
                  ? "landing-nav__lang-link"
                  : "landing-nav__lang-link landing-nav__lang-link--current"
              }
              href={href(localePath("ru"))}
              {...(isEnglish ? {} : { "aria-current": "page" as const })}
            >
              RU
            </a>
          </span>
        </nav>
      </DocsHeader>

      <main class="landing-shell">
        <Slot />
      </main>

      <Footer copyright={t.copyright}>
        <div class="landing-footer__brand">VibeVM — {t.footerTagline}</div>
        <div class="landing-footer__links">
          <a href={GITHUB_URL} rel="noopener">
            GitHub
          </a>
          <a href={GITVERSE_URL} rel="noopener">
            GitVerse
          </a>
          <a href={href(localePath(other))}>{other.toUpperCase()}</a>
        </div>
      </Footer>
    </div>
  );
});
