import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

import {
  packageManagerEnvironment,
  parseContext,
  renderedTrees,
  siteConfig,
  siteOutput,
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
