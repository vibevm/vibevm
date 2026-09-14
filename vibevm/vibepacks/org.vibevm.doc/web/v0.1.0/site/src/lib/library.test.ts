/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

/**
 * The page libraries, built from what `vibe doc build` wrote.
 *
 * Until the trees carried the library, this was two JSON files imported
 * by the bundler and one fixture page rendered into every address; the
 * site could not publish a documentation even when the deployment had
 * rendered one, and said so in a line of the build output. What is
 * measured here is the seam that replaced it — trees in, addresses and
 * islands out — over the package's own fixture trees, which are `vibe
 * doc build` outputs in miniature with the pipeline's own golden as one
 * of their islands.
 *
 * Two things these cases guard against more than any other.
 *
 * A silent fallback: a build handed real documentation that quietly
 * rendered the fixture instead would pass every other gate in this
 * package — the pages would generate, the count would match, the links
 * would resolve. So the fork on `VIBE_DOC_OUT` is asserted in both
 * directions, and the islands are compared BY BYTES against the files
 * the tree carries rather than by a substring anything could satisfy.
 *
 * And a silent merge: a build handed two documentations used to refuse,
 * and the wrong repair would be to fold them into one library — which
 * would serve one under the other's coordinate and let a real adaptation
 * collide with it. So the grouping is asserted by the coordinate each
 * address comes out at, not by how many libraries were counted.
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
import { librariesOf as machineLibrariesOf } from "../seo/editions.ts";
import {
  isProjection,
  librariesOf,
  parseLibraries,
  siteAddressesOf,
  siteLanguages,
  sourceEdition,
  type Library,
} from "./library.ts";
import { catalogueEntries, catalogueShelves } from "./catalogue.ts";
import type { Contents } from "./contents.ts";
import { parseDocManifest } from "./manifest.ts";
import { viewOf } from "./view.ts";
import type { ContentsItem } from "@vibe-docs/design";

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = resolve(HERE, "..", "fixtures");
const TREE = join(FIXTURES, "doc-build");
const PAIR = join(FIXTURES, "doc-build-pair");
const PAIR_RU = join(FIXTURES, "doc-build-pair-ru");
const PACKAGE = ["com.example.docs", "fixture-manual", "0.1.0"];

/** The bytes of one file of a fixture tree, as the build reads them. */
function treeFile(tree: string, ...parts: string[]): string {
  return readFileSync(join(tree, ...parts), "utf8");
}

/** One manifest beside the fixtures, as the type the site works in. */
function fixture(name: string): DocManifest {
  const parsed = parseDocManifest(
    JSON.parse(readFileSync(join(FIXTURES, name), "utf8")),
  );
  if (!parsed.ok) throw new Error(`${name}: ${parsed.error.path}`);
  return parsed.value;
}

/** The libraries of the trees a deployment would name. */
function fromTrees(...trees: string[]): readonly Library[] {
  return parseLibraries(buildLibrary({ VIBE_DOC_OUT: trees.join(";") }));
}

/** The library of the fixture tree alone: one edition, no adaptation. */
function fromTree(): Library {
  const found = fromTrees(TREE)[0];
  if (found === undefined) throw new Error("the fixture tree carries none");
  return found;
}

/** The fixture pair: one source and its adaptation, as manifests. */
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
    const libraries = parseLibraries(named);
    assert.equal(libraries.length, 1, "one documentation in two languages");
    assert.deepEqual(
      libraries[0]?.editions.map((one) => one.segment),
      [null, "ru"],
    );
  });
});

