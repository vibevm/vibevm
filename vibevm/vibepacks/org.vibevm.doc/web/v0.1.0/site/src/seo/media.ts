/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PREVIEW-COMPOSED */

/**
 * Where a package's link preview is, when the build was given one.
 *
 * The page cannot work the address out. A documentation package's card
 * images are written by `vibe doc build` under content-hashed names, and
 * the page manifest does not carry those names — a hole in the manifest
 * schema, already filed and already assigned (X-042). Until it is
 * closed, the build reads the tree it is copying, finds the 1200×630
 * card the pipeline composed (D-20, `##CARD-PREVIEW-COMPOSED`), and
 * hands the address to the pages through the environment, the same way
 * the origin and the analytics id arrive (`config.ts`).
 *
 * A build that was given no tree has no card and says so by leaving the
 * map empty: the page then names the site's own `og.png`, which the same
 * build always writes. A preview is a promise that an address answers
 * with an image, and a guessed one would break it on every share.
 */

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
