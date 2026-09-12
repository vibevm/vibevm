/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-PAGE-COUNT-GATE */

/**
 * Which addresses this build renders — read from the library, never
 * from a list kept beside it.
 *
 * The static generator is the reason this matters more than it looks.
 * It under-generates silently and still exits 0: a wrong base yields no
 * pages at all and an empty sitemap, with no error anywhere. So the
 * build gate counts what the generator says it made against what the
 * manifests say exists — and both numbers have to come from the same
 * place, or the gate is comparing a build against its own opinion.
 */

import { siteAddresses } from "./library.ts";
import { BUILT } from "./library-source.ts";

/** Every documentation address, as the catch-all route's parameters. */
export function documentationParams(): { path: string }[] {
  return siteAddresses(BUILT).map((address) => ({ path: address.path }));
}
