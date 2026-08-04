/**
 * ts-extract — the Compiler-API fact extractor behind the `ts-tsc`
 * conform frontend and the TypeScript specmap scanner
 * (DEFERRALS-CLOSEOUT-PLAN v0.1, D2).
 *
 * One batched run per tree: walk the given roots (or an explicit file
 * list), parse each `.ts`/`.tsx`/`.mts`/`.cts` with the TypeScript
 * Compiler API, and stream ONE NDJSON record per file to stdout. The
 * record carries (a) conform facts — imports, the §8 `unsafe`-set
 * occurrences with AST-accurate classification, exported items, whole-
 * file metrics — and (b) specmap markers — the §9 JSDoc spec tags. The
 * Rust bridge (`typescript-ai-native-extract-bridge`) parses the stream; `protocol`
 * versions the record shape, and a bump retires conform's cache slots
 * wholesale via the frontend version.
 *
 * `typescript` is resolved from the CONSUMER's project (--root), never
 * bundled: it is the same install the tsc floor step already needs, so
 * the structural gate adds no new dependency. Resolution failure is a
 * hard, recipe-carrying error (exit 3) — never a silent skip.
 *
 * The file is erasable-syntax-only TypeScript: node >= 22.6 runs it
 * directly under type-stripping; no build step, no devDependency of its
 * own. Unparseable constructs degrade to a `degraded: true` record with
 * zero facts (the B5 rule) — one broken file never blinds the gate.
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { join, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { exit } from "node:process";

const PROTOCOL = 1;

/** The §9 tag vocabulary (PROP-014 edge kinds + the module-scope tag). */
const SPEC_TAGS = new Set([
  "implements",
  "verifies",
  "documents",
  "deviates",
  "informs",
  "scope",
]);

const SOURCE_EXTENSIONS = [".ts", ".tsx", ".mts", ".cts"];
const SKIP_DIRS = new Set([
  "node_modules",
  "dist",
  "build",
  "coverage",
  ".git",
  "vibedeps",
  "target",
]);

interface UnsafeFact {
  fact: "ts_unsafe";
  kind:
    | "any_type"
    | "as_cross"
    | "non_null"
    | "ts_ignore"
    | "ts_expect_error";
  line: number;
  reason: string | null;
}

interface ImportFact {
  fact: "import";
  to_path: string;
  line: number;
}

interface ItemFact {
  fact: "item";
  kind: string;
  symbol: string;
  line: number;
  is_exported: boolean;
  has_doc_example: boolean;
}

interface MetricsFact {
  fact: "file_metrics";
  lines: number;
}

interface EnvReadFact {
  fact: "ts_env_read";
  source: "process.env" | "import.meta.env";
  line: number;
}

/**
 * The `ts-seam-error-cites-req` signal (B-033 TS twin, §3.2): a
 * discriminated-union error type alias `E`. `symbol` is the alias name;
 * `cites_req` is whether the union cites a `spec://` REQ (a JSDoc
 * `@implements`/`@documents` marker on the alias OR a `spec://` substring
 * in a variant member). `in_test` is file-grain, stamped by the bridge
 * from the record — same posture as `EnvReadFact`.
 */
interface TsSeamErrorFact {
  fact: "ts_seam_error";
  symbol: string;
  cites_req: boolean;
  line: number;
}

/**
 * A comment carrying an invariant marker (`INVARIANT:` / `WARNING:` /
 * `PANICS:` / …), normalised to the config vocabulary's spelling. The
 * `ts-tsc` comment stream emits one per comment whose lead carries a
 * marker; `in_test` is file-grain, stamped by the bridge from the
 * record — same posture as `TsSeamErrorFact`. Consumed by
 * `invariant-comment-position` (R3-003).
 */
interface InvariantCommentFact {
  fact: "invariant_comment";
  marker: string;
  line: number;
}

/**
 * A swept test matrix (R-060): `kind` is `"bitmask"` (a `1 << n` / `2 ** n` /
 * `Math.pow(2, n)` loop bound) or `"nested-loops"` (a ≥3-deep Cartesian
 * nest); `detail` carries the bound text or the depth. Emitted only in test
 * files (`*.test.ts` / `*.spec.ts` / `__tests__`). Consumed by
 * `declared-test-matrices`.
 */
interface TestSweepFact {
  fact: "test_sweep";
  kind: "bitmask" | "nested-loops";
  detail: string;
  line: number;
}

