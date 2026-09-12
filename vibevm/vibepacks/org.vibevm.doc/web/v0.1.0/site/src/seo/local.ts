/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LOCAL-EXEMPT */

/**
 * What a page carries when it is being read from a machine's own store
 * rather than from the domain.
 *
 * `##SEO-LOCAL-EXEMPT` is one sentence — «the local mode publishes none
 * of this and contacts nothing external» — and it is not a detail. A
 * reader running `vibe doc serve` is looking at documentation of packages
 * that may be proprietary and are certainly not on the public site; a
 * page there that carried `rel=canonical` to `vibevm.org` would declare
 * itself a copy of a page that does not exist, `hreflang` would name
 * addresses on a domain this reader never asked about, and an Open Graph
 * card would point at an image on somebody else's host. None of it is
 * published, so none of it is built.
 *
 * The local head is therefore written here rather than filtered out of
 * the public one. A filter would have to build the public head first —
 * calling the analytics tag to throw it away, composing structured data
 * about a page nobody indexes — and the whole claim of this file is that
 * on a local page those things never happen at all.
 *
 * What stays is what a reader and their agent actually use: the title,
 * the page's own leading summary, and the three machine surfaces that lie
 * beside the page as files. Their addresses are root-relative, which in
 * the embedded build means the reader's own origin.
 */

import type { DocumentHeadValue } from "@qwik.dev/router";

/** The head of one documentation page, as a local reader gets it. */
export function localPageHead(
  title: string,
  summary: string,
  projections: NonNullable<DocumentHeadValue["links"]>,
): DocumentHeadValue {
  return {
    title,
    meta: [{ name: "description", content: summary }],
    links: [...projections],
  };
}

/**
 * The head of a package's page or a language's catalogue, locally.
 *
 * The same two values and the one link a shelf offers. There is no
 * `canonical` even between a package's two version spellings: locally
 * they are two addresses of the store's current content, nobody is
 * choosing between them for an index, and a tag that exists to settle
 * that choice has nothing to settle.
 */
export function localShelfHead(
  title: string,
  description: string,
  links: NonNullable<DocumentHeadValue["links"]> = [],
): DocumentHeadValue {
  return {
    title,
    meta: [{ name: "description", content: description }],
    links: [...links],
  };
}

/**
 * The tag shapes a public page carries and a local page must not, as
 * patterns over built HTML.
 *
 * The list is here, beside the builders, because it is the same
 * statement read from the other end: the builders say what a local head
 * IS, and these say what a local page must never turn out to contain.
 * `tools/build.mjs` runs them over every page of the embedded output —
 * the guarantee is about what was written to disk, and only the output
 * can be asked about that.
 */
export const PUBLIC_ONLY: readonly {
  readonly what: string;
  readonly pattern: RegExp;
}[] = [
  { what: "rel=canonical", pattern: /<link\b[^>]*\brel="canonical"/i },
  { what: "an hreflang annotation", pattern: /\bhreflang="/i },
  { what: "an Open Graph tag", pattern: /<meta\b[^>]*\bproperty="og:/i },
  { what: "a Twitter card tag", pattern: /<meta\b[^>]*\bname="twitter:/i },
  { what: "structured data", pattern: /type="application\/ld\+json"/i },
  { what: "a robots directive", pattern: /<meta\b[^>]*\bname="robots"/i },
  { what: "the analytics tag", pattern: /\bdata-website-id=/i },
];
