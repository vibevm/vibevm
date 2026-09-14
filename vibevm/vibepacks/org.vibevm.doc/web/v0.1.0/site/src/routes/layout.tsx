/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$ } from "@qwik.dev/core";
import { useLocation } from "@qwik.dev/router";
import {
  DocsHeader,
  Footer,
  LanguageSelector,
  SearchBox,
} from "@vibe-docs/design";

import { docSegments, href } from "../lib/href.ts";
import { BUILT } from "../lib/library-source.ts";
import { findInDocumentation } from "../reader/search.ts";
import { headerLanguageChoices } from "../lib/view.ts";

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
 */
export default component$(() => {
  const location = useLocation();
  if (docSegments(location.url.pathname) === null) return <Slot />;
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
        <LanguageSelector
          label="Language"
          items={headerLanguageChoices(BUILT, location.url.pathname)}
        />
      </DocsHeader>
      <main>
        <Slot />
      </main>
      <Footer copyright="© 2026 Oleg Chirukhin" />
    </>
  );
});
