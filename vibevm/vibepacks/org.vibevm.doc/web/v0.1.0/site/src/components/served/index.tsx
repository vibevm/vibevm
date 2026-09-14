/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE */

import {
  component$,
  useSignal,
  useVisibleTask$,
  type Signal,
} from "@qwik.dev/core";
import {
  Card,
  CodeChrome,
  DocsNav,
  Fab,
  ForAgent,
  Lightbox,
  PackageHeader,
  PageMeta,
  Prose,
  ReturnToPlace,
  RulePanel,
  SettingsPanel,
  Shelf,
  TabPills,
  Toc,
  VersionSwitch,
} from "@vibe-docs/design";

import { ISLAND_PLACEHOLDER } from "../../lib/island-placeholder.ts";
import { AGENT_LEAD, MEASURE, PLATFORMS } from "../../lib/reading.ts";
import { DOC_GLYPH } from "../../lib/view.ts";
import { startReader } from "../../reader/mount.ts";
import {
  dressHead,
  readServedPage,
  type ServedPage,
} from "../../reader/served.ts";
import { Island } from "../island/index.tsx";

/**
 * The local reader's page: the island the server glued in, dressed in
 * the chrome of the documentation the reader is actually serving.
 *
 * It is a file of its own and not a branch of the route, because it is
 * a different world: the route resolves an address out of a library
 * baked into the build, and this resolves nothing — it asks a server on
 * the reader's own machine what page it just served. The two share the
 * furniture and no logic at all, which is exactly the split that makes
 * the route's own half readable.
 *
 * The shell `vibe` carries is one prerendered template, built against
 * this package's fixture library, so everything it says about a package
 * is the fixture's until it asks. It asks once, at the start of the
 * route, for `<base>manifest.json` — which the reader already
 * publishes — and the navigation, the meta row, the version switch, the
 * projections and the «for an agent» block are then this documentation's.
 *
 * The island is not in that conversation. It arrived finished in the page
 * the server dressed, and nothing here re-renders it: the signal is read
 * by the three small components around it and never by this one, so the
 * region holding the island is rendered once, on the server, and left
 * alone. That is what closes the trap in the serialised state, which
 * still carries the island marker as a plain string — a client re-render
 * would find the marker and show an empty page (finding P4-O4, A-4).
 * Links between pages are ordinary links for the same reason: the server
 * renders the page that was asked for, per request (`##LOCAL-SERVE`).
 *
 * Neither language control is here, and neither is an omission. A reader
 * is pointed at ONE package, and another language is another package
 * with a manifest of its own — there is nothing to switch to, and a
 * selector with one entry would promise otherwise. The site's own
 * language is the other half of the same fact: this shell is inside an
 * editor, where the furniture is the editor's business and not the
 * site's.
 */
export const ServedReader = component$(() => {
  const served = useSignal<ServedPage | null>(null);

  useVisibleTask$(({ cleanup }) => {
    let stop: (() => void) | undefined = undefined;
    void readServedPage().then((page) => {
      served.value = page;
      if (page === null) return;
      dressHead(page);
      if (page.view.kind !== "page") return;
      const view = page.view;
      stop = startReader({
        uri: view.uri,
        md: view.links[0]?.href ?? "",
        xml: view.links[1]?.href ?? "",
        llms: view.links[2]?.href ?? "",
        textLanguage: view.textLanguage,
        addressLanguage: view.address.lang,
        fallback: view.fallback,
        sourceHref: view.canonical,
        mount: view.mount,
      });
    });
    cleanup(() => stop?.());
  });

  return (
    <div class="doc-view doc-view--page">
      <ServedNav served={served} />
      <Toc label="Contents" rulesLabel="Rules this page cites" />
      <Prose measure={MEASURE}>
        <ServedHead served={served} />
        <CodeChrome>
          <Island html={ISLAND_PLACEHOLDER} />
        </CodeChrome>
        <ServedAgent served={served} compact={false} />
      </Prose>
      <RulePanel
        label="The rule this page quotes"
        copyLabel="copy address"
        openLabel="open the rule"
        closeLabel="close"
      />
      <SettingsPanel label="Reading settings" />
      <ReturnToPlace label="back to where you were" />
      <Lightbox label="The picture or table you opened" closeLabel="close" />
      <Fab label="For an agent: the address of this place" glyph="{ }">
        <ServedAgent served={served} compact={true} />
      </Fab>
    </div>
  );
});

