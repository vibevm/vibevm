/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * Where this build's island HTML comes from — the one thing that differs
 * between the two adapters, and it is a value, not a code path.
 *
 * The static build for the server bakes a rendered page in: the
 * documentation is known at build time, so the page ships finished. The
 * embedded build cannot know it — `vibe doc serve` renders the island
 * for the page a reader asked for, out of the machine store, at request
 * time — so it bakes in a placeholder the server replaces with the
 * island it just rendered.
 *
 * Both are the same route, the same component and the same markup around
 * the island. That is what «one shell, two adapters» has to mean to be
 * worth anything: if the difference were a second component or a second
 * route tree, the embedded reader and the site would drift apart exactly
 * where nobody looks.
 */

/** Replaced at build time by each adapter's Vite configuration. */
declare const __VIBE_ISLAND_HTML__: string;

/** The island this build shows. */
export const ISLAND_HTML = __VIBE_ISLAND_HTML__;
