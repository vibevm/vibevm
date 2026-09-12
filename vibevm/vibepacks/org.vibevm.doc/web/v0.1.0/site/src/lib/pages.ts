/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-PAGE-COUNT-GATE */

/**
 * Which pages this build renders — read from the page manifest, never
 * from a list kept beside it.
 *
 * The static generator is the reason this matters more than it looks.
 * It under-generates silently and still exits 0: a wrong base yields no
 * pages at all and an empty sitemap, with no error anywhere. So the
 * build gate counts what the generator says it made against what the
 * manifest says exists — and both numbers have to come from the same
 * place, or the gate is comparing a build against its own opinion.
 */

import fixture from "../fixtures/manifest.json";
import { parseDocManifest } from "./manifest.ts";
import type { DocAddress } from "./href.ts";

/**
 * The extension a page's manifest path carries (`.xml`, `.md`) is part
 * of the file, not of the address: the address ends in a slash and the
 * projections lie beside it as files.
 */
function documentOf(path: string): string {
  return path.replace(/\.(xml|md)$/, "");
}

/**
 * Every page of the documentation this build ships, as addresses.
 *
 * Until a real documentation package is on the machine this is the
 * pipeline's own fixture — one page carrying every block of the genre
 * once. The shape of what replaces it is the same: a manifest in, a list
 * of addresses out.
 */
export function documentationPages(): DocAddress[] {
  const parsed = parseDocManifest(fixture);
  if (!parsed.ok) {
    throw new Error(
      `the page manifest this build renders from is not a manifest: ` +
        `${parsed.error.path} — ${parsed.error.reason}`,
    );
  }
  const { group, name, version } = parsed.value.package;
  return parsed.value.pages.map((page) => ({
    lang: null,
    group,
    name,
    version,
    document: documentOf(page.path),
  }));
}

/** The same pages as the catch-all route's parameters. */
export function documentationParams(): { path: string }[] {
  return documentationPages().map((address) => ({
    path: `${address.group}/${address.name}/${address.version}/${address.document}`,
  }));
}
