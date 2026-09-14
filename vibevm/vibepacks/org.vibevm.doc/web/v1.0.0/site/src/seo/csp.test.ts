/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-SSR */

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

import {
  CSP_CONF_LIMIT,
  cspConf,
  cspPolicy,
  hashOf,
  hashesIn,
  inlineScripts,
} from "./csp.ts";

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

/**
 * The header the serving container sends is generated from the line this
 * build wrote and from nothing else. A `map` over the media type rather
 * than a header on the whole server, because the policy is one hash per
 * distinct inline script and only a document can execute one: sending
 * the list with every chunk and every font would spend kilobytes per
 * request protecting a file that cannot run anything (X-044).
 */
test("the serving fragment carries the policy on documents and nowhere else", () => {
  const policy = cspPolicy([hashOf("theme()"), hashOf("scroll()")]);
  const conf = cspConf(policy);
  assert.match(conf, /map \$sent_http_content_type \$vibe_csp \{/);
  assert.match(conf, /default\s+"";/);
  assert.match(
    conf,
    new RegExp(
      `~\\*\\^text/html\\s+"${policy.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}";`,
    ),
  );
  assert.match(conf, /\}\n$/);
});

/**
 * A policy that cannot be written as one quoted value is refused rather
 * than emitted: a configuration file that parses differently from how it
 * reads would take the whole domain down at the next restart, and the
 * cause would be a quote nobody looked at.
 */
test("a policy that cannot be quoted is refused", () => {
  assert.throws(() => cspConf('script-src "self"'), /quote/);
  assert.throws(
    () => cspConf("default-src 'self'\nscript-src 'self'"),
    /newline/,
  );
});

/**
 * The ceiling, measured in the image the site is served from: nginx
 * stops reading a parameter at 4096 bytes and refuses to start. A site
 * of one manual never reaches it; a site of the whole registry passes it
 * twenty times over on its first render, which is X-044's revision
 * trigger arriving rather than a bug.
 *
 * What the generator does about it is the only thing it can do without
 * taking somebody else's decision: it writes no policy, says so in the
 * file, and leaves the variable defined so that the domain still comes
 * up. Emitting the policy anyway would stop the server; emitting half of
 * one would block scripts the pages need.
 */
test("a policy the server cannot parse becomes no policy, loudly", () => {
  const oversized = cspPolicy(
    Array.from({ length: 200 }, (_, n) => hashOf(`script${n}()`)),
  );
  assert.ok(oversized.length > CSP_CONF_LIMIT);
  const conf = cspConf(oversized);
  assert.match(conf, /map \$sent_http_content_type \$vibe_csp \{/);
  assert.match(conf, /NO POLICY IS SERVED/);
  assert.match(conf, new RegExp(`${oversized.length}`));
  assert.equal(conf.includes("text/html"), false);
  assert.equal(hashesIn(conf).length, 0);
});

/**
 * The generator over the real thing: the `csp.txt` this package's own
 * build wrote, with every hash it found in the pages it rendered. A
 * fixture would prove the shape; only the build's own output proves that
 * what the build writes is what the generator accepts — which is the one
 * failure that would reach a reader rather than a test.
 *
 * It runs when a build has been made. A clone with nothing built has no
 * output to read, and a test that invented one would be testing the
 * fixture above a second time.
 */
test("the generator accepts the policy this package's build wrote", () => {
  const built = new URL("../../dist/csp.txt", import.meta.url);
  if (!existsSync(built)) return;
  const policy = readFileSync(built, "utf8");
  const hashes = hashesIn(policy);
  assert.ok(hashes.length > 0, "the built policy names at least one hash");
  const conf = cspConf(policy);
  for (const hash of hashes) assert.ok(conf.includes(hash));
  assert.equal(hashesIn(conf).length, hashes.length);
  /* The fragment is one map and one line inside it: a policy of a
     hundred and fifty hashes is still one value, not a hundred and
     fifty directives. */
  assert.equal(
    conf.split("\n").filter((one) => one.includes("text/html")).length,
    1,
  );
});
