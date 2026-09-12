/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

/**
 * The page library, built from what `vibe doc build` wrote.
 *
 * Until the trees carried the library, this was two JSON files imported
 * by the bundler and one fixture page rendered into every address; the
 * site could not publish a documentation even when the deployment had
 * rendered one, and said so in a line of the build output. What is
 * measured here is the seam that replaced it — a tree in, addresses and
 * islands out — over the package's own fixture tree, which is a `vibe
 * doc build` output in miniature with the pipeline's own golden as one
 * of its two islands.
 *
 * The one thing these cases guard against more than any other is a
 * silent fallback. A build handed real documentation that quietly
 * rendered the fixture instead would pass every other gate in this
 * package: the pages would generate, the count would match, the links
 * would resolve. So the fork on `VIBE_DOC_OUT` is asserted in both
 * directions, and the islands are compared BY BYTES against the files
 * the tree carries rather than by a substring anything could satisfy.
 */

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

import { islandsOf } from "../../../tools/doc-library.mjs";
import { readTrees } from "../../../tools/doc-surfaces.mjs";
import { buildLibrary } from "../../../tools/library-source.mjs";
import type { DocManifest } from "../generated/doc-manifest.ts";
import { addressesOf, editionsOf } from "../seo/editions.ts";
import {
  parseLibrary,
  siteAddresses,
  sourceEdition,
  type Library,
} from "./library.ts";
import { parseDocManifest } from "./manifest.ts";
import { viewOf } from "./view.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = resolve(HERE, "..", "fixtures");
const TREE = join(FIXTURES, "doc-build");
const PACKAGE = ["com.example.docs", "fixture-manual", "0.1.0"];

/** The bytes of one file of the fixture tree, as the build reads them. */
function treeFile(...parts: string[]): string {
  return readFileSync(join(TREE, ...parts), "utf8");
}

/** One manifest beside the fixtures, as the type the site works in. */
function fixture(name: string): DocManifest {
  const parsed = parseDocManifest(
    JSON.parse(readFileSync(join(FIXTURES, name), "utf8")),
  );
  if (!parsed.ok) throw new Error(`${name}: ${parsed.error.path}`);
  return parsed.value;
}

/** The library of the fixture tree alone: one edition, no adaptation. */
function fromTree(): Library {
  return parseLibrary(buildLibrary({ VIBE_DOC_OUT: TREE }));
}

/** The library of the fixture pair: the source and its adaptation. */
function bothFixtures(): readonly DocManifest[] {
  return [fixture("manifest.json"), fixture("manifest-ru.json")];
}

describe("the library a build renders from", () => {
  it("is the trees' when the deployment named trees", () => {
    const named = buildLibrary({ VIBE_DOC_OUT: TREE });
    assert.equal(named.length, 1);
    assert.equal(named[0]?.name, "com.example.docs/fixture-manual");

    const library = fromTree();
    const source = sourceEdition(library);
    assert.equal(library.editions.length, 1);
    assert.equal(source.tag, "en");
    assert.equal(source.segment, null);
    assert.equal(source.card.name, "fixture-manual");
    assert.equal(source.pages.length, 2);
  });

  it("is the package's own fixture pair when it named none", () => {
    const named = buildLibrary({});
    assert.deepEqual(named.map((one) => one.name).sort(), [
      "fixtures/manifest-ru.json",
      "fixtures/manifest.json",
    ]);
    const library = parseLibrary(named);
    assert.equal(library.editions.length, 2);
    assert.deepEqual(
      library.editions.map((one) => one.segment),
      [null, "ru"],
    );
  });

  it("refuses a set that is two documentations rather than one in two languages", () => {
    const source = fixture("manifest.json");
    const second: DocManifest = {
      ...source,
      package: { ...source.package, name: "another-manual" },
    };
    assert.throws(
      () => editionsOf([source, second]),
      /2 source manifests in one library/,
    );
  });
});

