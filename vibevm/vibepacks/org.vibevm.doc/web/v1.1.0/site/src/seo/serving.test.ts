/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-CONTAINER-NGINX */

/**
 * What the serving configuration promises, read off the file.
 *
 * The pages of this site make promises a page cannot keep on its own. A
 * `canonical` that names a directory is a promise that the address
 * without the slash sends a reader to the one with it. A Markdown
 * projection beside every page is a promise that the projection arrives
 * as text in the reader's own encoding. A policy computed from the bytes
 * of the pages is a promise that the bytes are served under it. Every
 * one of those is kept by two lines in a configuration file, in a
 * container, on a machine this package never sees.
 *
 * So the file is read as text and each promise is asserted against it.
 * That is a weaker statement than a request and a response — the live
 * probe in the deployment atom is the strong one — and it is the
 * statement that survives: it runs on a machine with no Docker, in a
 * clone with nothing installed, in the same second as the rest of the
 * floor, and it fails on the edit that removed a rule rather than on the
 * next deploy that noticed.
 *
 * The compose file the same rules are deployed by is read beside it, in
 * `deployment.test.ts`.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const NGINX = readFileSync(
  new URL("../../../docker/nginx.conf", import.meta.url),
  "utf8",
);

/** The file with every comment line dropped: the rules, and only them. */
function rules(text: string): string {
  return text
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("#"))
    .join("\n");
}

const RULES = rules(NGINX);

/**
 * The builder keeps its state and the rendered trees inside the output
 * directory on purpose — the question they answer is «what is in THIS
 * directory» — and the cost of that decision is one rule here. Without
 * it the domain would publish the renderer's kitchen: every documentation
 * tree twice over, and a state file naming every coordinate the site has
 * ever rendered.
 */
test("the builder's own directory is not part of the domain", () => {
  assert.match(RULES, /location\s+\^~\s+\/\.vibe-site\s*\{[^}]*return\s+404/);
});

/**
 * The optimizer's manifest is build metadata no page fetches. The build
 * removes it from the output; this is the second of the two places, and
 * a hygiene rule enforced in one of two places is enforced in neither
 * (`##STACK-BUILD-HYGIENE`).
 */
test("the optimizer's manifest is answered 404 wherever it appears", () => {
  assert.match(
    RULES,
    /location\s+~\s+\/q-manifest\\\.json\$\s*\{[^}]*return\s+404/,
  );
});

/**
 * A year of immutable caching is safe for exactly one kind of file: one
 * whose address changes when its content does. Three directories hold
 * those, and the landing's rule named a fourth — `/_astro/` — which this
 * build never writes, so the rule was dead and the real assets were
 * served uncached (X-040).
 */
test("the immutable cache names the three content-addressed directories", () => {
  const cache =
    /location\s+~\s+\^\/\(assets\|build\|fonts\)\/\s*\{([^}]*)\}/.exec(RULES);
  assert.notEqual(cache, null);
  const body = cache?.[1] ?? "";
  assert.match(body, /expires\s+1y/);
  assert.match(body, /Cache-Control\s+"public,\s*immutable"/);
  assert.equal(RULES.includes("_astro"), false);
});

/**
 * An unhashed file under an unchanging address must not be immutable:
 * the social card and the icon are rewritten by every build, and a year
 * of caching would pin the old one on every share for that year.
 */
