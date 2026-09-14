/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * The marker the embedded build ships where the static build ships a
 * rendered page.
 *
 * It lives alone in this file for one reason: the embedded adapter's
 * Vite configuration has to name it, and a Vite configuration is loaded
 * by Node before any build-time replacement has happened. Importing it
 * from the module that reads the replaced value would run that module's
 * `__VIBE_ISLAND_HTML__` and fail before the build starts.
 *
 * An HTML comment, so a template served by mistake renders as an empty
 * page rather than as a word; and a distinctive one, so the server that
 * fills it in can find it with a plain string search instead of a parse.
 */
export const ISLAND_PLACEHOLDER = "<!--vibe-doc-island-->";