type ExtractFact =
  | UnsafeFact
  | ImportFact
  | ItemFact
  | MetricsFact
  | EnvReadFact
  | TsSeamErrorFact
  | InvariantCommentFact
  | TestSweepFact;

interface Marker {
  tag: string;
  uri: string;
  reason: string | null;
  symbol: string | null;
  line: number;
}

interface FileRecord {
  protocol: number;
  file: string;
  in_test: boolean;
  degraded: boolean;
  facts: ExtractFact[];
  markers: Marker[];
}

function usage(): never {
  console.error(
    "usage: node extract.ts --root <dir> [--files <a.ts> <b.ts> ...]",
  );
  exit(2);
}

function parseArgs(argv: string[]): { root: string; files: string[] } {
  let root: string | null = null;
  const files: string[] = [];
  let mode: "none" | "files" = "none";
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--root") {
      const value = argv[i + 1];
      if (value === undefined) usage();
      root = value;
      i += 1;
      mode = "none";
    } else if (arg === "--files") {
      mode = "files";
    } else if (arg !== undefined && mode === "files") {
      files.push(arg);
    } else {
      usage();
    }
  }
  if (root === null) usage();
  return { root: resolve(root), files };
}

/**
 * Resolve the CONSUMER's `typescript` install relative to the project
 * root. Exit 3 with the recipe when absent — the bridge maps this to
 * its `typescript-unresolvable` error class.
 */
async function loadTypescript(root: string): Promise<TsModule> {
  const requireFromRoot = createRequire(
    pathToFileURL(join(root, "package.json")).href,
  );
  let resolved: string;
  try {
    resolved = requireFromRoot.resolve("typescript");
  } catch {
    console.error(
      `ts-extract: cannot resolve \`typescript\` from \`${root}\`. ` +
        "The structural gate parses with the project's own compiler — " +
        "run `npm install -D typescript` (the tsc floor step needs it too).",
    );
    exit(3);
  }
  const loaded = (await import(pathToFileURL(resolved).href)) as {
    default: TsModule;
  };
  return loaded.default;
}

/**
 * The slice of the `typescript` module surface this extractor uses —
 * declared here (structurally) because the module is loaded dynamically
 * from the consumer's install, so its own declaration files are not
 * available to the type-checker at authoring time.
 */
interface TsModule {
  version: string;
  ScriptTarget: { Latest: number };
  LanguageVariant: { Standard: number };
  SyntaxKind: {
    AnyKeyword: number;
    EndOfFileToken: number;
    SingleLineCommentTrivia: number;
    MultiLineCommentTrivia: number;
    PropertyAccessExpression: number;
    ElementAccessExpression: number;
    ForStatement: number;
    ForOfStatement: number;
    ForInStatement: number;
    WhileStatement: number;
    DoStatement: number;
  };
  createSourceFile(
    name: string,
    text: string,
    target: number,
    setParents: boolean,
  ): SourceFile;
  createScanner(
    target: number,
    skipTrivia: boolean,
    variant: number,
    text: string,
  ): Scanner;
  forEachChild(node: Node, cb: (child: Node) => void): void;
  getJSDocTags(node: Node): JsDocTag[];
  getTextOfJSDocComment(comment: unknown): string | undefined;
  isAsExpression(node: Node): boolean;
  isNonNullExpression(node: Node): boolean;
  isImportDeclaration(node: Node): boolean;
  isExportDeclaration(node: Node): boolean;
  isStringLiteral(node: Node): boolean;
  isCallExpression(node: Node): boolean;
  isIdentifier(node: Node): boolean;
  isTypeReferenceNode(node: Node): boolean;
  isUnionTypeNode(node: Node): boolean;
  isTypeLiteralNode(node: Node): boolean;
  isFunctionDeclaration(node: Node): boolean;
  isClassDeclaration(node: Node): boolean;
  isInterfaceDeclaration(node: Node): boolean;
  isTypeAliasDeclaration(node: Node): boolean;
  isEnumDeclaration(node: Node): boolean;
  isVariableStatement(node: Node): boolean;
  isModuleDeclaration(node: Node): boolean;
}

interface Node {
  kind: number;
  parent?: Node;
  getStart(sf?: SourceFile): number;
  getText(sf?: SourceFile): string;
}

interface SourceFile extends Node {
  statements: { length: number };
  text: string;
  getLineAndCharacterOfPosition(pos: number): { line: number };
}

