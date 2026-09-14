/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * The reader: every behaviour the documentation page has, started once
 * and taken down once.
 *
 * It is plain TypeScript and not a set of Qwik components, and that is
 * forced rather than chosen. The page a reader reads is an ISLAND —
 * finished HTML the Rust pipeline rendered, inserted as a string, never
 * turned into a component tree and never re-rendered, because a second
 * renderer would be a second opinion about the same bytes. Nothing
 * inside it can carry a framework handler. So the behaviour reaches it
 * the way any script would: one listener per region, delegation over
 * what was clicked, and no state the framework has to serialize.
 *
 * The route calls this from a visible task and returns what it gives
 * back. Every module here follows the same contract — start, return the
 * teardown — so a page that is left never keeps a listener on the
 * document, and the whole reader can be switched off by dropping one
 * call.
 */

import { startAnchors } from "./anchors.ts";
import { startAgentSurface } from "./agent.ts";
import { startCodeChrome } from "./code.ts";
import { startOverlay } from "./overlay.ts";
import { startToc } from "./toc.ts";
import { startCatalogue, type CatalogueEdition } from "./catalogue.ts";
import { startCatalogueTabs } from "./catalogue-tabs.ts";
import { startDocLanguage } from "./doc-language.ts";
import { isEmbedded, publishSettings, startEmbedding } from "./embedding.ts";
import { startFallback } from "./fallback.ts";
import { startLanguageSwitch } from "./language.ts";
import { startPlatformSwitch } from "./platform.ts";
import { startPosition } from "./position.ts";
import { startReadingMode } from "./reading-mode.ts";
import { startRuleTransclusion } from "./rules.ts";
import { startSettings } from "./settings.ts";

/**
 * What the reader needs to know about the page it is on, worked out at
 * build time and handed over as plain values.
 *
 * The reader computes no addresses of its own. The `spec://` citation,
 * the projections beside the page and the source page a fallback points
 * at are all decided by the site's one address function and arrive here
 * already spelled — so the behaviour cannot disagree with the links the
 * page renders.
 */
export type ReaderContext = {
  /** The `spec://…@<version>/<document>` citation of this page. */
  readonly uri: string;
  /** The page's `.md` projection, beside it as a file. */
  readonly md: string;
  /** The page's `.xml` projection. */
  readonly xml: string;
  /** The package's agent index. */
  readonly llms: string;
  /** The language of the TEXT — the source's when this is a fallback. */
  readonly textLanguage: string;
  /** The language segment of the ADDRESS, or `null` for the source. */
  readonly addressLanguage: string | null;
  /** True when the source's text is being served under another language. */
  readonly fallback: boolean;
  /** The source page this one falls back to, as an address. */
  readonly sourceHref: string;
  /** Where the documentation is mounted, so links can be re-languaged. */
  readonly mount: string;
};

/**
 * The stamp a page carries while its behaviours are listening.
 *
 * A documentation page is interactive-looking from the first frame and
 * inert until this module's chunk has loaded: the settings gear, the
 * platform pills and the block numbers are all markup with a listener
 * added later, and a click that arrives in between is a click that goes
 * nowhere. The page says which of the two states it is in, in one place,
 * so that a stylesheet and a test can both ask.
 */
const LIVE = "data-reader";

/** Start every behaviour; the returned function stops all of them. */
export function startReader(context: ReaderContext): () => void {
  const embedded = isEmbedded();

  const settings = startSettings({
    persist: !embedded,
    publish: embedded ? publishSettings : () => undefined,
  });

  const stops = [
    settings.stop,
    // The chrome over the island first: the contents, the toolbars and
    // the overlay all add elements the behaviours below then listen
    // over, and a listener attached before the element exists is a
    // listener that never fires.
    startCodeChrome(),
    startOverlay(),
    startToc(),
    startPlatformSwitch(),
    startRuleTransclusion(),
    startAnchors(),
    startLanguageSwitch(),
    startDocLanguage(context.addressLanguage),
    startFallback({
      addressLanguage: context.addressLanguage,
      fallback: context.fallback,
      mount: context.mount,
    }),
    startPosition(),
    startReadingMode(),
    startAgentSurface(context.uri),
    startEmbedding({
      onSettings: settings.receive,
      files: [context.md, context.xml, context.llms],
    }),
  ];

  document.documentElement.setAttribute(LIVE, embedded ? "embedded" : "on");

  return () => {
    document.documentElement.removeAttribute(LIVE);
    for (const stop of stops) stop();
  };
}

/**
 * The catalogue's much smaller reader: which shelf, which language, and
 * the one decision the door has to take.
 *
 * It shares nothing with the page's reader because it has nothing to
 * share — there is no island, no block to cite and no place to return
 * to. What it does have is two narrowings of one list, which are
 * deliberately two behaviours: a tab says what KIND of thing a reader
 * wants and a filter says which language of it, and a reader changing
 * one is not changing the other.
 */
export function startCatalogueReader(
  editions: readonly CatalogueEdition[],
  addressLanguage: string | null,
): () => void {
  const stops = [
    startCatalogueTabs(),
    startLanguageSwitch(),
    startDocLanguage(addressLanguage),
    startCatalogue(editions),
  ];
  return () => {
    for (const stop of stops) stop();
  };
}
