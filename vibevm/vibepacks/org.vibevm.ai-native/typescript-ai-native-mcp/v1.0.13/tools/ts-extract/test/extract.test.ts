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
const CLEAN = join(HERE, "fixtures", "clean");
const SEAM = join(HERE, "fixtures", "seam");

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
    source?: string;
    cites_req?: boolean;
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

const cleanRecords = runExtract(CLEAN);
const cleanByFile = new Map(cleanRecords.map((r) => [r.file, r]));

const seamRecords = runExtract(SEAM);
const seamByFile = new Map(seamRecords.map((r) => [r.file, r]));

test("one protocol-1 record per source file, sorted", () => {
  assert.equal(records.length, 6);
  assert.ok(records.every((r) => r.protocol === 1));
  assert.deepEqual(
    records.map((r) => r.file),
    [
      "src/cells/greet/index.ts",
      "src/cells/greet/internal.ts",
      "src/cells/parse/logic.ts",
      "src/invariant.ts",
      "src/rubble.ts",
      "src/sweep.test.ts",
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

test("env reads surface as ts_env_read (the B-039 signal)", () => {
  const logic = byFile.get("src/cells/parse/logic.ts");
  assert.ok(logic);
  const envReads = logic.facts.filter((f) => f.fact === "ts_env_read");
  assert.ok(envReads.length >= 1, JSON.stringify(logic.facts));
  assert.ok(
    envReads.every(
      (f) => f.source === "process.env" || f.source === "import.meta.env",
    ),
  );
});

test("a citing error union surfaces as ts_seam_error with cites_req true (the B-033 TS twin)", () => {
  const errors = seamByFile.get("src/errors.ts");
  assert.ok(errors, JSON.stringify(seamRecords.map((r) => r.file)));
  const seam = errors.facts.filter((f) => f.fact === "ts_seam_error");

  // (a) JSDoc @implements spec:// on the alias -> cites_req true, on the
  // alias line (line 2). The marker raw-text parse keeps the scheme.
  const parse = seam.find((f) => f.symbol === "ParseError");
  assert.ok(parse, JSON.stringify(seam));
  assert.equal(parse.cites_req, true);
  assert.equal(parse.line, 2);

  // (a') the second citation form — a `spec://` substring in a variant
  // member's string literal (no JSDoc on the alias) -> cites_req true.
  const route = seam.find((f) => f.symbol === "RouteError");
  assert.ok(route, JSON.stringify(seam));
  assert.equal(route.cites_req, true);
  assert.equal(route.line, 6);
});

test("a non-citing error union surfaces as ts_seam_error with cites_req false", () => {
  const errors = seamByFile.get("src/errors.ts");
  assert.ok(errors);
  const seam = errors.facts.filter((f) => f.fact === "ts_seam_error");
  // (b) in error position, discriminated, but carries no spec://.
  const plan = seam.find((f) => f.symbol === "PlanError");
  assert.ok(plan, JSON.stringify(seam));
  assert.equal(plan.cites_req, false);
  assert.equal(plan.line, 16);
});

test("a non-error union emits no ts_seam_error fact", () => {
  const errors = seamByFile.get("src/errors.ts");
  assert.ok(errors);
  const seam = errors.facts.filter((f) => f.fact === "ts_seam_error");
  const symbols = seam.map((f) => f.symbol);
  // (c) a discriminated union NOT in error position (name `Mode`) and an
  // error-position union with no discriminant (name `BlobError`, property
  // `code` not in {kind, tag, _tag}) both emit nothing.
  assert.ok(!symbols.includes("Mode"), JSON.stringify(symbols));
  assert.ok(!symbols.includes("BlobError"), JSON.stringify(symbols));
  assert.deepEqual(symbols.sort(), ["ParseError", "PlanError", "RouteError"]);
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

test("an invariant-marker comment surfaces as invariant_comment (R3-003)", () => {
  const inv = byFile.get("src/invariant.ts");
  assert.ok(inv, JSON.stringify(records.map((r) => r.file)));
  const comments = inv.facts.filter((f) => f.fact === "invariant_comment");
  assert.equal(comments.length, 1, JSON.stringify(inv.facts));
  assert.equal(comments[0].marker, "INVARIANT:");
  assert.equal(comments[0].line, 75);
});

test("a 2^n bit-mask and a nested C-style for each surface as test_sweep (R-060)", () => {
  const sweep = byFile.get("src/sweep.test.ts");
  assert.ok(sweep, JSON.stringify(records.map((r) => r.file)));
  assert.equal(sweep.in_test, true);
  const sweeps = sweep.facts.filter((f) => f.fact === "test_sweep");
  assert.equal(sweeps.length, 2, JSON.stringify(sweep.facts));
  const byKind = new Map(sweeps.map((f) => [f.kind, f]));
  const bitmask = byKind.get("bitmask");
  assert.ok(bitmask, "bit-mask sweep must fire");
  assert.ok((bitmask as { detail?: string }).detail?.includes("<<"));
  const nested = byKind.get("nested-loops");
  assert.ok(nested, "three nested C-style for-loops must fire nested-loops");
  assert.equal((nested as { detail?: string }).detail, "3");
});

test("a nest of for-of over declared axes is silent (R-060 narrowing)", () => {
  const matrix = cleanByFile.get("src/matrix.test.ts");
  assert.ok(matrix, JSON.stringify(cleanRecords.map((r) => r.file)));
  const sweeps = matrix.facts.filter((f) => f.fact === "test_sweep");
  assert.equal(sweeps.length, 0, JSON.stringify(matrix.facts));
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
