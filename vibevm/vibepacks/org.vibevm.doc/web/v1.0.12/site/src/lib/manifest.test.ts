import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import { parseDocManifest } from "./manifest.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures");
const FIXTURE = join(FIXTURES, "manifest.json");
const ADAPTATION = join(FIXTURES, "manifest-ru.json");

function fixture(file: string = FIXTURE): unknown {
  return JSON.parse(readFileSync(file, "utf8"));
}

test("the source manifest parses", () => {
  const parsed = parseDocManifest(fixture());
  assert.equal(
    parsed.ok,
    true,
    parsed.ok ? "" : `${parsed.error.path}: ${parsed.error.reason}`,
  );
  if (!parsed.ok) return;
  assert.equal(parsed.value.package.group, "com.example.docs");
  assert.equal(parsed.value.package.lang, "en");
  assert.equal(parsed.value.pages.length, 2);
  assert.equal(parsed.value.pages[1]?.path, "guide/every-block.xml");
});

/**
 * The adaptation is the fallback's whole subject: it is one page short
 * of the source on purpose, so the gap the site materialises is a gap
 * the fixture actually has rather than one a test pretends to.
 */
test("the adaptation parses and is a page short of its source", () => {
  const source = parseDocManifest(fixture());
  const adapted = parseDocManifest(fixture(ADAPTATION));
  assert.equal(
    adapted.ok,
    true,
    adapted.ok ? "" : `${adapted.error.path}: ${adapted.error.reason}`,
  );
  if (!adapted.ok || !source.ok) return;
  assert.equal(adapted.value.package.lang, "ru");
  assert.equal(adapted.value.package.translation?.status, "official");
  assert.equal(
    adapted.value.package.translation?.package,
    `${source.value.package.group}/${source.value.package.name}`,
  );
  assert.equal(adapted.value.pages.length, source.value.pages.length - 1);
});

test("an absent optional field stays absent rather than becoming undefined", () => {
  const parsed = parseDocManifest(fixture());
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal("translation" in parsed.value.package, false);
});

/**
 * This parser is the one door from bytes into the type, so a member it
 * drops is a member the whole shell cannot see — whatever the generated
 * type promises about it. Both fixtures declare who wrote their prose,
 * and the assertion is that the word survives the crossing.
 */
test("who wrote the prose survives the crossing from bytes to type", () => {
  const source = parseDocManifest(fixture());
  const adapted = parseDocManifest(fixture(ADAPTATION));
  assert.equal(source.ok, true);
  assert.equal(adapted.ok, true);
  if (!source.ok || !adapted.ok) return;
  assert.equal(source.value.package.authorship, "ai");
  assert.equal(adapted.value.package.authorship, "mixed");
});

/**
 * Absence is permissive and nonsense is not. A manifest written before
 * the field existed is still a manifest; a fourth word is a document
 * claiming something the vocabulary cannot say, and it is refused by the
 * name of the field it stands in.
 */
test("a fourth authorship is refused, and none at all is not", () => {
  const broken = JSON.parse(JSON.stringify(fixture()));
  broken.package.authorship = "committee";
  const refused = parseDocManifest(broken);
  assert.equal(refused.ok, false);
  if (refused.ok) return;
  assert.equal(refused.error.path, "$.package.authorship");
  assert.match(refused.error.reason, /mixed/);

  const silent = JSON.parse(JSON.stringify(fixture()));
  delete silent.package.authorship;
  const parsed = parseDocManifest(silent);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal("authorship" in parsed.value.package, false);
});

/**
 * A bridge's two lists, and the one thing that must never happen to
 * them: either being filled from the other. An empty list is a published
 * fact and is kept empty.
 */
