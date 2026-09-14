/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PREVIEW-COMPOSED */

/**
 * Where a package's link preview is.
 *
 * The manifest says. `media` carries the three card images at the
 * addresses `vibe doc build` wrote them to — `media/<content name>`,
 * relative to the base the documentation is served under
 * (`##CARD-MEDIA-ROLES`) — and the shell shows what it names and
 * computes nothing (`##PIPE-SHELL-PARSES-NOTHING`).
 *
 * It has not always said. The field is optional because the format is
 * read permissively (PROP-044 §4.4): a manifest written before the
 * addresses were carried is still a manifest, and absent means «this
 * document predates the field», never «this package has no picture». For
 * those, the build reads the tree it is copying, finds the 1200×630 card
 * the pipeline composed (D-20, `##CARD-PREVIEW-COMPOSED`), and hands the
 * address to the pages through the environment, the same way the origin
 * and the analytics id arrive (`config.ts`) — the search that was the
 * only path before X-055 and is now the second one.
 *
 * A build that has neither has no card and says so: the page then names
 * the site's own `og.png`, which the same build always writes. A preview
 * is a promise that an address answers with an image, and a guessed one
 * would break it on every share.
 */

import type { DocPackage } from "../generated/doc-manifest.ts";
import { packageHref } from "../lib/href.ts";

/** The environment name both stages read. */
export const DOC_MEDIA_ENV = "VITE_DOC_MEDIA";

/** The card images of one package, as the build found them. */
export type PackageMedia = {
  /** Root-relative address of the 1.91:1 card, or `undefined`. */
  readonly preview?: string;
};

/** Keyed by `group/name@version` — the coordinate, never the address. */
export type MediaMap = Readonly<Record<string, PackageMedia>>;

/** A map read defensively: anything that is not the shape is nothing. */
export function parseMediaMap(raw: unknown): MediaMap {
  if (typeof raw !== "string" || raw.trim().length === 0) return {};
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return {};
  }
  if (typeof parsed !== "object" || parsed === null) return {};
  const out: Record<string, PackageMedia> = {};
  for (const [coordinate, value] of Object.entries(parsed)) {
    if (typeof value !== "object" || value === null) continue;
    /* Spread rather than assert: `as` would be a claim about bytes this
       module was handed, and the whole reason it looks at them is that
       nobody can make that claim (R-16). */
    const fields: Record<string, unknown> = { ...value };
    const preview = fields["preview"];
    if (typeof preview === "string" && preview.length > 0) {
      out[coordinate] = { preview };
    }
  }
  return out;
}

function ambient(): unknown {
  const vite: ImportMetaEnv | undefined = import.meta.env;
  if (vite !== undefined) return vite[DOC_MEDIA_ENV];
  if (typeof process === "undefined") return undefined;
  return process.env[DOC_MEDIA_ENV];
}

/** The map this build was given. */
export const DOC_MEDIA: MediaMap = parseMediaMap(ambient());

/** The card of one package, by coordinate; `undefined` when there is none. */
export function previewOf(
  group: string,
  name: string,
  version: string,
): string | undefined {
  return DOC_MEDIA[`${group}/${name}@${version}`]?.preview;
}

/**
 * The card of one documentation: what its own manifest names, and the
 * build's search of the tree only for a manifest that names nothing.
 *
 * The version is always the NUMBER and never `latest`: `latest` is an
 * address and the card belongs to the publication, so a share of the
 * `latest` page still names the picture of the version it is showing.
 */
export function cardPreviewOf(card: DocPackage): string | undefined {
  const declared = card.media?.preview;
  if (declared !== undefined && declared.length > 0) {
    const at = packageHref({
      lang: null,
      group: card.group,
      name: card.name,
      version: card.version,
    });
    return `${at}${declared}`;
  }
  return previewOf(card.group, card.name, card.version);
}
