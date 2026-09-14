/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$, useStyles$, useVisibleTask$ } from "@qwik.dev/core";
import {
  DocsHeader,
  Footer,
  SearchBox,
  SiteLanguageSwitch,
  ThemeSwitch,
} from "@vibe-docs/design";

import { href } from "../lib/href.ts";
import { SITE_LANGUAGE_LABEL } from "../lib/site-language.ts";
import { findInDocumentation } from "../reader/search.ts";
import { rememberSiteLanguage } from "../reader/site-language.ts";
import { startThemeSwitch } from "../reader/theme.ts";
import styles from "./chrome.css?inline";
import {
  GITHUB_URL,
  GITVERSE_URL,
  LOCALES,
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
 * The theme switch and the search box hang in both, at the same corner
 * and in the same order. A reader who darkened the manual and then
 * walked back out to the front page should not have to find the control
 * again, and a reader who arrives at the front door already knowing what
 * they are looking for should not have to enter the manual first to be
 * allowed to ask for it.
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
  const locale = props.locale;

  /* The two behaviours the landing has. A documentation page starts the
     whole reader and the theme is one of its settings; here there is no
     column to widen and no block numbers to hide, so the switch is
     started on its own, from the same three functions.

     The second is the site's language, and here it is a recording
     rather than a translation: `/` and `/ru/` are the same page written
     out in two languages, so arriving at one of them IS the choice, and
     what it is worth remembering for is the manual — whose furniture is
     one prerendered set of English words until a reader says otherwise.  */
  useVisibleTask$(() => {
    rememberSiteLanguage(locale);
    return startThemeSwitch();
  });

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
          <span class="landing-nav__search">
            <SearchBox
              label={t.search}
              placeholder={t.searchPlaceholder}
              shortcut="Ctrl K"
              emptyLabel={t.searchEmpty}
              find$={findInDocumentation}
            />
          </span>
          <span class="landing-nav__lang">
            <SiteLanguageSwitch
              label={t.siteLanguage}
              items={LOCALES.map((one) => ({
                language: one,
                label: SITE_LANGUAGE_LABEL[one],
                /* Here the language IS a place: the landing is written
                   out in full at `/` and `/ru/`, so the entry is a link
                   and the address is the answer. */
                href: href(localePath(one)),
                current: one === locale,
              }))}
            />
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
