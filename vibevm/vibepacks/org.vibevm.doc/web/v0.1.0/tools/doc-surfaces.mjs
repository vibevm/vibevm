#!/usr/bin/env node
// The agent surfaces of the documentation, copied from the pipeline's
// output into the site's output: the Markdown and XML projection beside
// every page, the four `llms` tiers of every package, and the card
// images (PROP-057 `##SEO-RAW-PROJECTIONS`, `##SEO-LLMS-FILES`).
//
// Copied and never computed. Every one of these files is written by
// `vibe doc build` from the package's own source, with the block numbers
// the pipeline assigned (R-26) and the citations it resolved; a site
// build that produced its own Markdown from the rendered island would be
// a second renderer, disagreeing with the first the first time either
// changed. The site's part is to put what Rust wrote at the address the
// address map gives it.
//
// Where a tree comes from: `VIBE_DOC_OUT`, a list of `vibe doc build`
// output directories separated by the platform's path delimiter. A
// deployment runs the pipeline once per package and per projection and
// names the outputs here; the default is the site's own fixture tree, so
// that a build with nothing installed still produces a complete site to
// measure (`site/src/fixtures/doc-build`).
//
// The one thing the copy is allowed to decide is WHERE. A translation is
// served under the SOURCE package's coordinate with a language segment
// in front of it (D-06), so a tree built from `…-ru` lands under
// `/doc/ru/<source>/…`; and each page's surfaces are written at both
// spellings of the version, because `latest` is an address and a page
// there names its own `.md` beside it.

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
} from "node:fs";
import { delimiter, dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { href, packagePath } from "../site/src/lib/href.ts";
import {
  LATEST,
  coordinateOf,
  documentOf,
  sourceOf,
} from "../site/src/seo/editions.ts";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURE_TREE = join(PACKAGE_ROOT, "site", "src", "fixtures", "doc-build");

/** The environment name a deployment hands its rendered trees in. */
export const TREES_ENV = "VIBE_DOC_OUT";

/** The four tiers, and the order a report prints them in. */
const LLMS_FILES = [
  "llms.txt",
  "llms-small.txt",
  "llms-medium.txt",
  "llms-full.txt",
];

/** The trees this build was given, or the fixture when it was given none. */
export function treePaths(env = process.env) {
  const named = (env[TREES_ENV] ?? "")
    .split(delimiter)
    .map((one) => one.trim())
    .filter((one) => one.length > 0);
  if (named.length > 0) return named;
  return existsSync(FIXTURE_TREE) ? [FIXTURE_TREE] : [];
}

function walk(dir, found = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, found);
    else found.push(full);
  }
  return found;
}

/**
 * What one `vibe doc build` output is: the coordinate it was built for,
 * and the files under it.
 *
 * The coordinate is read off the directories rather than out of the
 * manifest, because the directories ARE the address map the pipeline
 * writes into (`##SITE-MOUNT`) and a tree with no manifest — a
 * projection-only run, which is what `--format md` produces beside the
 * `--format html` one — still has to be placed.
 */
export function readTree(root) {
  if (!existsSync(root)) {
    throw new Error(`${TREES_ENV}: ${root} does not exist`);
  }
  const files = walk(root).map((file) =>
    file
      .slice(root.length + 1)
      .split(sep)
      .join("/"),
  );
  const coordinate = files
    .map((file) => file.split("/"))
    .filter((parts) => parts.length > 3 && (parts[0] ?? "").includes("."))
    .map((parts) => parts.slice(0, 3).join("/"))
    .find(() => true);
  if (coordinate === undefined) {
    throw new Error(`${root}: no <group>/<name>/<version>/ tree inside it`);
  }
  const [group = "", name = "", version = ""] = coordinate.split("/");
  return { root, files, group, name, version, prefix: `${coordinate}/` };
}

/** Every tree named, read once. */
export function readTrees(paths) {
  return paths.map((one) => readTree(one));
}

/**
 * The trees that carry one edition's package — all of them, in the order
 * they were named.
 *
 * All of them, because one package is rendered more than once: `vibe doc
 * build` writes ONE projection per run, so a deployment that publishes
 * the Markdown and the XML beside the islands runs it three times into
 * three directories that share a coordinate. Taking the first would
 * publish whichever projection happened to be named first and silently
 * drop the others.
 */
function treesOf(trees, edition) {
  const card = edition.manifest.package;
  return trees.filter(
    (tree) => tree.group === card.group && tree.name === card.name,
  );
}

/**
 * The card image a package's pages name in `og:image`.
 *
 * The pipeline writes the composed 1200×630 preview beside the icon and
 * the banner under content-hashed names, and the manifest does not say
 * which is which (X-042). Until it does, the card is found the way a
 * browser would have to: the only raster image in `media/`, whose PNG
 * header says 1200×630 — the proportion `##CARD-PREVIEW-COMPOSED`
 * composes it at. A tree without one simply has no card, and the page
 * falls back to the site's own.
 */
function previewIn(trees) {
  for (const tree of trees) {
    for (const file of tree.files) {
      if (!file.startsWith("media/") || !file.endsWith(".png")) continue;
      const bytes = readFileSync(join(tree.root, file.split("/").join(sep)));
      if (bytes.length < 24) continue;
      if (bytes.readUInt32BE(16) !== 1200 || bytes.readUInt32BE(20) !== 630) {
        continue;
      }
      return file;
    }
  }
  return undefined;
}

/** Where an edition's surfaces are served, at one spelling of the version. */
function baseOf(editions, edition, version) {
  const source = sourceOf(editions);
  return href(packagePath(coordinateOf(source, edition.segment, version)));
}

