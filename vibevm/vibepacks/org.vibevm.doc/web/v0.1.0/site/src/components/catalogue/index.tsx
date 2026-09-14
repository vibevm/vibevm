/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

import { component$, useVisibleTask$ } from "@qwik.dev/core";
import {
  AuthorshipFilter,
  Card,
  LanguageSelector,
  Prose,
  SectionHead,
  Shelf,
  TabPills,
} from "@vibe-docs/design";

import { SITE } from "../../config.ts";
import { authorshipChoices } from "../../lib/authorship.ts";
import {
  catalogueDoors,
  catalogueShelves,
  shelfLanguageChoices,
  type CatalogueTab,
} from "../../lib/catalogue.ts";
import { BUILT } from "../../lib/library-source.ts";
import { DocBar } from "../doc-bar/index.tsx";
import { startCatalogueReader } from "../../reader/mount.ts";
import { docFileHref } from "../../seo/editions.ts";
import { DOC_GLYPH } from "../../lib/view.ts";

/** The measure a catalogue reads at: cards, not sentences. */
const MEASURE = 960;

/** The name the pills and the panels of this one row share. */
const GROUP = "catalogue";

/** What each shelf is called, and what its order and marks mean. */
const SHELVES: Readonly<
  Record<CatalogueTab, { title: string; caption: string; empty: string }>
> = {
  featured: {
    title: "Featured",
    caption:
      "Where this site asks a new reader to start. Everything else it carries is on the two shelves beside this one.",
    empty: "This build carries none of the documentations named as featured.",
  },
  documents: {
    title: "Documents",
    caption:
      "Every documentation somebody wrote, the featured ones included. A star marks an adaptation the author of the documentation named; each documentation's source is listed first, because it is what officiality is measured against.",
    empty: "This build carries no documentation.",
  },
  projections: {
    title: "Projections",
    caption:
      "Printed by the pipeline out of a package's own bytes, so that every package has something to read. Nobody wrote these pages, and every card here says so.",
    empty: "This build carries no generated documentation.",
  },
};

export type CatalogueProps = {
  /** The language segment this catalogue is at, or `null` for the door. */
  readonly lang: string | null;
};

/**
 * Every documentation this build carries, on the three shelves a reader
 * asked for: what to start with, what was written, and what was printed.
 *
 * The three are not three slices of one list and are not offered as
 * such. **Featured** is the deployment's own answer to «where do I
 * start» and is the only shelf a person chose. **Documents** is every
 * documentation somebody wrote, the featured ones included, because a
 * shelf that hid what it had just recommended would send a reader
 * hunting for half of the answer. **Projections** is what the pipeline
 * printed from packages' own bytes — a different kind of thing, which
 * every card on it says in as many words.
 *
 * Which shelf is open is a query on the address and is remembered, so a
 * reader who found the projections has something to send somebody and
 * comes back to where they were. The build opens the page on the
 * featured shelf, unless this build carries none of them: a door that
 * opened on an empty shelf would answer nobody's question, and a build
 * of a fork or of this package's own fixtures is exactly that case.
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
 * shelves and differ only in which entry is the reader's. Two components
 * would be two lists to keep in step, and the one nobody was looking at
 * would go stale.
 *
 * The door takes one decision, once per session: the language a reader
 * chose before, then the one their browser asks for, then the
 * documentation's own. A language's own catalogue takes none — arriving
 * at it IS the choice.
 */
export const Catalogue = component$<CatalogueProps>((props) => {
  const shelves = catalogueShelves(BUILT, SITE.featured);
  const doors = catalogueDoors(BUILT);
  const atDoor = props.lang === null;
  const lang = props.lang;
  const open: CatalogueTab =
    (shelves[0]?.entries.length ?? 0) > 0 ? "featured" : "documents";

  /* `document-idle` and not the default. The default waits for the
     component's own element to be scrolled into view, and two of the
     three shelves below start `hidden` — an element that is hidden never
     intersects anything, so the whole of the door's behaviour would wait
     for a shelf the reader has not asked for. Nothing here is about a
     region being seen in any case: which shelf is open, which language
     is offered and which door a reader is sent through are all decided
     for the page. */
  useVisibleTask$(() => startCatalogueReader(atDoor ? doors : [], lang), {
    strategy: "document-idle",
  });

  return (
    <Prose measure={MEASURE}>
      <SectionHead
        title="Documentation"
        moreHref={docFileHref("llms.txt")}
        moreLabel="the catalogue for an agent"
      />
      {/* Three controls, three questions. Which shelf to stand at, which
          language of the DOCUMENTATION to be offered on it, and whose
          words are in it — none of them the language the buttons are in,
          which is the switch in the corner of the header above. The two
          filters narrow together: a reader may ask for the Russian half
          of what a person wrote. */}
      <DocBar>
        <TabPills
          label="Catalogue"
          name={GROUP}
          items={shelves.map((shelf) => ({
            label: SHELVES[shelf.tab].title,
            value: shelf.tab,
            current: shelf.tab === open,
          }))}
        />
        <LanguageSelector
          label="Documentation language"
          items={shelfLanguageChoices(BUILT, props.lang)}
        />
        <AuthorshipFilter
          label="Who wrote the prose"
          items={authorshipChoices()}
        />
      </DocBar>
      {shelves.map((shelf) => (
        <div
          key={shelf.tab}
          data-tab-panel={shelf.tab}
          data-tab-group={GROUP}
          hidden={shelf.tab !== open}
        >
          <Shelf
            title={SHELVES[shelf.tab].title}
            caption={SHELVES[shelf.tab].caption}
            emptyLabel={SHELVES[shelf.tab].empty}
            empty={shelf.entries.length === 0}
          >
            {shelf.entries.map((one) => (
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
                editionLang={one.tag}
                generated={one.projection}
                {...(one.authorship === undefined
                  ? {}
                  : { authorship: one.authorship })}
              />
            ))}
          </Shelf>
        </div>
      ))}
    </Prose>
  );
});
