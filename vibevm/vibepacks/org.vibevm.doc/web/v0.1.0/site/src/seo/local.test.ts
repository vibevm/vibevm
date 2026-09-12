/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LOCAL-EXEMPT */

import assert from "node:assert/strict";
import test from "node:test";

import { PUBLIC_ONLY, localPageHead, localShelfHead } from "./local.ts";

const PROJECTIONS = [
  { rel: "alternate", type: "text/markdown", href: "/doc/g.h/n/1.0/page.md" },
  {
    rel: "alternate",
    type: "application/xml",
    href: "/doc/g.h/n/1.0/page.xml",
  },
  { rel: "alternate", type: "text/plain", href: "/doc/g.h/n/1.0/llms.txt" },
];

/**
 * What a local page keeps is what a reader and their agent use: the
 * title, the page's own summary, and the three files beside it.
 */
test("a local page keeps its title, its summary and its machine surfaces", () => {
  const head = localPageHead(
    "Versions and updates",
    "A version is a promise.",
    [...PROJECTIONS],
  );
  assert.equal(head.title, "Versions and updates");
  assert.deepEqual(head.meta, [
    { name: "description", content: "A version is a promise." },
  ]);
  assert.deepEqual(head.links, PROJECTIONS);
  assert.equal(head.links?.length, 3);
});

/**
 * And what it does not keep is the whole of `##SEO-LOCAL-EXEMPT`. A page
 * read out of a machine's own store must not declare itself a copy of a
 * page on a public domain, name addresses on that domain, describe
 * itself to an index nobody submitted it to, or report to an analytics
 * property.
 */
test("a local page publishes none of the public head", () => {
  for (const head of [
    localPageHead("Title", "Summary", [...PROJECTIONS]),
    localShelfHead("Fixture Manual", "What it covers.", [
      { rel: "alternate", type: "text/plain", href: "/doc/g.h/n/1.0/llms.txt" },
    ]),
  ]) {
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
  }
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
