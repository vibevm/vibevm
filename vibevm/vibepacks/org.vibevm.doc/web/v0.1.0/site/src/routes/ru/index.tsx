/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import { component$ } from "@qwik.dev/core";
import { SectionHead } from "@vibe-docs/design";

import { href } from "../../lib/href.ts";

/**
 * The Russian landing — a second route, and that is the claim being
 * made here.
 *
 * The language of the landing is a directory in the route tree, exactly
 * as the language of a documentation page is a segment of its address:
 * one build, one base, two addresses that a crawler and a citation can
 * both hold on to. Its content moves across with the rest of the
 * landing; what this route proves today is that the address exists and
 * the static build generates it.
 */
export default component$(() => {
  return (
    <section class="landing">
      <SectionHead
        title="Документация"
        moreHref={href("")}
        moreLabel="In English"
      />
    </section>
  );
});