interface Scanner {
  scan(): number;
  getTokenText(): string;
  getTokenStart(): number;
}

interface JsDocTag extends Node {
  tagName: { text: string };
  comment?: unknown;
}

function lineOf(sf: SourceFile, pos: number): number {
  return sf.getLineAndCharacterOfPosition(pos).line + 1;
}

/**
 * The fixed invariant-marker vocabulary the extractor emits — the
 * canonical spelling the config dictionary uses. The rule re-checks the
 * active config vocabulary, so the extractor emits generously; all five
 * are colon-bearing labeled tags (a marker is a labeled tag, not a prose
 * word), so each is self-anchoring.
 */
const INVARIANT_MARKERS = [
  "INVARIANT:",
  "WARNING:",
  "PANICS:",
  "MUST:",
  "NEVER:",
];

/**
 * The canonical invariant marker a comment leads with, or `null` when it
 * leads with none. Detection is anchored at the comment's first content
 * token (after the `//` / `/*` / `*` introducer and whitespace): a marker
 * not at the very start is not detected. This matches the all-caps
 * section-header convention and — deliberately — does not flag prose:
 * every marker is a colon-bearing labeled tag, so a bare must / never /
 * panics mid-sentence (or even leading one) is not an invariant
 * declaration and does not fire.
 *
 * Recorded limit: a marker embedded mid-comment is not seen; the match is
 * case-sensitive to the config's canonical spelling, so `// invariant:`
 * (lowercase) is not detected.
 */
function invariantMarkerOf(commentText: string): string | null {
  const lead = commentText.replace(/^[/!*!\s]+/u, "").trimStart();
  for (const marker of INVARIANT_MARKERS) {
    if (!lead.startsWith(marker)) continue;
    const bare = !marker.endsWith(":");
    const rest = lead.slice(marker.length);
    if (!bare) return marker;
    if (rest.length === 0 || !/[\w]/.test(rest.charAt(0))) return marker;
  }
  return null;
}

/** `@ts-expect-error -- reason` / `@ts-ignore` in one comment string. */
const SUPPRESSION = /@ts-(expect-error|ignore)(?:\s*--\s*(\S[^\n*]*))?/;

/**
 * Spec tag text, taken from the tag's RAW SOURCE, not `.comment`:
 * TypeScript PARSES some of our tag names (`@implements` most
 * prominently — its class-expression slot eats the `spec` scheme and
 * `.comment` keeps only `://…`), so the raw text is the one shape that
 * is stable across recognised and unrecognised tags (the Phase 0 spike
 * finding).
 */
const TAG_TEXT = /@([a-zA-Z-]+)\s+(\S+)(?:\s+(\S[^\n*]*))?/;

function markerFromTag(
  sf: SourceFile,
  tag: JsDocTag,
  ownerSymbol: string | null,
): Marker | null {
  const name = tag.tagName.text;
  if (!SPEC_TAGS.has(name)) return null;
  const raw = tag.getText(sf);
  const parsed = TAG_TEXT.exec(raw);
  if (parsed === null || parsed[2] === undefined) return null;
  return {
    tag: name,
    uri: parsed[2],
    reason: parsed[3]?.trim() ?? null,
    symbol: ownerSymbol,
    line: lineOf(sf, tag.getStart(sf)),
  };
}

interface DeclarationInfo {
  kind: string;
  symbol: string | null;
  isExported: boolean;
}

function declarationInfo(ts: TsModule, node: Node): DeclarationInfo | null {
  const named = node as unknown as {
    name?: { text?: string };
    modifiers?: Array<{ getText(): string }>;
    declarationList?: { declarations: Array<{ name: { getText(): string } }> };
  };
  const exported =
    named.modifiers?.some((m) => m.getText() === "export") ?? false;
  if (ts.isFunctionDeclaration(node)) {
    return { kind: "function", symbol: named.name?.text ?? null, isExported: exported };
  }
  if (ts.isClassDeclaration(node)) {
    return { kind: "class", symbol: named.name?.text ?? null, isExported: exported };
  }
  if (ts.isInterfaceDeclaration(node)) {
    return { kind: "interface", symbol: named.name?.text ?? null, isExported: exported };
  }
  if (ts.isTypeAliasDeclaration(node)) {
    return { kind: "type", symbol: named.name?.text ?? null, isExported: exported };
  }
  if (ts.isEnumDeclaration(node)) {
    return { kind: "enum", symbol: named.name?.text ?? null, isExported: exported };
  }
  if (ts.isVariableStatement(node)) {
    const first = named.declarationList?.declarations[0];
    return {
      kind: "const",
      symbol: first ? first.name.getText() : null,
      isExported: exported,
    };
  }
  if (ts.isModuleDeclaration(node)) {
    return { kind: "module", symbol: named.name?.text ?? null, isExported: exported };
  }
  return null;
}

