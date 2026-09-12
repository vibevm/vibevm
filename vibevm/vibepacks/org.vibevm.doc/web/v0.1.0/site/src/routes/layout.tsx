/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$ } from "@qwik.dev/core";
import { DocsHeader, Footer, SearchBox } from "@vibe-docs/design";

import { href } from "../lib/href.ts";

/**
 * The chrome of the public site — the header and the footer the landing
 * and the documentation share.
 *
 * It belongs to the static build only, and that is the whole point of
 * where the file sits: the embedded adapter's route directory is
 * `src/routes/doc`, so this layout is simply not in its tree. The shell
 * `vibe` embeds carries no site header, no site search and no site
 * footer, because none of them mean anything inside an editor — and the
 * exclusion needs no flag, no condition and no second component.
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
  return (
    <>
      <DocsHeader brand="VibeVM" homeHref={href("")}>
        <SearchBox
          label="Search the documentation"
          placeholder="Search"
          shortcut="Ctrl K"
        />
      </DocsHeader>
      <main>
        <Slot />
      </main>
      <Footer copyright="© 2026 Oleg Chirukhin" />
    </>
  );
});
