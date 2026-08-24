/**
 * Protocol tests for ts-oracle (TCG-PROTOCOL-v0.1): drive the real
 * oracle process over NDJSON stdio against the fixture project, and
 * hold the D1 fact-parity contract against ts-extract on the same
 * fixture file. `node --test "test/*.test.ts"` (explicit glob — a bare
 * dir is "a module").
 */
import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createInterface, type Interface } from "node:readline";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";
import { readFileSync } from "node:fs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ORACLE = resolve(HERE, "..", "oracle.ts");
const FIXTURE = resolve(HERE, "fixtures", "proj");
const EXTRACT = resolve(HERE, "..", "..", "ts-extract", "extract.ts");

interface Pending {
  resolve(v: Record<string, unknown>): void;
}

class OracleClient {
  private child: ChildProcess;
  private rl: Interface;
  private nextId = 1;
  private pending = new Map<number, Pending>();
  readonly stderrLines: string[] = [];

  constructor() {
    this.child = spawn(process.execPath, [ORACLE], {
      stdio: ["pipe", "pipe", "pipe"],
    });
    if (
      this.child.stdout === null ||
      this.child.stdin === null ||
      this.child.stderr === null
    ) {
      throw new Error("oracle stdio not piped");
    }
    this.rl = createInterface({ input: this.child.stdout });
    this.rl.on("line", (line) => {
      let msg: Record<string, unknown>;
      try {
        msg = JSON.parse(line) as Record<string, unknown>;
      } catch {
        return;
      }
      const id = msg["id"];
      if (typeof id === "number") {
        const p = this.pending.get(id);
        if (p !== undefined) {
          this.pending.delete(id);
          p.resolve(msg);
        }
      }
    });
    createInterface({ input: this.child.stderr }).on("line", (l) =>
      this.stderrLines.push(l),
    );
  }

  send(
    op: string,
    params: Record<string, unknown>,
    proto = 1,
  ): Promise<Record<string, unknown>> {
    const id = this.nextId;
    this.nextId += 1;
    const line = `${JSON.stringify({ proto, id, op, params })}\n`;
    return new Promise((res) => {
      this.pending.set(id, { resolve: res });
      this.child.stdin?.write(line);
    });
  }

  async shutdown(): Promise<number | null> {
    await this.send("shutdown", {});
    return new Promise((res) => {
      this.child.on("exit", (code) => res(code));
    });
  }

  kill(): void {
    this.child.kill();
  }
}

function expectOk(msg: Record<string, unknown>): Record<string, unknown> {
  assert.equal(msg["ok"], true, `expected ok response: ${JSON.stringify(msg)}`);
  return msg["result"] as Record<string, unknown>;
}

function expectErr(msg: Record<string, unknown>): Record<string, unknown> {
  assert.equal(msg["ok"], false, `expected error response: ${JSON.stringify(msg)}`);
  return msg["error"] as Record<string, unknown>;
}

let oracle: OracleClient;

before(async () => {
  oracle = new OracleClient();
  const result = expectOk(
    await oracle.send("init", {
      root: FIXTURE,
      cells_dir: "src/cells",
      seam: "index",
    }),
  );
  assert.match(String(result["ts_version"]), /^\d+\./);
  assert.ok((result["root_files"] as number) >= 2, "fixture has 2 cells");
});

after(async () => {
  const code = await oracle.shutdown();
  assert.equal(code, 0, "graceful shutdown exits 0");
});

test("validate: clean fixture file has no error diagnostics, carries facts and markers", async () => {
  const r = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  const diags = r["diagnostics"] as Array<Record<string, unknown>>;
  assert.equal(
    diags.filter((d) => d["category"] === "error").length,
    0,
    JSON.stringify(diags),
  );
  assert.equal(r["degraded"], false);
  const facts = r["facts"] as Array<Record<string, unknown>>;
  const markers = r["markers"] as Array<Record<string, unknown>>;
  assert.ok(facts.some((f) => f["fact"] === "file_metrics"));
  assert.ok(
    facts.some(
      (f) => f["fact"] === "item" && f["symbol"] === "parseGuestName",
    ),
  );
  // the sanctioned brand cast is an as_cross fact — policy is Rust's job
  assert.ok(
    facts.some((f) => f["fact"] === "ts_unsafe" && f["kind"] === "as_cross"),
  );
  assert.ok(markers.some((m) => m["tag"] === "scope"));
  assert.ok(
    markers.some(
      (m) =>
        m["tag"] === "implements" &&
        String(m["uri"]).includes("#req-greet") &&
        m["symbol"] === "greet",
    ),
  );
});

