/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-SSR */

import assert from "node:assert/strict";
import test from "node:test";

import { cspPolicy, hashOf, hashesIn, inlineScripts } from "./csp.ts";

/**
 * What the policy is FOR is the difference between a script a browser
 * runs and a block of data that happens to be spelled `<script>`. The
 * framework writes a page's resumability state into elements of the
 * second kind, and a hash of one of those is a hash no browser ever
 * looks for — a policy that grew by three entries per page and protected
 * nothing.
 */
test("a data block is not a script, and an external one is not inline", () => {
  const html = [
    '<script type="qwik/state">[1,2,3]</script>',
    '<script type="application/ld+json">{"@type":"Thing"}</script>',
    '<script src="/build/app.js" type="module"></script>',
    "<script>theme()</script>",
    '<script type="module">start()</script>',
    '<script type="text/javascript">legacy()</script>',
  ].join("");
  assert.deepEqual(
    inlineScripts(html).map((one) => one.body),
    ["theme()", "start()", "legacy()"],
  );
});

/**
 * The tag ends at a `>` that is not inside a quoted value. The framework
 * writes a head script's own source into an attribute of the same
 * element, so a scan that stopped at the first `>` would cut the body in
 * half and hash the half — and the page would then be blocked by the
 * policy that was computed from it.
 */
test("a `>` inside an attribute does not end the tag", () => {
  const html = '<script data-note="a > b" type="module">run()</script>';
  assert.deepEqual(
    inlineScripts(html).map((one) => one.body),
    ["run()"],
  );
});

test("the hash is over the bytes, and the policy names it once", () => {
  const body = "(function(){})()";
  const hash = hashOf(body);
  assert.match(hash, /^'sha256-[A-Za-z0-9+/=]+'$/);
  assert.equal(hashOf(body), hash);
  assert.notEqual(hashOf(`${body} `), hash);

  const policy = cspPolicy([hash]);
  assert.deepEqual(hashesIn(policy), [hash]);
  assert.match(policy, /script-src 'self' 'sha256-/);
});

/**
 * No external source anywhere in it: `default-src 'self'` is the floor
 * and every narrower directive names `'none'` or `'self'`. A policy that
 * let one host in would be a policy that documented an exception nobody
 * asked for (R-09).
 */
test("the policy names no host at all", () => {
  const policy = cspPolicy([hashOf("x")]);
  assert.equal(/https?:/.test(policy), false);
  assert.match(policy, /default-src 'self'/);
  assert.match(policy, /object-src 'none'/);
  assert.match(policy, /frame-ancestors 'none'/);
});
