/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import { component$, useStyles$, useVisibleTask$ } from "@qwik.dev/core";
import {
  useLocation,
  type DocumentHead,
  type StaticGenerateHandler,
} from "@qwik.dev/router";
import {
  Card,
  DocsNav,
  Fab,
  ForAgent,
  PackageHeader,
  PageMeta,
  Prose,
  ReturnToPlace,
  RulePanel,
  SettingsPanel,
  Shelf,
  TabPills,
  VersionSwitch,
} from "@vibe-docs/design";

import { Island } from "../../../components/island/index.tsx";
import { ISLAND_HTML } from "../../../lib/island-source.ts";
import { documentationParams } from "../../../lib/pages.ts";
import { startReader } from "../../../reader/mount.ts";
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
  script: [
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
    }),
  );

  return (
    <div class="doc-view" lang={view.textLanguage}>
      <DocsNav label="Pages of this documentation" items={view.nav} />
      <Prose measure={MEASURE}>
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
        <Island html={ISLAND_HTML} />
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
          empty={view.nav.length === 0}
        >
          {view.nav.map((page) => (
            <Card
              key={page.href}
              title={page.label}
              href={page.href}
              publisher={view.publisher}
              coordinate={view.coordinate}
              abstract={view.abstract}
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
  const raw = location.params["path"] ?? "";
  const view = viewOf(raw);

  if (view === null) {
    return (
      <Prose measure={MEASURE}>
        <p class="doc-view__unaddressed">
          This is not a documentation address: <code>{raw}</code>
        </p>
      </Prose>
    );
  }

  return view.kind === "page" ? (
    <DocumentationPage view={view} />
  ) : (
    <PackagePage view={view} />
  );
});

/**
 * What the document's head says about this address.
 *
 * The description is the page's own leading fact, taken and never
 * composed: a summary the pipeline wrote would be a second description
 * of the page that nobody proofreads.
 */
export const head: DocumentHead = ({ params }) => {
  const view = viewOf(params["path"] ?? "");
  if (view === null) return { title: "Not a documentation address" };
  if (view.kind === "package") {
    return {
      title: view.title,
      meta:
        view.description === undefined
          ? []
          : [{ name: "description", content: view.description }],
    };
  }
  return {
    title: view.title,
    meta: [{ name: "description", content: view.summary }],
    scripts: [MANUAL_SCROLL],
  };
};

/**
 * Which addresses the static generator writes to disk.
 *
 * It answers from the library rather than from a list, so the build
 * gate's two numbers — addresses declared, pages generated — cannot
 * drift apart by someone editing one of them.
 */
export const onStaticGenerate: StaticGenerateHandler = () => {
  return { params: documentationParams() };
};