test("validate: an overlay with a seeded type error reports it WITHOUT touching disk", async () => {
  const original = readFileSync(
    join(FIXTURE, "src/cells/greet/index.ts"),
    "utf8",
  );
  const seeded = `${original}\nconst bad: number = "oops";\n`;
  const r = expectOk(
    await oracle.send("validate", {
      file: "src/cells/greet/index.ts",
      content: seeded,
    }),
  );
  const diags = r["diagnostics"] as Array<Record<string, unknown>>;
  assert.ok(
    diags.some((d) => d["code"] === 2322),
    `TS2322 expected: ${JSON.stringify(diags)}`,
  );
  // the ephemeral overlay is gone: a plain validate is clean again
  const clean = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  const cleanDiags = clean["diagnostics"] as Array<Record<string, unknown>>;
  assert.equal(cleanDiags.filter((d) => d["category"] === "error").length, 0);
});

test("update: a standing overlay persists across validates until cleared", async () => {
  const original = readFileSync(
    join(FIXTURE, "src/cells/greet/index.ts"),
    "utf8",
  );
  expectOk(
    await oracle.send("update", {
      file: "src/cells/greet/index.ts",
      content: `${original}\nconst standing: string = 42;\n`,
    }),
  );
  const withOverlay = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  assert.ok(
    (withOverlay["diagnostics"] as Array<Record<string, unknown>>).some(
      (d) => d["code"] === 2322,
    ),
  );
  expectOk(
    await oracle.send("update", {
      file: "src/cells/greet/index.ts",
      content: null,
    }),
  );
  const cleared = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  assert.equal(
    (cleared["diagnostics"] as Array<Record<string, unknown>>).filter(
      (d) => d["category"] === "error",
    ).length,
    0,
  );
});

test("complete: prefix-filtered entries carry type text; any-typed entries are flagged unsafe", async () => {
  const dirty = readFileSync(
    join(FIXTURE, "src/cells/dirty/index.ts"),
    "utf8",
  );
  const probe = `${dirty}\nexport function probe(): number {\n  return anyTh\n}\n`;
  const lines = probe.split("\n");
  const lineNo = lines.findIndex((l) => l.includes("return anyTh")) + 1;
  const character = lines[lineNo - 1]!.indexOf("anyTh") + "anyTh".length;
  const r = expectOk(
    await oracle.send("complete", {
      file: "src/cells/dirty/index.ts",
      content: probe,
      position: { line: lineNo, character },
      prefix: "anyTh",
      max: 10,
    }),
  );
  const entries = r["entries"] as Array<Record<string, unknown>>;
  const hit = entries.find((e) => e["name"] === "anyThing");
  assert.ok(hit !== undefined, JSON.stringify(entries));
  assert.ok(String(hit["type_text"]).length > 0);
  assert.equal(hit["unsafe"], true, "any-typed completion must be flagged");
});

test("complete: a clean typed candidate is not flagged", async () => {
  const original = readFileSync(
    join(FIXTURE, "src/cells/greet/index.ts"),
    "utf8",
  );
  const probe = `${original}\nexport function probe2(n: GuestName): string {\n  return gre\n}\n`;
  const lines = probe.split("\n");
  const lineNo = lines.findIndex((l) => l.includes("return gre")) + 1;
  const character = lines[lineNo - 1]!.indexOf("gre") + "gre".length;
  const r = expectOk(
    await oracle.send("complete", {
      file: "src/cells/greet/index.ts",
      content: probe,
      position: { line: lineNo, character },
      prefix: "gre",
      max: 10,
    }),
  );
  const entries = r["entries"] as Array<Record<string, unknown>>;
  const hit = entries.find((e) => e["name"] === "greet");
  assert.ok(hit !== undefined, JSON.stringify(entries));
  assert.equal(hit["unsafe"], false);
  assert.match(String(hit["type_text"]), /GuestName/);
});

