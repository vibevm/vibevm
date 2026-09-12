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
