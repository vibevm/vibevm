# AGENTIC-TCG-TS-PLAN v0.1 — the agentic type oracle for TypeScript

_Status: **EXECUTED 2026-07-07 — Phases 0–7 complete, floor green at
close** (self-check 13 steps exit 0; conform 0 with 11 crates gated;
specmap 592/578/590, 0 orphans/0 warnings; demo floor 7/7;
fresh_ts_project + the live MCP chain green; corpus agreement 100% at
p50 19.3 ms). Every deliverable of the mandate shipped: the oracle,
the bridge + `tcg-typescript` slot binary, the portable `vibe-tcg`
crate mounted by vibe-mcp as four `tcg_*` tools, the spec set, and the
automated two-arm battery — whose §4.3 prediction was FALSIFIED in its
opt-in delivery form and recorded honestly (see §4.3 OUTCOME and the
with-tools REPORT; the Stage-B delivery backlog is the owner's next
call). Per-phase commit map: the `docs(plan)`→`build(deps)` chain of
2026-07-07 (`00fd17e`…), git log is the authoritative record.
Previously: ACCEPTED with owner amendments, 2026-07-07 (same day as
authoring). Owner review resolved the three §17 points: (1) the names
(`tcg_*` tools, bin `tcg-typescript`) are approved as proposed; (2)
PROP-026 stays vibevm-hosted BUT the product-side seam must be
**maximally abstract and detached** — the tcg tool family lands in a
dedicated product crate (`vibe-tcg`) with zero vibe-mcp dependency, so a
future decision to extract it into a SEPARATE standalone MCP server is a
new thin binary, not a surgery (D4/D6/D9 amended, §2 tree updated); (3)
the agent battery is **automated, not manual**: the opencode CLI agent
(on PATH; fallback `C:\opt\nvm\v24.18.0\node_modules\opencode-ai\bin\
opencode.exe`) with model "gpt-oss-20b (free)" is the weak-agent test
subject; the harness script is written and its CONTROL arm (tools
withheld — the oracle does not exist yet) executed at acceptance time,
so Phase 6 starts from a working harness and a recorded pre-oracle
baseline (D7 rewritten). Originally written as DRAFT against tree
`f083f6b` (the deferrals-closeout checkpoint; floor green, 69 local
commits ahead of origin, mirror held). Commissioned by the owner the
same day: build the
AGENTIC delivery of the vibe-tcg-ts line — the oracle, the vibe-mcp tool
family, discipline-aware answers, and the quantitative battery — and put
full specifications for every feature into `typescript-ai-native`.
Token-level (logit-mask) TCG is explicitly re-dispositioned to the VERY FAR
future (owner: "очень-очень далёкое будущее" — it waits on `vibe-llm` and a
local inference substrate) and is NOT part of this campaign. Cold-executable:
any phase is a safe stop; the floor must be green at every phase boundary._

Mandate (owner, 2026-07-07): implement the agentic-mode approach from the
session's analysis — (1) the long-lived TypeScript oracle (incremental
LanguageService, in-memory overlays), (2) the `tcg_*` tool family in
vibe-mcp, (3) discipline-aware responses, (4) the quantitative test
battery — as the practical near-term delivery of the tcg line, with a new
`vibe-agentic-tcg-ts.md` component brief SEPARATE from the token-level
`vibe-tcg-ts.md`, and feature specs added to the `typescript-ai-native`
package. Production-grade quality bar applies (the standing owner
directive in `spec/boot/90-user.md`: no MVP framing, no stub subcommands
as shipped surface).

## 0. Why this exists (one screen)

True type-constrained decoding masks logits inside the sampler — impossible
over a hosted agent (the API returns sampled text, never distributions).
But most of the mask's VALUE is not the mask: it is (a) the information
"what is in scope here, what type-checks", (b) feedback latency (an
in-memory validate in milliseconds instead of write-file → full floor →
parse errors → retry), and (c) the discipline enforced at generation time.
All three are deliverable TODAY to the agent as tools: an oracle the agent
consults instead of a mask the sampler obeys. The by-construction
*guarantee* stays with the floor (conform/tsc gates) — the oracle reduces
red iterations; the gates stay the truth.

The oracle is shared infrastructure with the far-future token-level line:
the same long-lived language-service process that answers the agent over
MCP today is the completability oracle a logit-masker will query when
`vibe-llm` exists. Nothing built here is thrown away then; only the
consumer of the answers changes.

Clean-room note: this campaign does not open `eth-sri/type-constrained-
code-generation` AT ALL. The agentic oracle shares no form with their
artifact (theirs: a decoder-integrated prefix automaton; ours: a
language-service tool surface over the public TypeScript Compiler API,
Apache-2.0). The binding rule in `spec/boot/90-user.md` is satisfied
trivially: their code is not read, not needed. It re-binds only when the
token-level campaign is ever commissioned.

## 1. Current-state facts, verified at authoring (do not re-discover)

- **ts-extract is one-shot and syntactic.** `packages/org.vibevm/
  typescript-ai-native/v0.3.0/tools/ts-extract/extract.ts` (542 lines,
  ZERO exports, ends `await main()`): `createSourceFile` + scanner only —
  no `Program`, no `LanguageService`. Protocol const `PROTOCOL = 1`
  (extract.ts:33). Consumer-`typescript` resolution: `loadTypescript`
  (extract.ts:143) — `createRequire(root/package.json).resolve` then
  dynamic import; failure = exit 3 + recipe. Facts: `ts_unsafe`
  (any_type/as_cross/non_null/ts_ignore/ts_expect_error + reason),
  `import`, `item`, `file_metrics`; markers: the six §9 JSDoc tags read
  from RAW tag text (extract.ts:251) + comment-stream `@scope`
  (extract.ts:470). B5: `degraded: true` records.
- **The extractor ships EMBEDDED in the Rust binary**, not as an npm
  artifact: `ts_extract_bridge::EXTRACTOR_SOURCE = include_str!(…)`
  (ts-extract-bridge/src/lib.rs:136) + content-addressed
  `materialise_extractor` → `<root>/target/conform/ts-extract/
  extract-<hash16>.ts` (lib.rs:141). The oracle inherits this delivery.
- **The bridge is one-shot too**: `extract_tree` (lib.rs:158) spawns
  `Command::new("node")` DIRECTLY (no `cmd /c`, node from PATH),
  `.output()` waits to exit. Reusable as-is for a second protocol:
  `FileRecord`/`RawFact`/`RawMarker` (serde), `parse_ndjson` (pure,
  line-oriented, replay-tested), `conform_facts` lowering (lib.rs:189),
  `BridgeError` 4-way taxonomy (NodeMissing / TypescriptUnresolvable ↔
  exit 3 / ExtractorFailed / Protocol).