test("consecutive DIFFERENT overlays of one file each get fresh answers (the corpus cache bug)", async () => {
  const original = readFileSync(
    join(FIXTURE, "src/cells/greet/index.ts"),
    "utf8",
  );
  // first ephemeral overlay: type error A
  const a = expectOk(
    await oracle.send("validate", {
      file: "src/cells/greet/index.ts",
      content: `${original}\nconst a: number = "A";\n`,
    }),
  );
  assert.ok(
    (a["diagnostics"] as Array<Record<string, unknown>>).some(
      (d) => d["code"] === 2322,
    ),
  );
  // second ephemeral overlay, same file, DIFFERENT error class — the
  // language service must not serve the previous program state
  const b = expectOk(
    await oracle.send("validate", {
      file: "src/cells/greet/index.ts",
      content: `${original}\nexport const c = frobnicate("x");\n`,
    }),
  );
  const codes = (b["diagnostics"] as Array<Record<string, unknown>>).map(
    (d) => d["code"],
  );
  assert.ok(codes.includes(2304), `expected TS2304, got ${codes}`);
  assert.ok(!codes.includes(2322), "the previous overlay's error must be gone");
  // and back to clean disk
  const clean = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  assert.equal(
    (clean["diagnostics"] as Array<Record<string, unknown>>).filter(
      (d) => d["category"] === "error",
    ).length,
    0,
  );
});

test("type: quick info renders the signature", async () => {
  const original = readFileSync(
    join(FIXTURE, "src/cells/greet/index.ts"),
    "utf8",
  );
  const lines = original.split("\n");
  const lineNo =
    lines.findIndex((l) => l.includes("export function parseGuestName")) + 1;
  const character = lines[lineNo - 1]!.indexOf("parseGuestName") + 2;
  const r = expectOk(
    await oracle.send("type", {
      file: "src/cells/greet/index.ts",
      position: { line: lineNo, character },
    }),
  );
  assert.match(
    String(r["display"]),
    /parseGuestName\(input: unknown\): Result<GuestName, ParseError>/,
  );
});

test("scope: cell/seam context and the branded heuristic", async () => {
  const r = expectOk(
    await oracle.send("scope", { file: "src/cells/greet/index.ts" }),
  );
  assert.equal(r["cell"], "greet");
  assert.equal(r["seam_file"], "src/cells/greet/index.ts");
  const branded = r["branded"] as Array<Record<string, unknown>>;
  const guest = branded.find((b) => b["name"] === "GuestName");
  assert.ok(guest !== undefined, JSON.stringify(branded));
  assert.equal(guest["heuristic"], true);
  const symbols = r["symbols"] as Array<Record<string, unknown>>;
  assert.ok(symbols.some((s) => s["name"] === "parseGuestName"));
});

test("degraded and rubble: parser-recoverable garbage diagnoses, zero-statement text degrades, no crash", async () => {
  // Parser-RECOVERABLE garbage: statements exist, so per the extract
  // semantics this is NOT degraded — but syntactic diagnostics fire.
  const recovered = expectOk(
    await oracle.send("validate", {
      file: "src/cells/greet/index.ts",
      content: "export function {{{",
    }),
  );
  assert.equal(recovered["degraded"], false);
  const diags = recovered["diagnostics"] as Array<Record<string, unknown>>;
  assert.ok(diags.length > 0, "syntactic diagnostics expected");

  // Non-empty text that parses to ZERO statements is the extract B5
  // degraded case — the parity-relevant one.
  const degraded = expectOk(
    await oracle.send("validate", {
      file: "src/cells/greet/index.ts",
      content: "/* just a comment, no statements */",
    }),
  );
  assert.equal(degraded["degraded"], true);

  // and the session is still alive:
  const alive = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  assert.equal(alive["degraded"], false);
});

test("protocol: unknown op names the known set; proto mismatch is rejected", async () => {
  const unknown = expectErr(await oracle.send("frobnicate", {}));
  assert.equal(unknown["kind"], "protocol");
  assert.match(String(unknown["recipe"]), /init.*validate.*shutdown/s);
  const mismatch = expectErr(await oracle.send("validate", {}, 99));
  assert.equal(mismatch["kind"], "protocol");
});

test("fact parity with ts-extract on the same fixture file (D1)", async () => {
  const oracleSide = expectOk(
    await oracle.send("validate", { file: "src/cells/greet/index.ts" }),
  );
  const run = spawnSync(
    process.execPath,
    [EXTRACT, "--root", FIXTURE, "--files", "src/cells/greet/index.ts"],
    { encoding: "utf8" },
  );
  assert.equal(run.status, 0, run.stderr);
  const record = JSON.parse(run.stdout.trim()) as Record<string, unknown>;
  const canon = (v: unknown): string =>
    JSON.stringify(
      (v as Array<Record<string, unknown>>)
        .map((x) => JSON.stringify(x))
        .sort(),
    );
  assert.equal(
    canon(oracleSide["facts"]),
    canon(record["facts"]),
    "facts must match ts-extract's",
  );
  assert.equal(
    canon(oracleSide["markers"]),
    canon(record["markers"]),
    "markers must match ts-extract's",
  );
});
