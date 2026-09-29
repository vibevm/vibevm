#!/usr/bin/env node
// The static server the end-to-end tests read the built site through.
//
// Forty lines and no dependency, on purpose. What the tests need is the
// bytes the deployment will serve and the two rules a static host has to
// follow for this site — `<route>/index.html` behind a directory
// address, and a 308 for the same address without its trailing slash
// (`##SITE-TRAILING-SLASH`). A dev server would give them something
// else: transformed modules, a module graph, and its own opinion about
// routing. Then the test would be measuring the dev server.
//
// It binds 127.0.0.1 and nothing else, and it refuses any path that
// climbs out of the output directory.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve, sep } from "node:path";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", process.argv[3] ?? "dist");
const PORT = Number.parseInt(process.argv[2] ?? "4173", 10);

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".webp": "image/webp",
  ".woff2": "font/woff2",
  ".txt": "text/plain; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
};

function within(candidate) {
  const full = resolve(candidate);
  return full === ROOT || full.startsWith(ROOT + sep);
}

createServer((request, response) => {
  const url = new URL(request.url ?? "/", `http://127.0.0.1:${PORT}`);
  const path = decodeURIComponent(url.pathname);
  const file = join(ROOT, normalize(path));

  if (!within(file)) {
    response.writeHead(403).end("no");
    return;
  }

  let target = file;
  if (path.endsWith("/")) {
    target = join(file, "index.html");
  } else if (existsSync(file) && statSync(file).isDirectory()) {
    // The address of a page ends in a slash; without it the host sends
    // the reader to the address that exists.
    response.writeHead(308, { location: `${path}/${url.search}` }).end();
    return;
  }

  if (!existsSync(target) || statSync(target).isDirectory()) {
    response.writeHead(404, { "content-type": "text/plain" }).end("not found");
    return;
  }

  response.writeHead(200, {
    "content-type": TYPES[extname(target)] ?? "application/octet-stream",
    "cache-control": "no-store",
  });
  createReadStream(target).pipe(response);
}).listen(PORT, "127.0.0.1", () => {
  process.stdout.write(`serving ${ROOT} on http://127.0.0.1:${PORT}/\n`);
});