describe("as many libraries as there are source documentations", () => {
  it("gives two documentations a library each", () => {
    const libraries = fromTrees(TREE, PAIR);
    assert.deepEqual(
      libraries.map(
        (one) => `${one.source.card.group}/${one.source.card.name}`,
      ),
      ["com.example.docs/fixture-manual", "com.example.docs/pair"],
    );
    for (const library of libraries) {
      assert.equal(library.editions.length, 1, "neither adapts the other");
    }
  });

  it("attributes an adaptation to the source it names, not to the first one", () => {
    const libraries = fromTrees(TREE, PAIR, PAIR_RU);
    assert.equal(libraries.length, 2);

    const manual = libraries[0];
    const pair = libraries[1];
    assert.equal(manual?.source.card.name, "fixture-manual");
    assert.deepEqual(
      manual?.editions.map((one) => one.segment),
      [null],
      "the manual was not given the pair's Russian adaptation",
    );
    assert.equal(pair?.source.card.name, "pair");
    assert.deepEqual(
      pair?.editions.map((one) => [one.segment, one.card.name]),
      [
        [null, "pair"],
        ["ru", "pair-ru"],
      ],
    );
    assert.equal(
      pair?.editions[1]?.official,
      true,
      "the source's own group published it, so it wears the star",
    );
  });

  it("serves each library under its own coordinate, the adaptation behind its source's", () => {
    const paths = siteAddressesOf(fromTrees(TREE, PAIR, PAIR_RU)).map(
      (one) => one.path,
    );
    assert.ok(
      paths.includes("com.example.docs/fixture-manual/0.1.0/guide/every-block"),
    );
    assert.ok(paths.includes("com.example.docs/pair/0.1.0/guide/one"));
    assert.ok(paths.includes("ru/com.example.docs/pair/0.1.0/guide/one"));
    assert.equal(
      paths.filter((one) => one.includes("pair-ru")).length,
      0,
      "an adaptation is never served under its own coordinate (D-06)",
    );
    assert.deepEqual(
      paths.filter((one) => !one.includes("/")),
      ["ru"],
      "a catalogue is a language of the site, written once",
    );
  });

  it("keeps one catalogue per language when two libraries share one", () => {
    const libraries = [
      ...fromTrees(PAIR, PAIR_RU),
      ...librariesOf(bothFixtures()),
    ];
    assert.equal(libraries.length, 2, "two documentations, both adapted to ru");
    assert.deepEqual(
      siteLanguages(libraries).map((one) => [one.segment, one.count]),
      [
        [null, 2],
        ["ru", 2],
      ],
    );
    assert.deepEqual(
      siteAddressesOf(libraries)
        .filter((one) => one.kind === "catalogue")
        .map((one) => one.path),
      ["ru"],
    );
  });

  it("is a library of its own when the source of an adaptation is absent", () => {
    const libraries = fromTrees(PAIR_RU);
    assert.equal(libraries.length, 1);
    const source = libraries[0]?.source;
    assert.equal(source?.card.name, "pair-ru");
    assert.equal(
      source?.segment,
      null,
      "with no source here to stand behind, it stands under its own coordinate",
    );
    assert.ok(
      siteAddressesOf(libraries)
        .map((one) => one.path)
        .includes("com.example.docs/pair-ru/0.1.0/guide/one"),
    );
  });
});

