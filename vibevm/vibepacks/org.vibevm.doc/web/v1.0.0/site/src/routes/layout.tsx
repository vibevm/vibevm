/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$, useVisibleTask$ } from "@qwik.dev/core";
import { useLocation } from "@qwik.dev/router";
import {
  DocsHeader,
  Footer,
  SearchBox,
  SiteLanguageSwitch,
  ThemeSwitch,
} from "@vibe-docs/design";

import { docSegments, href, parseDocTarget } from "../lib/href.ts";
import { SITE_LANGUAGES, SITE_LANGUAGE_LABEL } from "../lib/site-language.ts";
import { findInDocumentation } from "../reader/search.ts";
import { startSiteLanguage } from "../reader/site-language.ts";
import { startThemeSwitch } from "../reader/theme.ts";

/**
 * The chrome of the public site — the header and the footer the
 * documentation wears.
 *
 * It belongs to the static build only, and that is the whole point of
 * where the file sits: the embedded adapter's route directory is
 * `src/routes/doc`, so this layout is simply not in its tree. The shell
 * `vibe` embeds carries no site header, no site search and no site
 * footer, because none of them mean anything inside an editor — and that
 * exclusion needs no flag, no condition and no second component.
 *
 * **The landing steps out of it here, and that IS a condition.** It
 * should not have to be: a landing address is `index@landing.tsx`, which
 * names `layout-landing!.tsx`, and the `!` means «this layout is the
 * top» — the chain above it does not run. The pinned beta does not
 * honour it. It resolves the named layout and then renders this one
 * around it anyway, and it says so on every build: «the "top" layout
 * feature … has been deprecated». The output was two headers, two
 * footers, two `<main>` elements and two fields carrying the id
 * `search-box-input` on every landing page — one set painting exactly
 * over the other, which is why it went unseen until the search box
 * grew a behaviour and the duplicate id started to matter.
 *
 * So this layout asks the one question it can answer without knowing
 * anything about the landing: is this address the documentation's. The
 * question is `lib/href.ts`'s, not a path written out here, and the
 * condition goes away the day the router's own mechanism works — or the
 * day the route tree is rearranged around the group feature the
 * deprecation notice points at.
 *
 * The child route arrives through `<Slot />`, never through
 * `<RouterOutlet />`. The outlet belongs to the document root and means
 * «the matched route tree goes here»; a layout is already inside that
 * tree, so an outlet within one asks the router to render the tree
 * inside itself. The beta does exactly that, forever, in a synchronous
 * loop that produces no error and no output — the page simply never
 * finishes rendering.
 *
 * **The corner of the header is the SITE's language and no longer the
 * documentation's.** They were one control, so a reader who wanted the
 * buttons in Russian had to move the manual into Russian as well, and a
 * reader of the Russian manual could not have the buttons in English at
 * all. Which edition of a text you are reading is a fact about the text
 * and lives in the address; what the furniture says is a preference and
 * lives with the theme. The documentation's language is offered where it
 * is about something — on the shelf it narrows and on the page it moves
 * you to.
 */
export default component$(() => {
  const location = useLocation();
  const segments = docSegments(location.url.pathname);
  const doc = segments !== null;
  /* A document page starts the whole reader, and the reading settings
     are one of the two callers of `reader/theme.ts`; every other address
     under the documentation — the catalogue, a package's own page —
     starts nothing at all and would carry a switch that did nothing. So
     the layout asks which of the two this address is, with the same
     parser every link on the site is built with, and starts the switch
     only where nothing else will. Two listeners for one control would
     both work and would be two answers to one question. */
  const read = segments !== null && parseDocTarget(segments)?.kind === "page";

  /* The reader's own language for the furniture, applied on arrival.
     The landing does the other half: its two addresses ARE the two
     languages, so it records which door was used rather than
     translating a page that is already written out twice. */
  useVisibleTask$(() => {
    if (!doc) return undefined;
    const stops = [startSiteLanguage(), ...(read ? [] : [startThemeSwitch()])];
    return () => {
      for (const stop of stops) stop();
    };
  });

  if (!doc) return <Slot />;
  return (
    <>
      <DocsHeader brand="VibeVM" homeHref={href("")}>
        <SearchBox
          label="Search the documentation"
          placeholder="Search"
          shortcut="Ctrl K"
          emptyLabel="Nothing here carries that word."
          find$={findInDocumentation}
        />
        <SiteLanguageSwitch
          label="Site language"
          items={SITE_LANGUAGES.map((language) => ({
            language,
            label: SITE_LANGUAGE_LABEL[language],
            /* No address: inside the manual the interface language
               changes the words and never the page, so a reader who
               copies this address hands over a document and not their
               own preferences. The mark is put on by the behaviour,
               which is the only thing that knows what was stored. */
            current: false,
          }))}
        />
        {/* The site's one theme switch, in the same corner and the same
            order as the landing's. It used to be the landing's alone,
            and the reading panel's on a document page — two controls for
            one choice, and none at all on the catalogue and on a
            package's own page. */}
        <ThemeSwitch
          label="Theme"
          lightLabel="Light"
          darkLabel="Dark"
          systemLabel="System"
          compact={true}
        />
      </DocsHeader>
      <main>
        <Slot />
      </main>
      <Footer copyright="© 2026 Oleg Chirukhin" />
    </>
  );
});
