/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$, useStyles$, useVisibleTask$ } from "@qwik.dev/core";
import { DocsHeader, Footer, ThemeSwitch } from "@vibe-docs/design";

import { href } from "../lib/href.ts";
import { startThemeSwitch } from "../reader/theme.ts";
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
 * ask different things of a reader: the landing offers the way in to the
 * documentation and the two source mirrors, which a page already inside
 * the manual has no use for. One shape, two fillings.
 *
 * The theme switch hangs in both, at the same corner. A reader who
 * darkened the manual and then walked back out to the front page should
 * not have to find the control again, and should certainly not find the
 * page light.
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

  /* The one behaviour the landing has. A documentation page starts the
     whole reader and the theme is one of its settings; here there is no
     column to widen and no block numbers to hide, so the switch is
     started on its own, from the same three functions. */
  useVisibleTask$(() => startThemeSwitch());

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
          <ThemeSwitch
            label={t.theme}
            lightLabel={t.themeLight}
            darkLabel={t.themeDark}
            systemLabel={t.themeSystem}
            compact={true}
          />
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
