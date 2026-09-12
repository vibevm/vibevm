/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

import { component$, useVisibleTask$ } from "@qwik.dev/core";
import { Card, Prose, SectionHead, Shelf } from "@vibe-docs/design";

import { catalogueHref, packageHref } from "../../lib/href.ts";
import { coordinate, editions } from "../../lib/library.ts";
import { BUILT } from "../../lib/library-source.ts";
import { startCatalogueReader } from "../../reader/mount.ts";
import { DOC_GLYPH } from "../../lib/view.ts";

/** The measure a catalogue reads at: cards, not sentences. */
const MEASURE = 960;

export type CatalogueProps = {
  /** The language segment this catalogue is at, or `null` for the door. */
  readonly lang: string | null;
};

/**
 * Every documentation this build carries, and every language it carries
 * it in.
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
  const all = editions(BUILT);
  const doors = all.map((one) => ({
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
        moreHref={packageHref(coordinate(BUILT, props.lang))}
        moreLabel="the package"
      />
      <Shelf
        title="Editions"
        caption="A star marks an adaptation the author of the documentation named. The source is listed first; it is what officiality is measured against."
        emptyLabel="This build carries no documentation."
        empty={all.length === 0}
      >
        {all.map((one) => (
          <Card
            key={one.tag}
            title={one.title}
            href={packageHref(coordinate(BUILT, one.segment))}
            publisher={one.publisher}
            coordinate={`${one.card.group}/${one.card.name}@${one.card.version}`}
            {...(one.card.description === undefined
              ? {}
              : { description: one.card.description })}
            abstract={one.card.abstract}
            status={
              one.segment === null
                ? one.card.status
                : one.official
                  ? "official"
                  : "community"
            }
            glyph={DOC_GLYPH}
            abstractLabel="what it covers"
          />
        ))}
      </Shelf>
    </Prose>
  );
});
