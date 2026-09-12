import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import {
  docHref,
  docPath,
  href,
  parseDocAddress,
  projectionHref,
} from "./href.ts";

const PACKAGE_ROOT = join(import.meta.dirname, "..", "..", "..");

test("href owns the leading slash so JSX never writes one", () => {
  assert.equal(href(""), "/");
  assert.equal(href("ru/"), "/ru/");
  assert.equal(href("/ru/"), "/ru/");
  assert.equal(href("doc/manifest.json"), "/doc/manifest.json");
});

test("a documentation address ends in a slash, with and without a language", () => {
  const address = {
    lang: null,
    group: "org.vibevm.core",
    name: "vibevm-docs",
    version: "0.1.0",
    document: "model/lock-and-store",
  } as const;
  assert.equal(
    docPath(address),
    "doc/org.vibevm.core/vibevm-docs/0.1.0/model/lock-and-store/",
  );
  assert.equal(
    docHref(address),
    "/doc/org.vibevm.core/vibevm-docs/0.1.0/model/lock-and-store/",
  );
  assert.equal(
    docHref({ ...address, lang: "ru" }),
    "/doc/ru/org.vibevm.core/vibevm-docs/0.1.0/model/lock-and-store/",
  );
});

test("a projection is the page's path as a file, so it loses the slash", () => {
  const address = {
    lang: null,
    group: "org.vibevm.core",
    name: "vibevm-docs",
    version: "latest",
    document: "faq",
  } as const;
  assert.equal(
    projectionHref(address, "md"),
    "/doc/org.vibevm.core/vibevm-docs/latest/faq.md",
  );
  assert.equal(
    projectionHref(address, "xml"),
    "/doc/org.vibevm.core/vibevm-docs/latest/faq.xml",
  );
});

test("the language segment is told from a group by the dot, not by a list", () => {
  const withoutLang = parseDocAddress([
    "org.vibevm.core",
    "vibevm-docs",
    "0.1.0",
    "faq",
  ]);
  assert.deepEqual(withoutLang, {
    lang: null,
    group: "org.vibevm.core",
    name: "vibevm-docs",
    version: "0.1.0",
    document: "faq",
  });

  const withLang = parseDocAddress([
    "ru",
    "org.vibevm.core",
    "vibevm-docs",
    "0.1.0",
    "faq",
  ]);
  assert.equal(withLang?.lang, "ru");
  assert.equal(withLang?.group, "org.vibevm.core");

  const deep = parseDocAddress([
    "org.vibevm.core",
    "vibevm-docs",
    "latest",
    "model",
    "lock",
  ]);
  assert.equal(deep?.document, "model/lock");
});

test("what is not an address parses as none of them", () => {
  assert.equal(parseDocAddress([]), null);
  assert.equal(
    parseDocAddress(["org.vibevm.core", "vibevm-docs", "0.1.0"]),
    null,
    "no document",
  );
  assert.equal(
    parseDocAddress(["ru", "not-a-group", "name", "1.0", "page"]),
    null,
    "group has no dot",
  );
  assert.equal(
    parseDocAddress(["Not-A-Language", "name", "1.0", "page"]),
    null,
  );
});

test("parse and print are each other's inverse", () => {
  for (const segments of [
    ["org.vibevm.core", "vibevm-docs", "0.1.0", "faq"],
    [
      "ru",
      "org.vibevm.core",
      "vibevm-docs",
      "latest",
      "model",
      "lock-and-store",
    ],
  ]) {
    const address = parseDocAddress(segments);
    assert.notEqual(address, null);
    if (address === null) continue;
    assert.equal(docPath(address), `doc/${segments.join("/")}/`);
  }
});

/**
 * The rule the other tests cannot state: no component writes a site path
 * by hand. One function owns the base and the trailing slash, and a
 * literal that bypasses it is right in one build and wrong in the other
 * — which nothing else in the toolchain would catch, because both builds
 * succeed.
 */
test("no source outside href.ts writes a documentation path as a literal", () => {
  const roots = [
    join(PACKAGE_ROOT, "site", "src"),
    join(PACKAGE_ROOT, "design", "src"),
  ];
  const allowed = join(PACKAGE_ROOT, "site", "src", "lib", "href.ts");
  const offenders: string[] = [];

  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      if (statSync(full).isDirectory()) {
        if (entry === "generated" || entry === "fixtures") continue;
        walk(full);
        continue;
      }
      if (!/\.tsx?$/.test(entry)) continue;
      if (full === allowed || full.endsWith(".test.ts")) continue;
      if (/["'`]\/doc\//.test(readFileSync(full, "utf8"))) offenders.push(full);
    }
  };

  for (const root of roots) walk(root);
  assert.deepEqual(
    offenders,
    [],
    "call href()/docHref() instead of writing the path",
  );
});
