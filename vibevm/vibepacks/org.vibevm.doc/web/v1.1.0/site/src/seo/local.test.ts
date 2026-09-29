/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LOCAL-EXEMPT */

import assert from "node:assert/strict";
import test from "node:test";

import { PUBLIC_ONLY, localHead } from "./local.ts";

/**
 * A page of the local reader declares no head of its own, and that is a
 * statement about what the embedded build IS rather than about SEO.
 *
 * One route template is dressed over every page of every package, so a
 * title or a projection link prerendered into it names the fixture the
 * shell was built against. The reader used to correct those bytes on the
 * way out — and an element inserted into the head of a resumed Qwik
 * container stops resumption dead (`Code(Q27)`), which takes the table of
 * contents, the anchors, the settings and the manifest read with it. With
 * no `<title>` in the template both of the server's edits are no-ops by
 * their own terms, and the shell appends the title and the links itself
 * once it knows which page it is showing.
 */
test("a page of the local reader declares no head of its own", () => {
  const head = localHead();
  assert.equal(head.title, undefined, "no title for the server to rewrite");
  assert.equal(head.links, undefined, "no links for the server to replace");
  assert.equal(head.meta, undefined);
  assert.equal(head.scripts, undefined);
  assert.deepEqual(Object.keys(head), []);
});

/**
 * And a head with nothing in it publishes none of the public head, which
 * is the whole of `##SEO-LOCAL-EXEMPT` by construction: a page read out
 * of a machine's own store cannot declare itself a copy of a page on a
 * public domain, name addresses on that domain, describe itself to an
 * index nobody submitted it to, or report to an analytics property.
 */
test("a local page publishes none of the public head", () => {
  const head = localHead();
  const links = head.links ?? [];
  assert.equal(
    links.some((link) => link.rel === "canonical"),
    false,
    "no canonical",
  );
  assert.equal(
    links.some((link) => "hreflang" in link),
    false,
    "no hreflang",
  );
  const meta = head.meta ?? [];
  assert.equal(
    meta.some((tag) => (tag.property ?? "").startsWith("og:")),
    false,
    "no Open Graph",
  );
  assert.equal(
    meta.some((tag) => (tag.name ?? "").startsWith("twitter:")),
    false,
    "no Twitter card",
  );
  assert.equal(
    meta.some((tag) => tag.name === "robots"),
    false,
    "no robots directive",
  );
  assert.equal(head.scripts, undefined, "no structured data, no analytics");
});

/**
 * The patterns the build runs over the embedded output are checked
 * against the shapes they name.
 *
 * A gate whose pattern has quietly stopped matching is worse than no
 * gate: it reports green over an output it is no longer reading. So each
 * one is shown the tag it exists to find, and a page that carries none
 * of them.
 */
test("every pattern the embedded gate uses finds the tag it names", () => {
  const public_ = [
    '<link rel="canonical" href="/doc/g.h/n/latest/page/">',
    '<link rel="alternate" hreflang="ru" href="https://vibevm.org/doc/ru/">',
    '<meta property="og:image" content="https://vibevm.org/og.png">',
    '<meta name="twitter:card" content="summary_large_image">',
    '<script type="application/ld+json">{}</script>',
    '<meta name="robots" content="noindex">',
    '<script defer src="/u/s.js" data-website-id="x" data-host-url="y"></script>',
  ];
  assert.equal(PUBLIC_ONLY.length, public_.length);
  PUBLIC_ONLY.forEach((rule, index) => {
    assert.equal(
      rule.pattern.test(public_[index] ?? ""),
      true,
      `${rule.what} is not matched by its own pattern`,
    );
  });

  const local =
    '<title>Versions</title><meta name="description" content="A version is a promise.">' +
    '<link rel="alternate" type="text/markdown" href="/doc/g.h/n/1.0/page.md">';
  for (const rule of PUBLIC_ONLY) {
    assert.equal(
      rule.pattern.test(local),
      false,
      `${rule.what} on a local page`,
    );
  }
});