describe("the addresses a library declares", () => {
  it("gives a tree's documentation its package page and one page each", () => {
    const paths = siteAddresses(fromTree()).map((one) => one.path);
    assert.deepEqual(paths, [
      "com.example.docs/fixture-manual/0.1.0",
      "com.example.docs/fixture-manual/0.1.0/reference/addresses",
      "com.example.docs/fixture-manual/0.1.0/guide/every-block",
    ]);
  });

  it("serves an adaptation under the source's coordinate with its language in front", () => {
    const all = siteAddresses(parseLibrary(named(bothFixtures())));
    assert.deepEqual(
      all.filter((one) => one.kind === "catalogue").map((one) => one.path),
      ["ru"],
    );
    assert.ok(
      all.some(
        (one) =>
          one.path ===
            "ru/com.example.docs/fixture-manual/0.1.0/guide/every-block" &&
          !one.fallback,
      ),
      "the adaptation's own page stands at the source's coordinate behind `ru/`",
    );
    assert.ok(
      all.some(
        (one) =>
          one.path ===
            "ru/com.example.docs/fixture-manual/0.1.0/reference/addresses" &&
          one.fallback,
      ),
      "a page the adaptation has not reached is still an address, as a fallback",
    );
    assert.equal(
      all.filter((one) => one.path.includes("fixture-manual-ru")).length,
      0,
      "an adaptation is never served under its own coordinate (D-06)",
    );
  });

  it("writes every page at both spellings of the version", () => {
    const editions = editionsOf([fixture("manifest.json")]);
    const pages = addressesOf(editions)
      .filter((one) => one.kind === "page")
      .map((one) => one.href);
    assert.deepEqual([...pages].sort(), [
      "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
      "/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/",
      "/doc/com.example.docs/fixture-manual/latest/guide/every-block/",
      "/doc/com.example.docs/fixture-manual/latest/reference/addresses/",
    ]);
  });
});

/** The manifests, named the way a build hands them to the parser. */
function named(
  manifests: readonly DocManifest[],
): { name: string; value: unknown }[] {
  return manifests.map((manifest) => ({
    name: `${manifest.package.group}/${manifest.package.name}`,
    value: JSON.parse(JSON.stringify(manifest)) as unknown,
  }));
}

describe("the island behind each address", () => {
  it("is the bytes of that page's own index.html, at both spellings", () => {
    const trees = readTrees([TREE]);
    const editions = editionsOf([fixture("manifest.json")]);
    const islands = islandsOf(trees, editions, addressesOf(editions));

    const golden = treeFile(...PACKAGE, "guide", "every-block", "index.html");
    const other = treeFile(...PACKAGE, "reference", "addresses", "index.html");
    assert.notEqual(golden, other, "the fixture tree has two different pages");

    for (const version of ["0.1.0", "latest"]) {
      assert.equal(
        islands.get(
          `/doc/com.example.docs/fixture-manual/${version}/guide/every-block/`,
        ),
        golden,
      );
      assert.equal(
        islands.get(
          `/doc/com.example.docs/fixture-manual/${version}/reference/addresses/`,
        ),
        other,
      );
    }
  });

  it("is the source's own when the adaptation has not reached the page", () => {
    const trees = readTrees([TREE]);
    const editions = editionsOf(bothFixtures());
    const islands = islandsOf(trees, editions, addressesOf(editions));
    const golden = treeFile(...PACKAGE, "guide", "every-block", "index.html");
    assert.equal(
      islands.get(
        "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
      ),
      golden,
      "the adaptation was given no tree here, so it reads the source's island",
    );
  });
});

describe("the package page of a library out of a tree", () => {
  it("shows the tree's own card and its pages", () => {
    const view = viewOf(fromTree(), "com.example.docs/fixture-manual/0.1.0");
    assert.equal(view?.kind, "package");
    if (view?.kind !== "package") return;
    assert.equal(view.title, "Fixture Manual");
    assert.equal(view.publisher, "com.example.docs");
    assert.equal(view.coordinate, "com.example.docs/fixture-manual@0.1.0");
    assert.equal(view.nav.length, 2);
    assert.deepEqual(view.adaptations, []);
  });
});
