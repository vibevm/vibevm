/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * Which of the two builds this is, asked of the one thing that already
 * differs between them.
 *
 * The static build bakes a rendered island into the page; the embedded
 * build bakes the placeholder that `vibe doc serve` replaces with the
 * island it rendered for the page a reader asked for
 * (`lib/island-source.ts`). That substitution is the definition of the
 * local reader's build, so it is also the honest way to recognise it —
 * no second `define` to keep in step with the first, and no environment
 * variable a deployment could set by mistake.
 *
 * Both sides of the comparison are string literals by the time the
 * bundler sees them, so it folds to a constant and the branch the build
 * does not take is dropped with it.
 */

import { ISLAND_PLACEHOLDER } from "../lib/island-placeholder.ts";
import { ISLAND_HTML } from "../lib/island-source.ts";

/** True in the shell `vibe` serves from a machine's own store. */
export const IS_LOCAL_READER: boolean = ISLAND_HTML === ISLAND_PLACEHOLDER;
