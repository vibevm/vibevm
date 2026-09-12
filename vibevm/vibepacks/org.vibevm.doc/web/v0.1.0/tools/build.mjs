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
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { siteConfig } from "../site/src/config.ts";
import { packagePath, href } from "../site/src/lib/href.ts";
import { catalogueLlmsTxt, siteManifest } from "../site/src/seo/catalogue.ts";
import {
  LATEST,
  addressesOf,
  coordinateOf,
  docFileHref,
  editionsOf,
  sourceOf,
} from "../site/src/seo/editions.ts";
import { DOC_MEDIA_ENV } from "../site/src/seo/media.ts";
import { resolveTableOf, resolverPage } from "../site/src/seo/resolve.ts";
import {
  copySurfaces,
  fullCorpus,
  mediaMapOf,
  readTrees,
  treePaths,
} from "./doc-surfaces.mjs";
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

/** Every page manifest this build renders from, read as data. */
function manifests() {
  const dir = join(SITE_ROOT, "src", "fixtures");
  const files = readdirSync(dir).filter(
    (entry) => entry.startsWith("manifest") && entry.endsWith(".json"),
  );
  if (files.length === 0) {
    throw new Error(`${dir}: no page manifest to build from`);
  }
  return files.map((entry) => {
    const file = join(dir, entry);
    const manifest = JSON.parse(readFileSync(file, "utf8"));
    if (!Array.isArray(manifest.pages)) {
      throw new Error(`${file}: the manifest declares no pages array`);
    }
    return manifest;
  });
}

/**
 * Addresses the library declares.
 *
 * Every LANGUAGE carries every page of the SOURCE — an adaptation that
 * has not reached a page yet still answers at its address, with the
 * source's text and a notice, which is what makes a language never a 404
 * (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`). So the count is: one door,
 * one catalogue per adaptation, and for every edition its own package
 * page plus one page per page of the source.
 *
 * It reads the manifests itself rather than importing the site's
 * library, and that is the whole point of the gate: two independent
 * counts of the same thing. A driver that asked the application how many
 * pages it had would be asking the build to confirm its own opinion.
 */
function docAddressCount() {
  const all = manifests();
  const source = all.find((one) => one.package?.translation === undefined);
  if (source === undefined) {
    throw new Error("no source manifest: every one of them is a translation");
  }
  const adaptations = all.length - 1;
  return (
    1 + // the door at /doc/
    adaptations + // one catalogue per adapted language
    all.length * (1 + source.pages.length) // a package page plus every page
  );
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

/**
 * The documentation trees this build publishes the surfaces of, read
 * before anything is rendered.
 *
 * Before, because one thing in them is needed by the pages themselves:
 * the address of the composed card a page names in `og:image`. The
 * pipeline writes it under a content hash and the manifest does not
 * carry the name (X-042), so the build finds it in the tree and hands
 * the map to both Vite runs through the environment — the same channel
 * the origin and the analytics id arrive on (`site/src/config.ts`).
 */
const TREES = mode === "static" ? readTrees(treePaths()) : [];
const EDITIONS = editionsOf(manifests());
const MEDIA = mode === "static" ? mediaMapOf(TREES, EDITIONS) : {};

/**
 * Everything the documentation half of the domain publishes beside its
 * pages: the projections and `llms` tiers copied from the pipeline, the
 * catalogue and manifest composed from the page manifests, the sitemap
 * index, and the resolver with its table.
 *
 * One step, after the pages and before the root files, because all of it
 * describes the pages that were just written and the root files describe
 * it in turn: `robots.txt` names the sitemap, the root `llms.txt` names
 * the catalogue.
 */
function writeDocumentationSurfaces(outDirName) {
  const out = join(SITE_ROOT, outDirName);
  const config = siteConfig(process.env);
  const source = sourceOf(EDITIONS);
  const addresses = addressesOf(EDITIONS);
  const number = source.manifest.package.version;
  let written = 0;

  const write = (address, contents) => {
    const file = join(out, address.replace(/^\/+/, "").split("/").join(sep));
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, contents, "utf8");
    written += 1;
  };

  const copied = copySurfaces(TREES, EDITIONS, out);

  /* The page manifest of each edition, at both spellings of the version.
     It is the manifest the pipeline wrote, re-serialised rather than
     re-derived: the site never recomputes what Rust computed, and the
     one thing it adds is the address the bytes are served at. */
  for (const edition of EDITIONS) {
    for (const version of [number, LATEST]) {
      const base = href(
        packagePath(coordinateOf(source, edition.segment, version)),
      );
      write(
        `${base}manifest.json`,
        `${JSON.stringify(edition.manifest, null, 2)}\n`,
      );
    }
  }

  write(
    docFileHref("manifest.json"),
    `${JSON.stringify(siteManifest(EDITIONS, addresses), null, 2)}\n`,
  );
  write(
    docFileHref("llms.txt"),
    catalogueLlmsTxt(config.origin, EDITIONS, addresses),
  );
  const corpus = fullCorpus(TREES, EDITIONS, config.origin);
  if (corpus !== null) write(docFileHref("llms-full.txt"), corpus);

  write(
    docFileHref("resolve.json"),
    `${JSON.stringify(resolveTableOf(addresses), null, 2)}\n`,
  );
  write(docFileHref("resolve/index.html"), resolverPage());

  process.stdout.write(
    `build (${mode}): ${copied.files} file(s) copied from ${TREES.length} documentation tree(s) for ${copied.editions} edition(s) (${copied.fallbacks} page(s) in a language that does not carry them); ${written} written — catalogue, manifests, resolver\n`,
  );
  if (copied.unplaced.length > 0) {
    process.stdout.write(
      `build (${mode}): ${copied.unplaced.length} tree(s) the page library does not carry — their surfaces are published at their own coordinate and nothing on the site links them: ${copied.unplaced.join(", ")}\n`,
    );
  }
}