describe("the addresses a library declares", () => {
  it("gives a tree's documentation its package page and one page each", () => {
    const paths = siteAddressesOf([fromTree()]).map((one) => one.path);
    assert.deepEqual(paths, [
      "com.example.docs/fixture-manual/0.1.0",
      "com.example.docs/fixture-manual/0.1.0/reference/addresses",
      "com.example.docs/fixture-manual/0.1.0/guide/every-block",
    ]);
  });

  it("serves an adaptation under the source's coordinate with its language in front", () => {
    const all = siteAddressesOf(librariesOf(bothFixtures()));
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
    const pages = machineLibrariesOf([fixture("manifest.json")])
      .flatMap((one) => one.addresses)
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

describe("the island behind each address", () => {
  it("is the bytes of that page's own index.html, at both spellings", () => {
    const trees = readTrees([TREE]);
    const libraries = machineLibrariesOf([fixture("manifest.json")]);
    const islands = islandsOf(trees, libraries);

    const golden = treeFile(
      TREE,
      ...PACKAGE,
      "guide",
      "every-block",
      "index.html",
    );
    const other = treeFile(
      TREE,
      ...PACKAGE,
      "reference",
      "addresses",
      "index.html",
    );
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
    const islands = islandsOf(trees, machineLibrariesOf(bothFixtures()));
    const golden = treeFile(
      TREE,
      ...PACKAGE,
      "guide",
      "every-block",
      "index.html",
    );
    assert.equal(
      islands.get(
        "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
      ),
      golden,
      "the adaptation was given no tree here, so it reads the source's island",
    );
  });

  it("never reads one library's page into another library's address", () => {
    const trees = readTrees([TREE, PAIR, PAIR_RU]);
    const islands = islandsOf(
      trees,
      machineLibrariesOf([
        fixture("manifest.json"),
        ...readManifests(PAIR, PAIR_RU),
      ]),
    );
    assert.equal(
      islands.get("/doc/com.example.docs/pair/0.1.0/guide/one/"),
      treeFile(
        PAIR,
        "com.example.docs",
        "pair",
        "0.1.0",
        "guide",
        "one",
        "index.html",
      ),
    );
    assert.equal(
      islands.get("/doc/ru/com.example.docs/pair/0.1.0/guide/one/"),
      treeFile(
        PAIR_RU,
        "com.example.docs",
        "pair-ru",
        "0.1.0",
        "guide",
        "one",
        "index.html",
      ),
      "the Russian address shows the Russian text, not the source's",
    );
  });
});

/** Every page the contents column lists, pinned first, groups in order. */
function listed(contents: Contents): ContentsItem[] {
  return [
    ...contents.pinned,
    ...contents.sections.flatMap((section) => [...section.items]),
  ];
}

/** The manifests of some fixture trees, as the build reads them. */
function readManifests(...trees: string[]): DocManifest[] {
  return trees.map((tree) => {
    const parsed = parseDocManifest(
      JSON.parse(readFileSync(join(tree, "manifest.json"), "utf8")),
    );
    if (!parsed.ok) throw new Error(`${tree}: ${parsed.error.path}`);
    return parsed.value;
  });
}

describe("the package page of a library out of a tree", () => {
  it("shows the tree's own card and its pages", () => {
    const view = viewOf([fromTree()], "com.example.docs/fixture-manual/0.1.0");
    assert.equal(view?.kind, "package");
    if (view?.kind !== "package") return;
    assert.equal(view.title, "Fixture Manual");
    assert.equal(view.publisher, "com.example.docs");
    assert.equal(view.coordinate, "com.example.docs/fixture-manual@0.1.0");
    assert.equal(listed(view.contents).length, 2);
    assert.deepEqual(view.adaptations, []);
  });

  it("answers from the library the address names, over many", () => {
    const libraries = fromTrees(TREE, PAIR, PAIR_RU);
    const view = viewOf(libraries, "com.example.docs/pair/0.1.0/guide/one");
    assert.equal(view?.kind, "page");
    if (view?.kind !== "page") return;
    assert.equal(view.title, "One");
    assert.deepEqual(
      listed(view.contents).map((one) => one.label),
      ["Two", "One"],
      "the navigation is the pair's pages and never the manual's",
    );
    assert.equal(
      viewOf(libraries, "com.example.docs/nothing-here/0.1.0/guide/one"),
      null,
      "an address of a documentation this build does not carry is not a page",
    );
  });
});

describe("the door", () => {
  it("lists every edition of every library, source before adaptation", () => {
    const entries = catalogueEntries(fromTrees(TREE, PAIR, PAIR_RU), []);
    assert.deepEqual(
      entries.map((one) => [one.title, one.href, one.status]),
      [
        [
          "Fixture Manual",
          "/doc/com.example.docs/fixture-manual/0.1.0/",
          "community",
        ],
        ["The Pair", "/doc/com.example.docs/pair/0.1.0/", "community"],
        ["Пара", "/doc/ru/com.example.docs/pair/0.1.0/", "official"],
      ],
    );
  });

  /**
   * Featured names a DOCUMENTATION, so it is matched on the coordinate
   * every address of a library is built on — the source's. An adaptation
   * of a featured manual is that manual in another language and belongs
   * on the same shelf; a coordinate this build does not carry is simply
   * not there, because a featured list is a wish of the deployment's
   * and never a claim about what was rendered.
   */
  it("features by the documentation's coordinate, adaptations included", () => {
    const shelves = catalogueShelves(fromTrees(TREE, PAIR, PAIR_RU), [
      "com.example.docs/pair",
      "com.example.docs/nothing-here",
    ]);
    assert.deepEqual(
      shelves.map((shelf) => [
        shelf.tab,
        shelf.entries.map((one) => one.title),
      ]),
      [
        ["featured", ["The Pair", "Пара"]],
        ["documents", ["Fixture Manual", "The Pair", "Пара"]],
        ["projections", []],
      ],
    );
  });

  /**
   * The one signal a manifest carries today. A level-zero rendering is a
   * package printing its own bytes, so its own coordinate stands among
   * its subjects; a `doc` package names something other than itself. The
   * fixtures are all the second kind, which is what makes the first kind
   * worth asserting rather than assuming.
   */
  it("tells a rendering of a package from a documentation about one", () => {
    const documentation = fixture("manifest.json").package;
    assert.equal(isProjection(documentation), false);

    const rendering = {
      ...documentation,
      group: "com.example",
      name: "subject",
    };
    assert.equal(isProjection(rendering), true);

    /* And the field the manifest is gaining is believed over the signal.
       It is composed rather than written as a literal because the
       generated type does not carry it yet: a manifest from a newer
       pipeline is exactly a card with one more member on it. */
    assert.equal(
      isProjection(Object.assign({}, rendering, { projection: false })),
      false,
    );
    assert.equal(
      isProjection(Object.assign({}, documentation, { projection: true })),
      true,
    );
  });
});