test("a bridge's two authorships arrive apart and stay apart", () => {
  const source = parseDocManifest(fixture());
  assert.equal(source.ok, true);
  if (!source.ok) return;
  const bridge = source.value.package.bridge;
  assert.notEqual(bridge, undefined);
  assert.deepEqual(bridge?.maintainers, ["The fixture's maintainer"]);
  assert.deepEqual(bridge?.upstream_authors, [
    "The author of the bytes the fixture wraps",
  ]);
  assert.equal(bridge?.upstream_license, "Apache-2.0");

  const nobody = JSON.parse(JSON.stringify(fixture()));
  nobody.package.bridge.maintainers = [];
  delete nobody.package.bridge.upstream_license;
  const parsed = parseDocManifest(nobody);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.deepEqual(parsed.value.package.bridge?.maintainers, []);
  assert.deepEqual(parsed.value.package.bridge?.upstream_authors, [
    "The author of the bytes the fixture wraps",
  ]);
  assert.equal(
    "upstream_license" in (parsed.value.package.bridge ?? {}),
    false,
  );

  const broken = JSON.parse(JSON.stringify(fixture()));
  delete broken.package.bridge.upstream_authors;
  const refused = parseDocManifest(broken);
  assert.equal(refused.ok, false);
  if (refused.ok) return;
  assert.equal(refused.error.path, "$.package.bridge.upstream_authors");
});

/**
 * The level-zero mark. It is `false` by being absent on the wire, so the
 * three states a reader of the type sees are «said true», «said false»
 * and «said nothing» — and the last one is what a manifest written
 * before the field says.
 */
test("the level-zero mark is read as written, and absence is not false", () => {
  const marked = JSON.parse(JSON.stringify(fixture()));
  marked.package.projection = true;
  const rendering = parseDocManifest(marked);
  assert.equal(rendering.ok, true);
  if (!rendering.ok) return;
  assert.equal(rendering.value.package.projection, true);

  const parsed = parseDocManifest(fixture());
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal("projection" in parsed.value.package, false);

  const broken = JSON.parse(JSON.stringify(fixture()));
  broken.package.projection = "yes";
  const refused = parseDocManifest(broken);
  assert.equal(refused.ok, false);
  if (refused.ok) return;
  assert.equal(refused.error.path, "$.package.projection");
});

/**
 * The word the rendered package calls itself by. All eight cross, not
 * just the one a boolean could be read backwards into: the point of
 * carrying the member is that a projection of a `tool` arrives marked as
 * a tool. Absence stays permissive for a manifest written before the
 * member; a ninth word is refused by the name of its field.
 */
test("the kind of the rendered package crosses as itself", () => {
  const tool = JSON.parse(JSON.stringify(fixture()));
  tool.package.kind = "tool";
  tool.package.projection = true;
  const projected = parseDocManifest(tool);
  assert.equal(projected.ok, true);
  if (!projected.ok) return;
  assert.equal(projected.value.package.kind, "tool");
  assert.equal(projected.value.package.projection, true);

  const parsed = parseDocManifest(fixture());
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal("kind" in parsed.value.package, false);

  const broken = JSON.parse(JSON.stringify(fixture()));
  broken.package.kind = "pack";
  const refused = parseDocManifest(broken);
  assert.equal(refused.ok, false);
  if (refused.ok) return;
  assert.equal(refused.error.path, "$.package.kind");
  assert.match(refused.error.reason, /flow/);
});

test("a value outside a closed vocabulary is refused by name", () => {
  const document = fixture();
  assert.equal(typeof document, "object");
  const broken = JSON.parse(JSON.stringify(document));
  broken.package.status = "semi-official";
  const parsed = parseDocManifest(broken);
  assert.equal(parsed.ok, false);
  if (parsed.ok) return;
  assert.equal(parsed.error.path, "$.package.status");
  assert.match(parsed.error.reason, /community/);
});

test("the failure names the path, not just the fact of failing", () => {
  const broken = JSON.parse(JSON.stringify(fixture()));
  delete broken.pages[0].summary;
  const parsed = parseDocManifest(broken);
  assert.equal(parsed.ok, false);
  if (parsed.ok) return;
  assert.equal(parsed.error.path, "$.pages[0].summary");
});

test("anything that is not a manifest is not a manifest", () => {
  for (const value of [null, 7, "a manifest", [], {}]) {
    assert.equal(parseDocManifest(value).ok, false);
  }
});

