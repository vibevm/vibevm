#!/usr/bin/env node
// `pnpm build:static` / `pnpm build:embedded` — the two adapters, and the
// gate that decides whether either of them actually built the site.
//
// The gate exists because the static generator cannot be trusted to fail.
// Measured on this pinned beta: `trailingSlash: false` produced one page
// out of seven, and a wrong base produced none at all with an empty
// sitemap — both with exit code 0 and no message. A build step that reads
// the exit code and stops there would have shipped an empty site twice.
//
// So the number the generator prints is compared against the number of
// addresses the page manifests declare, plus the landing routes found on
// disk. Two independent counts of the same thing: if the generator
// quietly stops producing pages, they disagree, and the build is red
// (PROP-057 `##STACK-PAGE-COUNT-GATE`).
//
// The hygiene pass afterwards removes `q-manifest.json` — 54 kB of
// optimizer metadata that no page requests and that has no business
// being served (`##STACK-BUILD-HYGIENE`).

import { spawnSync } from "node:child_process";
import {
  existsSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { writeRootFiles } from "./root-files.mjs";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SITE_ROOT = join(PACKAGE_ROOT, "site");

/** The two builds, and everything that differs between them. */
const MODES = {
  static: {
    clientConfig: "vite.config.ts",
    adapterConfig: join("adapters", "static", "vite.config.ts"),
    outDir: "dist",
    /** The landing is part of the site build and not of the embedded one. */
    countsLanding: true,
  },
  embedded: {
    clientConfig: "vite.config.embedded.ts",
    adapterConfig: join("adapters", "embedded", "vite.config.ts"),
    outDir: "dist-embedded",
    countsLanding: false,
  },
};

const mode = process.argv[2];
const plan = Object.prototype.hasOwnProperty.call(MODES, mode ?? "")
  ? MODES[mode]
  : undefined;
if (plan === undefined) {
  process.stderr.write(
    `build: expected one of ${Object.keys(MODES).join(", ")}, got ${mode ?? "nothing"}\n`,
  );
  process.exit(2);
}

/**
 * Addresses the library declares: for every edition its own package page
 * and each page it carries, plus the one catalogue at `/doc/`.
 *
 * It reads the manifests itself rather than importing the site's
 * library, and that is the whole point of the gate: two independent
 * counts of the same thing. A driver that asked the application how many
 * pages it had would be asking the build to confirm its own opinion.
 */
function docAddressCount() {
  const dir = join(SITE_ROOT, "src", "fixtures");
  const manifests = readdirSync(dir).filter(
    (entry) => entry.startsWith("manifest") && entry.endsWith(".json"),
  );
  if (manifests.length === 0) {
    throw new Error(`${dir}: no page manifest to build from`);
  }
  let addresses = 1; // the catalogue at /doc/
  for (const entry of manifests) {
    const file = join(dir, entry);
    const manifest = JSON.parse(readFileSync(file, "utf8"));
    if (!Array.isArray(manifest.pages)) {
      throw new Error(`${file}: the manifest declares no pages array`);
    }
    addresses += 1 + manifest.pages.length;
  }
  return addresses;
}

/**
 * Landing routes found on disk: every page module under `src/routes`
 * that is not inside the documentation tree. Counted rather than listed,
 * so a route added tomorrow raises the expected number by itself.
 *
 * A page module is `index` or `404`, optionally followed by `@<layout>`
 * — Qwik Router's way of saying which layout the route wears, and the
 * way the landing addresses ask for their own chrome instead of the
 * documentation's. The `404` module is a page like any other here: the
 * router builds it to `/404.html`, which is the file `error_page 404`
 * has always served.
 */
const PAGE_MODULE = /^(index|404)(@[^.]+)?\.tsx$/;

function landingRouteCount() {
  const routes = join(SITE_ROOT, "src", "routes");
  const doc = join(routes, "doc");
  let found = 0;
  const walk = (dir) => {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      if (statSync(full).isDirectory()) {
        if (full === doc) continue;
        walk(full);
      } else if (PAGE_MODULE.test(entry)) {
        found += 1;
      }
    }
  };
  walk(routes);
  return found;
}

function vite(configRelative) {
  const bin = join(PACKAGE_ROOT, "node_modules", "vite", "bin", "vite.js");
  const run = spawnSync(
    process.execPath,
    [bin, "build", "-c", configRelative],
    {
      cwd: SITE_ROOT,
      encoding: "utf8",
      shell: false,
    },
  );
  process.stdout.write(run.stdout ?? "");
  process.stderr.write(run.stderr ?? "");
  if (run.status !== 0) {
    process.stderr.write(
      `build: vite build -c ${configRelative} exited ${run.status}\n`,
    );
    process.exit(run.status ?? 1);
  }
  return `${run.stdout ?? ""}${run.stderr ?? ""}`;
}

// 1. The client build, then the adapter build that renders the pages.
vite(plan.clientConfig);
const ssg = vite(plan.adapterConfig);

// 2. What the generator says it made. An absent line is itself a
//    failure: the generator prints nothing at all when it renders
//    nothing, which is the silent case this gate is here for.
const reported = /- Generated:\s+(\d+)\s+page/.exec(ssg);
const generated = reported === null ? 0 : Number.parseInt(reported[1], 10);

// 3. What the site says it has.
const expected =
  docAddressCount() + (plan.countsLanding ? landingRouteCount() : 0);

process.stdout.write(
  `\nbuild (${mode}): generated ${generated} page(s), expected ${expected}\n`,
);
if (generated !== expected) {
  process.stderr.write(
    [
      `build (${mode}): the page count does not match.`,
      "",
      `  the generator reported : ${generated}`,
      `  the library declares   : ${docAddressCount()}`,
      plan.countsLanding
        ? `  landing routes on disk : ${landingRouteCount()}`
        : "  landing routes        : not in this build",
      "",
      "The generator under-generates silently and exits 0, so this",
      "comparison — not the exit code — is what says the site was built.",
      "Look at the base path and at `trailingSlash` first: both produce",
      "exactly this symptom.",
      "",
    ].join("\n"),
  );
  process.exit(1);
}

// 4. Hygiene: the optimizer's manifest is build metadata, not a page
//    asset, and nothing on the site fetches it.
const stray = join(SITE_ROOT, plan.outDir, "q-manifest.json");
if (existsSync(stray)) {
  rmSync(stray);
  process.stdout.write(
    `build (${mode}): removed ${plan.outDir}/q-manifest.json from the output\n`,
  );
}

// 5. The machine files at the root of the domain: `robots.txt`, the two
//    llms indexes, the sitemap, the feed, the IndexNow key file, the
//    social card and the public copies of the fonts. They describe the
//    pages that were just built, so they are written from what is on
//    disk rather than kept by hand (`##SITE-ONE-SITE`).
//
//    Only for the build that serves a domain. The embedded output is a
//    route template `vibe` fills in on a reader's own machine: it has no
//    domain to describe, and a sitemap of it would name addresses that
//    exist nowhere.
if (plan.countsLanding) {
  const roots = writeRootFiles(plan.outDir);
  process.stdout.write(
    `build (${mode}): root files ${roots.written.join(", ")}; ${roots.faces} font file(s) at /fonts/, ${roots.rewritten} page(s) repointed at the bundled faces, ${roots.crawlers} crawler name(s) from ${roots.sources} provider page(s)\n`,
  );
}

process.stdout.write(`build (${mode}): ok\n`);
