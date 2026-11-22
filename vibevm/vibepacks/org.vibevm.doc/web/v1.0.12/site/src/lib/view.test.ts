/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-READER */

/**
 * What one documentation ADDRESS means, where the answer is the second
 * order the documentation has.
 *
 * `library.ts`'s own tests cover which library answers for an address and
 * which pages exist at it. This covers what a page and a package page are
 * handed once that is settled and the documentation declared a learning
 * path: which page a reader is asked to begin at, how the pages are
 * grouped on the shelf, and where the path leads from a page of it.
 *
 * It is measured over the fixture library rather than a manifest written
 * inside the test, because the interesting mistakes are about THAT
 * library: its manifest's first page is not its path's first page, and its
 * adaptation names one chapter of the two. The rules of the path itself —
 * the numbering, the appendix, the captions — are measured over libraries
 * written inline in `contents.test.ts`, where a case can be built that the
 * fixture cannot carry.
 */

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import { buildLibrary } from "../../../tools/library-source.mjs";
import type { DocManifest } from "../generated/doc-manifest.ts";
import { librariesOf, parseLibraries, type Library } from "./library.ts";
import { parseDocManifest } from "./manifest.ts";
import { viewOf } from "./view.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = resolve(HERE, "..", "fixtures");
const TREE = join(FIXTURES, "doc-build");
const PAIR = join(FIXTURES, "doc-build-pair");

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

/** One manifest beside the fixtures, as the type the site works in. */
function fixture(name: string): DocManifest {
  const parsed = parseDocManifest(
    JSON.parse(readFileSync(join(FIXTURES, name), "utf8")),
  );
  if (!parsed.ok) throw new Error(`${name}: ${parsed.error.path}`);
  return parsed.value;
}

/** The fixture pair: one source and its adaptation, as manifests. */
function bothFixtures(): readonly DocManifest[] {
  return [fixture("manifest.json"), fixture("manifest-ru.json")];
}

/**
 * The package's own page, in the order its author asked a person to read
 * it (`##NAV-CHAPTERS-READER`).
 *
 * It is measured over the fixture library rather than a manifest written
 * inside the test, because the two things that can go wrong here are
 * about that library: which page a reader is sent to first, and whether
 * the shelf's groups are the chapters the fixture actually declares.
 */
describe("the learning path on a package's own page", () => {
  it("shelves the pages by chapter, in the order of the path", () => {
    const view = viewOf([fromTree()], "com.example.docs/fixture-manual/0.1.0");
    assert.equal(view?.kind, "package");
    if (view?.kind !== "package") return;
    assert.deepEqual(
      view.chapters?.map((chapter) => [
        chapter.number,
        chapter.title,
        chapter.pages.map((one) => one.title),
      ]),
      [
        ["1", "Reading a page", ["Every block once"]],
        ["", "Tables to look things up in", ["Addresses"]],
      ],
      "the appendix is named and not numbered, and the cards are the same " +
        "cards the flat shelf would carry",
    );
    assert.deepEqual(
      view.pages.map((one) => one.title),
      ["Addresses", "Every block once"],
      "the manifest's own order is untouched beside it: two orders, and " +
        "the path moves neither the layer law nor the shelf that shows it",
    );
  });

  it("opens the documentation at the first page of the path", () => {
    const view = viewOf([fromTree()], "com.example.docs/fixture-manual/0.1.0");
    assert.equal(view?.kind, "package");
    if (view?.kind !== "package") return;
    assert.equal(
      view.start,
      "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
      "which is the second page of the manifest, not the first: the layer " +
        "law orders the corpus for a machine and the path for a person",
    );
    assert.notEqual(view.start, view.pages[0]?.href);
  });

  it("names the chapters in the words of the edition being read", () => {
    const view = viewOf(
      librariesOf(bothFixtures()),
      "ru/com.example.docs/fixture-manual/0.1.0",
    );
    assert.equal(view?.kind, "package");
    if (view?.kind !== "package") return;
    assert.deepEqual(
      view.chapters?.map((chapter) => chapter.title),
      ["Как читать страницу", "Tables to look things up in"],
      "the adaptation named the first chapter; the appendix it did not " +
        "name keeps the source's words",
    );
    assert.equal(
      view.start,
      "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
      "and the path leads into the reader's own language",
    );
  });

  it("leaves a documentation that declared no path exactly as it was", () => {
    const view = viewOf(fromTrees(PAIR), "com.example.docs/pair/0.1.0");
    assert.equal(view?.kind, "package");
    if (view?.kind !== "package") return;
    assert.equal(view.chapters, null);
    assert.equal(
      view.start,
      view.pages[0]?.href,
      "with no path declared, the manifest's first page is where a reader " +
        "is asked to begin",
    );
    assert.deepEqual(view.contents.chapters, []);
  });

  it("ends each page of the path with the neighbours it has", () => {
    const at = (document: string) =>
      viewOf([fromTree()], `com.example.docs/fixture-manual/0.1.0/${document}`);
    const first = at("guide/every-block");
    assert.equal(first?.kind, "page");
    if (first?.kind !== "page") return;
    assert.equal(first.path?.previous, undefined);
    assert.equal(first.path?.next?.title, "Addresses");
    assert.deepEqual(first.path?.next?.chapter, {
      title: "Tables to look things up in",
    });

    const last = at("reference/addresses");
    assert.equal(last?.kind, "page");
    if (last?.kind !== "page") return;
    assert.equal(last.path?.previous?.title, "Every block once");
    assert.deepEqual(last.path?.previous?.chapter, {
      number: "1",
      title: "Reading a page",
    });
    assert.equal(last.path?.next, undefined);
  });

  it("shows no neighbours at all on a page of a documentation with no path", () => {
    const view = viewOf(
      fromTrees(PAIR),
      "com.example.docs/pair/0.1.0/guide/one",
    );
    assert.equal(view?.kind, "page");
    if (view?.kind !== "page") return;
    assert.equal(view.path, null);
  });
});
