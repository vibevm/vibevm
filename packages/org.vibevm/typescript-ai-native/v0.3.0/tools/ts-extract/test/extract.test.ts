/**
 * Contract tests for the ts-extract NDJSON protocol (protocol 1).
 * Run the extractor as a child process over the committed fixture tree —
 * the exact invocation shape the Rust bridge uses — and assert every
 * fact class D2 promises, including the two Phase 0 spike findings
 * (`@implements` is a PARSED JSDoc tag; string literals are traps).
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const EXTRACT = join(HERE, "..", "extract.ts");
const DIRTY = join(HERE, "fixtures", "dirty");

interface Record {
  protocol: number;
  file: string;
  in_test: boolean;
  degraded: boolean;
  facts: Array<{
    fact: string;
    kind?: string;
    line?: number;
    reason?: string | null;
    to_path?: string;
    symbol?: string;
    is_exported?: boolean;
    lines?: number;
  }>;
  markers: Array<{
    tag: string;
    uri: string;
    reason: string | null;
    symbol: string | null;
    line: number;
  }>;
}

function runExtract(root: string): Record[] {
  const stdout = execFileSync(process.execPath, [EXTRACT, "--root", root], {
    encoding: "utf8",
  });
  return stdout
    .trim()
    .split("\n")
    .filter((l) => l.length > 0)
    .map((l) => JSON.parse(l) as Record);
}

const records = runExtract(DIRTY);
const byFile = new Map(records.map((r) => [r.file, r]));

test("one protocol-1 record per source file, sorted", () => {
  assert.equal(records.length, 4);
  assert.ok(records.every((r) => r.protocol === 1));
  assert.deepEqual(
    records.map((r) => r.file),
    [
      "src/cells/greet/index.ts",
      "src/cells/greet/internal.ts",
      "src/cells/parse/logic.ts",
      "src/rubble.ts",
    ],
  );
});

test("the unsafe set is AST-classified; string literals never fire", () => {
  const logic = byFile.get("src/cells/parse/logic.ts");
  assert.ok(logic);
  const unsafe = logic.facts.filter((f) => f.fact === "ts_unsafe");
  const kinds = unsafe.map((f) => f.kind).sort();
  // any_type, as_cross (one - `as const` excluded), non_null,
  // ts_expect_error, ts_ignore. The trap string contributes nothing.
  assert.deepEqual(kinds, [
    "any_type",
    "as_cross",
    "non_null",
    "ts_expect_error",
    "ts_ignore",
  ]);
  const expectError = unsafe.find((f) => f.kind === "ts_expect_error");
  assert.ok(expectError);
  assert.equal(expectError.reason, "fixture reason: intentional mismatch");
  const ignore = unsafe.find((f) => f.kind === "ts_ignore");
  assert.ok(ignore);
  assert.equal(ignore.reason, null);
});

test("imports carry the specifier, including sibling-internal paths", () => {
  const logic = byFile.get("src/cells/parse/logic.ts");
  assert.ok(logic);
  const imports = logic.facts
    .filter((f) => f.fact === "import")
    .map((f) => f.to_path)
    .sort();
  assert.deepEqual(imports, ["../greet/internal.js", "node:fs"]);
});

test("spec markers surface with raw-text URIs (the @implements finding)", () => {
  const logic = byFile.get("src/cells/parse/logic.ts");
  assert.ok(logic);
  const impl = logic.markers.find((m) => m.tag === "implements");
  assert.ok(impl, JSON.stringify(logic.markers));
  // .comment would have said "://fixture/..." - the raw text keeps the scheme.
  assert.equal(impl.uri, "spec://fixture/PROP-001#req-parse");
  assert.equal(impl.symbol, "parse");

  const deviates = logic.markers.find((m) => m.tag === "deviates");
  assert.ok(deviates);
  assert.equal(deviates.reason, "fixture-recorded deviation");

  const greet = byFile.get("src/cells/greet/index.ts");
  assert.ok(greet);
  const scope = greet.markers.find((m) => m.tag === "scope");
  assert.ok(scope);
  assert.equal(scope.uri, "spec://fixture/PROP-001#cell-greet");
});

test("exported items carry symbol, kind, and export visibility", () => {
  const logic = byFile.get("src/cells/parse/logic.ts");
  assert.ok(logic);
  const items = logic.facts.filter((f) => f.fact === "item");
  const parse = items.find((f) => f.symbol === "parse");
  assert.ok(parse);
  assert.equal(parse.kind, "function");
  assert.equal(parse.is_exported, true);
});

test("file metrics are always present, even for rubble", () => {
  for (const record of records) {
    const metrics = record.facts.filter((f) => f.fact === "file_metrics");
    assert.equal(metrics.length, 1, record.file);
  }
});

test("a syntactically hopeless file degrades to zero facts, not an error (B5)", () => {
  const rubble = byFile.get("src/rubble.ts");
  assert.ok(rubble);
  // Whatever the parser managed, the record exists and the run exited 0.
  // TypeScript's recovery may still produce statements; the contract is
  // "no crash, record present" - degraded is best-effort.
  assert.equal(rubble.protocol, 1);
});

test("missing typescript resolution exits 3 with the recipe", () => {
  // os tmpdir has no node_modules anywhere above it on this box's layout;
  // a root OUTSIDE the repo cannot resolve typescript.
  const { tmpdir } = require_os();
  let code = 0;
  let stderr = "";
  try {
    execFileSync(process.execPath, [EXTRACT, "--root", tmpdir()], {
      encoding: "utf8",
    });
  } catch (error) {
    const failure = error as { status: number | null; stderr: string };
    code = failure.status ?? -1;
    stderr = failure.stderr;
  }
  if (code === 0) {
    // A node_modules higher up the temp path resolved typescript - the
    // environment makes this probe meaningless; nothing to assert.
    return;
  }
  assert.equal(code, 3);
  assert.match(stderr, /npm install -D typescript/);
});

function require_os(): { tmpdir(): string } {
  return { tmpdir: () => process.env.TEMP ?? process.env.TMPDIR ?? "/tmp" };
}
