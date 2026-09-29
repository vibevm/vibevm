/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ANALYTICS */

import type { DocumentHeadValue } from "@qwik.dev/router";

import { SITE } from "../config.ts";
import { href } from "../lib/href.ts";

/**
 * The analytics tag, or nothing at all — on every public page of the
 * domain, landing and documentation alike.
 *
 * First-party, self-hosted, cookie-less and served by the domain itself
 * (D-24): the site emits one `<script>` and never touches `/u/s.js` or
 * `/u/e`, which the domain answers. `##SITE-ANALYTICS` says the
 * documentation carries the same tag with the same website id as the
 * landing, so there is one function and not two — two copies of one tag
 * would eventually disagree about which property they report to, and the
 * disagreement would be invisible until someone read the numbers.
 *
 * It is absent from the embedded build without a condition: the local
 * reader has no domain to report to, and the routes that call this are
 * the only ones that ask (R-09).
 *
 * Both values come from the site's configuration and neither is written
 * here (D-24). The id identifies a live property. The host it reports to
 * is the domain itself by default — that is what «first-party,
 * self-hosted» means, and it is why the site serves neither `/u/s.js`
 * nor `/u/e` and describes neither in its serving configuration — but it
 * is stated rather than assumed, because a preview build reports to the
 * live property while living somewhere else entirely.
 */
export function analytics(): NonNullable<DocumentHeadValue["scripts"]> {
  if (SITE.umamiWebsiteId.length === 0) return [];
  const host = SITE.umamiHostUrl.length === 0 ? SITE.origin : SITE.umamiHostUrl;
  /* Qwik types a head script by the HTML attributes it knows about, and
     `data-*` is not among them: inside JSX TypeScript waives its check
     for any hyphenated attribute, but a plain object gets no waiver.
     Composing the tag out of its two halves states what it carries
     without asserting a type over it, which an `as` would. */
  const tag = Object.assign(
    { defer: true, src: href("u/s.js") },
    {
      "data-website-id": SITE.umamiWebsiteId,
      "data-host-url": host,
    },
  );
  return [tag];
}
