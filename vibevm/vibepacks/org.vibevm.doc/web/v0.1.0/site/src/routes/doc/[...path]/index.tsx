/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import { component$, useStyles$, useVisibleTask$ } from "@qwik.dev/core";
import {
  useLocation,
  type DocumentHead,
  type StaticGenerateHandler,
} from "@qwik.dev/router";
import {
  Card,
  CitedRules,
  CodeChrome,
  Contents,
  Fab,
  FallbackNotice,
  ForAgent,
  LanguageSelector,
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
import { DocBar } from "../../../components/doc-bar/index.tsx";
import { Island } from "../../../components/island/index.tsx";
import { ServedReader } from "../../../components/served/index.tsx";
import { ISLAND_PLACEHOLDER } from "../../../lib/island-placeholder.ts";
import { BUILT } from "../../../lib/library-source.ts";
import { documentationParams } from "../../../lib/pages.ts";
import {
  AGENT_LEAD,
  MEASURE,
  PLATFORMS,
  SHELF_MEASURE,
} from "../../../lib/reading.ts";
import { startDocLanguage } from "../../../reader/doc-language.ts";
import { startReader } from "../../../reader/mount.ts";
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
      {/* The manual on the left, this page on the right, the text
          between them. Which is which is the whole of the reader's
          question: one list says where in the manual they are, the
          other where in the page. */}
      <Contents
        label="Contents"
        pinned={view.contents.pinned}
        sections={view.contents.sections}
      />
      <Toc label="On this page" />
      <Prose measure={MEASURE}>
        {view.fallback ? (
          <FallbackNotice
            asked={FALLBACK_ASKED}
            given={FALLBACK_GIVEN}
            dismissLabel="ok"
          />
        ) : null}
        {/* The documentation's own two controls, side by side: which
            language of this page to read, and which version of it. The
            language of the SITE is the switch in the corner of the
            header above, and the three are never the same question.
            The platform below is not one of them — it hides blocks of
            THIS text and belongs with the text. */}
        <DocBar>
          <LanguageSelector
            label="Documentation language"
            items={[...view.languages]}
          />
          <VersionSwitch label="Version" items={view.versions} />
        </DocBar>
        <TabPills label="Platform" items={PLATFORMS} />
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
        {/* Last in the column, folded: what the page stands on, for a
            reader who has finished reading it. The markers in the text
            are where a rule is read — this is only the list of them. */}
        <CitedRules label="Rules this page cites" />
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
  /* The one behaviour an index has. There is no island here, nothing to
     cite and no place to return to — only the language filter, which is
     the reader's and therefore lives in their browser. */
  const address = view.address.lang;
  useVisibleTask$(() => startDocLanguage(address));
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
        {/* A package page is an index, so the documentation's language
            narrows what stands on its shelves rather than moving the
            reader: every edition of this documentation is already named
            below, each with its own address. */}
        <DocBar>
          <LanguageSelector
            label="Documentation language"
            items={[...view.languages]}
          />
          <VersionSwitch label="Version" items={[...view.versions]} />
        </DocBar>
        <Shelf
          title="Documentation"
          caption="A star marks documentation the subject itself points at. The order says the same thing: primary, then official, then community."
          emptyLabel="Nothing documents this subject yet."
          empty={false}
        >
          <Card
            title={view.title}
            href={view.pages[0]?.href ?? view.llms}
            publisher={view.publisher}
            coordinate={view.coordinate}
            {...(view.description === undefined
              ? {}
              : { description: view.description })}
            abstract={view.abstract}
            status={view.status}
            glyph={DOC_GLYPH}
            abstractLabel="what it covers"
            editionLang={view.textLanguage}
            {...(view.authorship === undefined
              ? {}
              : { authorship: view.authorship })}
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
              editionLang={one.tag}
              {...(one.authorship === undefined
                ? {}
                : { authorship: one.authorship })}
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