- **Per-file rule evaluation already exists**: `conform_core::check(rules,
  &[SourceFacts], scope)` (conform-core/src/finding.rs:79) is pure over
  any fact set; BOTH TS rules are file-grain-correct (`TsUnsafeInDomain` —
  purely per-file; `TsCellIsolation::new(cells_dir, seam)` resolves
  imports lexically against the file's own path). The rule-set assembly
  `build_rules` is PRIVATE in `conform-cli-typescript/src/lib.rs:39`
  (~10 lines; constructors are pub) — needs a pub seam, not a rewrite.
- **conform.toml `[typescript]`** (`conform_core::config::TsConfig`,
  config.rs:99): `roots` (default `["src"]`), `exclude_substrings`,
  `cells_dir: Option<String>` (None disables cell isolation), `seam`
  (default `"index"`), `floor_disable` (step+reason). Root `max_file_lines`
  600.
- **vibe-mcp**: hand-rolled JSON-RPC 2.0 over line-delimited stdio
  (`crates/vibe-mcp/src/jsonrpc.rs`; PROTOCOL_VERSION "2024-11-05",
  lib.rs:60). Server loop `Server<T: Transport>::run` (lib.rs:216) lives
  for the whole agent session. Tool seam: `McpTool` trait (tools.rs:35 —
  `descriptor()` with inline `json!` schema + `run(&self, args,
  &ServerContext)`), registration = one vec in `default_tools()`
  (tools.rs:44), dispatch is name-generic (lib.rs:280) and never edited
  per tool. `ServerContext` holds ONLY `project_root` (lib.rs:78); the
  lockfile is re-read per call (lib.rs:95); NO long-lived state, NO child
  processes anywhere in the crate; `run` takes `&self` + `&ServerContext`
  (immutable) — a persistent child needs net-new interior mutability.
  vibe-mcp depends on `vibe-core` only, NOT `vibe-workspace`
  (Cargo.toml:13-22). Entry: `vibe mcp serve` →
  `resolve_project_root_required` (vibe-cli commands/mcp/mod.rs:337).
  Test model: `tests/tools_oracle.rs` (fixture lockfile + direct
  `Tool.run` + end-to-end `dispatch_one` over `MemoryTransport`).
- **PROP-025 dispatch exists**: `vibe bin list/build/path/exec`
  (vibe-cli commands/bin.rs) — `collect_binaries(project_root)`
  (bin.rs:47: `Workspace::discover` → lockfile → `vibedeps_slot` →
  `Manifest::read` → `manifest.binaries`), `DeclaredBinary::artifact()`
  (bin.rs:36: `slot/target/release/<name>[.exe]`), consent
  `consent_to_build` (bin.rs:93: `org.vibevm` allow-listed, else
  `--assume-yes`). All of it PRIVATE to the vibe-cli binary crate —
  vibe-mcp cannot import it today. Slot primitives are public in
  `vibe-workspace` (`Workspace::discover` lib.rs:341, `vibedeps_slot`
  lib.rs:428) and `vibe-core` (`Lockfile::read/find`,
  `Manifest::read`, `BinaryDecl` manifest/package/binary.rs:38).
- **PROP-018**: `tcg_*` queries are ALGORITHMIC (deterministic), i.e. the
  `query_package` path — plain `McpTool`s, no `check_affinity`, no
  `Intent`, no relay. The affinity machinery is not touched.
- **ts-demo is the testbed**: `research/ts-demo` — cells
  `src/cells/{greeting,farewell}` + `src/core/text.ts`; branded
  `GuestName` with `parseGuestName` as its only constructor
  (greeting/index.ts:48; the sanctioned `as` at :64 is the ONE frozen
  baseline finding `ts-unsafe-in-domain|src/cells/greeting/index.ts|
  as_cross#64`); tsconfig at the §1 floor; specmap.toml namespace
  `ts-demo` with two `[[external_specs]]` pointing into `vibedeps/*/spec`
  (version-bearing paths — must bump with the stack version);
  `vibe.toml` requires `stack:org.vibevm/typescript-ai-native = "^0.3"`.
- **Hermetic node pattern**: `discipline-cli-typescript/tests/
  fresh_ts_project.rs:33` junctions `tools/ts-extract/node_modules` into
  a temp project (`cmd /c mklink /J`, verbatim-prefix stripped) so
  `typescript` resolves offline with no per-run npm.
- **The TS package does NOT self-trace** (no specmap.toml at package
  root, no `scope!` outside vendor) — unlike rust-ai-native. New package
  specs therefore need no self-trace wiring; vibevm-side code cites the
  vibevm-hosted PROP-026 instead (indexed natively; 0-dangling floor
  unaffected).
- **Spec-format precedent**: mechanisms docs carry `{#anchors}` +
  `` `req rN` `` units (discipline-core `spec/mechanisms/*`: 57 markers
  across 4 files). Package tool briefs (`spec/typescript/tools/*.md`)
  are prose. GUIDE §14 (guide lines 125-133) still says "wrap and
  extend" — predates the clean-room directive; must be rewritten.
- **Naming space**: `vibe.toml` declares 3 `[[binary]]` entries
  (discipline-/conform-/specmap-typescript); the `<tool>-<language>`
  suffix convention is standing. ROADMAP's last milestone number is
  M1.23 (`vibe-tcg` Stage 1, PLANNED) — this campaign takes M1.24.
- Node on this box: v24.18.0 (strip-types stable); consumer typescript
  6.0.3 proven by the deferrals Phase-0 spike; `package-lock.json`
  committed / `node_modules` gitignored is the standing pattern.

## 2. Target end-state (what done looks like)

```
vibevm/
├─ spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md      NEW (vibevm-side spec)
├─ spec/terraforms/AGENTIC-TCG-TS-PLAN-v0.1.md            this file
├─ crates/vibe-workspace/src/bins.rs                      NEW cell: DeclaredBinary +
│                                                          collect_binaries (extracted
│                                                          from vibe-cli, shared)
├─ crates/vibe-tcg/                                       NEW product crate: the tool
│                                                          family (descriptors, schemas,
│                                                          run logic, OracleRegistry)
│                                                          behind `trait TcgHost`; NO
│                                                          vibe-mcp dependency (owner
│                                                          amendment: liftable into a
│                                                          standalone MCP server)
├─ crates/vibe-mcp/src/tcg.rs                             NEW cell: THIN adapter only —
│                                                          McpTool impls delegating to
│                                                          vibe-tcg
├─ research/tcg-bench/                                    NEW: the automated opencode
│                                                          battery (harness + tasks),
│                                                          bench corpus, baseline
│                                                          REPORTs, runbook
├─ research/ts-demo/                                      unchanged (requires ^0.4)
└─ packages/org.vibevm/typescript-ai-native/v0.4.0/       (bumped from v0.3.0)
    ├─ vibe.toml                                          [[binary]] × 4 (+ tcg-typescript)
    ├─ tools/ts-extract/                                  unchanged
    ├─ tools/ts-oracle/                                   NEW: oracle.ts (self-contained,
    │   ├─ oracle.ts                                       LanguageService + overlays,
    │   ├─ package.json / package-lock.json                NDJSON duplex, embedded via
    │   └─ test/oracle.test.ts                             include_str! like ts-extract)
    ├─ crates/tcg-oracle-bridge/                          NEW: persistent child client
    │                                                      (OracleClient seam, corr-ids,
    │                                                      timeouts, replay-tested)
    ├─ crates/tcg-cli-typescript/                         NEW: bin `tcg-typescript`
    │                                                      (serve / validate / scope /
    │                                                       complete / type / bench)
    └─ spec/typescript/
        ├─ GUIDE-AI-NATIVE-TYPESCRIPT.md                  §14 rewritten (clean-room,
        │                                                  agentic-first, both briefs)
        ├─ tools/vibe-tcg-ts.md                           token-level brief: clean-room
        │                                                  posture fixed, VERY-FAR-future
        │                                                  disposition, pointer to sibling
        ├─ tools/vibe-agentic-tcg-ts.md                   NEW: the agentic component
        │                                                  brief at FULL 7-section parity
        └─ mechanisms/
            ├─ TCG-ORACLE-v0.1.md                         NEW: process model, overlays,
            │                                              LS lifecycle, degraded rules
            └─ TCG-PROTOCOL-v0.1.md                       NEW: both NDJSON hops, message
                                                           schemas, error taxonomy
```

Runtime topology (three processes, two thin NDJSON hops):

```
agent (Claude Code etc.)
  │  MCP tools/call: tcg_validate / tcg_scope / tcg_complete / tcg_type
  ▼
vibe mcp serve                     (long-lived per agent session)
  │  OracleRegistry: lazy per-language handle, restart-on-crash,
  │  kill-on-drop; resolves the stack slot via lockfile (PROP-025
  │  dispatch), builds org.vibevm bins on demand, refuses third-party
  │  builds with the `vibe bin build` recipe
  ▼
tcg-typescript serve               (long-lived, slot-resident artifact)
  │  discipline enrichment: conform.toml policy → build_rules →
  │  conform_core::check over the per-file facts; advice strings citing
  │  spec:// REQs; cell/seam/brand context
  ▼
node <oracle.ts>                   (long-lived, materialised from the
       incremental LanguageService,  embedded source, consumer's own
       in-memory overlays)           typescript install)
```

## 3. Decisions (D1–D10)

### D1 — a NEW self-contained `tools/ts-oracle/oracle.ts`, embedded like the extractor

The oracle is a sibling tool, NOT an extension of `extract.ts`: the
extractor is a zero-export one-shot script over the *syntactic* API; the
oracle is a long-lived duplex server over the *LanguageService* API.
Mixing them couples two lifecycles and two `typescript`-surface slices in
one file. Delivery reuses the proven embedding: `tcg-oracle-bridge`
carries `ORACLE_SOURCE = include_str!("../../../tools/ts-oracle/oracle.ts")`
and materialises content-addressed to
`<root>/target/tcg/ts-oracle/oracle-<hash16>.ts`. Because exactly ONE
file is materialised, `oracle.ts` must be SELF-CONTAINED: the ~120 lines
of per-file fact/marker logic it shares with `extract.ts` (unsafe-set
classification, JSDoc raw-text markers, comment-stream scan) are
consciously duplicated with a header pointer both ways, and a package
test keeps the two behaviourally aligned on a shared fixture (same file
in → same facts out, modulo record framing).
*Rejected:* importing from `extract.ts` (zero exports today; a
cross-file import breaks single-file materialisation); a shared
`tools/ts-shared/` module (same materialisation break); extending
`extract.ts` in place (lifecycle + API mixing).

### D2 — oracle protocol: NDJSON duplex, correlation ids, its own version

`ORACLE_PROTOCOL = 1`, independent of the extractor's `PROTOCOL = 1`
(different channel, different message set; versions move independently).
Requests `{proto, id, op, params}` / responses `{proto, id, ok,
result | error}` — one JSON object per line, both directions. Ops (all
specified in TCG-PROTOCOL-v0.1 with full schemas):

- `init {root}` → `{ts_version, config_file, root_files}` — builds the
  `LanguageServiceHost` over the project's `tsconfig.json`
  (`readConfigFile` + `parseJsonConfigFileContent`), consumer
  `typescript` resolved exactly like the extractor (same recipe error on
  failure).
- `validate {file, content?}` → `{diagnostics[], facts[], markers[],
  degraded}` — `content` present = in-memory overlay (the file need not
  exist on disk); absent = disk state. Diagnostics from
  `getSyntacticDiagnostics` + `getSemanticDiagnostics` (code, message,
  line, char, category); facts/markers from the D1-duplicated per-file
  extraction so the Rust side can run conform rules on the SAME payload.
- `scope {file, position?}` → `{symbols[], cell, seam_file, branded[]}` —
  in-scope value/type symbols with type strings (LS completions at a
  neutral position + checker type text), the file's cell (from
  `cells_dir`), its seam file, and branded types exported at
  reachable seams (v0.1 heuristic: exported type aliases whose
  declaration text matches the intersection-brand shape
  `& { readonly __brand:` — honestly labelled `heuristic: true`).
