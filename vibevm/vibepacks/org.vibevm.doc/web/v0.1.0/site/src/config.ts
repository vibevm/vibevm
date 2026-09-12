/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ANALYTICS */

/**
 * What the site knows about the domain it is being built for.
 *
 * Three values arrive from the build environment rather than from the
 * source, and each for its own reason. The origin, because a package
 * published as source must be buildable for a domain that is not this
 * one. The Umami website id and the IndexNow key, because they identify
 * a live property: they belong to the deployment, not to the repository,
 * and the defaults here are deliberately empty so that a build which was
 * told nothing publishes nothing rather than publishing a guess
 * (`##SITE-ANALYTICS`, `##SEO-INDEXNOW`).
 *
 * An empty value is not a hole to be filled in later by whoever notices:
 * it is the instruction. No website id means no analytics tag in the
 * page at all — not a tag with an empty attribute, which would load the
 * script and report nothing. No IndexNow key means no key file at the
 * domain root — not a file containing the empty string, which would fail
 * the protocol's own verification and look like a misconfigured site.
 *
 * The same module answers both readers. Inside the bundle Vite replaces
 * `import.meta.env` with the environment it was given; inside Node —
 * `tools/root-files.mjs`, which writes the machine files of the domain
 * root — `import.meta.env` does not exist and `process.env` is the same
 * environment by another name. One definition of the names, two build
 * stages, no chance of the page and the key file disagreeing about which
 * domain they are for.
 */

/** The environment names, written once because two stages read them. */
export const ENV_NAMES = {
  origin: "VITE_SITE_ORIGIN",
  umamiWebsiteId: "VITE_UMAMI_WEBSITE_ID",
  indexNowKey: "VITE_INDEXNOW_KEY",
  lastmod: "VITE_SITE_LASTMOD",
} as const;

/**
 * The domain the site is built for when the environment names none.
 *
 * It is a default and not a constant: `vibe doc serve` and a fork both
 * build this same source for somewhere else. What it may never be is
 * absent — `canonical`, `hreflang` and the sitemap are absolute
 * addresses by specification, and an empty origin would produce a page
 * that silently claims to live at `/`.
 */
const DEFAULT_ORIGIN = "https://vibevm.org";

export type SiteConfig = {
  /** Scheme and host, no trailing slash: `https://vibevm.org`. */
  readonly origin: string;
  /** The Umami property id, or `""` — which means «render no tag». */
  readonly umamiWebsiteId: string;
  /** The IndexNow key, or `""` — which means «write no key file». */
  readonly indexNowKey: string;
  /** `YYYY-MM-DD` for `sitemap.xml`; the build date unless told otherwise. */
  readonly lastmod: string;
};

/** A record read defensively: anything that is not a string is absent. */
function read(
  env: Readonly<Record<string, unknown>>,
  name: string,
): string | undefined {
  const value = env[name];
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
}

/** Today in UTC, as a sitemap writes a date. */
function today(): string {
  const now = new Date();
  const month = `${now.getUTCMonth() + 1}`.padStart(2, "0");
  const day = `${now.getUTCDate()}`.padStart(2, "0");
  return `${now.getUTCFullYear()}-${month}-${day}`;
}

/**
 * The configuration a given environment describes.
 *
 * Exported as a function of its input rather than as a value read from
 * the ambient one, so the Node stage can pass `process.env` and a test
 * can pass a literal.
 */
export function siteConfig(env: Readonly<Record<string, unknown>>): SiteConfig {
  const origin = read(env, ENV_NAMES.origin) ?? DEFAULT_ORIGIN;
  return {
    origin: origin.replace(/\/+$/, ""),
    umamiWebsiteId: read(env, ENV_NAMES.umamiWebsiteId) ?? "",
    indexNowKey: read(env, ENV_NAMES.indexNowKey) ?? "",
    lastmod: read(env, ENV_NAMES.lastmod) ?? today(),
  };
}

/**
 * The ambient environment, whichever of the two stages is asking.
 *
 * `import.meta.env` is what Vite substitutes into the bundle; it does
 * not exist under plain Node, where `process.env` carries the same
 * variables. The `typeof` guard is not defensive style — `process` is an
 * undeclared identifier in a browser, and naming it unguarded would
 * throw at module evaluation rather than return nothing.
 */
function ambient(): Readonly<Record<string, unknown>> {
  const vite: ImportMetaEnv | undefined = import.meta.env;
  if (vite !== undefined) return vite;
  if (typeof process === "undefined") return {};
  return process.env;
}

/** The configuration this build was given. */
export const SITE: SiteConfig = siteConfig(ambient());
