import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

import {
  packageManagerEnvironment,
  parseContext,
  reconcileTree,
  renderedTrees,
  siteConfig,
  siteOutput,
  webInputFingerprint,
} from "./build.mjs";

test("the lifecycle envelope supplies the selected host checkout", () => {
  const root = resolve("host");
  const context = parseContext(
    JSON.stringify({ envelope: 1, project: { root } }),
  );
  assert.equal(context.project.root, root);
});

test("the derived builder configuration names only the local host", () => {
  const root = resolve("host");
  const config = siteConfig(root);
  assert.match(config, /\[source\.host\]/u);
  assert.match(config, /debounce_minutes = 0/u);
  assert.ok(config.includes(root.replaceAll("\\", "/")));
  assert.doesNotMatch(config, /source\.registry/u);
  assert.equal(siteOutput(root), join(root, ".vibe", "site-build", "site"));
});

test("package installation inherits npm config unless the extension overrides it", () => {
  const inherited = packageManagerEnvironment(
    { execution: { config: {} } },
    { HOME: "/home/developer" },
    "linux",
  );
  assert.deepEqual(inherited, { HOME: "/home/developer" });

  const overridden = packageManagerEnvironment(
    {
      execution: {
        config: { npm_registry: " https://registry.npmmirror.com " },
      },
    },
    { HOME: "/home/developer" },
    "linux",
  );
  assert.equal(
    overridden.NPM_CONFIG_REGISTRY,
    "https://registry.npmmirror.com",
  );
});

test("only the three documentation projections become site inputs", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-site-build-"));
  const slot = join(root, ".vibe-site", "trees", "org.example.docs@1.0.0");
  for (const format of ["html", "md", "xml", "scratch"]) {
    const tree = join(slot, format);
    mkdirSync(tree, { recursive: true });
    writeFileSync(join(tree, "witness"), format);
  }
  assert.deepEqual(
    renderedTrees(root).map((tree) => tree.slice(slot.length + 1)),
    ["html", "md", "xml"],
  );
});

test("site publication reuses equal files and removes stale chunks", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-site-reconcile-"));
  const staging = join(root, "next");
  const destination = join(root, "site");
  mkdirSync(join(staging, "assets"), { recursive: true });
  mkdirSync(join(destination, "assets"), { recursive: true });
  writeFileSync(join(staging, "index.html"), "new");
  writeFileSync(join(staging, "assets", "same.js"), "same");
  writeFileSync(join(destination, "index.html"), "old");
  writeFileSync(join(destination, "assets", "same.js"), "same");
  writeFileSync(join(destination, "assets", "stale.js"), "stale");

  assert.deepEqual(reconcileTree(staging, destination), {
    removed: 1,
    reused: 1,
    written: 1,
  });
  assert.equal(readFileSync(join(destination, "index.html"), "utf8"), "new");
  assert.equal(existsSync(join(destination, "assets", "stale.js")), false);
});

test("the web input fingerprint moves with a rendered document", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-site-fingerprint-"));
  writeFileSync(join(root, "page.html"), "one");
  const first = webInputFingerprint([root]);
  writeFileSync(join(root, "page.html"), "two");
  assert.notEqual(webInputFingerprint([root]), first);
});
