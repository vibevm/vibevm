/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";
import { Card, Prose, SectionHead, Shelf } from "@vibe-docs/design";

import { packageHref } from "../../lib/href.ts";
import { coordinate, editions } from "../../lib/library.ts";
import { DOC_GLYPH } from "../../lib/view.ts";

/** The measure a catalogue reads at: cards, not sentences. */
const MEASURE = 960;

/**
 * The catalogue: every documentation this build carries, in every
 * language it carries it in.
 *
 * It has no language segment of its own, and that is deliberate: it is
 * the door, and which language a reader walks through it into is their
 * choice — remembered, asked of the browser, and only then decided by
 * the documentation's own language. That decision belongs to the reader
 * behaviour; what this page does is show every door at once, so a reader
 * who lands on the wrong one is never stuck.
 */
export default component$(() => {
  const all = editions();
  return (
    <Prose measure={MEASURE}>
      <SectionHead
        title="Documentation"
        moreHref={packageHref(coordinate(null))}
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
            href={packageHref(coordinate(one.segment))}
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

export const head: DocumentHead = {
  title: "Documentation",
  meta: [
    {
      name: "description",
      content:
        "Every documentation package this site carries, and every language it is published in.",
    },
  ],
};