- `complete {file, position, content?}` → `{entries[]}` — LS
  `getCompletionsAtPosition`, each entry `{name, kind, type_text,
  unsafe: bool}` (`unsafe` marks continuations that would introduce a
  §8-banned form, e.g. an `any`-typed symbol).
- `type {file, position, content?}` → `{display, documentation}` — LS
  `getQuickInfoAtPosition`.
- `update {file, content|null}` → `{version}` — set/clear an overlay
  without validating (null clears).
- `shutdown {}` → process exit 0.

Overlay semantics: the host keeps `Map<path, {content, version}>`;
`getScriptVersion` increments per update; non-overlaid paths fall through
to disk. Every op that cannot produce an answer degrades per B5 — a
well-formed `{ok: false, error: {kind, detail, recipe?}}`, never a crash;
unknown op → error with the known-op list (forward compatibility).

### D3 — Rust delivery: `tcg-oracle-bridge` + bin `tcg-typescript` in the stack

Two new crates in the package workspace (mirroring the
extract-bridge/CLI split):

- **`tcg-oracle-bridge`** (lib): `ORACLE_SOURCE` embedding +
  materialisation; serde types for every D2 message; a `trait
  OracleClient` seam (the hooks.rs `HookRunner`/`InterpreterProbe`
  lesson) with `SystemOracleClient` — spawn `node <oracle.ts>` with
  piped stdio (`Command::new("node")`, no shell, PATH-resolved node like
  the extract bridge), retained `Child`, request/response by correlation
  id, per-request timeout, kill-on-drop (Windows-verified), restart
  policy = the CALLER re-inits on `OracleGone`. Error taxonomy extends
  the bridge convention: `NodeMissing / TypescriptUnresolvable /
  OracleCrashed / Protocol / Timeout`, each with a REQ citation + fix
  surface. Replay tests (recorded response streams) keep the crate
  node-free in unit tests.
