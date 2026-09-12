/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

/**
 * The libraries THIS build was given, and the one place they are read.
 *
 * A deployment renders its documentation with `vibe doc build` and names
 * the output directories in `VIBE_DOC_OUT`; each tree carries the
 * manifest of one edition, and those manifests are what the site's
 * pages, shelves, navigation and addresses are made of. They arrive
 * through the same channel the island used to and the card addresses
 * still do — a value substituted by each adapter's Vite configuration —
 * because a bundler cannot import a file whose path is only known to the
 * deployment.
 *
 * A list and not one library: the manifests are grouped into as many
 * libraries as they hold source documentations, which is one over a
 * manual and its translations and forty-eight over a registry render.
 *
 * A build given no tree gets the package's own fixture pair instead, so
 * a fresh clone still produces a complete site with every link answered.
 * The substitution happens in one place either way: the configuration
 * decides which manifests, this file turns them into libraries, and
 * nothing downstream can tell which of the two it is looking at.
 *
 * It is its own module for the reason the island placeholder is: the
 * value only exists inside a build, so a test or a Node tool that wanted
 * the library's LOGIC would otherwise evaluate an identifier that is not
 * there. `library.ts` is therefore pure and importable anywhere, and the
 * one line that touches the build is here.
 */

import { parseLibraries, type Library } from "./library.ts";

/** Replaced at build time by each adapter's Vite configuration. */
declare const __VIBE_DOC_MANIFESTS__: readonly {
  readonly name: string;
  readonly value: unknown;
}[];

/** The libraries of the documentation this build renders. */
export const BUILT: readonly Library[] = parseLibraries(__VIBE_DOC_MANIFESTS__);
