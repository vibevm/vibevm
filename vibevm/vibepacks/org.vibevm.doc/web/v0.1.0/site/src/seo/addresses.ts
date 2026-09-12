/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-CANONICAL-LATEST */

/**
 * The `latest` addresses, which the generator writes beside the numbered
 * ones.
 *
 * `latest` is not a label on a page, it is an address. A citation
 * without a version resolves to it (D-06), the version switch offers it
 * beside the number as the other address of the same content, and
 * `##SITE-CANONICAL-LATEST` makes it the one a numbered page points its
 * `rel=canonical` at. All three were true of the shell before this atom
 * and none of them had a page behind it: the link existed, the address
 * did not.
 *
 * So the static build materialises it. The pages are the same pages —
 * the route reads `latest` as «the newest version» and has since the
 * shell was assembled — and what makes the two addresses one page rather
 * than two is the canonical the numbered one carries.
 *
 * The list is derived from the library, like everything else the
 * generator is given, and the build driver counts the same addresses
 * from the manifests independently: the page-count gate is two counts of
 * one thing, and a shared list would make it one count twice
 * (`##STACK-PAGE-COUNT-GATE`).
 */

import { docSegments, packageHref, docHref } from "../lib/href.ts";
import {
  coordinate,
  documentOf,
  editions,
  sourceEdition,
} from "../lib/library.ts";
import { LATEST } from "./editions.ts";

/** The catch-all route's parameter for one address, or nothing. */
function paramOf(path: string): { path: string } | null {
  const segments = docSegments(path);
  return segments === null ? null : { path: segments.join("/") };
}

/**
 * Every `latest` address: each edition's package page, and each page of
 * the source in each edition — the same set as the numbered addresses,
 * with one segment changed.
 *
 * The catalogues are not in it. `vibevm.org/doc/ru/` names a language and no
 * version at all, so there is nothing for `latest` to spell.
 */
export function latestAliasParams(): { path: string }[] {
  const out: { path: string }[] = [];
  const documents = sourceEdition().pages.map((page) => documentOf(page.path));

  for (const edition of editions()) {
    const at = { ...coordinate(edition.segment), version: LATEST };
    const pkg = paramOf(packageHref(at));
    if (pkg !== null) out.push(pkg);
    for (const document of documents) {
      const page = paramOf(docHref({ ...at, document }));
      if (page !== null) out.push(page);
    }
  }
  return out;
}
