/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import {
  component$,
  useSignal,
  useStyles$,
  useVisibleTask$,
  type Signal,
} from "@qwik.dev/core";
import {
  useLocation,
  type DocumentHead,
  type StaticGenerateHandler,
} from "@qwik.dev/router";
import {
  Card,
  CodeChrome,
  DocsNav,
  Fab,
  FallbackNotice,
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

import { Catalogue } from "../../../components/catalogue/index.tsx";
import { Island } from "../../../components/island/index.tsx";
import { ISLAND_PLACEHOLDER } from "../../../lib/island-placeholder.ts";
import { BUILT } from "../../../lib/library-source.ts";
import { documentationParams } from "../../../lib/pages.ts";
import { startReader } from "../../../reader/mount.ts";
import {
  dressHead,
  readServedPage,
  type ServedPage,
} from "../../../reader/served.ts";
import { latestAliasParams } from "../../../seo/addresses.ts";
import { documentationHead } from "../../../seo/head.ts";
import { IS_LOCAL_READER } from "../../../seo/mode.ts";
import {
  DOC_GLYPH,
  viewOf,
  type PackageView,
  type PageView,
} from "../../../lib/view.ts";
import styles from "./styles.css?inline";

/** The reading measure, until the reader's own setting overrides it. */
const MEASURE = 740;

/** The measure a shelf reads at: cards, not sentences. */
const SHELF_MEASURE = 960;

/**
 * The platforms the switch offers. A fixed row and not a scan of the
 * island: a page is written for the platforms the product runs on, and
 * one that happens to mention only two of them must still let a reader
 * say which they are on — otherwise the switch would appear and vanish
 * with the text.
 */
const PLATFORMS = [
  { label: "Windows", value: "windows", current: true },
  { label: "macOS", value: "macos", current: false },
  { label: "Linux", value: "linux", current: false },
] as const;

const AGENT_LEAD =
  "This page has a machine mirror. The citation carries the version rather than latest, so what an agent quotes does not move under it.";

/**
 * The bilingual notice a fallback page carries, in the two languages
 * that matter: the one the reader asked for and the one they are being
 * handed. Both lines are written out rather than composed, because a
 * sentence assembled from fragments reads like one.
 */
const FALLBACK_ASKED =
  "Эта страница ещё не переведена — вы читаете её на языке источника.";
const FALLBACK_GIVEN =
  "This page has not been adapted yet; you are reading it in the source language.";

/**
 * Nobody moves the reader except the reader.
 *
 * Two mechanisms would otherwise, and both run before any module of this
 * page does, which is why this is a script in the head and not a line in
 * the behaviour. The browser restores the scroll position of a reloaded
 * document; and the router keeps its own copy in `history.state` and
 * scrolls to it from its bootstrap, which is the one that actually moved
 * the page here. The promise the return-to-place button makes is that a
 * reader who comes back to a page lands where the page starts and is
 * OFFERED their place (`##READER-NO-AUTOSCROLL`) — a page that jumps on
 * reload has broken it before the button is rendered.
 *
 * Back and forward are the exception, and the navigation type is how it
 * is told: returning to a page you just left SHOULD land where you were,
 * because that is what «back» means. Only a fresh load or a reload is
 * cleared.
 *
 * The cost is a second inline script for a Content-Security-Policy to
 * carry the hash of, beside the theme's. The cheaper home for these
 * lines is the theme script itself, which already runs first for exactly
 * this kind of reason.
 */
const MANUAL_SCROLL = {
  key: "scroll-restoration",
  /* `dangerouslySetInnerHTML` and not `script`: both put the lines
     inside the element, but the latter also writes them out a second
     time as an attribute of the same tag — the same duplication the
     structured data hit, and here it is a script's source sitting in an
     attribute where a policy cannot account for it. */
  dangerouslySetInnerHTML: [
    "(function(){",
    "var n=performance.getEntriesByType('navigation')[0];",
    "if(n&&n.type==='back_forward')return;",
    "history.scrollRestoration='manual';",
    "var s=history.state;",
    "if(s&&s._qRouterScroll){delete s._qRouterScroll;history.replaceState(s,'');}",
    "})()",
  ].join(""),
} as const;

/** One documentation page: the meta row, the island, the panels over it. */
const DocumentationPage = component$<{ view: PageView }>((props) => {
  const view = props.view;

  useVisibleTask$(() =>
    startReader({
      uri: view.uri,
      md: view.links[0]?.href ?? "",
      xml: view.links[1]?.href ?? "",
      llms: view.links[2]?.href ?? "",
      textLanguage: view.textLanguage,
      addressLanguage: view.address.lang,
      fallback: view.fallback,
      sourceHref: view.canonical,
      mount: view.mount,
    }),
  );

  return (
    <div class="doc-view doc-view--page" lang={view.textLanguage}>
      <DocsNav label="Pages of this documentation" items={view.nav} />
      <Toc label="Contents" rulesLabel="Rules this page cites" />
      <Prose measure={MEASURE}>
        {view.fallback ? (
          <FallbackNotice
            asked={FALLBACK_ASKED}
            given={FALLBACK_GIVEN}
            dismissLabel="ok"
          />
        ) : null}
        <div class="doc-view__switches">
          <VersionSwitch label="Version" items={view.versions} />
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
          audiences={view.audiences}
          readingMinutes={view.readingMinutes}
          links={[...view.links]}
        />
        <CodeChrome>
          <Island html={ISLAND_PLACEHOLDER} />
        </CodeChrome>
        <ForAgent
          title="For an agent"
          lead={AGENT_LEAD}
          uri={view.uri}
          links={[...view.links]}
          copyLabel="copy"
          compact={false}
        />
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
        <ForAgent
          title="For an agent"
          lead={AGENT_LEAD}
          uri={view.uri}
          links={[...view.links]}
          copyLabel="copy"
          compact={true}
        />
      </Fab>
    </div>
  );
});

/** One package: its head, its shelves, its pages. */
const PackagePage = component$<{ view: PackageView }>((props) => {
  const view = props.view;
  return (
    <div class="doc-view" lang={view.textLanguage}>
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
      <Prose measure={SHELF_MEASURE}>
        <Shelf
          title="Documentation"
          caption="A star marks documentation the subject itself points at. The order says the same thing: primary, then official, then community."
          emptyLabel="Nothing documents this subject yet."
          empty={false}
        >
          <Card
            title={view.title}
            href={view.nav[0]?.href ?? view.llms}
            publisher={view.publisher}
            coordinate={view.coordinate}
            {...(view.description === undefined
              ? {}
              : { description: view.description })}
            abstract={view.abstract}
            status={view.status}
            glyph={DOC_GLYPH}
            abstractLabel="what it covers"
          />
        </Shelf>
        <Shelf
          title="Adaptations"
          caption="A star marks an adaptation the author of this documentation named. It means «named by them», not «approved by the subject»."
          emptyLabel="No adaptation of this documentation has been published."
          empty={view.adaptations.length === 0}
        >
          {view.adaptations.map((one) => (
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
              status={one.official ? "official" : "community"}
              glyph={DOC_GLYPH}
              abstractLabel="what it covers"
            />
          ))}
        </Shelf>
        <Shelf
          title="Pages"
          caption="In the order the layer law gives them: text that stands still before text that moves with the product."
          emptyLabel="This documentation has no pages yet."
          empty={view.pages.length === 0}
        >
          {view.pages.map((page) => (
            <Card
              key={page.href}
              title={page.title}
              href={page.href}
              publisher={view.publisher}
              coordinate={view.coordinate}
              {...(page.summary === undefined
                ? {}
                : { abstract: page.summary })}
              status={view.status}
              glyph={DOC_GLYPH}
              abstractLabel="what it covers"
            />
          ))}
        </Shelf>
      </Prose>
    </div>
  );
});

/**
 * The local reader's page: the island the server glued in, dressed in
 * the chrome of the documentation the reader is actually serving.
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
 * The language selector is absent and that is not an omission. A reader
 * is pointed at ONE package, and another language is another package
 * with a manifest of its own — there is nothing to switch to, and a
 * selector with one entry would promise otherwise.
 */
const ServedReader = component$(() => {
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

/**
 * One documentation address, at the shape D-06 gives it.
 *
 * A single catch-all route, not a tree of `[group]/[name]/[version]`
 * folders, because the address has an optional language in front of it
 * and a document path of any depth behind it — a pattern-per-shape route
 * tree would need one folder per possible depth and would still not
 * settle the language. The address map is deterministic instead: the
 * segments are parsed by one function, which is tested, and the same
 * function's inverse is what every link on the site is built with.
 *
 * The same route answers for a page and for the package's own page,
 * because they are the same address one segment apart.
 */
export default component$(() => {
  useStyles$(styles);
  const location = useLocation();
  /* The local reader answers for an address this build never saw. Its
     shell is one template that `vibe doc serve` dresses every page of
     every package in, so the route parameters baked into it name the
     fixture and not the page in the browser's address bar; what the page
     shows comes from the reader instead. The constant is a build-time
     literal, so the branch this build does not take is dropped with it. */
  if (IS_LOCAL_READER) return <ServedReader />;

  const raw = location.params["path"] ?? "";
  const view = viewOf(BUILT, raw);

  if (view === null) {
    return (
      <Prose measure={MEASURE}>
        <p class="doc-view__unaddressed">
          This is not a documentation address: <code>{raw}</code>
        </p>
      </Prose>
    );
  }

  if (view.kind === "catalogue") return <Catalogue lang={view.lang} />;
  return view.kind === "page" ? (
    <DocumentationPage view={view} />
  ) : (
    <PackagePage view={view} />
  );
});

/**
 * What the document's head says about this address.
 *
 * Which of several addresses is canonical, which languages answer for
 * the same page, what a share of it looks like and what a machine is
 * told about it are all one question about one address, and `seo/head.ts`
 * answers it for every shape the catch-all matches. The route adds the
 * one thing that is not about the address at all: the two lines that
 * keep the router from moving the reader.
 */
export const head: DocumentHead = ({ params }) => {
  const view = viewOf(BUILT, params["path"] ?? "");
  if (view === null) return { title: "Not a documentation address" };
  const seo = documentationHead(view, IS_LOCAL_READER);
  if (view.kind !== "page") return seo;
  return { ...seo, scripts: [...(seo.scripts ?? []), MANUAL_SCROLL] };
};

/**
 * Which addresses the static generator writes to disk.
 *
 * It answers from the library rather than from a list, so the build
 * gate's two numbers — addresses declared, pages generated — cannot
 * drift apart by someone editing one of them. The `latest` aliases are
 * the same addresses with one segment changed: they exist because the
 * version switch, a citation without a version and every numbered page's
 * `rel=canonical` all name them (`##SITE-CANONICAL-LATEST`).
 */
export const onStaticGenerate: StaticGenerateHandler = () => {
  return { params: [...documentationParams(), ...latestAliasParams()] };
};
