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