/**
 * The composition-root read bases the `ts-flag-sites` rule polices
 * (GUIDE-AI-NATIVE-TYPESCRIPT §7, B-039): `process.env` (Node) and
 * `import.meta.env` (Vite/bundler). Returns the base label when `node`
 * is a property- or element-access whose OBJECT is one of these bases,
 * else `null`. The access itself may be `.X` or `["X"]`; a bare
 * `process.env` with no further access is not, on its own, a read site,
 * and a chained `process.env.X.Y` emits once — at the `process.env.X`
 * access (its object is the `process.env` base; the outer `.Y` access's
 * object is `process.env.X`, which is not a base, so it does not fire).
 */
function envReadSource(
  ts: TsModule,
  sf: SourceFile,
  node: Node,
): "process.env" | "import.meta.env" | null {
  const isAccess =
    node.kind === ts.SyntaxKind.PropertyAccessExpression ||
    node.kind === ts.SyntaxKind.ElementAccessExpression;
  if (!isAccess) return null;
  const base = (node as unknown as { expression?: Node }).expression;
  if (base === undefined || base.kind !== ts.SyntaxKind.PropertyAccessExpression) {
    return null;
  }
  const baseName = (
    base as unknown as { name: { getText(sf: SourceFile): string } }
  ).name.getText(sf);
  if (baseName !== "env") return null;
  const owner = (base as unknown as { expression: Node }).expression;
  // `<owner>.env`: the owner is the global exterior handle. `process`
  // parses as an identifier; `import.meta` parses as a `MetaProperty`,
  // so match on the owner's text rather than a specific node shape.
  const ownerText = owner.getText(sf);
  if (ownerText === "process") return "process.env";
  if (ownerText === "import.meta") return "import.meta.env";
  return null;
}

/**
 * The closed discriminant-property vocabulary for the error-union
 * heuristic (`##ts-seam-heuristic`). A variant member «carries a
 * discriminant» when it owns a property named one of these — the TS
 * discriminated-union idiom the guide's `E` uses (`kind` dominates;
 * `tag`/`_tag` are the common alternates). A discriminant named outside
 * this set is a documented limit, never a silent claim.
 */
const SEAM_DISCRIMINANTS = new Set(["kind", "tag", "_tag"]);

/**
 * Does the `ts-seam-error-cites-req` signal fire on this type alias
 * (B-033 TS twin, §3.2)? The conservative default, measured against the
 * guide's canonical `Result<T, E>` form
 * (`GUIDE-AI-NATIVE-TYPESCRIPT.md:152,157,159`) and `research/ts-demo`:
 * a `type` alias whose RHS is a union of object-literal members each
 * carrying a discriminant property, in error position (named `*Error` or
 * `E`). Returns the `ts_seam_error` fact for a matching alias, else
 * `null` — the non-matching remainder is the documented limit.
 *
 * **Recorded limits (the `ts-flag-sites` precedent — never silent):**
 * (1) error position is by NAME only — «the second type argument of a
 * `Result<T, E>`» is NOT detected, as it needs cross-reference
 * resolution (finding a `Result<_, ThisAlias>` usage) the single-file
 * AST walk does not do; (2) the degenerate single-object-literal `E`
 * whose `kind` is itself a string-literal union (one object type, not a
 * `UnionTypeNode` RHS) is not seen; (3) a discriminant named outside
 * `{kind, tag, _tag}` is not seen.
 */
