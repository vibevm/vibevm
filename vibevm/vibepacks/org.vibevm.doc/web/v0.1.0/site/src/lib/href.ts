/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

/**
 * Every address the site writes, written in one place.
 *
 * Two rules meet here. The site is built with ONE base — `/` for the
 * server build, `/doc/` for the shell `vibe` embeds — and Qwik does not
 * rewrite `href` attributes or `<Link>` targets when the base is not `/`
 * (measured on the beta), so a literal path in JSX is right in one build
 * and wrong in the other. And every page address ends in a slash: the
 * static generator produces `<route>/index.html` and generates correctly
 * in no other mode, so an address without the slash is a 308 away from
 * being the address (`##SITE-TRAILING-SLASH`).
 *
 * So JSX never writes a path. It calls `href` — which owns the leading
 * slash, the base, and the trailing one — or `docHref`, which builds a
 * documentation address out of a coordinate rather than out of string
 * concatenation. A literal `/doc/…` anywhere else is caught by the test
 * beside this file.
 */

/**
 * The site's mount. `/` in both builds — and that is not a coincidence
 * to be simplified away: the static build serves the documentation from
 * the route directory `doc/` at base `/`, while the embedded build
 * serves those same routes from the root at base `/doc/`, and the two
 * arrive at the same public address from opposite directions. One
 * constant, one public address map, two builds.
 */
const SITE_BASE = "/";

/** The segment the documentation is mounted under, on the public site. */
const DOC_SEGMENT = "doc";

/** A page of a documentation package, as an address rather than a string. */
export type DocAddress = {
  /**
   * The language segment, or `null` for the documentation's own
   * language — which carries no prefix at all (D-06).
   */
  readonly lang: string | null;
  /** The package's group, a reverse-DNS coordinate. */
  readonly group: string;
  readonly name: string;
  /** A version number, or `latest`. */
  readonly version: string;
  /**
   * The document inside the package, without its extension and with
   * `/` between its parts: `model/lock-and-store`.
   */
  readonly document: string;
};

/**
 * A site-root-relative path (no leading slash) turned into the address
 * the browser gets. Directories keep their trailing slash; a file — a
 * projection like `…/page.md`, a machine file like `manifest.json` —
 * keeps its extension and gets none.
 */
export function href(path: string): string {
  const trimmed = path.replace(/^\/+/, "");
  return `${SITE_BASE}${trimmed}`;
}

/** The documentation part of an address, root-relative and slash-ended. */
export function docPath(address: DocAddress): string {
  const lang = address.lang === null ? "" : `${address.lang}/`;
  return `${DOC_SEGMENT}/${lang}${address.group}/${address.name}/${address.version}/${address.document}/`;
}

/** The address a link to a documentation page carries. */
export function docHref(address: DocAddress): string {
  return href(docPath(address));
}

/**
 * The address of a page's machine projection — the `.md` or `.xml` that
 * lies beside it as a file, which is why it is the page's path without
 * its trailing slash plus a suffix (`##DISC-MACHINE-MIRROR`).
 */
export function projectionHref(
  address: DocAddress,
  suffix: "md" | "xml",
): string {
  return href(`${docPath(address).slice(0, -1)}.${suffix}`);
}

/**
 * Read an address back out of the segments the catch-all route matched.
 *
 * The one ambiguity is the first segment: `/doc/ru/org.vibevm.core/…`
 * and `/doc/org.vibevm.core/…` differ only in whether a language sits in
 * front. It is settled by what a group IS — a reverse-DNS coordinate,
 * which always carries a dot — and what a language tag is, which never
 * does. No index, no lookup, no list of known languages to fall behind:
 * the address map stays deterministic, which is the whole point of D-06.
 *
 * `null` for anything that is not an address: too few segments, an empty
 * one, or a first segment that is neither a group nor a language tag.
 */
export function parseDocAddress(
  segments: readonly string[],
): DocAddress | null {
  const parts = segments.filter((segment) => segment.length > 0);
  const first = parts[0];
  if (first === undefined) return null;

  const hasLang = !first.includes(".");
  if (hasLang && !/^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(first)) return null;

  const lang = hasLang ? first : null;
  const rest = hasLang ? parts.slice(1) : parts;
  const group = rest[0];
  const name = rest[1];
  const version = rest[2];
  if (group === undefined || name === undefined || version === undefined)
    return null;
  if (!group.includes(".")) return null;

  const document = rest.slice(3).join("/");
  if (document.length === 0) return null;

  return { lang, group, name, version, document };
}
