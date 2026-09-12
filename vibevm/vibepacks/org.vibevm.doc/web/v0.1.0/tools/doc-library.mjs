#!/usr/bin/env node
// The page library, read out of the documentation trees a deployment
// names in `VIBE_DOC_OUT` (PROP-057 `##SITE-MOUNT`).
//
// Until this module existed the site rendered ONE fixture page into
// every address and built its shelves from two fixture manifests kept
// beside the source. The trees were copied past the pages as agent
// surfaces and nothing else: a build over the real manual published
// forty-eight Markdown files and not one of the pages they mirror, and
// said so in a line nobody could act on — «1 tree(s) the page library
// does not carry».
//
// Two things are read here and nothing else is decided:
//
//   1. the MANIFEST of each tree, which is the edition — its card, its
//      pages, its language, its officiality. The site never recomputes
//      what `vibe doc build` computed (`##PIPE-SHELL-PARSES-NOTHING`);
//      it re-serialises it and puts it at an address.
//   2. the ISLAND of each page, which is the `<document>/index.html` the
//      `--format html` run wrote. It is the finished page — numbered
//      blocks, resolved citations, executed examples — and the site's
//      part is to publish those bytes unchanged, which is the same
//      thing `vibe doc serve` does with the same bytes at request time.
//
// Both are looked for in EVERY tree of a coordinate. `vibe doc build`
// writes one projection per run, so a deployment that publishes the
// islands, the Markdown and the XML of one package names three
// directories that share a coordinate — and only one of them carries
// the islands (finding P4-O3, anomaly A-3).
//
// A build given no tree, or trees that carry no manifest, gets nothing
// from here and falls back to the fixture library the package keeps —
// which is what makes `pnpm build:static` on a fresh clone produce a
// complete site to measure.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { sourceOf } from "../site/src/seo/editions.ts";
import { fileIn, treesOf } from "./doc-surfaces.mjs";

/** What a `vibe doc build` tree calls its card and page list. */
const MANIFEST_FILE = "manifest.json";

/** What it calls a rendered page, under the document's own directory. */
const ISLAND_FILE = "index.html";

/**
 * The manifest of one tree, or nothing when that run wrote none.
 *
 * A projection-only run — `--format md` beside `--format html` — still
 * writes one, so this is rarely empty; a tree assembled by hand may be.
 */
function manifestIn(tree) {
  const file = join(tree.root, MANIFEST_FILE);
  if (!existsSync(file)) return undefined;
  const parsed = JSON.parse(readFileSync(file, "utf8"));
  if (parsed?.package === undefined || !Array.isArray(parsed.pages)) {
    throw new Error(
      `${file}: not a page manifest — no package, or no pages array`,
    );
  }
  return parsed;
}

/**
 * The editions the given trees describe: one manifest per coordinate,
 * in the order the trees were named.
 *
 * Deduplicated by coordinate rather than by bytes, because the three
 * runs of one package write three manifests that differ in exactly one
 * field — `rendered_at`, the instant of the run — and a library with the
 * same edition three times would materialise every address three times.
 * The first tree named wins, which is the order a deployment wrote.
 *
 * An empty result means «this build was given no library», never «this
 * library is empty»: the caller falls back to the fixtures, and a build
 * that silently rendered nothing would be the failure this distinction
 * exists to prevent.
 */
export function libraryManifests(trees) {
  const byCoordinate = new Map();
  for (const tree of trees) {
    const manifest = manifestIn(tree);
    if (manifest === undefined) continue;
    const card = manifest.package;
    const at = `${card.group}/${card.name}`;
    if (!byCoordinate.has(at)) byCoordinate.set(at, manifest);
  }
  return [...byCoordinate.values()];
}

/** The island of one document inside a tree of one coordinate. */
function islandRelative(tree, document) {
  return `${tree.prefix}${document}/${ISLAND_FILE}`;
}

/**
 * Every page address of the library, with the island bytes behind it.
 *
 * The rules are the ones the surfaces are copied by, because they are
 * the same rules: an edition is rendered from its own tree; a page it
 * does not carry shows the SOURCE's island under the edition's address,
 * which is the fallback that keeps a language from ever being a 404
 * (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`); and both spellings of the
 * version get the same bytes, because they are two addresses of one
 * page (`##SITE-CANONICAL-LATEST`).
 *
 * Whether an edition «carries» a page is asked of the ISLAND and not of
 * the Markdown beside it. A deployment may name only the `--format html`
 * tree, and then no `.md` exists anywhere — a test written against the
 * wrong file would report every page of that build as a translation
 * fallback.
 */
export function islandsOf(trees, editions, addresses) {
  const source = sourceOf(editions);
  const sourceTrees = treesOf(trees, source);
  const found = new Map();
  for (const address of addresses) {
    if (address.kind !== "page" || address.document === undefined) continue;
    const own = treesOf(trees, address.edition);
    const carries = own.some((tree) =>
      tree.files.includes(islandRelative(tree, address.document)),
    );
    const from = carries ? own : sourceTrees;
    const prefix = from[0]?.prefix;
    if (prefix === undefined) continue;
    const file = fileIn(from, `${prefix}${address.document}/${ISLAND_FILE}`);
    if (file === undefined) continue;
    found.set(address.href, readFileSync(file, "utf8"));
  }
  return found;
}