/**
 * The navigation of the documentation being served.
 *
 * Empty until the manifest is in, and empty is the right thing to show
 * meanwhile: the alternative is the fixture's list of pages, which names
 * documents this reader does not carry and addresses that answer 404.
 */
const ServedNav = component$<{ served: Signal<ServedPage | null> }>((props) => {
  const view = props.served.value?.view;
  const items = view === undefined || view.kind === "catalogue" ? [] : view.nav;
  return <DocsNav label="Pages of this documentation" items={[...items]} />;
});

/**
 * What stands above the island: the switches and the meta row of a page,
 * or the card and the shelves of a package.
 *
 * The package's card shows the drawn placeholder rather than the
 * pictures the manifest names. The addresses are there — `media` is read
 * out of the served manifest like every other value — but the reader
 * answers `<base>media/<name>` with a 404: the routes it serves are the
 * machine files, the pages, their projections and the shell's own
 * assets, and a package's pictures are none of those. Showing an address
 * that does not answer would put a broken image where a placeholder
 * belongs; the missing half is a route in the server, not a line here.
 */
const ServedHead = component$<{ served: Signal<ServedPage | null> }>(
  (props) => {
    const page = props.served.value;
    if (page === null) return null;
    const view = page.view;
    if (view.kind === "catalogue") return null;
    if (view.kind === "package") {
      return (
        <>
          <PackageHeader
            title={view.title}
            publisher={view.publisher}
            coordinate={view.coordinate}
            {...(view.description === undefined
              ? {}
              : { description: view.description })}
            glyph={DOC_GLYPH}
          >
            <a class="doc-package__llms" href={view.llms}>
              llms.txt
            </a>
          </PackageHeader>
          <Shelf
            title="Pages"
            caption="In the order the layer law gives them: text that stands still before text that moves with the product."
            emptyLabel="This documentation has no pages yet."
            empty={view.pages.length === 0}
          >
            {view.pages.map((one) => (
              <Card
                key={one.href}
                title={one.title}
                href={one.href}
                publisher={view.publisher}
                coordinate={view.coordinate}
                {...(one.summary === undefined
                  ? {}
                  : { abstract: one.summary })}
                status={view.status}
                glyph={DOC_GLYPH}
                abstractLabel="what it covers"
              />
            ))}
          </Shelf>
        </>
      );
    }
    return (
      <>
        <div class="doc-view__switches">
          <VersionSwitch label="Version" items={[...view.versions]} />
          <TabPills label="Platform" items={PLATFORMS} />
        </div>
        <PageMeta
          publisher={view.publisher}
          version={view.version}
          latest={view.latest}
          renderedAt={view.renderedAt}
          {...(view.reviewedAt === undefined
            ? {}
            : { reviewedAt: view.reviewedAt })}
          {...(view.adapts === undefined ? {} : { adapts: view.adapts })}
          audiences={[...view.audiences]}
          readingMinutes={view.readingMinutes}
          links={[...view.links]}
        />
      </>
    );
  },
);

/** The citation and the projections, once the reader has said what page this is. */
const ServedAgent = component$<{
  served: Signal<ServedPage | null>;
  compact: boolean;
}>((props) => {
  const view = props.served.value?.view;
  if (view === undefined || view.kind !== "page") return null;
  return (
    <ForAgent
      title="For an agent"
      lead={AGENT_LEAD}
      uri={view.uri}
      links={[...view.links]}
      copyLabel="copy"
      compact={props.compact}
    />
  );
});
