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
import { ISLAND_PLACEHOLDER } from "../site/src/lib/island-placeholder.ts";
import { catalogueLlmsTxt, siteManifest } from "../site/src/seo/catalogue.ts";
import {
  cspPolicy,
  hashOf,
  hashesIn,
  inlineScripts,
} from "../site/src/seo/csp.ts";
import {
  LATEST,
  addressesOfAll,
  coordinateOf,
  docFileHref,
  librariesOf,
} from "../site/src/seo/editions.ts";
import { PUBLIC_ONLY } from "../site/src/seo/local.ts";
import { DOC_MEDIA_ENV } from "../site/src/seo/media.ts";
import { resolveTableOf, resolverPage } from "../site/src/seo/resolve.ts";
import { sitemapOf } from "../site/src/seo/sitemap.ts";
import { islandsOf } from "./doc-library.mjs";
import {
  copySurfaces,
  fullCorpus,
  mediaMapOf,
  readTrees,
  treePaths,
} from "./doc-surfaces.mjs";
import { buildLibrary, fixtureLibrary } from "./library-source.mjs";
import { lintLinks } from "./lint-links.mjs";
import { staticOutDir } from "./out-dir.mjs";
import { writeRootFiles } from "./root-files.mjs";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SITE_ROOT = join(PACKAGE_ROOT, "site");

