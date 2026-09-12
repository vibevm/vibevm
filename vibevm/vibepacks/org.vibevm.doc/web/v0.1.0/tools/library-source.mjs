#!/usr/bin/env node
// Which manifests a build renders from, decided once for the two places
// that have to agree about it (PROP-057 `##SITE-MOUNT`).
//
// Those two are the Vite configuration, which substitutes the manifests
// into the bundle so the pages can be rendered from them, and the build
// driver, which counts the addresses they declare against the number the
// static generator reports. They must read the same set or the page-count
// gate compares a build against a different library
// (`##STACK-PAGE-COUNT-GATE`); they must still COUNT independently, and
// they do — the driver does arithmetic over the manifests while the site
// walks its own address map.
//
// The decision itself is one line: the documentation trees a deployment
// named in `VIBE_DOC_OUT`, and the package's own fixture pair when it
// named none. The fixtures are not a stand-in for a missing feature —
// they are what makes a clone with nothing installed build a complete
// site, with a language selector, a shelf, a fallback and every link
// answered.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { libraryManifests } from "./doc-library.mjs";
import { TREES_ENV, readTrees, treePaths } from "./doc-surfaces.mjs";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURES = join(PACKAGE_ROOT, "site", "src", "fixtures");

/**
 * A manifest as the bundle receives it: the bytes parsed, and the name
 * of where they came from.
 *
 * The name travels with the value because the parser on the other side
 * reports the path that failed, and «a manifest» is not a useful thing
 * to be told is malformed when a build was handed six of them.
 */
function named(name, value) {
  return { name, value };
}

/** The package's own fixture library: a source and one adaptation. */
export function fixtureLibrary() {
  const files = readdirSync(FIXTURES).filter(
    (entry) => entry.startsWith("manifest") && entry.endsWith(".json"),
  );
  if (files.length === 0) {
    throw new Error(`${FIXTURES}: no page manifest to build from`);
  }
  return files.map((entry) =>
    named(
      `fixtures/${entry}`,
      JSON.parse(readFileSync(join(FIXTURES, entry), "utf8")),
    ),
  );
}

/**
 * The library the deployment's trees describe, or the fixture pair when
 * it named none.
 *
 * The fork is on whether `VIBE_DOC_OUT` was SET, not on what happened to
 * be found: a build that was told nothing renders the package's own
 * fixtures exactly as it did before this existed, and a build that was
 * told where the documentation is renders that documentation or stops.
 * Falling back quietly is the one answer that must not be given — a
 * deployment that mis-spelt a path would publish a two-page fixture
 * manual under its own domain and every gate would be green.
 */
export function buildLibrary(env = process.env) {
  const named_ = (env[TREES_ENV] ?? "").trim();
  if (named_.length === 0) return fixtureLibrary();

  const trees = readTrees(treePaths(env));
  const found = libraryManifests(trees);
  if (found.length === 0) {
    throw new Error(
      `${TREES_ENV} names ${trees.length} tree(s) and none of them carries a manifest.json: ` +
        "there is nothing to build a page library from, and the fixtures are not a stand-in " +
        "for a deployment's own documentation",
    );
  }
  return found.map((manifest) =>
    named(`${manifest.package.group}/${manifest.package.name}`, manifest),
  );
}
