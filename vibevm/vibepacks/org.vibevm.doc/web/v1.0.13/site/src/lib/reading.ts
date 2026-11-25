/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

/**
 * The few constants a documentation page is read at, in one place
 * because two readers render that page.
 *
 * The site's build resolves an address and renders the page from the
 * library it was built with; the shell `vibe doc serve` embeds renders
 * the same page from a manifest it fetches on a reader's own machine.
 * They are two components on purpose — one knows its address and the
 * other is told — but a reading measure or a row of platforms that
 * differed between them would be the same page read two ways.
 */

/** The reading measure, until the reader's own setting overrides it. */
export const MEASURE = 740;

/** The measure a shelf reads at: cards, not sentences. */
export const SHELF_MEASURE = 960;

/**
 * The platforms the switch offers. A fixed row and not a scan of the
 * island: a page is written for the platforms the product runs on, and
 * one that happens to mention only two of them must still let a reader
 * say which they are on — otherwise the switch would appear and vanish
 * with the text.
 */
export const PLATFORMS = [
  { label: "Windows", value: "windows", current: true },
  { label: "macOS", value: "macos", current: false },
  { label: "Linux", value: "linux", current: false },
] as const;

/** What the «for an agent» panel says before it says an address. */
export const AGENT_LEAD =
  "This page has a machine mirror. The citation carries the version rather than latest, so what an agent quotes does not move under it.";

/**
 * The words the two views of the contents and the path through it wear
 * (`##NAV-CHAPTERS-READER`).
 *
 * They are here for the reason the measure and the platforms are: the
 * column and the pager are rendered by both readers, and a label written
 * out in each would be a label that goes out of step. Every one of them
 * is furniture, so each has a row in the site's language table and moves
 * with the reader's own choice of interface language.
 */
export const CONTENTS_VIEW_LABEL = "Contents view";
export const CONTENTS_PATH_LABEL = "In order";
export const CONTENTS_SECTIONS_LABEL = "By section";
export const PATH_LABEL = "The learning path";
export const PATH_CHAPTER_WORD = "Chapter";
export const PATH_PREVIOUS = "Previous";
export const PATH_NEXT = "Next";

/** What the «Pages» shelf says about its order, in each of the two. */
export const PAGES_BY_LAYER =
  "In the order the layer law gives them: text that stands still before text that moves with the product.";
export const PAGES_BY_PATH = "In the order of the learning path";

/** Where a package's own page asks a reader to begin. */
export const START_HERE = "Start here";