/** The two builds, and everything that differs between them. */
const MODES = {
  static: {
    clientConfig: "vite.config.ts",
    adapterConfig: join("adapters", "static", "vite.config.ts"),
    /** Where the Vite configuration was told to write, which the environment may move. */
    outDir: staticOutDir(),
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
 * Every page manifest this build renders from, read as data.
 *
 * The same set the Vite configuration substitutes into the bundle, out
 * of the same module — the deployment's trees for the site, the fixture
 * pair for the shell `vibe` embeds — because a gate that counted one
 * library against a build of another would be measuring nothing. What
 * stays independent is the COUNTING: below, this driver does arithmetic
 * over the manifests, while the site walks its own address map, and the
 * two numbers meet only in the comparison (`##STACK-PAGE-COUNT-GATE`).
 */
function manifests() {
  const named = mode === "static" ? buildLibrary() : fixtureLibrary();
  return named.map((one) => {
    if (!Array.isArray(one.value?.pages)) {
      throw new Error(`${one.name}: the manifest declares no pages array`);
    }
    return one.value;
  });
}

/**
 * Addresses the libraries declare.
 *
 * Every LANGUAGE carries every page of ITS SOURCE — an adaptation that
 * has not reached a page yet still answers at its address, with the
 * source's text and a notice, which is what makes a language never a 404
 * (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`). So the count is: one door,
 * one catalogue per adapted LANGUAGE of the whole site — three
 * documentations adapted into Russian are three cards on one shelf and
 * not three addresses — and, for every library, every edition's own
 * package page plus one page per page of that library's source.
 *
 * It reads the manifests itself rather than importing the site's
 * library, and that is the whole point of the gate: two independent
 * counts of the same thing. A driver that asked the application how many
 * pages it had would be asking the build to confirm its own opinion.
 */
function docAddressCount() {
  const all = manifests();
  const sources = new Map();
  for (const manifest of all) {
    if (manifest.package?.translation !== undefined) continue;
    const card = manifest.package;
    const at = `${card.group}/${card.name}`;
    if (!sources.has(at)) sources.set(at, { pages: manifest.pages.length, editions: 1 });
  }
  const languages = new Set();
  for (const manifest of all) {
    const adapts = manifest.package?.translation?.package;
    const mine = sources.get(adapts);
    if (adapts !== undefined && mine !== undefined) {
      mine.editions += 1;
      languages.add(manifest.package.lang);
      continue;
    }
    /* An adaptation whose source this build does not carry is a library
       of its own, under its own coordinate: it has no source here to be
       served behind, so it is counted as one. */
    const at = `${manifest.package.group}/${manifest.package.name}`;
    if (!sources.has(at)) {
      sources.set(at, { pages: manifest.pages.length, editions: 1 });
    }
  }
  /* Twice, because a page has two addresses: the version number and
     `latest`. They are the same content and the site says so — the
     numbered one carries `rel=canonical` to the other — but a citation
     without a version resolves to `latest` (D-06) and the version switch
     offers it, so it is an address the build has to write rather than a
     word on a page (`##SITE-CANONICAL-LATEST`). */
  const spellings = 2;
  let pages = 0;
  for (const library of sources.values()) {
    pages += library.editions * spellings * (1 + library.pages);
  }
  return (
    1 + // the door at /doc/
    languages.size + // one catalogue per adapted language of the site
    pages
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
const LIBRARIES = librariesOf(manifests());
const MEDIA = mode === "static" ? mediaMapOf(TREES, LIBRARIES) : {};

/**
 * The island of every page, put where the marker stands.
 *
 * This is the site's half of «one content path». `vibe doc build` has
 * already turned each page into finished HTML — numbered blocks,
 * resolved citations, executed examples — and the site publishes those
 * bytes unchanged; `vibe doc serve` does the same thing with the same
 * bytes at request time, into the same marker, which is what the parity
 * test measures. Neither of them re-renders a page, because a second
 * renderer disagrees with the first the first time either changes.
 *
 * It is done to the generated FILES rather than through the framework
 * on purpose. A page handed to the framework as a prop is serialised
 * into the document a second time, in the state the framework writes at
 * the end of it — the manual's pages average twenty-five kilobytes, so
 * that is the whole documentation shipped twice — and it would put every
 * island of every edition into the bundle that any one of them is
 * rendered from.
 *
 * Only the first occurrence is replaced, for the same reason the server
 * replaces only the first: the hole in the document comes before the
 * serialised state that mentions the marker as a string (finding P4-O4,
 * anomaly A-4).
 *
 * A page whose tree carries no island keeps the package's own fixture
 * page. A build given no tree is exactly that case for every address,
 * which is how a clone with nothing installed still renders a site whose
 * pages have something in them.
 */
function fillIslands(outDirName) {
  const out = join(SITE_ROOT, outDirName);
  const islands = islandsOf(TREES, LIBRARIES);
  const standIn = readFileSync(
    join(SITE_ROOT, "src", "fixtures", "island.html"),
    "utf8",
  );
  let rendered = 0;
  let fixture = 0;
  for (const file of htmlFiles(out)) {
    const html = readFileSync(file, "utf8");
    const at = html.indexOf(ISLAND_PLACEHOLDER);
    if (at === -1) continue;
    const address = `/${file
      .slice(out.length + 1)
      .split(sep)
      .join("/")}`.replace(/index\.html$/, "");
    const island = islands.get(address);
    if (island === undefined) fixture += 1;
    else rendered += 1;
    writeFileSync(
      file,
      `${html.slice(0, at)}${island ?? standIn}${html.slice(at + ISLAND_PLACEHOLDER.length)}`,
      "utf8",
    );
  }
  process.stdout.write(
    `build (${mode}): ${rendered} island(s) from the documentation trees, ${fixture} from the package's own fixture page
`,
  );
}

/** Every `.html` under a directory, as absolute paths. */
function htmlFiles(dir, found = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) htmlFiles(full, found);
    else if (entry.endsWith(".html")) found.push(full);
  }
  return found;
}

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
  const addresses = addressesOfAll(LIBRARIES);
  let written = 0;

  const write = (address, contents) => {
    const file = join(out, address.replace(/^\/+/, "").split("/").join(sep));
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, contents, "utf8");
    written += 1;
  };

  const copied = copySurfaces(TREES, LIBRARIES, out);

  /* The page manifest of each edition, at both spellings of the version.
     It is the manifest the pipeline wrote, re-serialised rather than
     re-derived: the site never recomputes what Rust computed, and the
     one thing it adds is the address the bytes are served at. */
  for (const library of LIBRARIES) {
    const source = library.source;
    const number = source.manifest.package.version;
    for (const edition of library.editions) {
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
  }

  write(
    docFileHref("manifest.json"),
    `${JSON.stringify(siteManifest(LIBRARIES), null, 2)}\n`,
  );
  write(docFileHref("llms.txt"), catalogueLlmsTxt(config.origin, LIBRARIES));
  const corpus = fullCorpus(TREES, LIBRARIES, config.origin);
  if (corpus !== null) write(docFileHref("llms-full.txt"), corpus);

  const sitemap = sitemapOf(config.origin, LIBRARIES);
  write(docFileHref("sitemap.xml"), sitemap.index);
  for (const part of sitemap.parts) write(part.href, part.xml);

  write(
    docFileHref("resolve.json"),
    `${JSON.stringify(resolveTableOf(addresses), null, 2)}\n`,
  );
  write(docFileHref("resolve/index.html"), resolverPage());

  process.stdout.write(
    `build (${mode}): ${copied.files} file(s) copied from ${TREES.length} documentation tree(s) for ${copied.editions} edition(s) of ${LIBRARIES.length} librar${LIBRARIES.length === 1 ? "y" : "ies"} (${copied.fallbacks} page(s) in a language that does not carry them); ${written} written — catalogue, manifests, ${sitemap.parts.length} sitemap part(s) over ${sitemap.addresses} address(es), resolver\n`,
  );
  if (copied.unplaced.length > 0) {
    process.stdout.write(
      `build (${mode}): ${copied.unplaced.length} tree(s) the page library does not carry — their surfaces are published at their own coordinate and nothing on the site links them: ${copied.unplaced.join(", ")}\n`,
    );
  }
}

/**
 * The embedded output, read back for what it must not contain.
 *
 * `##SEO-LOCAL-EXEMPT` is a promise about pages on a reader's own
 * machine: they publish none of the public head. The head builder makes
 * that true by never building it, and this says so about the bytes —
 * which is the only place the promise can actually be broken. The
 * patterns are the ones `seo/local.ts` keeps beside the builders, so the
 * statement and its check cannot drift apart.
 */
function checkLocalHead(outDirName) {
  const out = join(SITE_ROOT, outDirName);
  const offences = [];
  const pages = htmlFiles(out);
  for (const file of pages) {
    const html = readFileSync(file, "utf8");
    for (const rule of PUBLIC_ONLY) {
      if (rule.pattern.test(html)) {
        offences.push(`${file.slice(out.length + 1)} carries ${rule.what}`);
      }
    }
  }
  if (offences.length > 0) {
    process.stderr.write(
      [
        `build (${mode}): the local reader's pages publish what only the domain may:`,
        ...offences.map((one) => `  ${one}`),
        "",
        "A page served from a machine's own store must not declare itself a",
        "copy of a page on the public site (##SEO-LOCAL-EXEMPT).",
        "",
      ].join("\n"),
    );
    process.exit(1);
  }
  process.stdout.write(
    `build (${mode}): ${pages.length} page(s) carry none of the ${PUBLIC_ONLY.length} public-only tags\n`,
  );
}

/**
 * The policy line, from the bytes that were actually written.
 *
 * Every inline script of every page is hashed and the union goes into
 * `dist/csp.txt` for the deployment atom to serve as a header (X-035).
 * The set is then read back off the file and compared against a second
 * pass over the output: the check is worth its two seconds because the
 * failure it guards against is silent — a page whose script is missing
 * from the policy does not break at build time, it breaks in a browser
 * that has already been served the page.
 */
function writeCsp(outDirName) {
  const out = join(SITE_ROOT, outDirName);
  const scripts = new Map();
  for (const file of htmlFiles(out)) {
    for (const script of inlineScripts(readFileSync(file, "utf8"))) {
      const hash = hashOf(script.body);
      scripts.set(hash, (scripts.get(hash) ?? 0) + 1);
    }
  }
  const hashes = [...scripts.keys()].sort();
  writeFileSync(join(out, "csp.txt"), `${cspPolicy(hashes)}\n`, "utf8");

  const named = new Set(hashesIn(readFileSync(join(out, "csp.txt"), "utf8")));
  const again = new Set();
  for (const file of htmlFiles(out)) {
    for (const script of inlineScripts(readFileSync(file, "utf8"))) {
      again.add(hashOf(script.body));
    }
  }
  const missing = [...again].filter((hash) => !named.has(hash));
  const extra = [...named].filter((hash) => !again.has(hash));
  if (missing.length > 0 || extra.length > 0) {
    process.stderr.write(
      `build (${mode}): csp.txt names ${named.size} hash(es); the output carries ${again.size} (${missing.length} unnamed, ${extra.length} named but absent)\n`,
    );
    process.exit(1);
  }
  const total = [...scripts.values()].reduce((sum, count) => sum + count, 0);
  process.stdout.write(
    `build (${mode}): csp.txt — ${hashes.length} inline script hash(es) over ${total} occurrence(s), no external source\n`,
  );
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
//
//    The colour is taken off first. A parent that sets `FORCE_COLOR` —
//    a test runner, a CI — makes Vite wrap the number in escape codes,
//    and a gate that then read «0 pages» from a build that made
//    twenty-four would be a red with no cause anywhere near it.
const ESCAPE = String.fromCharCode(27);
const plain = ssg.replaceAll(new RegExp(`${ESCAPE}\\[[0-9;]*m`, "g"), "");
const reported = /- Generated:\s+(\d+)\s+page/.exec(plain);
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

// 6. The machine files at the root of the domain: `robots.txt`, the two
//    llms indexes, the sitemap, the feed, the IndexNow key file, the
//    social card and the public copies of the fonts. They describe the
//    pages that were just built, so they are written from what is on
//    disk rather than kept by hand (`##SITE-ONE-SITE`).
//
//    Only for the build that serves a domain. The embedded output is a
//    route template `vibe` fills in on a reader's own machine: it has no
//    domain to describe, and a sitemap of it would name addresses that
//    exist nowhere.
// 5. The island of every page, where the marker stands. Before every
//    step that reads the output: the policy hashes what a page carries,
//    the link check follows what a page links to, and until this runs a
//    page carries and links to nothing.
if (mode === "static") {
  fillIslands(plan.outDir);
}

if (plan.countsLanding) {
  writeDocumentationSurfaces(plan.outDir);

  const roots = writeRootFiles(plan.outDir);
  process.stdout.write(
    `build (${mode}): root files ${roots.written.join(", ")}; ${roots.faces} font file(s) at /fonts/, ${roots.rewritten} page(s) repointed at the bundled faces, ${roots.crawlers} crawler name(s) from ${roots.sources} provider page(s)\n`,
  );
  if (roots.unindexed > 0) {
    process.stdout.write(
      `build (${mode}): ${roots.unindexed} page(s) asked not to be indexed and left a sitemap\n`,
    );
  }

  writeCsp(plan.outDir);

  if (lintLinks(plan.outDir) !== 0) {
    process.stderr.write(
      `build (${mode}): the link check is red; the output is not publishable\n`,
    );
    process.exit(1);
  }
} else {
  // 7. The other build is read back for the opposite reason: a page of
  //    the local reader must publish none of the public head.
  checkLocalHead(plan.outDir);
}

process.stdout.write(`build (${mode}): ok\n`);
