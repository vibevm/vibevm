/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER */

/**
 * The resolver on a static host — a redirect table and the
 * one page that reads it.
 *
 * A server would answer this with a 302 computed from the address map,
 * which is deterministic and needs no index (D-06). A static host cannot
 * answer anything, so F-15 says the public site publishes the map as a
 * table and a page that follows it in the browser. The local reader,
 * which does have a server, keeps the route.
 *
 * The table is written from the manifests — every document, under both
 * spellings of its version — and carries each page's anchors so the
 * resolver can tell a fragment that exists from one that does not, and
 * land the reader on the page rather than on nothing when a citation has
 * gone stale. It never invents: an address the table does not hold is
 * reported to the reader, not guessed at.
 */

import { docFileHref, doorHref, LATEST, type Address } from "./editions.ts";

/** Where the table and the page that reads it are served. */
export const RESOLVE_TABLE = docFileHref("resolve.json");
export const RESOLVER_PAGE = docFileHref("resolve/");

/** What the table says about one document. */
export type ResolveEntry = {
  /** The page's address on this site. */
  readonly href: string;
  /** Every named anchor the page carries, in document order. */
  readonly anchors: readonly string[];
};

/** The table itself, as `resolve.json` holds it. */
export type ResolveTable = {
  readonly schema_version: 1;
  /** Keyed by `spec://` address without a fragment. */
  readonly documents: Readonly<Record<string, ResolveEntry>>;
};

/**
 * The `spec://` address of a page, in the spelling the table is keyed
 * by: with the version for a numbered address, without it for `latest`.
 *
 * Both are real citations. A citation that names a version quotes that
 * contract; one that omits it means «whichever is newest», which is what
 * the `latest` address is (D-06). The language is in neither: a
 * `spec://` address names the documentation, and which language a reader
 * read it in is not part of what they cite.
 */
function keyOf(address: Address): string | null {
  if (address.kind !== "page" || address.document === undefined) return null;
  if (address.edition.segment !== null) return null;
  const card = address.edition.manifest.package;
  const coordinate =
    address.version === LATEST
      ? `${card.group}/${card.name}`
      : `${card.group}/${card.name}@${address.version}`;
  return `spec://${coordinate}/${address.document}`;
}

/** The redirect table for every address the source editions publish. */
export function resolveTableOf(addresses: readonly Address[]): ResolveTable {
  const documents: Record<string, ResolveEntry> = {};
  for (const address of addresses) {
    const key = keyOf(address);
    if (key === null) continue;
    documents[key] = {
      href: address.href,
      anchors: address.page?.anchors ?? [],
    };
  }
  return { schema_version: 1, documents };
}

/**
 * The script the resolver page runs, as one line.
 *
 * It is inline and in the head for the same reason the theme script is:
 * everything it does must happen before anything is painted, and a
 * reader who asked to be sent somewhere should not first watch a page
 * arrive. Its `sha256` is in the policy the build writes, computed from
 * these bytes (X-035).
 *
 * It reads three things and trusts none of them. The `uri` parameter is
 * matched against the shape of a `spec://` address before it is used as
 * a key; the destination comes from the table and is never built from
 * the parameter, so a crafted `uri` cannot name an address of its own;
 * and the fragment travels only when the page says it has that anchor.
 */
const RESOLVER_SCRIPT = [
  "(function(){",
  "var q=new URLSearchParams(location.search).get('uri');",
  "function fail(m){document.addEventListener('DOMContentLoaded',function(){",
  "var e=document.getElementById('answer');if(e)e.textContent=m;});}",
  "if(!q||!/^spec:\\/\\/[^\\s]+$/.test(q)){fail('No spec:// address was asked for.');return;}",
  "var hash=q.indexOf('#');",
  "var key=hash===-1?q:q.slice(0,hash);",
  "var anchor=hash===-1?'':q.slice(hash+1);",
  `fetch(${JSON.stringify(RESOLVE_TABLE)}).then(function(r){return r.json();}).then(function(t){`,
  "var e=t.documents[key];",
  "if(!e){fail('This site does not carry '+key+'.');return;}",
  "var at=e.href+(anchor&&e.anchors.indexOf(anchor)!==-1?'#'+anchor:'');",
  "location.replace(at);",
  "}).catch(function(){fail('The resolver table could not be read.');});",
  "})()",
].join("");

/**
 * The resolver page.
 *
 * Hand-written rather than a route of the application, and deliberately:
 * this address is a machine's entry point, its answer is a redirect, and
 * loading a framework to perform one is a second's wait paid by every
 * agent that follows a citation. It is `noindex` because it is not a
 * page — it is a junction — and it says in words what it is doing, so a
 * reader who lands here with scripting off is not left staring at a
 * blank document.
 */
export function resolverPage(): string {
  return [
    "<!DOCTYPE html>",
    '<html lang="en">',
    "<head>",
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    '<meta name="robots" content="noindex, follow">',
    "<title>Resolve a spec:// address</title>",
    `<script>${RESOLVER_SCRIPT}</script>`,
    "</head>",
    "<body>",
    "<h1>Resolve a spec:// address</h1>",
    `<p>Ask for a page by its citation: <code>${RESOLVER_PAGE}?uri=spec://&lt;group&gt;/&lt;name&gt;/&lt;document&gt;#&lt;anchor&gt;</code>.`,
    " An address without a version resolves to the newest publication.</p>",
    '<p id="answer">Resolving&hellip;</p>',
    `<p><a href="${doorHref()}">The documentation this site carries</a>`,
    ` &middot; <a href="${docFileHref("manifest.json")}">manifest.json</a>`,
    ` &middot; <a href="${RESOLVE_TABLE}">resolve.json</a></p>`,
    "</body>",
    "</html>",
    "",
  ].join("\n");
}

/** The script the page carries, for the policy that has to name its hash. */
export const RESOLVER_INLINE_SCRIPT = RESOLVER_SCRIPT;
