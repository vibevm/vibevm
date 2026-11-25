/**
 * ts-oracle — the long-lived TypeScript language-service oracle behind
 * the agentic tcg line (AGENTIC-TCG-TS-PLAN v0.1; mechanisms:
 * TCG-ORACLE-v0.1, TCG-PROTOCOL-v0.1).
 *
 * One persistent process per project root: NDJSON requests on stdin,
 * NDJSON responses on stdout (correlation by `id`), human log lines on
 * stderr only. Ops: init / update / validate / scope / complete / type /
 * shutdown. Validation runs against IN-MEMORY overlays over an
 * incremental LanguageService — the agent checks an edit before any
 * byte reaches disk.
 *
 * `typescript` is resolved from the CONSUMER's project (--root /
 * init.root), never bundled — the same install the tsc floor step needs
 * (exit 3 with the recipe when absent). The project config is read via
 * getParsedCommandLineOfConfigFile — the SAME path tsc uses — so option
 * assembly cannot drift from the floor.
 *
 * The file is erasable-syntax-only TypeScript (node >= 22.6 strip-types)
 * and is SELF-CONTAINED by design: it is embedded into the Rust bridge
 * via include_str! and materialised as ONE file (TCG-ORACLE-v0.1 §1), so
 * it must not import sibling tool files. The per-file fact/marker logic
 * below (§ "fact extraction") is consciously DUPLICATED from
 * ts-extract/extract.ts; the package's fact-parity test keeps the two
 * behaviourally aligned. Fix bugs in BOTH places.
 *
 * B5 extended (TCG-ORACLE-v0.1 §5): no input kills the process — every
 * op failure is an {ok:false} response and the loop continues.
 */

import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve, dirname } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
import { performance } from "node:perf_hooks";
import { exit, argv, stdin, stdout, stderr } from "node:process";

const ORACLE_PROTOCOL = 1;

/** The §9 tag vocabulary — kept identical to ts-extract (parity test). */
const SPEC_TAGS = new Set([
  "implements",
  "verifies",
  "documents",
  "deviates",
  "informs",
  "scope",
]);

// ---------------------------------------------------------------------------
// Protocol shapes (TCG-PROTOCOL-v0.1 §1–§2)
// ---------------------------------------------------------------------------