/**
 * The learning path across the erasure boundary (`##NAV-CHAPTERS`).
 *
 * The fixture declares one, so the rows themselves are measured on the
 * bytes the site is actually built from; the state of every documentation
 * written before the rows existed is the same fixture with the table taken
 * away. The site turns on exactly that distinction: no path means the
 * sections view it always showed, a declared path means the contents
 * opens on the path, and an empty array read as «no path» would hide a
 * package that opened the table and named nothing.
 */
test("a declared learning path crosses, and no path stays no path", () => {
  const parsed = parseDocManifest(fixture());
  assert.equal(
    parsed.ok,
    true,
    parsed.ok ? "" : `${parsed.error.path}: ${parsed.error.reason}`,
  );
  if (!parsed.ok) return;
  const path = parsed.value.navigation?.chapters;
  assert.equal(path?.length, 2);
  assert.equal(path?.[0]?.id, "reading");
  assert.equal(path?.[0]?.title, "Reading a page");
  assert.deepEqual(path?.[0]?.pages, ["guide/every-block"]);
  // Absent is not `false` in the reading; it is `false` only in meaning.
  assert.equal("appendix" in (path?.[0] ?? {}), false);
  assert.equal(path?.[1]?.appendix, true);

  // A translation names the chapters of the path it takes and lists no
  // page of its own, which reaches the type as the empty array it wrote.
  const adapted = parseDocManifest(fixture(ADAPTATION));
  assert.equal(adapted.ok, true);
  if (!adapted.ok) return;
  assert.deepEqual(
    adapted.value.navigation?.chapters?.map((one) => [one.id, one.pages]),
    [["reading", []]],
  );

  const silent = JSON.parse(JSON.stringify(fixture()));
  delete silent.navigation.chapters;
  const none = parseDocManifest(silent);
  assert.equal(none.ok, true);
  if (!none.ok) return;
  assert.notEqual(none.value.navigation, undefined);
  assert.equal("chapters" in (none.value.navigation ?? {}), false);

  // An empty path is a package that opened the table and named nothing,
  // and it arrives as the empty array it wrote.
  const empty = JSON.parse(JSON.stringify(fixture()));
  empty.navigation.chapters = [];
  const opened = parseDocManifest(empty);
  assert.equal(opened.ok, true);
  if (!opened.ok) return;
  assert.deepEqual(opened.value.navigation?.chapters, []);
});

/**
 * A malformed chapter is a failure named by its own path. A contents
 * missing chapter four is worse than a shell that says which row it could
 * not read, so nothing here is dropped quietly.
 */
test("a malformed chapter is refused by the path of the field that failed", () => {
  const cases: readonly (readonly [unknown, string])[] = [
    [{ id: "guide", pages: [] }, "$.navigation.chapters[0].title"],
    [{ title: "The guide", pages: [] }, "$.navigation.chapters[0].id"],
    [{ id: "guide", title: "The guide" }, "$.navigation.chapters[0].pages"],
    [
      { id: "guide", title: "The guide", pages: "guide/every-block" },
      "$.navigation.chapters[0].pages",
    ],
    [
      { id: "guide", title: "The guide", pages: [7] },
      "$.navigation.chapters[0].pages[0]",
    ],
    [
      { id: "guide", title: "The guide", pages: [], appendix: "yes" },
      "$.navigation.chapters[0].appendix",
    ],
    ["a chapter", "$.navigation.chapters[0]"],
  ];
  for (const [row, path] of cases) {
    const broken = JSON.parse(JSON.stringify(fixture()));
    broken.navigation.chapters = [row];
    const refused = parseDocManifest(broken);
    assert.equal(refused.ok, false, `${path} must be refused`);
    if (refused.ok) continue;
    assert.equal(refused.error.path, path);
  }

  // The list itself is a list, and the failure says so at the table.
  const broken = JSON.parse(JSON.stringify(fixture()));
  broken.navigation.chapters = { guide: "The guide" };
  const refused = parseDocManifest(broken);
  assert.equal(refused.ok, false);
  if (refused.ok) return;
  assert.equal(refused.error.path, "$.navigation.chapters");
});
