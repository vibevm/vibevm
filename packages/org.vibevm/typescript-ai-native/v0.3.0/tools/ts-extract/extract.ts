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
 * Rust bridge (`ts-extract-bridge`) parses the stream; `protocol`
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

type ExtractFact = UnsafeFact | ImportFact | ItemFact | MetricsFact;

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

  const visit = (node: Node): void => {
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
    ts.forEachChild(node, visit);
  };
  try {
    visit(sf);
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