interface UnsafeFact {
  fact: "ts_unsafe";
  kind: "any_type" | "as_cross" | "non_null" | "ts_ignore" | "ts_expect_error";
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

interface Position {
  line: number; // 1-based
  character: number; // 0-based
}

interface OracleDiagnostic {
  code: number;
  category: "error" | "warning" | "suggestion" | "message";
  message: string;
  line: number;
  character: number;
}

interface OracleRequest {
  proto?: number;
  id?: number;
  op?: string;
  params?: Record<string, unknown>;
}

interface OracleErrorBody {
  kind:
    | "node-missing"
    | "typescript-unresolvable"
    | "oracle-crashed"
    | "protocol"
    | "timeout";
  detail: string;
  recipe?: string;
}

// ---------------------------------------------------------------------------
// The typescript module surface this oracle uses — declared structurally
// because the module is loaded dynamically from the consumer's install
// (the ts-extract technique, widened to the LanguageService API).
// ---------------------------------------------------------------------------

interface TsModule {
  version: string;
  sys: TsSystem;
  ScriptTarget: { Latest: number };
  LanguageVariant: { Standard: number };
  SyntaxKind: {
    AnyKeyword: number;
    EndOfFileToken: number;
    SingleLineCommentTrivia: number;
    MultiLineCommentTrivia: number;
  };
  DiagnosticCategory: {
    Error: number;
    Warning: number;
    Suggestion: number;
    Message: number;
  };
  ScriptSnapshot: {
    fromString(text: string): unknown;
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
  createDocumentRegistry(): unknown;
  createLanguageService(host: unknown, registry: unknown): LanguageService;
  getParsedCommandLineOfConfigFile(
    configFileName: string,
    optionsToExtend: unknown,
    host: unknown,
  ): ParsedCommandLine | undefined;
  getDefaultLibFilePath(options: unknown): string;
  flattenDiagnosticMessageText(text: unknown, newline: string): string;
  displayPartsToString(parts: unknown): string;
  forEachChild(node: Node, cb: (child: Node) => void): void;
  getJSDocTags(node: Node): JsDocTag[];
  isAsExpression(node: Node): boolean;
  isNonNullExpression(node: Node): boolean;
  isImportDeclaration(node: Node): boolean;
  isExportDeclaration(node: Node): boolean;
  isStringLiteral(node: Node): boolean;
  isCallExpression(node: Node): boolean;
  isTypeReferenceNode(node: Node): boolean;
  isFunctionDeclaration(node: Node): boolean;
  isClassDeclaration(node: Node): boolean;
  isInterfaceDeclaration(node: Node): boolean;
  isTypeAliasDeclaration(node: Node): boolean;
  isEnumDeclaration(node: Node): boolean;
  isVariableStatement(node: Node): boolean;
  isModuleDeclaration(node: Node): boolean;
  isIntersectionTypeNode(node: Node): boolean;
  isTypeLiteralNode(node: Node): boolean;
  isPropertySignature(node: Node): boolean;
}

interface TsSystem {
  fileExists(path: string): boolean;
  readFile(path: string): string | undefined;
  readDirectory(
    path: string,
    extensions?: readonly string[],
    exclude?: readonly string[],
    include?: readonly string[],
    depth?: number,
  ): string[];
  directoryExists(path: string): boolean;
  getDirectories(path: string): string[];
  realpath?(path: string): string;
  useCaseSensitiveFileNames: boolean;
}

interface ParsedCommandLine {
  fileNames: string[];
  options: Record<string, unknown>;
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
  getLineAndCharacterOfPosition(pos: number): {
    line: number;
    character: number;
  };
  getPositionOfLineAndCharacter(line: number, character: number): number;
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

interface TsDiagnostic {
  code: number;
  category: number;
  messageText: unknown;
  file?: SourceFile;
  start?: number;
}

interface CompletionEntry {
  name: string;
  kind: string;
  sortText: string;
}
interface CompletionInfo {
  entries: CompletionEntry[];
}
interface QuickInfo {
  displayParts?: unknown;
  documentation?: unknown;
}
interface CompletionEntryDetails {
  displayParts?: unknown;
}

interface LanguageService {
  getSyntacticDiagnostics(file: string): TsDiagnostic[];
  getSemanticDiagnostics(file: string): TsDiagnostic[];
  getCompletionsAtPosition(
    file: string,
    position: number,
    options: Record<string, unknown>,
  ): CompletionInfo | undefined;
  getCompletionEntryDetails(
    file: string,
    position: number,
    entryName: string,
    formatOptions: undefined,
    source: undefined,
    preferences: undefined,
    data: undefined,
  ): CompletionEntryDetails | undefined;
  getQuickInfoAtPosition(file: string, position: number): QuickInfo | undefined;
  getProgram(): { getSourceFile(file: string): SourceFile | undefined } | undefined;
}

// ---------------------------------------------------------------------------
// Consumer-typescript resolution (the ts-extract technique)
// ---------------------------------------------------------------------------

async function loadTypescript(root: string): Promise<TsModule> {
  const requireFromRoot = createRequire(
    pathToFileURL(join(root, "package.json")).href,
  );
  let resolved: string;
  try {
    resolved = requireFromRoot.resolve("typescript");
  } catch {
    stderr.write(
      `ts-oracle: cannot resolve \`typescript\` from \`${root}\`. ` +
        "The oracle answers with the project's own compiler - " +
        "run `npm install -D typescript` (the tsc floor step needs it too).\n",
    );
    exit(3);
  }
  const loaded = (await import(pathToFileURL(resolved).href)) as {
    default: TsModule;
  };
  return loaded.default;
}

// ---------------------------------------------------------------------------
// Fact extraction — DUPLICATED from ts-extract/extract.ts (TCG-ORACLE §1;
// the fact-parity test pins this). Operates on text, not disk.
// ---------------------------------------------------------------------------

const SUPPRESSION = /@ts-(expect-error|ignore)(?:\s*--\s*(\S[^\n*]*))?/;
const TAG_TEXT = /@([a-zA-Z-]+)\s+(\S+)(?:\s+(\S[^\n*]*))?/;

function lineOf(sf: SourceFile, pos: number): number {
  return sf.getLineAndCharacterOfPosition(pos).line + 1;
}

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

interface FactExtraction {
  facts: ExtractFact[];
  markers: Marker[];
  degraded: boolean;
}

function extractFactsFromText(
  ts: TsModule,
  relPath: string,
  text: string,
): FactExtraction {
  const out: FactExtraction = { facts: [], markers: [], degraded: false };
  const lines = text.length === 0 ? 0 : text.split("\n").length;
  out.facts.push({ fact: "file_metrics", lines });

  let sf: SourceFile;
  try {
    sf = ts.createSourceFile(relPath, text, ts.ScriptTarget.Latest, true);
  } catch {
    out.degraded = true;
    return out;
  }
  if (sf.statements.length === 0 && text.trim().length > 0) {
    out.degraded = true;
    return out;
  }

  const visit = (node: Node): void => {
    if (node.kind === ts.SyntaxKind.AnyKeyword) {
      out.facts.push({
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
        out.facts.push({
          fact: "ts_unsafe",
          kind: "as_cross",
          line: lineOf(sf, node.getStart(sf)),
          reason: null,
        });
      }
    }
    if (ts.isNonNullExpression(node)) {
      out.facts.push({
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
        out.facts.push({
          fact: "import",
          to_path: (spec as unknown as { text: string }).text,
          line: lineOf(sf, node.getStart(sf)),
        });
      }
    }
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
        out.facts.push({
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
      out.facts.push({
        fact: "item",
        kind: decl.kind,
        symbol: decl.symbol,
        line: lineOf(sf, node.getStart(sf)),
        is_exported: decl.isExported,
        has_doc_example: /```|@example/.test(docText.slice(0, 2000)),
      });
      for (const tag of tags) {
        const marker = markerFromTag(sf, tag, decl.symbol);
        if (marker !== null) out.markers.push(marker);
      }
    } else {
      for (const tag of ts.getJSDocTags(node)) {
        const marker = markerFromTag(sf, tag, null);
        if (marker !== null) out.markers.push(marker);
      }
    }
    ts.forEachChild(node, visit);
  };
  try {
    visit(sf);
  } catch {
    out.degraded = true;
    out.facts = out.facts.filter((f) => f.fact === "file_metrics");
    out.markers = [];
    return out;
  }

  // Comment stream: suppressions + detached file-level @scope.
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
          out.facts.push({
            fact: "ts_unsafe",
            kind: match[1] === "ignore" ? "ts_ignore" : "ts_expect_error",
            line: lineOf(sf, start),
            reason: match[2]?.trim() ?? null,
          });
        }
        const scopeMatch = /@scope\s+(\S+)/u.exec(commentText);
        if (scopeMatch !== null && scopeMatch[1] !== undefined) {
          out.markers.push({
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

  const seen = new Set<string>();
  out.markers = out.markers.filter((m) => {
    const key = `${m.tag} ${m.uri} ${m.line}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  return out;
}

// ---------------------------------------------------------------------------
// The oracle session — one LanguageService per init root (TCG-ORACLE §3)
// ---------------------------------------------------------------------------

function norm(p: string): string {
  return p.replace(/\\/g, "/");
}

class OracleSession {
  private readonly ts: TsModule;
  readonly root: string;
  private readonly parsed: ParsedCommandLine;
  private readonly overlays = new Map<
    string,
    { content: string; version: number }
  >();
  /** Session-monotonic script version. NEVER derived from the current
   * overlay: an ephemeral overlay is removed after its query, and a
   * later overlay of the same file would otherwise REUSE the version
   * number while carrying different text — the language service then
   * serves the cached program state (caught by the differential
   * corpus: five seeded cases answered from the clean-disk cache). */
  private nextVersion = 1;
  private readonly service: LanguageService;
  private cellsDir: string | null;
  private seamName: string;
  configFile: string;

  constructor(
    ts: TsModule,
    root: string,
    configFile: string,
    parsed: ParsedCommandLine,
    cellsDir: string | null,
    seamName: string,
  ) {
    this.ts = ts;
    this.root = root;
    this.configFile = configFile;
    this.parsed = parsed;
    this.cellsDir = cellsDir;
    this.seamName = seamName;

    const overlays = this.overlays;
    const host = {
      getScriptFileNames: (): string[] => {
        const names = [...parsed.fileNames.map(norm)];
        for (const k of overlays.keys()) if (!names.includes(k)) names.push(k);
        return names;
      },
      getScriptVersion: (f: string): string => {
        const o = overlays.get(norm(f));
        if (o !== undefined) return `o${o.version}`;
        // Disk files version by mtime, or an agent that WRITES between
        // queries would keep getting the cached program (same class as
        // the ephemeral-overlay bug the corpus caught).
        try {
          return `m${statSync(f).mtimeMs}`;
        } catch {
          return "absent";
        }
      },
      getScriptSnapshot: (f: string): unknown => {
        const o = overlays.get(norm(f));
        if (o !== undefined) return ts.ScriptSnapshot.fromString(o.content);
        if (!ts.sys.fileExists(f)) return undefined;
        return ts.ScriptSnapshot.fromString(ts.sys.readFile(f) ?? "");
      },
      getCurrentDirectory: (): string => root,
      getCompilationSettings: (): Record<string, unknown> => parsed.options,
      getDefaultLibFileName: (o: unknown): string =>
        ts.getDefaultLibFilePath(o),
      fileExists: (f: string): boolean =>
        overlays.has(norm(f)) || ts.sys.fileExists(f),
      readFile: (f: string): string | undefined =>
        overlays.get(norm(f))?.content ?? ts.sys.readFile(f),
      readDirectory: ts.sys.readDirectory.bind(ts.sys),
      directoryExists: ts.sys.directoryExists.bind(ts.sys),
      getDirectories: ts.sys.getDirectories.bind(ts.sys),
      realpath: ts.sys.realpath?.bind(ts.sys),
    };
    this.service = ts.createLanguageService(host, ts.createDocumentRegistry());
  }

  tsVersion(): string {
    return this.ts.version;
  }

  rootFileCount(): number {
    return this.parsed.fileNames.length;
  }

  /** Absolute normalised path for a root-relative protocol path. */
  private abs(rel: string): string {
    return norm(resolve(this.root, rel));
  }

  update(rel: string, content: string | null): number {
    const key = this.abs(rel);
    if (content === null) {
      this.overlays.delete(key);
      // Deleting still moves the session clock: the next read of this
      // file (disk state) must not collide with a stale cached version.
      this.nextVersion += 1;
      return this.nextVersion;
    }
    this.nextVersion += 1;
    this.overlays.set(key, { content, version: this.nextVersion });
    return this.nextVersion;
  }

  /** Run `body` with a one-shot overlay when `content` is given. */
  private withEphemeralOverlay<T>(
    rel: string,
    content: string | undefined,
    body: () => T,
  ): T {
    if (content === undefined) return body();
    const key = this.abs(rel);
    const prev = this.overlays.get(key);
    this.nextVersion += 1;
    this.overlays.set(key, { content, version: this.nextVersion });
    try {
      return body();
    } finally {
      this.nextVersion += 1;
      if (prev === undefined) this.overlays.delete(key);
      else
        this.overlays.set(key, {
          content: prev.content,
          version: this.nextVersion,
        });
    }
  }

  private currentText(rel: string): string {
    const key = this.abs(rel);
    const o = this.overlays.get(key);
    if (o !== undefined) return o.content;
    return readFileSync(key, "utf8");
  }

  private toDiagnostic(d: TsDiagnostic): OracleDiagnostic {
    const categories: Record<number, OracleDiagnostic["category"]> = {
      [this.ts.DiagnosticCategory.Error]: "error",
      [this.ts.DiagnosticCategory.Warning]: "warning",
      [this.ts.DiagnosticCategory.Suggestion]: "suggestion",
      [this.ts.DiagnosticCategory.Message]: "message",
    };
    let line = 0;
    let character = 0;
    if (d.file !== undefined && d.start !== undefined) {
      const lc = d.file.getLineAndCharacterOfPosition(d.start);
      line = lc.line + 1;
      character = lc.character;
    }
    return {
      code: d.code,
      category: categories[d.category] ?? "message",
      message: this.ts.flattenDiagnosticMessageText(d.messageText, "\n"),
      line,
      character,
    };
  }

  validate(rel: string, content: string | undefined): Record<string, unknown> {
    return this.withEphemeralOverlay(rel, content, () => {
      const key = this.abs(rel);
      const text = this.currentText(rel);
      const extraction = extractFactsFromText(this.ts, rel, text);
      let diagnostics: OracleDiagnostic[] = [];
      try {
        const syntactic = this.service.getSyntacticDiagnostics(key);
        const semantic =
          syntactic.length > 0 ? [] : this.service.getSemanticDiagnostics(key);
        diagnostics = [...syntactic, ...semantic].map((d) =>
          this.toDiagnostic(d),
        );
      } catch {
        extraction.degraded = true;
      }
      return {
        diagnostics,
        facts: extraction.facts,
        markers: extraction.markers,
        degraded: extraction.degraded,
      };
    });
  }

  private offsetOf(rel: string, pos: Position): number {
    const text = this.currentText(rel);
    // Build the offset from the text directly — 1-based line, 0-based char.
    let line = 1;
    let offset = 0;
    while (line < pos.line) {
      const nl = text.indexOf("\n", offset);
      if (nl < 0) break;
      offset = nl + 1;
      line += 1;
    }
    return Math.min(offset + pos.character, text.length);
  }

  private cellOf(rel: string): string | null {
    if (this.cellsDir === null) return null;
    const n = norm(rel);
    const prefix = norm(this.cellsDir) + "/";
    if (!n.startsWith(prefix)) return null;
    const rest = n.slice(prefix.length);
    const slash = rest.indexOf("/");
    return slash < 0 ? rest : rest.slice(0, slash);
  }

  /** Exported brand-shaped type aliases in a seam file (heuristic — see
   * TCG-ORACLE §4: syntactic intersection-brand detection, not checker
   * work; labelled heuristic:true in every answer). */
  private brandedInFile(absPath: string): string[] {
    const ts = this.ts;
    let text: string;
    try {
      text = this.overlays.get(norm(absPath))?.content ?? readFileSync(absPath, "utf8");
    } catch {
      return [];
    }
    let sf: SourceFile;
    try {
      sf = ts.createSourceFile(absPath, text, ts.ScriptTarget.Latest, true);
    } catch {
      return [];
    }
    const found: string[] = [];
    const visit = (node: Node): void => {
      if (ts.isTypeAliasDeclaration(node)) {
        const alias = node as unknown as {
          name?: { text?: string };
          type: Node;
          modifiers?: Array<{ getText(): string }>;
        };
        const exported =
          alias.modifiers?.some((m) => m.getText() === "export") ?? false;
        if (exported && alias.name?.text !== undefined) {
          const t = alias.type;
          if (ts.isIntersectionTypeNode(t)) {
            const members = (t as unknown as { types: Node[] }).types;
            const hasBrand = members.some(
              (m) =>
                ts.isTypeLiteralNode(m) &&
                (m as unknown as { members: Node[] }).members.some(
                  (p) =>
                    ts.isPropertySignature(p) &&
                    (p as unknown as { name: { getText(): string } }).name
                      .getText()
                      .includes("__brand"),
                ),
            );
            if (hasBrand) found.push(alias.name.text);
          }
        }
      }
      ts.forEachChild(node, visit);
    };
    try {
      visit(sf);
    } catch {
      /* heuristic — partial answers are fine */
    }
    return found;
  }

  scope(rel: string, pos: Position | undefined): Record<string, unknown> {
    const key = this.abs(rel);
    const text = this.currentText(rel);
    const offset =
      pos === undefined ? Math.max(text.length - 1, 0) : this.offsetOf(rel, pos);

    let symbols: Array<Record<string, unknown>> = [];
    try {
      const info = this.service.getCompletionsAtPosition(key, offset, {});
      // In-scope identifiers only: filter member/global noise down to the
      // entries the checker sorts as locals-and-imports first.
      const entries = info?.entries ?? [];
      symbols = entries
        .filter((e) => e.sortText <= "15")
        .slice(0, 200)
        .map((e) => ({ name: e.name, kind: e.kind, type_text: "" }));
    } catch {
      /* degraded: empty symbol set */
    }

    const cell = this.cellOf(rel);
    let seamFile: string | null = null;
    const branded: Array<Record<string, unknown>> = [];
    if (this.cellsDir !== null) {
      const cellsAbs = resolve(this.root, this.cellsDir);
      if (cell !== null) {
        seamFile = norm(
          join(this.cellsDir, cell, `${this.seamName}.ts`),
        );
      }
      // branded exports across every cell's seam
      try {
        for (const entry of readdirSync(cellsAbs)) {
          const seamAbs = join(cellsAbs, entry, `${this.seamName}.ts`);
          if (existsSync(seamAbs)) {
            for (const name of this.brandedInFile(seamAbs)) {
              branded.push({
                name,
                seam: norm(join(this.cellsDir, entry, `${this.seamName}.ts`)),
                heuristic: true,
              });
            }
          }
        }
      } catch {
        /* heuristic — partial answers are fine */
      }
    }

    return { symbols, cell, seam_file: seamFile, branded };
  }

  complete(
    rel: string,
    pos: Position,
    content: string | undefined,
    prefix: string | undefined,
    max: number,
  ): Record<string, unknown> {
    return this.withEphemeralOverlay(rel, content, () => {
      const key = this.abs(rel);
      const offset = this.offsetOf(rel, pos);
      const info = this.service.getCompletionsAtPosition(key, offset, {});
      let entries = info?.entries ?? [];
      if (prefix !== undefined && prefix.length > 0) {
        entries = entries.filter((e) => e.name.startsWith(prefix));
      }
      entries = entries.slice(0, max);
      const out = entries.map((e) => {
        let typeText = "";
        let unsafe = false;
        // Details are per-entry checker work: only affordable AFTER the
        // prefix/max cut (TCG-PROTOCOL: prefix?/max? exist exactly for this).
        try {
          const details = this.service.getCompletionEntryDetails(
            key,
            offset,
            e.name,
            undefined,
            undefined,
            undefined,
            undefined,
          );
          if (details?.displayParts !== undefined) {
            typeText = this.ts.displayPartsToString(details.displayParts);
            unsafe = /\bany\b/.test(typeText);
          }
        } catch {
          /* detail-less entry is still an answer */
        }
        return { name: e.name, kind: e.kind, type_text: typeText, unsafe };
      });
      return { entries: out };
    });
  }

  quickInfo(
    rel: string,
    pos: Position,
    content: string | undefined,
  ): Record<string, unknown> {
    return this.withEphemeralOverlay(rel, content, () => {
      const key = this.abs(rel);
      const offset = this.offsetOf(rel, pos);
      const qi = this.service.getQuickInfoAtPosition(key, offset);
      return {
        display:
          qi?.displayParts !== undefined
            ? this.ts.displayPartsToString(qi.displayParts)
            : "",
        documentation:
          qi?.documentation !== undefined
            ? this.ts.displayPartsToString(qi.documentation)
            : "",
      };
    });
  }
}

// ---------------------------------------------------------------------------
// The NDJSON loop (TCG-PROTOCOL §1)
// ---------------------------------------------------------------------------

function respond(id: number | null, body: Record<string, unknown>): void {
  stdout.write(
    `${JSON.stringify({ proto: ORACLE_PROTOCOL, id, ...body })}\n`,
  );
}

function respondError(id: number | null, error: OracleErrorBody): void {
  respond(id, { ok: false, error });
}

const KNOWN_OPS = [
  "init",
  "update",
  "validate",
  "scope",
  "complete",
  "type",
  "shutdown",
];

async function main(): Promise<void> {
  // --root is optional pre-init context: init may (re)set the root.
  let bootRoot: string | null = null;
  for (let i = 2; i < argv.length; i += 1) {
    if (argv[i] === "--root") {
      const value = argv[i + 1];
      if (value !== undefined) bootRoot = resolve(value);
      i += 1;
    }
  }

  let session: OracleSession | null = null;
  let ts: TsModule | null = null;

  const initSession = async (
    root: string,
    cellsDir: string | null,
    seamName: string,
  ): Promise<Record<string, unknown>> => {
    if (ts === null || session === null || session.root !== root) {
      ts = await loadTypescript(root);
    }
    const configFile = join(root, "tsconfig.json");
    if (!existsSync(configFile)) {
      throw new Error(
        `no tsconfig.json at ${norm(configFile)} - the oracle reads the ` +
          "project config exactly as tsc does",
      );
    }
    let configError: string | null = null;
    const parseHost = {
      fileExists: ts.sys.fileExists.bind(ts.sys),
      readFile: ts.sys.readFile.bind(ts.sys),
      readDirectory: ts.sys.readDirectory.bind(ts.sys),
      getCurrentDirectory: (): string => root,
      useCaseSensitiveFileNames: ts.sys.useCaseSensitiveFileNames,
      onUnRecoverableConfigFileDiagnostic: (d: { messageText: unknown }): void => {
        configError = (ts as TsModule).flattenDiagnosticMessageText(
          d.messageText,
          "\n",
        );
      },
    };
    const parsed = ts.getParsedCommandLineOfConfigFile(configFile, {}, parseHost);
    if (parsed === undefined || configError !== null) {
      throw new Error(configError ?? "tsconfig parse returned nothing");
    }
    session = new OracleSession(
      ts,
      root,
      norm(configFile),
      parsed,
      cellsDir,
      seamName,
    );
    return {
      ts_version: session.tsVersion(),
      config_file: norm(configFile),
      root_files: session.rootFileCount(),
    };
  };

  if (bootRoot !== null) {
    // Fail fast on an unusable root before the first request arrives:
    // exit 3 = typescript unresolvable (loadTypescript's contract),
    // exit 4 = config unusable — both observable to the spawning bridge.
    try {
      await initSession(bootRoot, null, "index");
    } catch (err) {
      stderr.write(
        `ts-oracle: boot init failed for ${norm(bootRoot)}: ` +
          `${err instanceof Error ? err.message : String(err)}\n`,
      );
      exit(4);
    }
  }

  const rl = createInterface({ input: stdin, crlfDelay: Infinity });
  for await (const rawLine of rl) {
    const line = rawLine.trim();
    if (line.length === 0) continue;
    const t0 = performance.now();

    let req: OracleRequest;
    try {
      req = JSON.parse(line) as OracleRequest;
    } catch {
      respondError(null, {
        kind: "protocol",
        detail: "unparseable request line",
      });
      continue;
    }
    const id = typeof req.id === "number" ? req.id : null;
    if (req.proto !== ORACLE_PROTOCOL) {
      respondError(id, {
        kind: "protocol",
        detail: `proto ${String(req.proto)} != ${ORACLE_PROTOCOL}`,
      });
      continue;
    }
    const op = req.op ?? "";
    const params = req.params ?? {};

    try {
      if (op === "shutdown") {
        respond(id, { ok: true, result: {} });
        break;
      }
      if (op === "init") {
        const root = resolve(String(params["root"] ?? bootRoot ?? "."));
        const cellsDir =
          typeof params["cells_dir"] === "string"
            ? (params["cells_dir"] as string)
            : null;
        const seamName =
          typeof params["seam"] === "string" ? (params["seam"] as string) : "index";
        const result = await initSession(root, cellsDir, seamName);
        respond(id, { ok: true, result });
      } else if (session === null) {
        respondError(id, {
          kind: "protocol",
          detail: `op \`${op}\` before init`,
          recipe: "send {op: init, params: {root}} first",
        });
      } else if (op === "update") {
        const version = session.update(
          String(params["file"]),
          params["content"] === null ? null : String(params["content"] ?? ""),
        );
        respond(id, { ok: true, result: { version } });
      } else if (op === "validate") {
        const result = session.validate(
          String(params["file"]),
          typeof params["content"] === "string"
            ? (params["content"] as string)
            : undefined,
        );
        respond(id, { ok: true, result });
      } else if (op === "scope") {
        const result = session.scope(
          String(params["file"]),
          isPosition(params["position"]) ? (params["position"] as unknown as Position) : undefined,
        );
        respond(id, { ok: true, result });
      } else if (op === "complete") {
        if (!isPosition(params["position"])) {
          respondError(id, {
            kind: "protocol",
            detail: "complete needs params.position {line, character}",
          });
        } else {
          const result = session.complete(
            String(params["file"]),
            params["position"] as unknown as Position,
            typeof params["content"] === "string"
              ? (params["content"] as string)
              : undefined,
            typeof params["prefix"] === "string"
              ? (params["prefix"] as string)
              : undefined,
            typeof params["max"] === "number" ? (params["max"] as number) : 50,
          );
          respond(id, { ok: true, result });
        }
      } else if (op === "type") {
        if (!isPosition(params["position"])) {
          respondError(id, {
            kind: "protocol",
            detail: "type needs params.position {line, character}",
          });
        } else {
          const result = session.quickInfo(
            String(params["file"]),
            params["position"] as unknown as Position,
            typeof params["content"] === "string"
              ? (params["content"] as string)
              : undefined,
          );
          respond(id, { ok: true, result });
        }
      } else {
        respondError(id, {
          kind: "protocol",
          detail: `unknown op \`${op}\``,
          recipe: `known ops: ${KNOWN_OPS.join(", ")}`,
        });
      }
    } catch (err) {
      // B5: an op failure is an answer, never a crash.
      respondError(id, {
        kind: "oracle-crashed",
        detail: err instanceof Error ? err.message : String(err),
      });
    }
    stderr.write(`ts-oracle: ${op} ${(performance.now() - t0).toFixed(1)}ms\n`);
  }
  exit(0);
}

function isPosition(v: unknown): boolean {
  return (
    typeof v === "object" &&
    v !== null &&
    typeof (v as { line?: unknown }).line === "number" &&
    typeof (v as { character?: unknown }).character === "number"
  );
}

await main();