/**
 * A page that asks not to be indexed does not belong in the sitemap.
 *
 * The two say opposite things otherwise: a sitemap is «index this» and
 * `noindex` is «do not», and a crawler handed both spends a fetch to be
 * told to go away. The pages this is about are the translation
 * fallbacks — the source's text materialised under an adaptation's
 * address so a language never 404s (`##READER-LANGUAGE-SWITCH-KEEPS-
 * PLACE`) — and they carry `canonical` to the source as well, so what a
 * crawler should keep is named rather than merely implied.
 *
 * It reads the built HTML rather than being told, because that is what
 * the sitemap itself is written from, and a second list of «which pages
 * are fallbacks» would be a second thing to get wrong. It belongs to
 * whatever writes the sitemap and lives here until the two meet.
 */
function pruneSitemap(outDirName) {
  const out = join(SITE_ROOT, outDirName);
  const sitemap = join(out, "sitemap.xml");
  if (!existsSync(sitemap)) return 0;

  const xml = readFileSync(sitemap, "utf8");
  let removed = 0;
  const kept = xml.replace(/[ \t]*<url>[\s\S]*?<\/url>\n?/g, (entry) => {
    const loc = /<loc>([^<]*)<\/loc>/.exec(entry);
    if (loc === null) return entry;
    const path = new URL(loc[1]).pathname;
    const file = join(out, path.replace(/^\/+/, ""), "index.html");
    if (!existsSync(file)) return entry;
    const html = readFileSync(file, "utf8");
    if (!/<meta[^>]+name="robots"[^>]+content="[^"]*noindex/i.test(html)) {
      return entry;
    }
    removed += 1;
    return "";
  });
  if (removed > 0) writeFileSync(sitemap, kept);
  return removed;
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
      env: { ...process.env, [DOC_MEDIA_ENV]: JSON.stringify(MEDIA) },
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
  writeDocumentationSurfaces(plan.outDir);

  const roots = writeRootFiles(plan.outDir);
  process.stdout.write(
    `build (${mode}): root files ${roots.written.join(", ")}; ${roots.faces} font file(s) at /fonts/, ${roots.rewritten} page(s) repointed at the bundled faces, ${roots.crawlers} crawler name(s) from ${roots.sources} provider page(s)\n`,
  );
  const skipped = pruneSitemap(plan.outDir);
  if (skipped > 0) {
    process.stdout.write(
      `build (${mode}): ${skipped} page(s) asked not to be indexed and left the sitemap\n`,
    );
  }
}

process.stdout.write(`build (${mode}): ok\n`);