function seamErrorFromAlias(
  ts: TsModule,
  sf: SourceFile,
  node: Node,
): TsSeamErrorFact | null {
  const alias = node as unknown as { name: { text: string }; type?: Node };
  const symbol = alias.name.text;
  if (symbol !== "E" && !symbol.endsWith("Error")) return null;
  const rhs = alias.type;
  if (rhs === undefined || !ts.isUnionTypeNode(rhs)) return null;
  const members = (rhs as unknown as { types?: Node[] }).types ?? [];
  if (members.length === 0) return null;
  // Every member is an object literal carrying a discriminant property.
  for (const member of members) {
    if (!ts.isTypeLiteralNode(member)) return null;
    if (!hasDiscriminant(member)) return null;
  }
  // cites_req — a `spec://` REQ cited via a JSDoc `@implements`/
  // `@documents` marker on the alias, OR a `spec://` substring in a
  // variant-member string literal.
  const citesReq =
    aliasCitesReqViaMarker(ts, sf, node, symbol) || containsSpecUri(ts, rhs);
  return {
    fact: "ts_seam_error",
    symbol,
    cites_req: citesReq,
    line: lineOf(sf, node.getStart(sf)),
  };
}

/** Does this object-literal type own a discriminant property? */
function hasDiscriminant(typeLiteral: Node): boolean {
  const members =
    (typeLiteral as unknown as { members?: Node[] }).members ?? [];
  return members.some((m) => {
    const name = (m as unknown as { name?: { text?: string } }).name;
    return name !== undefined && SEAM_DISCRIMINANTS.has(name.text ?? "");
  });
}

/**
 * Does the alias carry a JSDoc `@implements`/`@documents` marker whose
 * parsed URI is a `spec://` REQ? Reuses `markerFromTag` so the raw-text
 * parse (stable across parsed and unparsed tag names — the Phase 0 spike
 * finding) does the work.
 */
function aliasCitesReqViaMarker(
  ts: TsModule,
  sf: SourceFile,
  node: Node,
  symbol: string,
): boolean {
  for (const tag of ts.getJSDocTags(node)) {
    const marker = markerFromTag(sf, tag, symbol);
    if (
      marker !== null &&
      (marker.tag === "implements" || marker.tag === "documents") &&
      marker.uri.includes("spec://")
    ) {
      return true;
    }
  }
  return false;
}

/** Does any `StringLiteral` inside this subtree carry a `spec://` REQ? */
function containsSpecUri(ts: TsModule, node: Node): boolean {
  if (ts.isStringLiteral(node)) {
    const text = (node as unknown as { text: string }).text;
    if (text.includes("spec://")) return true;
  }
  let found = false;
  ts.forEachChild(node, (child) => {
    if (!found && containsSpecUri(ts, child)) found = true;
  });
  return found;
}

/**
 * Classifies a loop node for the swept-matrix census (R-060), or returns
 * `null` for a non-loop. `"for"` is the C-style `for` — the only kind that
 * carries a numeric bound (the bit-mask signal); `for...of`/`for...in` are
 * `"range"` (they iterate a collection, no numeric bound), and `while`/`do`
 * are `"loop"`. All three count toward the Cartesian-nest depth.
 */
function loopKind(ts: TsModule, node: Node): "for" | "range" | "loop" | null {
  if (node.kind === ts.SyntaxKind.ForStatement) return "for";
  if (
    node.kind === ts.SyntaxKind.ForOfStatement ||
    node.kind === ts.SyntaxKind.ForInStatement
  ) {
    return "range";
  }
  if (
    node.kind === ts.SyntaxKind.WhileStatement ||
    node.kind === ts.SyntaxKind.DoStatement
  ) {
    return "loop";
  }
  return null;
}

/**
 * The `2^n` bit-mask signal (R-060) for a C-style `for`: does its
 * initializer / condition / incrementor carry a `1 << n` shift, a `2 ** n`
 * exponentiation, or a `Math.pow(2, …)` call? Returns the bound's text when
 * it does, else `null`. Each pattern anchors on a digit NOT preceded by a
 * word char or dot, so `buf1 << n` or `a2 ** n` (identifiers ending in a
 * digit) never false-fire.
 */
