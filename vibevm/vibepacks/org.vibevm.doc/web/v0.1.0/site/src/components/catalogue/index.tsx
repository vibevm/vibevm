/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

import { component$, useVisibleTask$ } from "@qwik.dev/core";
import { Card, Prose, SectionHead, Shelf } from "@vibe-docs/design";

import { catalogueHref } from "../../lib/href.ts";
import { siteLanguages } from "../../lib/library.ts";
import { BUILT } from "../../lib/library-source.ts";
import { startCatalogueReader } from "../../reader/mount.ts";
import { docFileHref } from "../../seo/editions.ts";
import { catalogueEntries, DOC_GLYPH } from "../../lib/view.ts";

/** The measure a catalogue reads at: cards, not sentences. */
const MEASURE = 960;

export type CatalogueProps = {
  /** The language segment this catalogue is at, or `null` for the door. */
  readonly lang: string | null;
};

/**
 * Every documentation this build carries, and every language it carries
 * each of them in.
 *
 * One card per EDITION and not per library: a reader looking for Russian
 * is looking for a text they can read, and a shelf that folded the
 * languages of a documentation into one row would answer that with a
 * shrug. The order is the libraries' own, and inside each the source
 * stands before its starred adaptations before the community's — the
 * three signals D-19 asks to agree.
 *
 * One component for both addresses the catalogue has — the door with no
 * language in it, and each language's own — because they show the same
 * shelf and differ only in which entry is the reader's. Two components would be
 * two lists to keep in step, and the one nobody was looking at would go
 * stale.
 *
 * The door takes one decision, once per session: the language a reader
 * chose before, then the one their browser asks for, then the
 * documentation's own. A language's own catalogue takes none — arriving
 * at it IS the choice.
 */
export const Catalogue = component$<CatalogueProps>((props) => {
  const all = catalogueEntries(BUILT);
  const doors = siteLanguages(BUILT).map((one) => ({
    tag: one.tag,
    href: catalogueHref(one.segment),
    source: one.segment === null,
  }));
  const atDoor = props.lang === null;

  useVisibleTask$(() => startCatalogueReader(atDoor ? doors : []));

  return (
    <Prose measure={MEASURE}>
      <SectionHead
        title="Documentation"
        moreHref={docFileHref("llms.txt")}
        moreLabel="the catalogue for an agent"
      />
      <Shelf
        title="Editions"
        caption="A star marks an adaptation the author of the documentation named. Each documentation's source is listed first; it is what officiality is measured against."
        emptyLabel="This build carries no documentation."
        empty={all.length === 0}
      >
        {all.map((one) => (
          <Card
            key={one.coordinate}
            title={one.title}
            href={one.href}
            publisher={one.publisher}
            coordinate={one.coordinate}
            {...(one.description === undefined
              ? {}
              : { description: one.description })}
            abstract={one.abstract}
            status={one.status}
            glyph={DOC_GLYPH}
            abstractLabel="what it covers"
          />
        ))}
      </Shelf>
    </Prose>
  );
});
