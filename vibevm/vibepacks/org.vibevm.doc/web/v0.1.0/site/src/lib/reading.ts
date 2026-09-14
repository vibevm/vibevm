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
