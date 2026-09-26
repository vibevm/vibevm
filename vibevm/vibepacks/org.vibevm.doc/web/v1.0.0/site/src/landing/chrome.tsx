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
import { isNewsPath, newsHref } from "../news/paths.ts";
import { findInDocumentation } from "../reader/search.ts";
import { rememberSiteLanguage } from "../reader/site-language.ts";
import { startThemeSwitch } from "../reader/theme.ts";
import { isVisionPath, visionHref } from "../vision/paths.ts";
import { type WhyPage, whyHref, whyPageOf, whyPath } from "../why/paths.ts";
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
  /**
   * Which page of this language the chrome is standing over, as its path
   * inside the locale — `""` for the landing, `"why/zap/"` for a Why
   * page. Two things read it: the entry that marks itself current, and
   * the language switch, which offers the other language's spelling of
   * THIS page rather than its front door.
   */
  readonly path: string;
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
  const here = props.path;

  /* The three Why pages, as the header and the footer both list them —
     one array rather than two lists that would drift the first time a
     fourth product joins the family. The label travels with the slug so
     that adding a page is one entry and not three edits. */
  const why: readonly { page: WhyPage; label: string }[] = [
    { page: "vibevm", label: t.navWhyVibevm },
    { page: "zap", label: t.navWhyZap },
    { page: "ai-native", label: t.navWhyAiNative },
  ];

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
        {/* The bar is TWO rows and says so.
            ---------------------------------------------------------
            It used to be one list of nine things with `flex-wrap` under
            it, which is not a composition but a permission: the row
            filled up, the last item fell through, and what landed on the
            second line was whatever happened to be last in the file —
            the theme switch, alone, under the brand, outside a bar that
            is 64px tall and therefore did not even contain it. Nothing
            chose that. This does.

            The first row is the software, where it is kept and where it
            is spoken about; the second is the argument for it. Each row
            is an element, so what stands on which line is a fact about
            the markup rather than about how wide the reader's window
            happened to be — and each row is paired with the control that
            belongs beside it, which is what keeps the pairing true at
            every width instead of only at the one that was measured. */}
        <nav class="landing-nav">
          <div class="landing-nav__row landing-nav__row--tools">
            <a class="landing-nav__link" href={href("doc/")}>
              {t.documentation}
            </a>
            <a class="landing-nav__link" href={GITHUB_URL} rel="noopener">
              GitHub
            </a>
            <a class="landing-nav__link" href={GITVERSE_URL} rel="noopener">
              GitVerse
            </a>
            {/* And, at the end of the row, where the project is spoken.
                It stands last because it is the only entry of this row
                that leads back into the site rather than out of it —
                after the manual and the two mirrors, the place to ask
                about them. A reader who has run out of things to read is
                exactly the reader who needs it. */}
            <a
              class="landing-nav__link"
              href={newsHref(locale)}
              {...(isNewsPath(here) ? { "aria-current": "page" as const } : {})}
            >
              {t.navNews}
            </a>
          </div>
          <div class="landing-nav__row landing-nav__row--story">
            {/* The essay stands first and the three product arguments
                after it: it is the worldview they are pieces of, and a
                reader who wants the whole picture should not have to
                find it through one of the parts. */}
            <a
              class="landing-nav__link"
              href={visionHref(locale)}
              {...(isVisionPath(here)
                ? { "aria-current": "page" as const }
                : {})}
            >
              {t.navVision}
            </a>
            {why.map((one) => (
              <a
                key={one.page}
                class="landing-nav__link"
                href={whyHref(one.page, locale)}
                {...(here === whyPath(one.page)
                  ? { "aria-current": "page" as const }
                  : {})}
              >
                {one.label}
              </a>
            ))}
          </div>
        </nav>

        {/* The search stands over the first row, because that row opens
            with the documentation and the two mirrors of its source, and
            this field searches exactly that. */}
        <span class="landing-nav__search">
          <SearchBox
            label={t.search}
            placeholder={t.searchPlaceholder}
            shortcut="Ctrl K"
            emptyLabel={t.searchEmpty}
            find$={findInDocumentation}
          />
        </span>

        {/* And the two preferences stand together under it, at the end
            of the second row. Which language the furniture speaks and
            which map of the palette it is drawn in are the same kind of
            answer — a fact about the reader rather than about the page —
            and the manual's header already carries them in this order,
            behind the same field. What changes here is only that the
            order is wrapped onto two lines deliberately; the corner, the
            sequence and the tab order a reader learns in one half of the
            site are the ones they find in the other. */}
        <div class="landing-nav__prefs">
          <span class="landing-nav__lang">
            <SiteLanguageSwitch
              label={t.siteLanguage}
              items={LOCALES.map((one) => ({
                language: one,
                label: SITE_LANGUAGE_LABEL[one],
                /* Here the language IS a place: the landing and each Why
                   page are written out in full in both languages, so the
                   entry is a link and the address is the answer — and
                   the address it names is THIS page's, not the front
                   door's. A switch that always pointed home would cost a
                   reader their place for the price of a translation they
                   already have. */
                href: href(`${localePath(one)}${here}`),
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
        </div>
      </DocsHeader>

      {/* The landing is a composition inside a column and the Why pages
          are long-form editorial that bleeds to the window: a frieze
          spanning the full width, an inverted band the page is cut in
          half by, a deep-space ground behind the whole of it. So the
          measure belongs to the page, and this asks the one question it
          can answer from the address — the same address it already read
          the language and the current entry off. A Why page brings its
          own `.why-shell` to every section that wants the column back. */}
      <main
        class={
          whyPageOf(here) === null && !isVisionPath(here)
            ? "landing-shell"
            : "landing-full"
        }
      >
        <Slot />
      </main>

      <Footer copyright={t.copyright}>
        <div class="landing-footer__brand">VibeVM — {t.footerTagline}</div>
        <div class="landing-footer__links">
          {why.map((one) => (
            <a key={one.page} href={whyHref(one.page, locale)}>
              {one.label}
            </a>
          ))}
          <a href={visionHref(locale)}>{t.navVision}</a>
          <a href={GITHUB_URL} rel="noopener">
            GitHub
          </a>
          <a href={GITVERSE_URL} rel="noopener">
            GitVerse
          </a>
          <a href={href(`${localePath(other)}${here}`)}>
            {other.toUpperCase()}
          </a>
        </div>
      </Footer>
    </div>
  );
});
