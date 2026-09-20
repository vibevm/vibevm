import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  APPLICATION_COMMANDS,
  CONTEXT_PROTOCOL,
  applyOperation,
  parseApplicationContext,
} from "./application.mjs";

// @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-LOCAL-PREVIEW-LAUNCHER

function context(root, operation = "install") {
  const settingsRoot = join(root, "settings");
  return {
    protocol: CONTEXT_PROTOCOL,
    operation,
    application: {
      id: "vibevm-doc",
      package: { group: "org.vibevm.doc", name: "web", version: "1.0.0" },
      installerPackage: {
        group: "org.vibevm.doc",
        name: "web",
        version: "1.0.0",
      },
      commands: [...APPLICATION_COMMANDS],
    },
    settingsRoot,
    hostRoot: join(settingsRoot, "opt", "apps", "vibevm-doc"),
    registryRoot: operation === "uninstall" ? null : join(root, "registry"),
    vibeExecutable: join(root, "vibe"),
    offline: true,
  };
}

async function fixtureSource(root) {
  const source = join(root, "source");
  for (const path of [
    "tooling/local-site",
    "node_modules/ignored",
    "site/dist",
  ])
    await mkdir(join(source, ...path.split("/")), { recursive: true });
  for (const [path, body] of [
    ["package.json", "{}"],
    ["pnpm-lock.yaml", "lockfileVersion: '9.0'"],
    ["vibevm-doc.sh", "#!/bin/sh\n"],
    ["vibevm-doc.ps1", "exit 0\n"],
    ["tooling/local-site/application.mjs", "// management\n"],
    ["tooling/local-site/run.mjs", "// runner\n"],
    ["node_modules/ignored/file", "not copied"],
    ["site/dist/index.html", "not copied"],
  ])
    await writeFile(join(source, ...path.split("/")), body);
  return source;
}

test("the application context is self-pinned and strict", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-doc-context-"));
  const value = context(root);
  assert.deepEqual(parseApplicationContext(value), value);
  assert.throws(
    () => parseApplicationContext({ ...value, extra: true }),
    /missing or unknown fields/u,
  );
  assert.throws(
    () =>
      parseApplicationContext({
        ...value,
        application: { ...value.application, commands: ["other"] },
      }),
    /commands differ/u,
  );
});

test("install retains management and deploys the three launcher genres", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-doc-install-"));
  const value = parseApplicationContext(context(root));
  const sourceRoot = await fixtureSource(root);
  const reply = await applyOperation(value, { sourceRoot });
  assert.equal(reply.status, "ready");
  assert.equal(reply.management.runtime, "node");
  assert.equal((await stat(reply.management.entry)).isFile(), true);
  assert.deepEqual(
    reply.launchers
      .map((launcher) => launcher.destination.split(/[\\/]/u).at(-1))
      .sort(),
    ["vibevm-doc", "vibevm-doc.cmd", "vibevm-doc.ps1"],
  );
  assert.equal(
    reply.launchers.every((launcher) =>
      /^[a-f0-9]{64}$/u.test(launcher.sha256),
    ),
    true,
  );
  assert.equal(
    await readFile(
      join(
        value.hostRoot,
        "generations",
        reply.management.entry.split(/[\\/]/u).at(-4),
        "node_modules",
        "ignored",
        "file",
      ),
      "utf8",
    ).catch(() => "absent"),
    "absent",
  );
});

test("a local-source install dispatches back through the checkout command", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-doc-local-source-"));
  const host = join(root, "checkout");
  const registry = join(host, "vibevm", "vibepacks");
  await mkdir(registry, { recursive: true });
  await writeFile(join(host, "vibe.toml"), "[project]\nname='host'\n");
  const value = parseApplicationContext({
    ...context(root),
    registryRoot: registry,
  });
  const reply = await applyOperation(value, {
    sourceRoot: await fixtureSource(root),
  });
  const powershell = reply.launchers.find((launcher) =>
    launcher.destination.endsWith("vibevm-doc.ps1"),
  );
  assert.notEqual(powershell, undefined);
  const body = await readFile(powershell.destination, "utf8");
  assert.match(body, /run vibevm-doc --path/u);
  assert.ok(body.includes(host));
});

test("uninstall removes owned launchers and the retained host", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-doc-uninstall-"));
  const installed = parseApplicationContext(context(root));
  await applyOperation(installed, { sourceRoot: await fixtureSource(root) });
  const removed = await applyOperation(
    parseApplicationContext(context(root, "uninstall")),
  );
  assert.equal(removed.status, "undeployed");
  await assert.rejects(stat(installed.hostRoot));
  for (const name of ["vibevm-doc", "vibevm-doc.cmd", "vibevm-doc.ps1"])
    await assert.rejects(
      stat(join(installed.settingsRoot, "opt", "bin", name)),
    );
});
