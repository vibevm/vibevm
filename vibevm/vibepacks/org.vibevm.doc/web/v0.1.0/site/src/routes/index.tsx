/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import { DocCard, SectionHead } from "@vibe-docs/design";

import { docHref } from "../lib/href.ts";
import { documentationPages } from "../lib/pages.ts";

/**
 * The site root — a placeholder, and honestly labelled as one.
 *
 * The landing that belongs here is the one the Astro site serves today,
 * moved across one line at a time with a parity test behind it; that is
 * its own atom and its own careful work, and guessing at it now would
 * make that move a rewrite. What this route does carry is the part the
 * documentation needs immediately: a way into the pages, built from the
 * manifest, so the static build has a landing route to prerender and the
 * page count has something to be wrong about.
 */
export default component$(() => {
  const pages = documentationPages();
  return (
    <section class="landing">
      <SectionHead
        title="Documentation"
        moreHref={docHref(pages[0] ?? EMPTY)}
        moreLabel="All pages"
      />
      <div class="landing__grid">
        {pages.map((page) => (
          <DocCard
            key={page.document}
            title={page.document}
            href={docHref(page)}
            publisher={page.group}
            summary={`${page.name} ${page.version}`}
            status="community"
          />
        ))}
      </div>
    </section>
  );
});

/** A coordinate that exists so an empty manifest still renders a page. */
const EMPTY = {
  lang: null,
  group: "org.vibevm.core",
  name: "vibevm-docs",
  version: "latest",
  document: "start",
} as const;