function bitmaskBoundOfFor(sf: SourceFile, node: Node): string | null {
  const forStmt = node as unknown as {
    initializer?: Node;
    condition?: Node;
    incrementor?: Node;
  };
  const parts = [forStmt.initializer, forStmt.condition, forStmt.incrementor];
  for (const part of parts) {
    if (part === undefined) continue;
    const text = part.getText(sf);
    if (/(^|[^\w.])1\s*<</u.test(text)) return text.trim();
    if (/(^|[^\w.])2\s*\*\*/u.test(text)) return text.trim();
    if (/Math\.pow\s*\(\s*2\b/u.test(text)) return text.trim();
  }
  return null;
}

function extractFile(ts: TsModule, absPath: string, relPath: string): FileRecord {
  const text = readFileSync(absPath, "utf8");
  const record: FileRecord = {
    protocol: PROTOCOL,
    file: relPath,
    in_test: /\.test\.|\.spec\.|__tests__/.test(relPath),
    degraded: false,
    facts: [],
    markers: [],
  };
  const lines = text.length === 0 ? 0 : text.split("\n").length;
  record.facts.push({ fact: "file_metrics", lines });

  let sf: SourceFile;
  try {
    sf = ts.createSourceFile(relPath, text, ts.ScriptTarget.Latest, true);
  } catch {
    record.degraded = true;
    return record;
  }
  // A non-empty file that parses to zero statements is syntactic rubble.
  if (sf.statements.length === 0 && text.trim().length > 0) {
    record.degraded = true;
    return record;
  }

  const visit = (node: Node, loopDepth: number): void => {
    const envSource = envReadSource(ts, sf, node);
    if (envSource !== null) {
      record.facts.push({
        fact: "ts_env_read",
        source: envSource,
        line: lineOf(sf, node.getStart(sf)),
      });
    }
    if (node.kind === ts.SyntaxKind.AnyKeyword) {
      record.facts.push({
        fact: "ts_unsafe",
        kind: "any_type",
        line: lineOf(sf, node.getStart(sf)),
        reason: null,
      });
    }
    if (ts.isAsExpression(node)) {
      const asserted = (node as unknown as { type: Node }).type;
      const isConst =
        ts.isTypeReferenceNode(asserted) &&
        (asserted as unknown as { typeName: Node }).typeName.getText(sf) ===
          "const";
      if (!isConst) {
        record.facts.push({
          fact: "ts_unsafe",
          kind: "as_cross",
          line: lineOf(sf, node.getStart(sf)),
          reason: null,
        });
      }
    }
    if (ts.isNonNullExpression(node)) {
      record.facts.push({
        fact: "ts_unsafe",
        kind: "non_null",
        line: lineOf(sf, node.getStart(sf)),
        reason: null,
      });
    }
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) {
      const spec = (node as unknown as { moduleSpecifier?: Node })
        .moduleSpecifier;
      if (spec !== undefined && ts.isStringLiteral(spec)) {
        record.facts.push({
          fact: "import",
          to_path: (spec as unknown as { text: string }).text,
          line: lineOf(sf, node.getStart(sf)),
        });
      }
    }
    // Dynamic import("...") — the graph edge exists at runtime too.
    if (ts.isCallExpression(node)) {
      const call = node as unknown as {
        expression: Node & { kind: number };
        arguments: Node[];
      };
      const callee = call.expression.getText(sf);
      const first = call.arguments[0];
      if (
        (callee === "import" || callee === "require") &&
        first !== undefined &&
        ts.isStringLiteral(first)
      ) {
        record.facts.push({
          fact: "import",
          to_path: (first as unknown as { text: string }).text,
          line: lineOf(sf, node.getStart(sf)),
        });
      }
    }
    if (ts.isTypeAliasDeclaration(node)) {
      const seam = seamErrorFromAlias(ts, sf, node);
      if (seam !== null) record.facts.push(seam);
    }
    const decl = declarationInfo(ts, node);
    if (decl !== null && decl.symbol !== null) {
      const tags = ts.getJSDocTags(node);
      const docText = tags.length > 0 ? node.getText(sf) : "";
      record.facts.push({
        fact: "item",
        kind: decl.kind,
        symbol: decl.symbol,
        line: lineOf(sf, node.getStart(sf)),
        is_exported: decl.isExported,
        has_doc_example: /```|@example/.test(docText.slice(0, 2000)),
      });
      for (const tag of tags) {
        const marker = markerFromTag(sf, tag, decl.symbol);
        if (marker !== null) record.markers.push(marker);
      }
    } else {
      for (const tag of ts.getJSDocTags(node)) {
        const marker = markerFromTag(sf, tag, null);
        if (marker !== null) record.markers.push(marker);
      }
    }
    // Swept test matrices (R-060): a loop node in a test file. A C-style
    // `for` with a `2^n` bound is a bit-mask sweep; any loop kind counts
    // toward the Cartesian-nest depth. Declared matrices (a table iterated
    // once) emit nothing.
    const loop = loopKind(ts, node);
    if (loop !== null && record.in_test) {
      const childDepth = loopDepth + 1;
      const line = lineOf(sf, node.getStart(sf));
      if (loop === "for") {
        const bound = bitmaskBoundOfFor(sf, node);
        if (bound !== null) {
          record.facts.push({
            fact: "test_sweep",
            kind: "bitmask",
            line,
            detail: bound,
          });
        }
      }
      if (childDepth >= 3) {
        record.facts.push({
          fact: "test_sweep",
          kind: "nested-loops",
          line,
          detail: String(childDepth),
        });
      }
      ts.forEachChild(node, (child) => visit(child, childDepth));
      return;
    }
    ts.forEachChild(node, (child) => visit(child, loopDepth));
  };
  try {
    visit(sf, 0);
  } catch {
    record.degraded = true;
    record.facts = record.facts.filter((f) => f.fact === "file_metrics");
    record.markers = [];
    return record;
  }

  // Comment stream: suppression directives live in trivia, not the AST.
  const scanner = ts.createScanner(
    ts.ScriptTarget.Latest,
    false,
    ts.LanguageVariant.Standard,
    text,
  );
  let token = scanner.scan();
  const seenCommentStarts = new Set<number>();
  while (token !== ts.SyntaxKind.EndOfFileToken) {
    if (
      token === ts.SyntaxKind.SingleLineCommentTrivia ||
      token === ts.SyntaxKind.MultiLineCommentTrivia
    ) {
      const start = scanner.getTokenStart();
      if (!seenCommentStarts.has(start)) {
        seenCommentStarts.add(start);
        const commentText = scanner.getTokenText();
        const match = SUPPRESSION.exec(commentText);
        if (match !== null) {
          record.facts.push({
            fact: "ts_unsafe",
            kind: match[1] === "ignore" ? "ts_ignore" : "ts_expect_error",
            line: lineOf(sf, start),
            reason: match[2]?.trim() ?? null,
          });
        }
        // A file-level `@scope` block is module-grain by definition and
        // may sit detached from any declaration (e.g. followed by a
        // second JSDoc block — TypeScript then attaches only the
        // nearest block to the node, orphaning the first). Catch it in
        // the comment stream; the marker dedup collapses the doubled
        // case where the AST DID attach it.
        const scopeMatch = /@scope\s+(\S+)/u.exec(commentText);
        if (scopeMatch !== null && scopeMatch[1] !== undefined) {
          record.markers.push({
            tag: "scope",
            uri: scopeMatch[1],
            reason: null,
            symbol: null,
            line: lineOf(sf, start),
          });
        }
        // An invariant marker leading the comment feeds
        // invariant-comment-position; the marker is normalised to the
        // config spelling, and in_test is stamped by the bridge.
        const invariant = invariantMarkerOf(commentText);
        if (invariant !== null) {
          record.facts.push({
            fact: "invariant_comment",
            marker: invariant,
            line: lineOf(sf, start),
          });
        }
      }
    }
    token = scanner.scan();
  }

  // Markers dedup: JSDoc tags attach to several AST layers at once.
  const seen = new Set<string>();
  record.markers = record.markers.filter((m) => {
    const key = `${m.tag} ${m.uri} ${m.line}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  return record;
}

function walkSources(root: string): string[] {
  const out: string[] = [];
  const stack = [root];
  while (stack.length > 0) {
    const dir = stack.pop();
    if (dir === undefined) break;
    let entries: string[];
    try {
      entries = readdirSync(dir);
    } catch {
      continue;
    }
    for (const entry of entries.sort()) {
      const full = join(dir, entry);
      let st;
      try {
        st = statSync(full);
      } catch {
        continue;
      }
      if (st.isDirectory()) {
        if (!SKIP_DIRS.has(entry) && !entry.startsWith(".")) stack.push(full);
      } else if (SOURCE_EXTENSIONS.some((ext) => entry.endsWith(ext))) {
        if (!entry.endsWith(".d.ts")) out.push(full);
      }
    }
  }
  return out.sort();
}

async function main(): Promise<void> {
  const { root, files } = parseArgs(process.argv);
  const ts = await loadTypescript(root);
  const targets =
    files.length > 0 ? files.map((f) => resolve(root, f)) : walkSources(root);
  for (const abs of targets) {
    const rel = relative(root, abs).split(sep).join("/");
    const record = extractFile(ts, abs, rel);
    process.stdout.write(`${JSON.stringify(record)}\n`);
  }
  console.error(
    `ts-extract: ${targets.length} file(s), typescript ${ts.version}, protocol ${PROTOCOL}.`,
  );
}

await main();
