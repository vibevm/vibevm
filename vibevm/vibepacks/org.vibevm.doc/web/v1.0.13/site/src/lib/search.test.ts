/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER */

/**
 * What the header's search box answers, asked without a browser.
 *
 * The box itself is shape and keyboard and cannot be judged here; what
 * can is the only part a reader actually complains about — whether their
 * words find the page they were reaching for. The owner's acceptance was
 * two sentences long: it finds a page by a word from the title, and by a
 * word from the abstract. Both are below, and so are the two answers
 * that are easy to get wrong in the other direction — a word nothing
 * carries, and a second word that should narrow rather than widen.
 */

import assert from "node:assert/strict";
import test from "node:test";

import {
  matches,
  readSearchIndex,
  SEARCH_MIN,
  type SearchIndex,
} from "./search.ts";

const INDEX: SearchIndex = {
  schema_version: 1,
  entries: [
    {
      title: "Lock and store",
      href: "/doc/org.example/manual/latest/model/lock-and-store/",
      context: "The manual · en",
      summary:
        "The lockfile pins every resolved version, and the store holds the bytes those versions name.",
      coordinate: "org.example/manual@1.0.0",
      kind: "page",
    },
    {
      title: "Install a package",
      href: "/doc/org.example/manual/latest/task/install/",
      context: "The manual · en",
      summary:
        "Adding a dependency resolves it, writes the lockfile and unpacks it into the project.",
      coordinate: "org.example/manual@1.0.0",
      kind: "page",
    },
    {
      title: "Руководство",
      href: "/doc/ru/org.example/manual/latest/",
      context: "Руководство · ru",
      summary: "Как устроен пакетный менеджер и что он делает с зависимостями.",
      coordinate: "org.example/manual-ru@1.0.0",
      kind: "documentation",
    },
  ],
};

const titlesOf = (query: string): string[] =>
  matches(INDEX, query).map((one) => one.title);

test("a word from a page's title finds that page", () => {
  assert.deepEqual(titlesOf("install"), ["Install a package"]);
});

/**
 * The abstract is the only field that knows what a page is ABOUT, and a
 * reader searching for a thing they cannot name is searching it.
 */
test("a word from a page's abstract finds that page", () => {
  assert.deepEqual(titlesOf("unpacks"), ["Install a package"]);
});

/**
 * Both pages answer for «lock» — one opens with the word, the other
 * mentions a lockfile in passing — and the one that opens with it comes
 * first. That order is the whole difference between a search and a list.
 */
test("a title outranks an abstract for the same word", () => {
  assert.deepEqual(titlesOf("lock"), ["Lock and store", "Install a package"]);
});

/** Two words are a narrowing and not a wish: both have to land. */
test("a second word narrows rather than widens", () => {
  assert.deepEqual(titlesOf("install unpacks"), ["Install a package"]);
  assert.deepEqual(titlesOf("store unpacks"), []);
});

/** The coordinate is identity, and a thing people paste into fields. */
test("the coordinate a page is published under finds it", () => {
  assert.equal(matches(INDEX, "manual-ru").length, 1);
});

/**
 * Cyrillic is not a special case and must not become one: a word
 * boundary is «the character before it is not a letter», which is a
 * question about Unicode rather than about ASCII.
 */
test("a Russian word finds a Russian edition, by title and by abstract", () => {
  assert.deepEqual(titlesOf("руководство"), ["Руководство"]);
  assert.deepEqual(titlesOf("зависимостями"), ["Руководство"]);
});

test("a word nothing carries finds nothing", () => {
  assert.deepEqual(titlesOf("kubernetes"), []);
});

/** One letter matches most of a corpus, which is not an answer. */
test("a query too short to mean anything is not answered", () => {
  assert.equal(SEARCH_MIN, 2);
  assert.deepEqual(titlesOf("l"), []);
  assert.deepEqual(titlesOf("   "), []);
});

test("the number of answers is bounded", () => {
  const many: SearchIndex = {
    schema_version: 1,
    entries: Array.from({ length: 30 }, (_unused, at) => ({
      title: `Lock ${at}`,
      href: `/doc/org.example/manual/latest/page-${at}/`,
      context: "The manual · en",
      summary: "",
      coordinate: "org.example/manual@1.0.0",
      kind: "page" as const,
    })),
  };
  assert.equal(matches(many, "lock").length, 8);
  assert.equal(matches(many, "lock", 3).length, 3);
});

/**
 * The index arrives over the network, so it arrives as `unknown`. A
 * half-written file is «no index» rather than an exception: the
 * catalogue is one click away in the same header, and a search that
 * finds nothing is a smaller failure than a page with a rejection
 * nobody caught.
 */
test("anything that is not an index is not one", () => {
  assert.equal(readSearchIndex(null), null);
  assert.equal(readSearchIndex({ entries: [] }), null);
  assert.equal(readSearchIndex({ schema_version: 2, entries: [] }), null);
  assert.equal(readSearchIndex({ schema_version: 1, entries: {} }), null);
  assert.equal(
    readSearchIndex({ schema_version: 1, entries: [{ title: "x" }] }),
    null,
  );
});

test("an index that is one is read whole", () => {
  const read = readSearchIndex(JSON.parse(JSON.stringify(INDEX)));
  assert.notEqual(read, null);
  assert.deepEqual(read, INDEX);
});
