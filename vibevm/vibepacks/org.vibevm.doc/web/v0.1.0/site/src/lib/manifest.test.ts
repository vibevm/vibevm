import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import { parseDocManifest } from "./manifest.ts";

const FIXTURE = join(import.meta.dirname, "..", "fixtures", "manifest.json");

function fixture(): unknown {
  return JSON.parse(readFileSync(FIXTURE, "utf8"));
}

test("the pipeline's own golden manifest parses", () => {
  const parsed = parseDocManifest(fixture());
  assert.equal(
    parsed.ok,
    true,
    parsed.ok ? "" : `${parsed.error.path}: ${parsed.error.reason}`,
  );
  if (!parsed.ok) return;
  assert.equal(parsed.value.package.group, "com.example.docs");
  assert.equal(parsed.value.package.lang, "en");
  assert.equal(parsed.value.pages.length, 1);
  assert.equal(parsed.value.pages[0]?.path, "guide/every-block.xml");
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