- **`tcg-cli-typescript`** (bin **`tcg-typescript`**): subcommands
  - `serve --root <dir>` — the long-lived stdio loop vibe-mcp drives:
    reads D2-shaped requests, forwards to the node child, ENRICHES
    responses (D5), writes them back. One protocol shape on both hops —
    the middle layer adds fields, never reshapes.
  - `validate <file> [--content-from -|<path>] [--json]`,
    `scope <file> [--position L:C]`, `complete <file> --position L:C`,
    `type <file> --position L:C` — one-shot forms (spawn, init, one op,
    shutdown): the agent-without-MCP path (`vibe bin exec
    tcg-typescript -- validate …`) and the debug surface. PROP-018 §2.8
    dual-transport, applied.
  - `bench --corpus <dir> --report <file>` — Phase 6's harness (runs the
    differential corpus + latency measurements, emits JSON + human
    summary).
  The manifest gains the 4th `[[binary]]` (`name = "tcg-typescript"`,
  `crate = "crates/tcg-cli-typescript"`).
*Rejected:* vibe-mcp spawning node directly (the product would need
conform-core — a product→package compile-time dependency, layering
violation; and TS-resolution/enrichment logic would leak into vibevm);
folding the oracle into `discipline-typescript` (the umbrella is the
GATE panel; the oracle is a generation-time SERVICE — different
lifecycle, different failure posture, separate bin keeps both honest).

### D4 — vibe-mcp surface: four `tcg_*` tools with a `language` parameter

Tools `tcg_validate`, `tcg_scope`, `tcg_complete`, `tcg_type` — thin
schema adapters over the D2 ops, each with `language` (v0.1: only
`"typescript"`; anything else → ToolError naming what IS supported).
Names are tcg-branded (the product line), language-parameterised so the
future Rust twin adds a language value, not four more tools.
`structuredContent` carries the enriched response verbatim; the text
content renders a compact human summary (findings first).

**Owner amendment (portability): the family lives in a dedicated
product crate `crates/vibe-tcg`, not inside vibe-mcp.** The crate
defines: the tool descriptors/JSON schemas, the run logic, the
`OracleRegistry`, and a NARROW host abstraction (`trait TcgHost`:
`project_root()` + the no-prompt consent policy) — and depends on
`vibe-workspace`/`vibe-core` only, NEVER on vibe-mcp. vibe-mcp gains a
thin adapter cell (`src/tcg.rs`): newtype wrappers implementing
`McpTool` by delegating to `vibe-tcg` and mapping its typed errors into
`ToolError`. Extracting the family into a standalone MCP server later =
one new binary crate reusing `vibe-tcg` + a JSON-RPC loop (vibe-mcp's
`Server<T: Transport>` is already generic); zero changes inside the
family. PROP-026 specifies the tools server-agnostically (operations
over a host context) with the MCP binding as one explicit adapter
section.

Lifecycle: the `vibe-tcg` `OracleRegistry` —
`Mutex<HashMap<Language, OracleHandle>>` (held by the adapter layer in
`ServerContext`), lazily populated on first use:
resolve the CURRENT project's lockfile → the typescript-ai-native slot →
the `tcg-typescript` artifact (`collect_binaries` shape); if the artifact
is missing and the group is `org.vibevm`, build it (the PROP-025 §3
consent rule: allow-listed); for any other group return a ToolError
carrying the exact `vibe bin build <name> --assume-yes` recipe (an MCP
server must not prompt). Spawn `tcg-typescript serve --root <project>`,
hold the handle across calls, kill on server drop; a dead child →
`OracleGone` → one transparent respawn attempt, then a recipe-carrying
error. Stack not installed → ToolError naming the `[requires]` line and
`vibe install`.
*Rejected:* `ts_*` names (tool-count explosion at the Rust twin);
`oracle_*` (collides with the Discipline's Class-D "differential oracle"
term in the same corpus); per-call spawn (cold init per query defeats
the latency point).

### D5 — discipline-aware enrichment lives in the Rust middle layer