test("the unhashed root files are not cached for a year", () => {
  assert.equal(/location[^\n]*og\.png/.test(RULES), false);
  assert.equal(/\\\.\(svg\|png\|jpg/.test(RULES), false);
});

/**
 * Four formats have to say `utf-8`, and they say it in two different
 * ways. Three of them nginx already knows a media type for, so naming
 * the type in `charset_types` is enough; `.md` it does not know at all,
 * and without a type of its own the projection beside every page would
 * be sent as `application/octet-stream` — a download, and outside the
 * charset list entirely.
 */
test("every text format the site publishes declares utf-8", () => {
  assert.match(RULES, /^\s*charset\s+utf-8;/m);
  const types = /charset_types([^;]*);/.exec(RULES)?.[1] ?? "";
  for (const type of [
    "text/plain",
    "text/markdown",
    "text/xml",
    "application/json",
  ]) {
    assert.ok(types.includes(type), `charset_types names ${type}`);
  }
  assert.match(
    RULES,
    /location\s+~\s+\\\.md\$\s*\{[^}]*default_type\s+text\/markdown/,
  );
});

/**
 * TLS terminates outside this container, so nginx cannot know the scheme
 * or the port a reader arrived on. Every redirect it writes is therefore
 * relative, and the two directives that make it so are written down
 * rather than inherited from a default that could change.
 */
test("every redirect this container writes is relative", () => {
  assert.match(RULES, /absolute_redirect\s+off;/);
  assert.match(RULES, /port_in_redirect\s+off;/);
  assert.match(RULES, /location\s+=\s+\/doc\s*\{\s*return\s+301\s+\/doc\/;/);
  assert.match(RULES, /location\s+=\s+\/en\/\s*\{\s*return\s+301\s+\/;/);
  assert.match(RULES, /location\s+=\s+\/en\s+\{\s*return\s+301\s+\/;/);
});

/**
 * The one absolute address a redirect may carry is `https://`, and the
 * two that do are the install endpoints the landing's own commands name
 * out loud. A plain `http://` anywhere would be the redirect loop
 * `##SEO-CHARSET-AND-REDIRECTS` is about.
 */
test("no rule sends a reader to a plaintext address", () => {
  assert.equal(/http:\/\//.test(NGINX), false);
  assert.match(
    RULES,
    /location\s+=\s+\/install\.sh\s*\{[^}]*return\s+302\s+https:\/\//,
  );
  assert.match(
    RULES,
    /location\s+=\s+\/install\.ps1\s*\{[^}]*return\s+302\s+https:\/\//,
  );
});

/**
 * The policy comes from a file the build wrote, and the file is included
 * rather than pasted: the hashes are computed from the bytes of the
 * pages, so a hand-kept list would go stale on the first rebuild and the
 * symptom would be a blocked script in a reader's browser (X-035).
 */
test("the policy is included from the file the build generates", () => {
  assert.match(RULES, /include\s+\/etc\/nginx\/csp\.conf;/);
  assert.match(
    RULES,
    /add_header\s+Content-Security-Policy\s+\$vibe_csp\s+always;/,
  );
  assert.equal(RULES.includes("sha256-"), false);
});

/**
 * What the landing's file already did and this one keeps: the security
 * headers, the revalidation of HTML, the 404 page and the directory
 * index. Each is one line, and each is the kind of line that disappears
 * in an edit nobody reviews twice.
 */
test("the inherited rules of the landing are still here", () => {
  assert.match(RULES, /add_header\s+X-Frame-Options\s+"SAMEORIGIN"\s+always;/);
  assert.match(
    RULES,
    /add_header\s+X-Content-Type-Options\s+"nosniff"\s+always;/,
  );
  assert.match(
    RULES,
    /add_header\s+Referrer-Policy\s+"strict-origin-when-cross-origin"\s+always;/,
  );
  assert.match(RULES, /^\s*expires\s+-1;/m);
  assert.match(RULES, /error_page\s+404\s+\/404\.html;/);
  assert.match(
    RULES,
    /try_files\s+\$uri\s+\$uri\/\s+\$uri\/index\.html\s+=404;/,
  );
});

/**
 * `add_header` in a location REPLACES the inherited set rather than
 * adding to it, so every location that sets a header of its own has to
 * restate the ones it still wants. This asserts the consequence rather
 * than the rule: no location may name a `Cache-Control` and then go
 * quiet about `nosniff`.
 */
test("a location that sets a header of its own keeps the security ones", () => {
  const blocks = [...RULES.matchAll(/location[^{]*\{([^}]*)\}/g)];
  for (const block of blocks) {
    const body = block[1] ?? "";
    if (!body.includes("add_header")) continue;
    if (!body.includes("Cache-Control")) continue;
    assert.match(body, /X-Content-Type-Options/);
    assert.match(body, /Referrer-Policy/);
    assert.match(body, /X-Frame-Options/);
  }
});
