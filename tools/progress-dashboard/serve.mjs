#!/usr/bin/env node
// Progress Control dashboard (campaign plan §7).
//
// Zero dependencies by law: node:http + node:fs only. Read-only by law:
// serves index.html and the campaign's run/state/*.json verbatim; computes
// nothing, parses no Markdown, writes nothing. Localhost only.
//
// Usage: node tools/progress-dashboard/serve.mjs [campaign-dir] [port]
//   campaign-dir defaults to the single campaigns/<id>/ under the repo root.

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..", "..");

function resolveCampaign(arg) {
  if (arg) return path.resolve(arg);
  const zone = path.join(repoRoot, "campaigns");
  const dirs = fs.existsSync(zone)
    ? fs
        .readdirSync(zone, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => path.join(zone, d.name))
    : [];
  if (dirs.length !== 1) {
    console.error(
      `expected exactly one campaign under ${zone}, found ${dirs.length}; pass the dir explicitly`,
    );
    process.exit(2);
  }
  return dirs[0];
}

const campaign = resolveCampaign(process.argv[2]);
const stateDir = path.join(campaign, "run", "state");
const resumePath = path.join(campaign, "run", "RESUME.md");
const port = Number(process.argv[3] ?? 7043);

const STATE_FILES = new Set([
  "campaign.json",
  "corpus.json",
  "findings.json",
  "tasks.json",
  "docdebt.json",
]);

const server = http.createServer((req, res) => {
  const url = new URL(req.url ?? "/", "http://localhost");
  const name = url.pathname.replace(/^\/+/, "");
  try {
    if (name === "" || name === "index.html") {
      res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      res.end(fs.readFileSync(path.join(here, "index.html")));
      return;
    }
    if (name === "resume.md") {
      res.writeHead(200, { "content-type": "text/plain; charset=utf-8" });
      res.end(fs.existsSync(resumePath) ? fs.readFileSync(resumePath) : "");
      return;
    }
    if (STATE_FILES.has(name)) {
      const p = path.join(stateDir, name);
      res.writeHead(200, { "content-type": "application/json; charset=utf-8" });
      res.end(fs.existsSync(p) ? fs.readFileSync(p) : "{}");
      return;
    }
    res.writeHead(404, { "content-type": "text/plain" });
    res.end("not found");
  } catch (e) {
    res.writeHead(500, { "content-type": "text/plain" });
    res.end(String(e));
  }
});

server.listen(port, "127.0.0.1", () => {
  console.log(`progress dashboard: http://127.0.0.1:${port}/`);
  console.log(`campaign: ${campaign}`);
  console.log(`state:    ${stateDir} (run \`vibe progress scan\` to refresh)`);
});