`tcg-typescript serve` reads the project's `conform.toml` once per init
(re-read on `init`): `TsConfig` gives `cells_dir`/`seam`/`exclude`;
root config gives `max_file_lines`. A new PUB seam in
`conform-cli-typescript` — `pub fn build_rules(&Config) ->
Vec<Box<dyn Rule>>` (today private at lib.rs:39; exporting it is a
package-internal refactor, no vendor-sync impact) — assembles the same
rule set the gate runs. On `validate`: `FileRecord`-shaped
facts/markers → `ts_extract_bridge::conform_facts` → `SourceFacts` →
`conform_core::check(rules, &[sf], None)` → findings merged into the
response as `conform_findings[]`, each flagged `baselined: bool` against
`conform-typescript-baseline.json` when present (the agent sees
sanctioned findings distinctly). On `scope`: cell/seam context from
`TsConfig` + the D2 branded list; `advice[]` strings in Class-F form
citing the GUIDE REQs (e.g. "a bare `string` crossing this seam should
be a branded type — spec://typescript-ai-native/guide §4"). On
`complete`: entries flagged `unsafe` get a one-line reason. The node
side stays policy-free (facts only); ALL policy interpretation is Rust —
one place, one truth, same engine as the gate.
*Rejected:* evaluating conform rules node-side (duplicates the rule
engine in a second language — divergence by construction); a second
rule-assembly in the oracle (drift; the pub seam keeps one).

### D6 — specs: two mechanisms + one full brief in the package; PROP-026 in vibevm

Package (`typescript-ai-native` — the owner's instruction "specs of all
features into the package"):
- `spec/typescript/tools/vibe-agentic-tcg-ts.md` — the owner-named
  component brief at FULL seven-section parity (problem · design stance ·
  component shape · staged ambition · licensing · risk register ·
  summary). States the mask-value decomposition (§0 here), the
  three-process topology, the shared-infrastructure claim toward the
  token-level future, and the honest limits (no by-construction
  guarantee; strong agents benefit less — DR1-015 inverted: tools you
  can ignore do not distort).
- `spec/typescript/mechanisms/TCG-ORACLE-v0.1.md` — the process model
  (`req rN` + `{#anchors}`): host/overlay semantics, LS lifecycle,
  consumer-typescript resolution, degraded rules, latency posture
  (measured, not gated), Windows child discipline.
- `spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md` — both hops' message
  schemas (versioned; `ORACLE_PROTOCOL`), the enrichment fields the
  middle layer adds, the error taxonomy, forward-compat rules.
- Rewrites: `tools/vibe-tcg-ts.md` — strike the "wrap PLDI'25 / check
  license before vendoring" Stage-1 (superseded by the clean-room
  directive), record the VERY-FAR-future disposition in the owner's
  terms (waits on `vibe-llm` + local inference; re-staged AFTER the
  agentic line), point to the sibling brief; the parity note shrinks to
  the token-level remainder. `GUIDE-AI-NATIVE-TYPESCRIPT.md` §14 —
  rewritten: clean-room posture, agentic-first order, both briefs
  linked, the "wrap and extend" sentence removed; §15 wiring gains move
  5 (the MCP tools + `vibe bin exec tcg-typescript` forms).
  `spec/rust/tools/vibe-tcg.md` (rust-ai-native) — one added paragraph:
  the agentic delivery shipped first on TypeScript; the Rust twin's
  Stage 2 (scope/name constraining) has an agentic analogue over
  rust-analyzer when commissioned.

vibevm: `spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md` — the
product-side seam (this is deliberately vibevm-hosted, a deviation from
the letter of "everything into the package" for a layering reason: the
MCP tool family, the OracleRegistry lifecycle, and the slot-dispatch
consent posture are PRODUCT surface, versioned with vibevm, citable by
product code without new `[[external_specs]]` wiring; approved by the
owner WITH the portability amendment). Sections: problem, the
server-agnostic tool operations over `TcgHost` (the D4 amendment: the
family is a crate any server binary can mount; vibe-mcp is ITS FIRST
HOST, not its home), tool schemas, registry lifecycle, PROP-025
dispatch + consent (no-prompt rule), language dispatch, failure
surfaces, the standalone-server extraction path (named explicitly so
the future move is a documented follow-up, not a redesign), non-goals
(no LSP relay, no reasoning ops, no affinity involvement), acceptance.
ROADMAP gains M1.24.

### D7 — the quantitative battery: hermetic correctness in the package, measurement in `research/tcg-bench`

Two distinct natures, two homes:
- **Correctness gates (hermetic, run with package tests):**
  (a) the *differential validate-vs-tsc corpus* — fixture fragments
  seeded with known error classes (wrong argument type, missing import,
  unbranded primitive across a seam, unsafe-set uses, clean controls);
  the oracle's `validate` diagnostics must agree with
  `tsc --noEmit`-class diagnostics on (file, line, code) with a
  ±position tolerance — the Class-D differential oracle applied to the
  oracle itself; (b) completions goldens at marked positions (expected
  entries present, known-bad absent); (c) protocol replay goldens (both
  hops, node-free); (d) the D1 extract/oracle fact-parity fixture. All
  node-dependent tests hard-fail with a recipe when node is absent
  (never skip), reusing the junction pattern for offline `typescript`.
- **Measurement (recorded, not gated) — AUTOMATED per the owner
  amendment:** `research/tcg-bench/` — `corpus/` (the seeded fragments,
  shared with (a) via the package fixtures where practical), `tasks/`
  (the agent task battery: 10–15 written tasks over ts-demo of the
  shape "add a farewell variant that takes GuestName", "call greeting
  from a new cell through the seam"), and the **opencode harness**:
  `run-battery.sh` drives the opencode CLI (`opencode run`, model
  **gpt-oss-20b (free)**; binary from PATH, fallback
  `C:\opt\nvm\v24.18.0\node_modules\opencode-ai\bin\opencode.exe`) —
  per task: a fresh throwaway work copy of ts-demo, one headless agent
  run with the task prompt, then MECHANICAL verification and metric
  capture: agent exit code, wall time, `tsc --noEmit` error count +
  hallucination-class codes (TS2304/TS2552/TS2339), demo tests
  (`node --test`), conform findings delta, unsafe-set introductions.
  Two arms: **control** (tools withheld — executed at plan acceptance,
  before the oracle exists: the pre-oracle baseline) and
  **with-tools** (after Phase 5; the same tasks with the `tcg_*`
  MCP tools / one-shot CLI available and the prompt naming them).
  `RUNBOOK.md` documents invocation + metric definitions;
  `REPORT-<date>-*.md` records each run (plus, after Phase 6,
  `tcg-typescript bench` output — per-op p50/p95 warm latency,
  cold-init time, differential agreement %). Latency targets are POSTED
  as expectations (validate p50 warm < 150 ms on ts-demo-class trees;
  cold init < 5 s; complete p50 < 200 ms) and verified by the report,
  not by CI assertions — timing gates on shared boxes are flake
  generators; the REPORT is the ratchet. The weak-model choice is the
  point: DR1-015 says constraints/help lift weak models most — the
  battery measures exactly that population, mechanically.

### D8 — package version: 0.3.0 → 0.4.0, one bump at campaign open

Content additions (2 crates, 1 tool, 3 spec docs, manifest) bump the
minor once, in Phase 1, so every later diff lands in final paths
(the deferrals precedent). Moves: `git mv packages/org.vibevm/
typescript-ai-native/{v0.3.0,v0.4.0}`; manifest `version = "0.4.0"`;
`research/ts-demo/vibe.toml` requires `^0.4`; ts-demo `specmap.toml`
`[[external_specs]]` slot paths bump `0.3.0` → `0.4.0` (version-bearing —
verified fact §1); `sync-engines.toml` target path updated (vendor dirs
ride the rename); re-materialise vibedeps + boot INDEX regen; vibevm's
own specmap/conform indexes must stay byte-stable modulo the renamed
paths (the rename is transparent to vibevm's scan roots — `packages/` is
not a scan root; `vibe check` + lockfile refresh prove it). Registry
publish stays owner-held, as with 0.3.0.

### D9 — extraction of the binary-resolution cell into `vibe-workspace`

`DeclaredBinary` (+ `artifact()`) and `collect_binaries` move from
`vibe-cli/src/commands/bin.rs` into a new `vibe-workspace/src/bins.rs`
pub cell (workspace-grain logic: lockfile → slot → manifest →
declarations → artifact path); `vibe-cli` re-imports (behaviour
identical — the existing bin.rs tests hold), `vibe-mcp` gains the
`vibe-workspace` dependency and consumes the same cell. Consent stays
CLI-side (`consent_to_build` needs a prompt; the MCP path never prompts
per D4).
*Rejected:* copying ~30 lines into vibe-mcp (two drifting copies of
dispatch-invariant logic — the exact drift class vendor-sync exists to
prevent, with no gate here to catch it).

### D10 — clean-room execution discipline (recorded for the audit trail)

The PLDI'25 repository is not opened, cloned, or excerpted at any phase
of this campaign; no design element here derives from its code. Concept
sources: the published paper's ideas as ALREADY summarised in our own
briefs (the asymmetry, the 74.8% figure, the completability framing —
all present in `vibe-tcg-ts.md`/`vibe-tcg.md` since before this plan),
the public TypeScript Compiler API and its documentation, and this
repository's own engines. Any future need to consult that repository
(none is foreseen for the agentic line) goes through the owner first.

## 4. Predictions (falsifiable, checked by the REPORT)

1. Warm `validate` on a ts-demo cell lands under 150 ms p50 (the LS
   program is incremental after init) — if falsified, the mitigation
   ladder is §8 R1.
2. The differential corpus agrees ≥ 95% on (file, code) between oracle
   diagnostics and tsc-class diagnostics; disagreements are position-
   grain, not existence-grain. Existence-grain disagreement = a bug, not
   a tolerance.
3. The agent battery with tools available shows a measurably lower
   floor-red rate and retry count than withheld (direction, not
   magnitude, is the claim at n≈12 tasks; magnitude needs more runs
   than v0.1 commissions).
   **OUTCOME (2026-07-07): DID NOT HOLD in the tested delivery form.**
   With opt-in one-shot CLI tools named in the prompt, GLM-5-Turbo
   scored 10/2 on BOTH arms with the identical two discipline
   regressions — the weak model never spontaneously consults a tool it
   is not forced to. The oracle's mechanics are proven separately
   (corpus 7/7, live MCP chain); the gap is DELIVERY. Stage-B backlog
   recorded in `research/tcg-bench/reports/REPORT-2026-07-07-with-tools.md`:
   forced-loop (write-path hook), an MCP-mounted battery arm, and an
   uptake metric. This is the §0 honesty note materialised: a tool you
   may ignore does not distort — and may also not help until the loop
   requires it.
4. No phase requires touching conform-core / specmap-core / vendored
   engines (the campaign is additive to the neutral core) — if
   falsified, the vendor-sync gate + a discipline-core version bump
   enter the affected phase explicitly.
5. `extract.ts` is not modified at all (fact-parity is held by
   duplication + test, not by refactoring the proven extractor).

## 5. Phase 0 — spikes (no commits; gates for everything after)

1. **LanguageService-under-strip-types spike**: a scratch `oracle-spike.ts`
   run by node 24 against `research/ts-demo` — build a
   `LanguageServiceHost` over the demo's tsconfig via the consumer's
   `typescript` (6.0.3), then: (a) semantic diagnostics for
   `greeting/index.ts` AS an overlay with a seeded type error (never
   touching disk); (b) `getCompletionsAtPosition` inside `greet()`;
   (c) `getQuickInfoAtPosition` on `parseGuestName`; (d) measure cold
   init and warm per-op times (three runs, note p50). Proves the D2 API
   surface exists and works erasable-only, and records the first latency
   facts. Abort criterion: if the LS API is unusable under strip-types
   (unexpected — it is plain JS at runtime), fall back to running the
   oracle through the consumer's `tsc.js` module path directly; the
   protocol does not change.
2. **Persistent-child spike (Rust)**: a scratch test in the package tree
   spawning `node -e '<echo loop>'` with piped stdio — write NDJSON,
   read NDJSON, drop → child gone (verify no zombie via tasklist),
   timeout path. Proves the D3 client mechanics on Windows before the
   bridge crate exists.
3. **Junction reuse check**: confirm the fresh_ts_project junction
   pattern serves a second tool dir (`tools/ts-oracle/node_modules` —
   or decide to point oracle tests at the ts-extract node_modules,
   one junction, since the devDep set is identical).
4. Findings land in the WAL session section; red spikes rewrite the
   affected decision IN THIS FILE before Phase 1 commits anything.

## 6. Phase 1 — the version bump + all specs

1. D8 moves first (`git mv`, manifest, ts-demo requires + external_specs
   paths, sync-engines.toml, re-materialise, boot INDEX regen).
2. Author the three package spec docs + the two rewrites (D6); author
   PROP-026; ROADMAP M1.24 entry.
3. Acceptance: `bash tools/self-check.sh` exit 0 (13 steps, incl.
   sync-engines over the renamed stack dir); `cargo xtask specmap
   --check` 0 dangling (PROP-026 anchors ingested); `vibe check` clean;
   ts-demo `vibe install --assume-yes` re-resolves against 0.4.0
   (PROP-011 §2.6 mutability) and its floor stays 7/7 green.
4. Commits (Rule 3 grouping):
   - `build(packages): bump typescript-ai-native to 0.4.0`
   - `docs(typescript-ai-native): the agentic tcg brief + mechanisms specs`
   - `docs(spec): PROP-026 - the tcg tool family in vibe-mcp`
   - `docs: roadmap M1.24 - the agentic tcg line`

## 7. Phase 2 — `tools/ts-oracle` (the node side)

1. `oracle.ts` per D1/D2: self-contained; the LanguageServiceHost +
   overlay map; all seven ops; B5 degraded posture; stderr = human log
   line per op (op, ms) so `serve` sessions are debuggable; stdout =
   protocol only (the extractor's stream discipline).
2. `package.json` (`@org.vibevm/ts-oracle`, `type: module`, devDep
   `typescript ^6.0.0`, test script `node --test "test/*.test.ts"`) +
   committed `package-lock.json`; node_modules gitignored (standing
   pattern) — Phase-0 item 3 decides whether tests junction ts-extract's
   install or carry their own.
3. `test/oracle.test.ts` (node:test, explicit globs): init on a fixture
   tree; validate clean vs seeded-error overlay (diagnostics appear
   WITHOUT disk writes); update/clear overlay roundtrip; completions
   golden at a marked position; type/quick-info golden; degraded record
   on rubble; unknown-op error shape; fact-parity fixture shared with
   ts-extract (D1).
4. Acceptance: `node --test` green in the tool dir; vibevm floor
   untouched (the tool is not yet wired to anything).
5. Commit: `feat(typescript-ai-native): ship the ts oracle - the
   language-service NDJSON server`.

## 8. Phase 3 — `tcg-oracle-bridge` + `tcg-typescript` (the Rust side)

1. `tcg-oracle-bridge` per D3: embedding + materialisation (content-
   addressed, `target/tcg/ts-oracle/`); serde message types; the
   `OracleClient` seam + `SystemOracleClient` (spawn/correlate/timeout/
   kill-on-drop); the five-way error taxonomy with REQ citations
   (citing `spec://typescript-ai-native/mechanisms/TCG-PROTOCOL` units);
   replay tests from recorded streams (node-free).
2. The `build_rules` pub seam in `conform-cli-typescript` (D5) — export
   + doctest; behaviour identical (the gate tests hold).
3. `tcg-cli-typescript` per D3: `serve` (enrichment per D5: conform
   findings + baselined flags + advice + cell/seam context), the four
   one-shot forms, `bench` (skeleton that Phase 6 fills: corpus walk +
   timing capture — shipped runnable, not stubbed: it runs the corpus
   it is given and reports; Phase 6 adds the corpus).
4. Manifest: the 4th `[[binary]]`; boot snippet
   `20-stack-typescript-ai-native.md` toolchain block lists it; package
   README row.
5. Tests: bridge replay suite; enrichment unit tests over fixture
   facts (findings merge, baselined flag honours the demo's frozen
   as_cross); hermetic end-to-end (junction + real node): init on a
   fixture project, seeded-error overlay validate → the diagnostic AND
   the conform finding both present, exit codes correct; one-shot
   `validate --json` snapshot.
6. Acceptance: package tests green (`cargo test` in the package
   workspace); `cargo run -q -p vibe-cli -- bin list` shows FOUR
   typescript-ai-native binaries; `vibe bin exec tcg-typescript --
   validate research/ts-demo/src/cells/greeting/index.ts --json` exits 0
   with zero diagnostics and exactly the one baselined finding; floor
   green; re-materialise vibedeps.
7. Commits:
   - `feat(typescript-ai-native): tcg-oracle-bridge - the persistent
     oracle client`
   - `refactor(conform): export the typescript rule-set assembly seam`
   - `feat(typescript-ai-native): tcg-typescript - serve, one-shot ops,
     bench frame`
   - `docs(packages): declare the tcg binary + boot toolchain row`
   - `build(deps): re-materialise vibedeps at 0.4.0+tcg`

## 9. Phase 4 — the vibe-mcp `tcg_*` family (PROP-026)

1. D9 extraction: `vibe-workspace/src/bins.rs` (+ doctest on the seam);
   `vibe-cli/commands/bin.rs` re-imports; existing bin tests green
   unchanged.
2. NEW crate `crates/vibe-tcg` (the owner-amended home): `trait
   TcgHost`, the four tool operations with their JSON schemas, the
   `OracleRegistry` (interior-mutable, lazy, kill-on-drop), the
   language dispatch, the failure surfaces (not-installed / not-built-
   third-party / oracle-gone recipes) — deps: vibe-core +
   vibe-workspace + the serde stack; ZERO vibe-mcp imports.
   `#[spec]`/`scope!` tags citing PROP-026 (vibevm self-trace: the new
   units join the 0-dangling index).
3. vibe-mcp: `vibe-tcg` dependency; `ServerContext` holds the registry
   handle; new `src/tcg.rs` ADAPTER cell — four newtypes implementing
   `McpTool` by delegation, typed-error → `ToolError` mapping;
   `default_tools()` grows four entries.
4. Tests: vibe-tcg unit suite with an `OracleClient` test double behind
   the registry seam (no node, no vibe-mcp); vibe-mcp
   `tests/tcg_tools.rs` on the `tools_oracle.rs` model —
   schema/dispatch/error-surface coverage through the adapter, a
   lockfile fixture WITHOUT the TS stack → the not-installed recipe,
   `tools/list` includes the four; one ignored-by-default integration
   test that runs the real chain end-to-end on this box (node + built
   artifact) for manual/pre-release runs.
5. Acceptance: vibe-tcg + vibe-mcp + vibe-workspace + vibe-cli tests
   green; `vibe mcp serve` manual probe on vibevm root — `tools/list`
   carries `tcg_*`, `tcg_validate` on a ts-demo file returns the
   enriched payload (recorded in the WAL; NOTE: the live agent session
   sees the new tools only after its MCP server restarts —
   owner-visible step); specmap 0 dangling; floor green.
6. Commits:
   - `refactor(workspace): extract declared-binary resolution into a
     shared cell`
   - `feat(tcg): the portable tcg tool family crate (PROP-026)`
   - `feat(mcp): mount the tcg family as MCP tools`

## 10. Phase 5 — enrichment polish + consumer front door

1. Scope/brand advice strings finalised (Class-F form, REQ-citing);
   completions `unsafe` flags verified against the §8 set; the
   `heuristic: true` label on brand detection honest in output.
2. GUIDE §15 wiring move 5 (MCP tools + one-shot forms); both SKILL.md
   twins gain a short "generation-time assistant" section (consult
   `tcg_validate` before writing a cell edit; the floor stays the
   truth); `vibe skill install` re-projection dogfooded.
3. Acceptance: enrichment tests green; `vibe check` clean; floor green;
   skills re-projected.
4. Commits: `feat(typescript-ai-native): discipline-aware oracle
   enrichment`, `docs(typescript-ai-native): guide + skills wiring for
   the agentic tcg`.

## 11. Phase 6 — the battery (D7)

1. The differential corpus: `corpus/` fixtures with seeded error classes
   + clean controls; the package test asserting (file, code)-grain
   agreement ≥ the D7 rule (existence-grain disagreement fails); the
   completions goldens.
2. `tcg-typescript bench --corpus … --report …` fills in: runs the
   corpus warm + cold, captures per-op latency distribution +
   agreement %, writes JSON + a human table.
3. `research/tcg-bench/` (the harness and the control-arm baseline
   EXIST since plan acceptance — the D7 amendment): add the
   **with-tools arm** — the same tasks re-run with the `tcg_*` surface
   available (the one-shot `vibe bin exec tcg-typescript -- …` forms
   named in the prompt, and/or the MCP tools when the runner agent
   mounts them), plus the oracle-latency section of
   `tcg-typescript bench`; write `REPORT-2026-07-XX-with-tools.md`
   comparing arms metric-by-metric against the control baseline + the
   predictions-vs-facts check (§4).
4. Acceptance: corpus tests green in the package suite; both arms'
   REPORTs committed with real numbers; predictions §4.1/§4.2/§4.3
   checked (falsified predictions rewrite the affected decision here +
   a WAL note, per the campaign form's honesty rule); floor green.
5. Commits: `test(typescript-ai-native): the differential validate
   corpus + completions goldens`, `feat(research): tcg-bench - the
   with-tools arm + comparison report`.

## 12. Phase 7 — campaign close

1. Final re-materialise (if any package content moved after Phase 5);
   regen specmap; full panel: `self-check.sh` 13 steps, specmap
   `--check` 0 dangling, conform 0, ts-demo floor 7/7,
   `fresh_ts_project` green, the Phase-4 manual MCP probe re-run.
2. WAL standing-line + session-section update; CONTINUE.md checkpoint;
   this plan's status line flips to EXECUTED with the per-phase commit
   map.
3. Commits: `build(deps): re-materialise vibedeps - campaign close` (if
   needed), `docs(wal)/docs(continue)` checkpoint pair.
4. Mirror and registry publish stay owner-held (standing policy; the
   0.4.0 publish joins the 0.4.0/0.4.0/0.3.0 owner-court item).

## 13. Risks & fallbacks

- **R1 — LS latency under overlays** on larger trees. Detection: Phase-0
  timings; the bench report. Ladder: (a) one LS per root, never per
  request; (b) `validate` returns semantic diagnostics for the ONE
  overlaid file only (never whole-program re-check); (c) if cold init
  dominates, pre-warm on `init` in the background of the first call;
  (d) if still red on ts-demo-class trees, the p50 target moves WITH a
  recorded reason in the REPORT — never silently.
- **R2 — zombie node children on Windows.** Kill-on-drop + explicit
  `shutdown` op + the Phase-0 spike proving both; the registry kills on
  server drop; the hermetic test asserts child exit.
- **R3 — oracle-vs-tsc diagnostic divergence.** Same engine underneath,
  but LS and CLI assemble options differently. The differential corpus
  is the detector; the fallback is reading compiler options EXACTLY as
  tsc does (`getParsedCommandLineOfConfigFile`) — spec'd in TCG-ORACLE
  as the required config path from the start.
- **R4 — strip-types constraints in oracle.ts** (erasable-only syntax).
  Same constraint the extractor already lives under; the Phase-0 spike
  is the detector; fallback: none needed (plain-JS-at-runtime API).
- **R5 — protocol drift across three hops.** One message shape both
  hops (the middle layer adds fields, never reshapes); versioned
  `ORACLE_PROTOCOL`; replay goldens on both sides; the fact-parity test
  pins the extractor-shared payload.
- **R6 — MCP server restart blindness** (a live agent session keeps the
  pre-campaign `vibe` binary running). Not a code risk — an operator
  fact: the WAL + the Phase-4 acceptance note it; the owner restarts
  the MCP server (or the session) to see the tools.
- **R7 — scope creep toward an LSP relay.** The surface is the FOUR ops
  + update/init/shutdown, full stop; anything further (rename, code
  actions, find-references) is named a non-goal and goes through the
  owner. The PROP-026 non-goals section carries the line.
- **R8 — the `build_rules` export tempting broader conform surface.**
  The seam exports assembly ONLY; rule semantics stay in conform-core;
  the vendored copies are untouched (prediction §4.4 gates this).

## 14. Non-goals (named, so they stay visible)

- Token-level / logit-mask TCG — the VERY-FAR-future line (owner
  disposition 2026-07-07); waits on `vibe-llm` (M0 stub) + a local
  inference substrate; re-planned separately when commissioned. The
  clean-room rule re-binds THERE.
- The Rust agentic twin (rust-analyzer-backed `tcg_rust`) — after this
  campaign proves the shape; the language parameter and PROP-026 are
  cut to admit it.
- An LSP relay / full editor-protocol surface (R7).
- Reasoning/relay (PROP-018 `Intent`) involvement — these tools are
  algorithmic, full stop.
- `vibe bin sync` shims, registry publish, mirror — standing separate
  items, untouched here.
- Extending `extract.ts` or the batch gate pipeline — the gate path is
  frozen and proven; the oracle is additive beside it.

## 15. Quick-start for the executing session

```sh
bash tools/self-check.sh; echo "EXIT=$?"          # must be 0 before anything
cargo xtask specmap --check                        # 584/571/583/0/0, 0 dangling
cargo xtask conform check                          # 0 findings
node --version                                     # >= 22.6 (this box: v24.18.0)
cd research/ts-demo && npm install && cd ../..     # warm the demo toolchain
# then Phase 0, in order; record spike findings in the WAL session section
```

## 16. Whole-campaign acceptance (what "done" looks like)

```sh
bash tools/self-check.sh; echo "EXIT=$?"                        # 0
cargo run -q -p vibe-cli -- bin list                             # 7 binaries; tcg-typescript listed
cargo run -q -p vibe-cli -- bin exec tcg-typescript -- \
    validate research/ts-demo/src/cells/greeting/index.ts --json # 0 diagnostics; 1 finding, baselined:true
# seeded-error overlay through the one-shot form → non-zero exit, the diagnostic named
cargo run -q -p vibe-cli -- mcp serve   # then over stdio: tools/list carries
                                        # tcg_validate/tcg_scope/tcg_complete/tcg_type;
                                        # tcg_validate on the demo returns diagnostics
                                        # + conform_findings + advice in structuredContent
cargo run -q -p vibe-cli -- bin exec tcg-typescript -- \
    bench --corpus research/tcg-bench/corpus --report /tmp/r.json # agreement >= 95%, report written
# research/tcg-bench/REPORT-*-baseline.md committed with real numbers;
# spec/typescript/tools/vibe-agentic-tcg-ts.md at full parity;
# TCG-ORACLE-v0.1 / TCG-PROTOCOL-v0.1 / PROP-026 anchors resolve (specmap 0 dangling);
# GUIDE §14 clean-room-rewritten; ts-demo floor 7/7; fresh_ts_project green
```

All commits local; mirror and registry publish stay held for the owner's
word, per standing policy.

## 17. Review points — RESOLVED by the owner (2026-07-07)

1. **Names** — approved as proposed (`tcg_*` tools, bin
   `tcg-typescript`).
2. **PROP-026 placement** — approved vibevm-hosted, WITH the
   portability amendment: the family is a dedicated `vibe-tcg` crate
   behind `TcgHost`, zero vibe-mcp dependency, so extracting a
   standalone MCP server later is a new thin binary (D4/D6/§2 carry
   the amendment).
3. **The battery** — superseded: automated, not manual. The opencode
   CLI (model gpt-oss-20b (free); PATH, fallback
   `C:\opt\nvm\v24.18.0\node_modules\opencode-ai\bin\opencode.exe`)
   drives the task battery headlessly; the harness was written and the
   control arm executed at acceptance time (D7 carries the full
   design; `research/tcg-bench/RUNBOOK.md` + the control REPORT are
   the acceptance artifacts).
