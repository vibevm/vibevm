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

/**
 * A documentation package at one version, in one language — the address
 * of its own page, the shelf of what documents it and what adapts it.
 */
export type PackageAddress = {
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
};

/**
 * A page of a documentation package, as an address rather than a string.
 *
 * A page address is its package's address with a document behind it, and
 * it is spelled that way rather than repeated field by field: the two
 * differ in exactly one thing, and a copy would let them drift.
 */
export type DocAddress = PackageAddress & {
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

/** The catalogue of the documentation the site carries, in one language. */
export function cataloguePath(lang: string | null): string {
  return lang === null ? `${DOC_SEGMENT}/` : `${DOC_SEGMENT}/${lang}/`;
}

/** The address of the catalogue; `/doc/` itself has no language segment. */
export function catalogueHref(lang: string | null): string {
  return href(cataloguePath(lang));
}

/** A package's own page — the shelf, the card and the abstract. */
export function packagePath(address: PackageAddress): string {
  const lang = address.lang === null ? "" : `${address.lang}/`;
  return `${DOC_SEGMENT}/${lang}${address.group}/${address.name}/${address.version}/`;
}

/** The address of a package's own page. */
export function packageHref(address: PackageAddress): string {
  return href(packagePath(address));
}

/**
 * The package's agent index, beside its pages as a file — the third
 * surface the «for an agent» panel hands over, after the page's own
 * `.md` and `.xml` (`##SEO-LLMS-FILES`).
 */
export function llmsHref(address: PackageAddress): string {
  return href(`${packagePath(address)}llms.txt`);
}

/** The documentation part of an address, root-relative and slash-ended. */
export function docPath(address: DocAddress): string {
  return `${packagePath(address)}${address.document}/`;
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

/**
 * The catch-all segments of a documentation address, read back out of a
 * path the browser is actually on — or `null` when the path is not the
 * documentation's at all.
 *
 * This is the inverse of `href` for the one caller that has a served
 * path and no parameters: the site header, which stands over the landing
 * and the documentation alike and has to tell which of the two it is
 * standing over.
 */
export function docSegments(pathname: string): string[] | null {
  const parts = pathname.split("/").filter((part) => part.length > 0);
  if (parts[0] !== DOC_SEGMENT) return null;
  return parts.slice(1);
}

/** What the catch-all route matched: one page, or one package's own page. */
export type DocTarget =
  | { readonly kind: "page"; readonly address: DocAddress }
  | { readonly kind: "package"; readonly address: PackageAddress };

/**
 * Read the catch-all route's segments as either address it may be.
 *
 * `/doc/<group>/<name>/<version>/` and `/doc/<group>/<name>/<version>/<document>/`
 * are the same address one segment apart, so one parser reads both and
 * says which it found. A second parser would be a second opinion about
 * where a language ends and a group begins.
 */
export function parseDocTarget(segments: readonly string[]): DocTarget | null {
  const page = parseDocAddress(segments);
  if (page !== null) return { kind: "page", address: page };

  const parts = segments.filter((segment) => segment.length > 0);
  const first = parts[0];
  if (first === undefined) return null;
  const hasLang = !first.includes(".");
  if (hasLang && !/^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(first)) return null;
  const rest = hasLang ? parts.slice(1) : parts;
  if (rest.length !== 3) return null;
  const [group, name, version] = rest;
  if (group === undefined || name === undefined || version === undefined)
    return null;
  if (!group.includes(".")) return null;

  return {
    kind: "package",
    address: { lang: hasLang ? first : null, group, name, version },
  };
}

/**
 * The `spec://` address of a page — what the «for an agent» panel hands
 * over, and the one place the version is spelled into a citation.
 *
 * The version is always a number and never `latest`, even when the
 * reader arrived at the `latest` address: an agent handed `latest` would
 * quote a page that moves under it, and R-26 says a citation carries the
 * version. The language is not in it at all — a `spec://` address names
 * the documentation, and which language a reader read it in is not part
 * of what they are citing.
 */
export function specUri(address: DocAddress): string {
  return `spec://${address.group}/${address.name}@${address.version}/${address.document}`;
}

/**
 * A `spec://` address turned back into a place on this site — the
 * inverse of `specUri`, and the one thing an embedding host may ask the
 * reader to do («open this»).
 *
 * A citation without a version means the newest, which is the `latest`
 * address; a fragment travels through untouched, because keeping a
 * reader's place is the whole reason a host sends one.
 */
export function hrefOfSpecUri(uri: string): string | null {
  const match = /^spec:\/\/([^/]+)\/([^/@]+)(?:@([^/]+))?\/(.+)$/.exec(uri);
  if (match === null) return null;
  const [, group, name, version, rest] = match;
  if (group === undefined || name === undefined || rest === undefined)
    return null;
  if (!group.includes(".")) return null;

  const hash = rest.indexOf("#");
  const document = hash === -1 ? rest : rest.slice(0, hash);
  const fragment = hash === -1 ? "" : rest.slice(hash);
  if (document.length === 0) return null;

  return (
    docHref({
      lang: null,
      group,
      name,
      version: version ?? "latest",
      document,
    }) + fragment
  );
}
