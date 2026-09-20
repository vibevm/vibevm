import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { request } from "node:http";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  createStaticServer,
  nativeStaticRoot,
  parseArguments,
  requestFile,
  staticRoot,
} from "./run.mjs";

// @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-LOCAL-PREVIEW-LAUNCHER

test("arguments are strict and preserve the loopback defaults", () => {
  assert.deepEqual(parseArguments([]), {
    build: true,
    install: true,
    port: 4322,
  });
  assert.deepEqual(
    parseArguments(["--port", "0", "--no-build", "--no-install"]),
    {
      build: false,
      install: false,
      port: 0,
    },
  );
  assert.throws(() => parseArguments(["--port", "65536"]), /0 through 65535/u);
  assert.throws(() => parseArguments(["--unknown"]), /unknown argument/u);
});

test("output and request paths stay inside the static tree", () => {
  const packageRoot = resolve("package-root");
  const siteRoot = resolve("site-root");
  assert.equal(staticRoot(packageRoot, {}), join(packageRoot, "site", "dist"));
  assert.equal(nativeStaticRoot(packageRoot), null);
  assert.equal(
    staticRoot(packageRoot, { VIBE_SITE_DIST: "preview" }),
    join(packageRoot, "site", "preview"),
  );
  assert.equal(
    requestFile(siteRoot, "/why/vibevm/"),
    join(siteRoot, "why", "vibevm", "index.html"),
  );
  assert.equal(requestFile(siteRoot, "/"), join(siteRoot, "index.html"));
  assert.equal(requestFile(siteRoot, "/%2e%2e%2foutside"), null);
});

test("the native output is selected only for the monorepo source slot", async () => {
  const host = await mkdtemp(join(tmpdir(), "vibevm-doc-host-"));
  const packageRoot = join(
    host,
    "vibevm",
    "vibepacks",
    "org.vibevm.doc",
    "web",
    "v1.0.0",
  );
  await mkdir(packageRoot, { recursive: true });
  await writeFile(join(host, "vibe.toml"), "[project]\nname='host'\n");
  assert.equal(
    nativeStaticRoot(packageRoot),
    join(host, ".vibe", "site-build", "site"),
  );
});

test("the server answers a prerendered route and its 404", async () => {
  const root = await mkdtemp(join(tmpdir(), "vibevm-doc-site-"));
  await mkdir(join(root, "vision"), { recursive: true });
  await writeFile(join(root, "vision", "index.html"), "<h1>Vision</h1>");
  await writeFile(join(root, "404.html"), "<h1>Missing</h1>");
  const server = createStaticServer(root);
  await new Promise((accept) => server.listen(0, "127.0.0.1", accept));
  const address = server.address();
  assert.notEqual(address, null);
  const port = typeof address === "object" ? address.port : 0;
  const get = (path) =>
    new Promise((accept, reject) => {
      const call = request({ host: "127.0.0.1", port, path }, (response) => {
        let body = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => (body += chunk));
        response.on("end", () => accept({ status: response.statusCode, body }));
      });
      call.once("error", reject);
      call.end();
    });
  assert.deepEqual(await get("/vision/"), {
    status: 200,
    body: "<h1>Vision</h1>",
  });
  assert.deepEqual(await get("/missing/"), {
    status: 404,
    body: "<h1>Missing</h1>",
  });
  await new Promise((accept, reject) =>
    server.close((error) => (error ? reject(error) : accept())),
  );
});
