/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * Which of the two builds this is.
 *
 * It used to be derived rather than declared: the static build baked a
 * rendered island into the page and the embedded one baked the
 * placeholder, so comparing the two told you which build you were in
 * without a second value to keep in step with the first.
 *
 * That derivation is gone because the thing it read is gone. The site
 * now carries a documentation of many pages, and a page's island is the
 * bytes of its own `<document>/index.html` — so BOTH builds ship the
 * placeholder, and the difference is who fills it: the build driver,
 * once, over the pages it just generated, or `vibe doc serve`, per
 * request, out of the machine store. One marker, two fillers, and the
 * bytes around the hole provably the same in both.
 *
 * So the build says which it is, in the same `define` channel the
 * library and the card addresses arrive on. Both sides of the comparison
 * are literals by the time the bundler sees them, so the branch this
 * build does not take is dropped with the constant.
 */

/** Replaced at build time by each adapter's Vite configuration. */
declare const __VIBE_LOCAL_READER__: boolean;

/** True in the shell `vibe` serves from a machine's own store. */
export const IS_LOCAL_READER: boolean = __VIBE_LOCAL_READER__;
