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
 * What a local page keeps is nothing at all, and the reason is not
 * `##SEO-LOCAL-EXEMPT` \u2014 it is what the embedded build IS. This build
 * produces ONE route template that `vibe doc serve` dresses every page of
 * every package in, so a title and three projection links prerendered
 * here name the fixture the shell was built against and not the page a
 * reader asked for. They were wrong in the template, and the server
 * corrected them by editing the bytes on the way out.
 *
 * That correction cannot stay. A Qwik container carries the shape of its
 * own document in its serialised state, and inserting an element into the
 * `<head>` of a resumed page desynchronises the two: the page throws
 * `Code(Q27)` while materialising, resumption stops, and NOTHING on the
 * page runs \u2014 not the table of contents, not the anchors, not the
 * settings, not the manifest read the chrome is built from. Measured
 * three ways over one served page: the template glued and otherwise
 * untouched resumes; the title rewritten in place resumes (the element
 * survives, only its text changes); the three `rel=alternate` links
 * removed and re-inserted after `</title>` does not.
 *
 * So the template declares no head of its own. With no `<title>` element
 * in it, both of the server's edits are no-ops by their own terms \u2014
 * `retitle` needs the element and `relink` inserts only after
 * `</title>` \u2014 and the shell writes the title and the three links
 * itself, from the manifest it reads at the start of the route, by
 * APPENDING them to the end of the document head. Appending there is the
 * one edit a resumed container tolerates, measured the same way, and the
 * party doing it is the one that knows which page is being served.
 */

import type { DocumentHeadValue } from "@qwik.dev/router";

/**
 * The head of any page of the local reader: empty.
 *
 * One function for the page, the package and the catalogue, because the
 * answer no longer depends on which of the three it is \u2014 and a builder
 * per shape would be three ways to accidentally put an element back.
 */
export function localHead(): DocumentHeadValue {
  return {};
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