/**
 * The map of coordinate to card, for the pages that have to name one.
 *
 * It is computed before the pages are rendered and handed to them
 * through the environment, because a page cannot work the address out
 * for itself: the name is a content hash the manifest does not carry
 * (X-042, `seo/media.ts`).
 */
export function mediaMapOf(trees, editions) {
  const map = {};
  for (const edition of editions) {
    const mine = treesOf(trees, edition);
    if (mine.length === 0) continue;
    const preview = previewIn(mine);
    if (preview === undefined) continue;
    const card = edition.manifest.package;
    map[`${card.group}/${card.name}@${card.version}`] = {
      preview: `${baseOf(editions, edition, card.version)}${preview}`,
    };
  }
  return map;
}

function copy(from, to) {
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(from, to);
}

/** One file, looked for in each of a package's trees in turn. */
function fileIn(trees, relative) {
  for (const tree of trees) {
    const full = join(tree.root, relative.split("/").join(sep));
    if (existsSync(full)) return full;
  }
  return undefined;
}

/** Every file of a shape a package's trees hold between them, once each. */
function filesIn(trees, keep) {
  const found = new Map();
  for (const tree of trees) {
    for (const file of tree.files) {
      if (keep(file) && !found.has(file)) found.set(file, tree);
    }
  }
  return [...found.keys()];
}

function outPath(outDir, address) {
  return join(outDir, address.replace(/^\/+/, "").split("/").join(sep));
}

/**
 * Copy every surface of every edition into the built site.
 *
 * The rules, in the order they are applied:
 *
 *   - an edition whose own tree was given is copied from it;
 *   - a page that edition does not carry is copied from the SOURCE's
 *     tree instead, which is exactly what the page at that address shows
 *     (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`): the surfaces of a
 *     fallback are the source's, or an agent asking for the Markdown of
 *     a page it can read would get a 404;
 *   - both spellings of the version are written, because both are
 *     addresses and a page at either one names its own `.md` beside it;
 *   - a tree for a package the page library does not carry is copied at
 *     its own coordinate and reported. Nothing links it — the site has no
 *     pages there — but it is what the deployment asked to publish, and
 *     silently dropping a rendered package would be the worst of the
 *     three possible answers.
 */
export function copySurfaces(trees, editions, outDir) {
  const source = sourceOf(editions);
  const sourceTrees = treesOf(trees, source);
  const documents = source.manifest.pages.map((page) => documentOf(page.path));
  const report = { files: 0, editions: 0, fallbacks: 0, unplaced: [] };
  const placed = new Set();

  for (const edition of editions) {
    const own = treesOf(trees, edition);
    for (const tree of own) placed.add(tree.root);
    if (own.length === 0 && sourceTrees.length === 0) continue;
    report.editions += 1;

    for (const version of [source.manifest.package.version, LATEST]) {
      const base = baseOf(editions, edition, version);

      for (const document of documents) {
        const carries = own.some((tree) =>
          tree.files.includes(`${tree.prefix}${document}.md`),
        );
        const from = carries ? own : sourceTrees;
        if (from.length === 0) continue;
        if (!carries && version === source.manifest.package.version) {
          report.fallbacks += 1;
        }
        const prefix = from[0]?.prefix ?? "";
        for (const suffix of ["md", "xml"]) {
          const file = fileIn(from, `${prefix}${document}.${suffix}`);
          if (file === undefined) continue;
          copy(file, outPath(outDir, `${base}${document}.${suffix}`));
          report.files += 1;
        }
      }

      const from = own.length > 0 ? own : sourceTrees;
      if (from.length === 0) continue;
      for (const name of LLMS_FILES) {
        const file = fileIn(from, name);
        if (file === undefined) continue;
        copy(file, outPath(outDir, `${base}${name}`));
        report.files += 1;
      }
      for (const media of filesIn(from, (one) => one.startsWith("media/"))) {
        const file = fileIn(from, media);
        if (file === undefined) continue;
        copy(file, outPath(outDir, `${base}${media}`));
        report.files += 1;
      }
    }
  }

  const seen = new Set();
  for (const tree of trees) {
    if (placed.has(tree.root)) continue;
    const coordinate = `${tree.group}/${tree.name}@${tree.version}`;
    if (!seen.has(coordinate)) {
      seen.add(coordinate);
      report.unplaced.push(coordinate);
    }
    for (const file of tree.files) {
      if (file.endsWith("index.html")) continue;
      const at = file.startsWith(tree.prefix)
        ? href(`doc/${file}`)
        : href(`doc/${tree.prefix}${file}`);
      const from = fileIn([tree], file);
      if (from === undefined) continue;
      copy(from, outPath(outDir, at));
      report.files += 1;
    }
  }

  return report;
}

/** The `llms-full.txt` of the whole documentation half: one per package. */
export function fullCorpus(trees, editions, origin) {
  const parts = [];
  for (const edition of editions) {
    const mine = treesOf(trees, edition);
    if (mine.length === 0) continue;
    const file = fileIn(mine, "llms-full.txt");
    if (file === undefined) continue;
    const card = edition.manifest.package;
    const at = baseOf(editions, edition, card.version);
    parts.push(
      `\n\n---\n## Documentation: ${card.title} (${card.lang})\n## URL: ${origin}${at}\n---\n\n${readFileSync(file, "utf8").trim()}\n`,
    );
  }
  if (parts.length === 0) return null;
  return `${[
    "# VibeVM documentation — full text",
    "",
    "> Every documentation this site carries, in every language, as one file.",
    `> The catalogue with one line per edition is at ${origin}${href("doc/llms.txt")}.`,
    "",
    "---",
  ].join("\n")}${parts.join("")}\n`;
}
