# AGENTIC-TCG-RUST-PLAN v0.1 — the agentic type oracle for Rust

_Status: **ACCEPTED with owner amendments, 2026-07-07 (same day as
authoring; execution NOT yet commissioned — the plan waits for the
owner's word).** Owner review resolved all seven §17 points: (1)
substrate approved — with `ra_ap_*` library embedding recorded in
ROADMAP.md's **Far backlog** as a deliberately distant capability
upgrade («сильно-сильно позже»), not a rejection forever (D1 carries
the note); (2) fidelity posture approved; (3) **NAMING AMENDMENT — a
standing policy**: every Rust artifact with a cross-language analog
ends in `-rust` (crate and module names included; executables and
externally visible libraries especially) — the bridge crate becomes
`tcg-oracle-bridge-rust`, the brief `vibe-agentic-tcg-rust.md`, and
the sweep of the package's existing unsuffixed CLI crates is
commissioned into Phase 1 (NEW D13); (4) the `research/rust-demo`
testbed approved — experiment there first; (5) rust-analyzer is a
STACK PREREQUISITE — installing ai-native-rust obliges the machine to
carry it; without the stack, skipping is clean (D11 reworded); (6) no
battery — confirmed; (7) the latency targets stand as INITIAL targets,
and a miss CANCELS NOTHING — it is reported to the owner (§4.2/R1
carry the rule). Originally DRAFT the same day; written against tree
`77218b5` (floor green; local == origin + 1 plan commit).
Commissioned by the owner as owner-court item 3 of the agentic-tcg
checkpoint: «напиши аналог vibe-agentic-tcg для Rust» — the Rust twin
of the agentic delivery, over rust-analyzer, that PROP-026 §2 and the
`language` parameter were deliberately cut to admit («a new language
value, not new tools»). Prior art:
[AGENTIC-TCG-TS-PLAN-v0.1](AGENTIC-TCG-TS-PLAN-v0.1.md) (EXECUTED) —
this plan mirrors its shape phase-for-phase where the languages agree
and states explicitly where they cannot. Cold-executable: any phase is
a safe stop; the floor must be green at every phase boundary. The Stage-B delivery experiments
([TCG-STAGE-B-DELIVERY-PLAN-v0.1](TCG-STAGE-B-DELIVERY-PLAN-v0.1.md))
are BACKLOGGED by the owner the same day — this campaign proves the
Rust oracle's MECHANICS (corpus + bench), and explicitly does NOT run
an agent battery (§14)._

Mandate (owner, 2026-07-07): build the Rust analogue of the
`vibe-agentic-tcg-ts` line — the long-lived Rust type oracle, the same
four `tcg_*` tools answering `language: "rust"`, discipline-aware
enrichment through the gate's own rules, and the quantitative
mechanics proof (differential corpus + bench) — with full
specifications into `stack:org.vibevm/rust-ai-native`. Production-grade
quality bar applies (the standing owner directive in
`spec/boot/90-user.md`: no MVP framing, no stub subcommands as shipped
surface).

## 0. Why this exists (one screen)

The TS campaign proved the shape: most of a logit-mask's value —
what is in scope, what type-checks, millisecond feedback instead of
write→floor→parse→retry, discipline enforced at generation time — is
deliverable to an agent as tools, while the by-construction guarantee
stays with the floor. Rust is the project's PRIMARY language (vibevm
itself, the discipline toolchain, every consumer this stack serves),
and today a Rust-editing agent has the floor's truth only at
write-grain latency: `cargo check` seconds after the file lands. The
Rust twin closes the same gap the TS oracle closed — and it is the
line's second language, which is what proves PROP-026's central bet:
that the family scales by adding a language VALUE, not a parallel tool
family.

One asymmetry is load-bearing and must be stated up front, not
discovered later: **the TS oracle IS the compiler** (the
LanguageService is tsc's own engine — agreement with tsc is agreement
by construction), while **rust-analyzer is NOT rustc**. r-a's native
diagnostics are a separate implementation with deliberately partial
coverage. The Rust oracle is therefore an APPROXIMATION of the floor's
truth, honestly labelled: the differential corpus curates error
classes inside r-a's native competence and pins them against `cargo
check`; blanket rustc parity is a named non-goal; the floor stays the
truth (§3 D2). Everything else — overlays, latency, enrichment,
consultation — carries over.

Clean-room note: this campaign does not open
`eth-sri/type-constrained-code-generation` at all (the standing
90-user.md rule; the TS campaign's D10 posture carries over verbatim).
Concept sources: our own briefs and engines, the public LSP 3.17
specification, and rust-analyzer's public protocol documentation.

## 1. Current-state facts, verified at authoring (do not re-discover)

Product side (all landed by the TS campaign, all additive-ready):

- **`vibe-tcg` is 887 lines and one arm away from Rust.**
  `LANGUAGES: [&str; 1] = ["typescript"]` (lib.rs:134) gates
  `run_tool`; `language_binary()` (registry.rs:52) maps
  `"typescript" → "tcg-typescript"` with `unreachable!` for the rest.
  The `Spawner` seam is `(artifact, root)` — language-free. The FOUR
  tool schemas are language-generic already (descriptions say "the
  project's own compiler"); `language_schema()` serialises `LANGUAGES`,
  so the JSON enum widens by itself.
- **But the recipes are TS-hardcoded**: `TcgError::StackNotInstalled`'s
  message embeds the literal TS requires line (lib.rs:69-75);
  `OracleGone`'s fix surface names `tcg-typescript` (lib.rs:92-98);
  `ProcessLink` stamps `language: "typescript"` into its spawn/gone
  errors (registry.rs:193-247). The Rust twin forces the
  parameterisation PROP-026 implied.
- **The registry mechanics are proven and reusable verbatim**: lazy
  spawn per language key, respawn-once, kill-on-drop, 60 s
  `REQUEST_TIMEOUT`, stale-frame skip, `oracle-crashed` mapping
  (registry.rs:150-317). The 60 s cap bounds the FIRST answer — a
  fact R1 must respect on large workspaces.
- **vibe-mcp's adapter is language-blind** (it mounts
  `vibe_tcg::tool_specs()` and delegates); `install.rs` gates that
  every served tool is named in `skill_template.md` (install.rs:212) —
  the template gains a Rust line, the gate itself is untouched.
- **PROP-026 anticipates this campaign by name** (§2: "the Rust twin
  later adds an enum value, not new tools"; §4 resolves "the stack
  slot that declares the language's oracle binary").

Package side (`stack:org.vibevm/rust-ai-native`, v0.4.0):

- **Three `[[binary]]` entries** (discipline-rust / conform-rust /
  specmap-rust) — `tcg-rust` becomes the 4th; `vibe bin list` today
  shows 7 binaries across both stacks, becomes 8.
- **`conform-frontend-rust` is STACK-AUTHORED** (crates/, not
  crates/vendor/) — `RustFrontend` implements the conform-core
  `Frontend` trait whose `extract(&self, file, crate_name, module,
  text) -> Vec<Fact>` is a PURE function over source text
  (facts.rs:160-173; syn-based, in-process, no-op `warm`). The Rust
  enrichment therefore needs NO third process and NO fact-shape
  duplication — the exact problem the TS campaign solved with
  conscious duplication (its D1) does not exist here.
- **The Rust fact vocabulary is rich enough today**: `Item`
  (is_pub/has_doctest), `Import`, `Ctor`, `UnsafeUse`, `ErrorVariant`,
  `FileMetrics`, `UnwrapUse`, `EnvRead` — with in_test/in_deviation
  scoping. The gate's Rust rules (no-unwrap-domain, unsafe-gate,
  ambient-env, file budget, Class-F/G) run over exactly these.
- **`conform-cli`'s rule assembly is PRIVATE** — pub surface is
  `load_config`/`load_config_or_default`/`run_check`/`run_freeze`
  (lib.rs:22-90). The `build_rules` pub seam must be cut, mirroring
  the TS campaign's D5 exactly; conform-cli is stack-authored, so
  this is a package-internal refactor, NOT a discipline-core bump.
- **There is no Rust cell-isolation conform rule and no `[rust]`
  cells topology in conform.toml** (vibevm's own config: roots,
  registry_file, audit_crates, max_file_lines, gated lists,
  env_roots). The `scope` op's cell/seam context for Rust derives
  from module paths, not from policy — v0.1 keeps that honest (§3 D6).
- **The package self-traces gate-only** (specmap.toml: namespace
  `rust-ai-native`, explicit scan_roots, CLI drivers exempt) — the two
  new crates join scan_roots; the driver joins `exempt`, the bridge
  does not.
- **spec/rust/ has no mechanisms/ dir yet** (GUIDE + tools/vibe-tcg.md
  + cards); the TS package precedent (spec/typescript/mechanisms/)
  supplies the shape. The GUIDE's §13 (wiring) / §14 (sweep idioms)
  are the extension points.

Substrate (verified on this box during authoring):

- **rust-analyzer was NOT present** (`rustup which` → "Unknown
  binary") and was installed during authoring: **rust-analyzer 1.93.1
  (01f6ddf7 2026-02-11)**, the stable-toolchain component matching
  rustc 1.93.1. Consumer resolution must treat absence as a
  first-class recipe (`rustup component add rust-analyzer`), because
  a fresh box fails exactly this way.
- r-a 2024+ supports **pull diagnostics** (`textDocument/diagnostic`),
  **utf-8 positionEncoding** negotiation, and the experimental
  **`rust-analyzer/serverStatus`** notification (`quiescent: true`) —
  all three are capability-gated at initialize and re-verified by the
  Phase-0 spike against 1.93.1, not assumed.
- **LSP positions are 0-based line / UTF-16 code units by default**;
  the tcg outer protocol is 1-based line / 0-based character. The
  conversion (and the encoding, when utf-8 is not granted) is
  bridge-internal — the outer protocol does not move (§3 D3).
- **The wire contract the relay must speak upward is pinned**:
  TCG-PROTOCOL v0.1 §1 envelopes (`{proto, id, op, params}` /
  `{proto, id, ok, result|error}`), §2 ops (init/update/validate/
  scope/complete/type/shutdown), §3 additive enrichment, §5 additive
  evolution. The product's `ProcessLink` special-cases only the
  `oracle-crashed` error kind — new environment-error kinds from a
  Rust relay pass through as recipe-carrying details (registry.rs:
  310-313), so the Rust taxonomy may rename its two environment rows
  without touching the product.
- Version-bearing paths that must bump with 0.4.0→0.5.0: vibevm
  `vibe.toml` `"stack:org.vibevm/rust-ai-native" = "^0.4.0"` (caret
  on 0.x does NOT admit 0.5), `sync-engines.toml` target
  `packages/org.vibevm/rust-ai-native/v0.4.0/crates/vendor`, the boot
  INDEX slot path (regenerated by install). vibevm's specmap
  `[[external_specs]]` names only discipline-core — untouched.
  ts-demo — untouched.
- Windows lessons in force: verbatim-free paths before child argv/URIs
  (fourth home now), kill-on-drop asserted, junction-free here (no
  node), `git commit -F -` heredoc, editor-tool edits only.

## 2. Target end-state (what done looks like)

```
vibevm/
├─ spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md   +history entry, §2/§4
│                                                       carry the rust rows
├─ spec/terraforms/AGENTIC-TCG-RUST-PLAN-v0.1.md       this file
├─ ROADMAP.md                                          M1.25
├─ crates/vibe-tcg/                                    LANGUAGES += "rust";
│                                                       language_binary arm;
│                                                       per-language recipes;
│                                                       ProcessLink de-hardcoded
├─ crates/vibe-mcp/src/skill_template.md               + the rust teaching line
├─ research/rust-demo/                                 NEW: the committed Rust
│                                                       consumer testbed (ts-demo
│                                                       mirror: cells, GuestName
│                                                       newtype, floor green)
├─ research/tcg-bench/
│   ├─ corpus-rust/{cases,content}/                    NEW: 7 differential cases
│   └─ reports/REPORT-<date>-rust-baseline.md          NEW: bench + agreement
└─ packages/org.vibevm/rust-ai-native/v0.5.0/          (bumped from v0.4.0)
    ├─ vibe.toml                                       [[binary]] × 4 (+ tcg-rust)
    ├─ crates/tcg-oracle-bridge-rust/                  NEW: the LSP client seam
    │                                                   (framing, handshake,
    │                                                   overlays, pull-diags,
    │                                                   encoding cell, replay-
    │                                                   tested; kill-on-drop)
    ├─ crates/tcg-cli-rust/                            NEW: bin `tcg-rust`
    │                                                   (serve / validate / scope /
    │                                                    complete / type / bench)
    ├─ crates/conform-cli-rust/                        RENAMED (D13, was
    ├─ crates/discipline-cli-rust/                      conform-cli / discipline-
    ├─ crates/specmap-cli-rust/                         cli / specmap-cli);
    │                                                   conform-cli-rust also
    │                                                   gains the pub
    │                                                   build_rules seam
    └─ spec/rust/
        ├─ GUIDE-AI-NATIVE-RUST.md                     §2 naming policy (D13) +
        │                                               §13 wiring move + the
        │                                               generation-time section
        ├─ tools/vibe-tcg-rust.md                      RENAMED from vibe-tcg.md
        │                                               (D13) + pointer to the
        │                                               sibling brief
        ├─ tools/vibe-agentic-tcg-rust.md              NEW: the component brief
        │                                               at FULL 7-section parity
        └─ mechanisms/
            ├─ TCG-ORACLE-RUST-v0.1.md                 NEW: the r-a process model
            └─ TCG-PROTOCOL-RUST-v0.1.md               NEW: outer-hop parity +
                                                        the LSP inner hop
```

Runtime topology (three processes, mirroring TS — but the facts hop is
gone: enrichment is in-process in the relay):

```
agent (Claude Code etc.)
  │  MCP tools/call: tcg_validate/scope/complete/type, language:"rust"
  ▼
vibe mcp serve                    (long-lived; OracleRegistry key "rust")
  │  TCG-PROTOCOL frames — IDENTICAL envelopes to the TS hop
  ▼
tcg-rust serve                    (long-lived, slot-resident artifact;
  │  self-inits: conform.toml → build_rules; RustFrontend.extract on
  │  overlay text + conform_core::check = enrichment, in-process)
  ▼
rust-analyzer                     (long-lived LSP child, the CONSUMER's
     component: rustup-resolved;   own toolchain; overlays via didOpen/
     pull diagnostics, hover,      didChange; never touches disk)
     completion)
```

## 3. Decisions (D1–D13)

### D1 — substrate: the consumer's rust-analyzer over LSP stdio

The oracle process is the consumer's own `rust-analyzer` binary spoken
to over the Language Server Protocol on piped stdio — the exact analog
of "the consumer's own typescript install". Resolution order (each
step recipe-carrying on failure): (1) `rustup which rust-analyzer`
run from the project root, so `rust-toolchain.toml` pinning is
honoured; (2) `rust-analyzer` on PATH; (3) hard fail:
`RustAnalyzerMissing` with `rustup component add rust-analyzer`. The
resolved path and version land in the `init` result (the `ts_version`
analog).
*Rejected for v0.1 — but FAR-BACKLOGGED, not buried (owner
disposition, §17.1):* embedding rust-analyzer as a library (the
`ra_ap_*` crates — r-a ships its internals as versioned crates:
`ra_ap_ide`, `ra_ap_hir`, `ra_ap_vfs`, …). In-process linking would
give LSP-free access to the full semantic model — custom traversals,
richer scope/brand answers, no child process — a real capability
upgrade for a much later day. Today it loses on weekly-release API
churn, an enormous dependency tree compiled into a slot binary, and
binding OUR r-a version instead of the consumer's toolchain,
violating the consumer-resolution posture both stacks established.
Recorded in ROADMAP.md's **Far backlog** with the picking-up
conditions; the brief's staged-ambition section (Phase 1) carries the
same disposition. *Rejected:* `cargo check` as the oracle —
disk-bound, seconds-grain, no completions/hover; that truth already
exists as the floor, and the oracle's whole point is the latency
class between keystroke and floor. *Rejected:* rustc internals —
nightly-only, unshippable.

### D2 — the fidelity posture: an honest approximation, curated classes

rust-analyzer's native diagnostics are not rustc. The campaign does
not pretend otherwise, anywhere:

- `TCG-ORACLE-RUST-v0.1` carries an explicit **approximation
  section**: what the oracle answers is r-a's view; the floor
  (`discipline-rust floor` → cargo check) remains the truth; a clean
  `validate` does not certify a clean floor.
- The differential corpus (D10) pins agreement on SEVEN CURATED
  classes chosen inside r-a's native competence (type mismatch,
  unresolved name, wrong arity, privacy violation, …), each mapped
  r-a-diagnostic-id ↔ rustc E-code in a COMMITTED mapping table. The
  claim is 100% existence-grain agreement ON THOSE CLASSES — not
  blanket parity, which is a named non-goal.
- The brief's honest-limits section states the delta class openly
  (borrow-check subtleties, trait-solver edge cases, macro-heavy code
  — r-a may be silent where rustc speaks).

This is the one place the Rust twin is structurally WEAKER than the
TS original, and saying so in the spec is what keeps the tool
trustworthy.

### D3 — `tcg-oracle-bridge-rust`: the LSP client as a seam, outer protocol unmoved

A new stack crate — the TS twin's stem (`tcg-oracle-bridge`) plus the
D13 language suffix — carrying its role with LSP mechanics inside:

- **Framing cell**: `Content-Length` header framing (read/write),
  JSON-RPC 2.0 envelopes; requests correlate by our ids; server
  NOTIFICATIONS (publishDiagnostics, progress, serverStatus) are
  routed by the reader thread into typed state cells, never matched
  to requests. Replay tests run the whole client against recorded
  transcripts — no rust-analyzer in the unit suite (the TS bridge's
  replay posture).
- **Handshake cell**: `initialize` requesting utf-8
  `positionEncoding`, pull diagnostics, and the experimental
  `serverStatus` notification; the GRANTED capability set is kept and
  every downstream feature keys off it (a capability the server did
  not grant degrades per B5 — a well-formed error/recipe, never a
  crash).
- **Position cell**: outer protocol positions (1-based line, 0-based
  character — TCG-PROTOCOL §2, unchanged for wire parity) ↔ LSP
  positions (0-based line; utf-8 offsets when granted, else UTF-16
  code units converted through the line's text). Unit-tested on
  non-ASCII lines; one corpus case carries Cyrillic content to pin it
  end-to-end.
- **Overlay cell**: `didOpen(uri, version 1, text)` claims a document
  (r-a stops reading disk for it — exactly our overlay semantics);
  `didChange` full-text with a per-document MONOTONIC version (the TS
  session-monotonic lesson is LSP-native law here); `didClose`
  releases back to disk (= `update {content: null}`). `validate`
  without `content` reads the disk file and opens with that text, so
  version bookkeeping has one owner.
- **Ops**: validate → `textDocument/diagnostic` (pull) for the one
  document; publishDiagnostics-wait (version-keyed, deadline-bounded)
  is the capability fallback the spike decides on. scope/complete →
  `textDocument/completion` (+ per-entry detail within the
  prefix/max cut, TCG-PROTOCOL §2 semantics preserved). type →
  `textDocument/hover`. shutdown → LSP `shutdown` + `exit`, then
  kill-on-drop as the backstop (test-asserted, the Windows lesson).
- **Quiescence**: after initialize, the bridge waits for
  `serverStatus.quiescent` (when granted) or the initial progress
  tokens to drain, bounded by a deadline; the first semantic answer
  before quiescence is legal but flagged `degraded: true` (B5) so
  callers can distinguish warm truth from cold best-effort.
- **Error taxonomy** (five kinds, mirroring the TS shape with two
  environment rows renamed): `RustAnalyzerMissing` /
  `WorkspaceUnloadable` (cargo metadata/project load failed — the
  `TypescriptUnresolvable` analog) / `OracleCrashed` / `Protocol` /
  `Timeout`, each REQ-citing with a fix surface. Wire kinds:
  `rust-analyzer-missing`, `workspace-unloadable`, and the three
  shared names. The product passes unknown kinds through as
  recipe-carrying details (verified fact §1), so no product edit
  rides on the renames.

### D4 — bin `tcg-rust` in the stack: serve / one-shot / bench

`tcg-cli-rust` (bin **`tcg-rust`** — the `<tool>-<language>` suffix
convention): `serve --root` (the persistent enriching relay, D5;
self-inits at start with a stderr boot line, mirroring
tcg-typescript's relay-owns-init lesson so a host's first frame can be
`validate`); one-shot `validate <file> [--content-from -|<path>]
[--json]` (exit 1 on an error-grade diagnostic OR a non-baselined
finding — the TS exit contract verbatim), `scope [--position L:C]`,
`complete --position L:C`, `type --position L:C`; `bench --corpus
<dir> --report <file> --root <dir>` (D10). The manifest gains the 4th
`[[binary]]`; the boot snippet toolchain block and the package README
gain the row.
*Rejected:* folding the oracle into `discipline-rust` — the umbrella
is the GATE panel, the oracle is a generation-time SERVICE (the TS
D3 reasoning holds unchanged).

### D5 — enrichment is in-process: the frontend IS the fact source

The relay reads the project's `conform.toml` once per init
(`conform_cli_rust::load_config_or_default`, origin printed — the
D13-renamed crate); a NEW pub
seam `conform_cli_rust::build_rules(&Config) -> Vec<Box<dyn Rule>>`
(mirroring the TS campaign's D5 export; today the assembly is private
in run_check) hands it the gate's OWN rule set. On `validate`: the
effective text (overlay or disk) goes through
`RustFrontend.extract(file, crate_name, module, text)` — the same
syn-based extraction the gate runs, called as a library —
into `conform_core::check`, findings flagged `baselined` against the
project's frozen ratchet baseline (the same file `run_check` reads),
plus `advice: [string]` in Class-F form citing GUIDE anchors
(`.unwrap()` in domain → GUIDE §6 + the `#[spec(deviates)]` recipe; a
missing doctest on a new pub seam → §3 Class G; `std::env` reads
outside env_roots → the R-001 rule; file over budget → §2). The
crate/module strings for the single file are computed by a small
relay-local helper mirroring the engine's path mapping — and an
enrichment test PINS the parity by running `conform-rust check` and
the relay's enrichment over the same demo file and diffing the
finding sets (drift between the two becomes a red test, not a silent
lie).
*Rejected:* evaluating rules bridge-side or oracle-side (the engine
exists in-language; any second evaluator is divergence by
construction). *Rejected:* exporting a module-path helper from
conform-core — that is a VENDORED crate; touching it means a
discipline-core version bump and a two-stack vendor re-sync, all to
save ten relay-local lines (prediction §4.3 gates this).

### D6 — scope/complete semantics for Rust: module cells, newtype brands

- `scope` → `{symbols, cell, seam_file, branded[]}` with the SAME
  response shape (wire parity): `symbols` from a completion sweep at
  the neutral position (the TS trick); `cell` = the module path
  derived from the file's location under `src/` (there is no `[rust]`
  cells topology in conform.toml — verified fact §1 — so v0.1 derives,
  never invents policy); `seam_file` = the enclosing `mod.rs`/`lib.rs`
  that re-exports it; `branded[]` carries the RUST brand analog — the
  seam's NEWTYPES: pub tuple structs with a single private field
  (the parse-constructor pattern the guide mandates), syn-detected,
  honestly labelled `heuristic: true` exactly like the TS
  intersection-brand heuristic.
- `complete` entries carry `unsafe: true` + a one-line reason when
  the entry would land a §6-banned form in domain code — v0.1 flags
  `unwrap`/`expect` completions on `Option`/`Result` receivers
  outside test files (name+receiver heuristic, honest label; policy
  finalisation stays relay-side, mirroring TCG-PROTOCOL §3).

### D7 — the testbed: `research/rust-demo`, a deliberate ts-demo mirror

A committed, floor-green Rust consumer, dependency-free by design
(fast r-a init, no network, no proc-macros): one lib crate with cells
`src/cells/greeting.rs` + `src/cells/farewell.rs` over
`src/core/text.rs`; a `GuestName` NEWTYPE with a private inner and
`parse_guest_name` as its only constructor (returning a thiserror,
REQ-citing error) — the same shape ts-demo brands, so cross-language
corpus cases rhyme; `vibe.toml` requiring `^0.5`, `conform.toml` +
frozen baseline, `specmap.toml` (namespace `rust-demo`,
`[[external_specs]]` into the 0.5.0 slots — version-bearing),
committed `Cargo.lock`, `[workspace]` with `exclude = ["vibedeps"]`
(the standing consumer lesson), the demo `AGENTS.md` riding pattern
copied from ts-demo (battery-ready for whenever Stage-B is
commissioned). Floor = `discipline-rust floor` all green; the
expectation is an EMPTY conform baseline — Rust's newtype needs no
cast, so unlike ts-demo there is no irreducible frozen finding
(prediction §4.6; if one proves irreducible it is frozen and named).
*Rejected:* using vibevm itself as the testbed (r-a cold init on the
full workspace is tens of seconds and entangles the campaign's
acceptance with an unrelated tree; vibevm-as-root stays a manual
smoke, not a gate).

### D8 — specs: brief + two mechanisms in the package; PROP-026 widens

Package (`rust-ai-native`, all under `spec/rust/`):

- `tools/vibe-agentic-tcg-rust.md` — the component brief at FULL
  seven-section parity (problem · design stance · component shape ·
  staged ambition · licensing · risk register · summary), stating the
  mask-value decomposition, the two-hop topology, the D2 fidelity
  posture, DR1-015 honesty (tools you can ignore do not distort — and
  the Stage-A null: they may also not help until delivery binds), and
  the shared-infrastructure claim toward the token-level far future.
  The staged-ambition section also records the `ra_ap_*` embedding as
  the owner's FAR-BACKLOG stage (D1) so the disposition lives in the
  package, not only in vibevm's ROADMAP. The file name carries the
  D13 suffix (the TS analog is `vibe-agentic-tcg-ts.md`).
- `mechanisms/TCG-ORACLE-RUST-v0.1.md` (`req` units + anchors): the
  r-a process model — resolution order (D1), LSP lifecycle,
  capability negotiation, overlay/version semantics, quiescence and
  the degraded flag, the APPROXIMATION section (D2), latency posture
  (measured, not gated), Windows child discipline.
- `mechanisms/TCG-PROTOCOL-RUST-v0.1.md`: the outer hop restated at
  WIRE PARITY with the TS TCG-PROTOCOL v0.1 (§1 envelopes, §2 ops,
  §3 additive enrichment, §5 evolution — differences called out
  explicitly: the two renamed environment error kinds, the
  `init`-result fields (`ra_version`, `toolchain`, `crates_loaded`),
  rust fact shapes in `validate.result.facts`), plus the inner hop:
  which LSP requests implement which op. Parity is ENFORCED at the
  product level (the same `vibe-tcg` client drives both relays in the
  live-chain tests) and by outer-frame replay goldens in each
  package; the restatement's drift risk is accepted and named.
- Rewrites: `tools/vibe-tcg.md` RENAMES to `tools/vibe-tcg-rust.md`
  (D13 — the far-future token tool line itself becomes
  `vibe-tcg-rust`, dissolving today's collision where the bare name
  `vibe-tcg` means BOTH the Rust token brief and the language-generic
  product crate) and gains the pointer paragraph (the agentic sibling
  exists; the token-level line stays VERY-FAR-FUTURE per the owner's
  standing disposition); GUIDE §2 gains the D13 naming-policy
  paragraph; GUIDE §13 gains the wiring
  move (`tcg_*` tools + `vibe bin exec tcg-rust -- …` one-shot forms)
  and a generation-time-assistant subsection mirroring the TS §14
  posture (consult before you write; the floor stays the truth); both
  Rust SKILL.md twins (/discipline-sweep, /terraform-rust) gain the
  generation-time section the TS twins got; boot snippet + README
  rows.

vibevm: PROP-026 §2/§4 gain the rust rows (language value, binary
mapping, the per-language requires recipe) + a history entry —
NO new PROP; ROADMAP.md gains M1.25.

### D9 — product wiring: the enum-value promise, cashed

`vibe-tcg`: `LANGUAGES` becomes `["typescript", "rust"]`;
`language_binary` gains the `"rust" → "tcg-rust"` arm; a per-language
table (binary, requires-line, one-shot recipe) replaces the hardcoded
TS strings in `StackNotInstalled`/`OracleGone` messages; `ProcessLink`
carries the language it spawned (no literal "typescript" in its
errors). vibe-mcp: `skill_template.md` teaches the rust value (the
served-tool gate is untouched — same four tools). Tests: vibe-tcg
unit coverage for the rust dispatch + per-language recipes through
doubles; vibe-mcp fixture WITHOUT the rust stack → the recipe names
the rust requires line; `live_chain_on_rust_demo` (ignored-by-default,
real chain on this box: r-a + built artifact) joins
`live_chain_on_ts_demo`. The four tools do NOT multiply — PROP-026 §2
holds by construction and §4.4 pins it.

### D10 — the mechanics proof: differential corpus + bench, no battery

`research/tcg-bench/corpus-rust/{cases,content}` — seven cases
mirroring the TS corpus grammar (`{file, content_from?, expect}`):

1. `01-clean-disk` — a demo file as-is: zero diagnostics.
2. `02-clean-add` — a NEW file as overlay (never on disk): zero
   diagnostics, proving overlay-only analysis.
3. `03-type-mismatch` — seeded `E0308`-class (r-a `type-mismatch`).
4. `04-unresolved-name` — `E0425`-class (r-a `unresolved-ident`-
   family).
5. `05-wrong-arg-count` — `E0061`-class.
6. `06-newtype-privacy` — constructing `GuestName`'s private inner
   from another cell: `E0603`/private-field-class — the discipline's
   brand rule made COMPILER-checkable, which is exactly Rust's edge
   over TS here.
7. `07-unwrap-in-domain` — compiles clean; expects the ENRICHMENT
   finding (`no-unwrap-domain`, non-baselined) — pinning the D5 hop.
   At least one case's content carries non-ASCII (Cyrillic) text to
   pin the position cell end-to-end.

Truth source: `cargo check --message-format=json` over a temp
materialisation of each case; the committed r-a-id ↔ E-code mapping
table (bench-owned, spec-referenced) translates. `tcg-rust bench`
runs the corpus warm + cold, reports per-op p50/p95, cold-init time,
agreement %; `REPORT-<date>-rust-baseline.md` commits the numbers.
Posted expectations (REPORT is the ratchet, never CI): existence-grain
agreement 7/7 on the curated classes; warm `validate` p50 < 500 ms on
rust-demo (r-a semantic analysis is heavier than the TS LS — the
target is honest, and moves only with a recorded reason); cold
init-to-quiescent < 15 s on the zero-dep demo; `complete` p50
< 300 ms.

**No agent battery in this campaign.** The Stage-B delivery
experiments are backlogged by the owner (this same day); an opt-in
Rust arm would re-measure the known Stage-A null at new cost. The
demo ships battery-ready so the future cross-language Stage-B pays
nothing extra.

### D11 — rust-analyzer is a stack prerequisite (owner resolution §17.5)

Installing `stack:org.vibevm/rust-ai-native` OBLIGES the machine to
carry rust-analyzer — a declared stack prerequisite, exactly as
node ≥ 22.6 is the TS stack's (README, GUIDE §13, and the boot
snippet's toolchain block all say so; the recipe is `rustup component
add rust-analyzer`). The boundary the owner drew: with the stack
installed the obligation is hard; without the stack there is none and
skipping is clean. Concretely:

- INSIDE the stack's own suite (the bridge/CLI e2e tests, the corpus,
  the demo floor) an absent rust-analyzer is a hard, recipe-carrying
  FAILURE — a stack holder without the prerequisite is a broken
  install, not an optional configuration (never-skip; the bridge's
  replay suite and every unit layer stay r-a-free regardless).
- OUTSIDE the stack nothing changes: a machine without ai-native-rust
  owes nothing — the `tcg_*` tools answer `language: "rust"` with the
  not-installed recipe, and the product-side live chain
  (`live_chain_on_rust_demo`) stays ignored-by-default like its TS
  twin. No vibevm-wide obligation is created.
- The vibevm dev repo HOLDS the stack (vibedeps), so dev boxes and
  self-check's package steps inherit the obligation. (This box: the
  component was absent and was installed during plan authoring,
  1.93.1 — the fresh-box failure mode is real.)

### D12 — version bump 0.4.0 → 0.5.0, one move at campaign open

The deferrals/TS-campaign ritual, applied to the rust stack:
`git mv packages/org.vibevm/rust-ai-native/{v0.4.0,v0.5.0}`; manifest
`version = "0.5.0"`; vibevm `vibe.toml` requires `^0.5.0` (caret on
0.x does not admit the minor); `sync-engines.toml` target path;
`vibe install` re-materialises (PROP-011 §2.6 mutability) and the
boot INDEX regenerates with the 0.5.0 slot path; `sync-engines
--check` green over the renamed dir; vibevm's specmap/conform stay
byte-stable modulo nothing (packages/ is not a scan root; external
specs name only discipline-core). rust-demo requires `^0.5` from
birth. Registry publish stays owner-held (joins the standing
publish-court item as 0.5.0/0.4.0/0.4.0).

### D13 — the language-suffix naming policy + the rename sweep (owner amendment §17.3)

A standing convention, set by the owner during this plan's review:
**every Rust artifact that has an analog in another language ends in
`-rust`** — crate and module names included; executable names and
externally visible libraries especially. The TS package already
complies (`discipline-cli-typescript`, `conform-cli-typescript`,
`tcg-cli-typescript`); the Rust package's BINARIES already comply
(`discipline-rust` / `conform-rust` / `specmap-rust`); its CLI crates
do not («а не просто cli как сейчас» — the owner's words point exactly
here), and the 0.5.0 boundary is the once-per-bump moment to fix them:

- `crates/conform-cli` → `crates/conform-cli-rust` (ident
  `conform_cli_rust`), `crates/discipline-cli` →
  `crates/discipline-cli-rust`, `crates/specmap-cli` →
  `crates/specmap-cli-rust`. Binary NAMES do not move.
- `env-audit` KEEPS its name — no cross-language analog; the rule
  binds analogs only.
- The vendored engines (conform-core, specmap-core, specmark,
  specmark-grammar) are language-NEUTRAL by design — outside the
  rule's reach, untouched (§4.3 still gates them mechanically).
- The new crates are born compliant: `tcg-oracle-bridge-rust` (the TS
  twin's stem + the suffix), `tcg-cli-rust`.
- Spec files with cross-language analogs follow too («всё, что
  относится к Rust и имеет аналоги» — the owner's word is "everything"):
  the new brief is `vibe-agentic-tcg-rust.md`, and the token-level
  brief `vibe-tcg.md` renames to `vibe-tcg-rust.md` with the
  far-future tool line becoming **`vibe-tcg-rust`** — dissolving a
  real collision: today the bare name `vibe-tcg` means BOTH the Rust
  token tool and the language-generic product crate
  (`crates/vibe-tcg`). After the rename the bare name belongs to the
  generic family alone. ROADMAP M1.23's mention updates alongside the
  M1.25 addition.
- `GUIDE-AI-NATIVE-RUST.md` §2 (naming) gains the policy paragraph so
  the convention outlives this plan.
- Deliberate boundary: cards and other prose under `spec/rust/` keep
  their names — the rule targets ARTIFACTS with analog-collision
  potential; a cards-wide rename would be churn without a consumer.
  Named here so the boundary is a decision, not an omission.

Ripple checklist (Phase 1 executes it as one commit): package
Cargo.toml members + inter-crate path-deps; `[[binary]]` `crate =`
paths in the package manifest; package specmap.toml
scan_roots/exempt; the boot snippet's `cargo install --path
…/crates/discipline-cli-rust` recipe; GUIDE §13 wiring texts; package
README; vibevm root Cargo.toml path-deps and the xtask shim imports
(`use conform_cli…` → `conform_cli_rust…`, the specmap shim alike);
self-check.sh wherever a step names a crate path; the re-materialised
vibedeps slot and regenerated boot INDEX.

## 4. Predictions (falsifiable, checked by the REPORT and the diff)

1. The differential corpus agrees 7/7 existence-grain on the curated
   classes (position at ±1-line tolerance). ANY existence-grain miss
   is a bug or a wrong class choice — it rewrites the corpus or the
   bridge, never the tolerance.
2. Warm `validate` p50 < 500 ms and cold init < 15 s on rust-demo;
   `complete` p50 < 300 ms. Falsified → the §13 R1 ladder, targets
   move only with a recorded reason in the REPORT — and per the
   owner's §17.7 resolution a miss CANCELS NOTHING: the campaign
   proceeds, and the miss is reported to the owner prominently (the
   REPORT, the WAL, and the campaign-close summary all carry it).
3. NO vendored-engine edits (conform-core / specmap-core / specmark
   untouched; `sync-engines --check` green throughout): the only
   conform-side change is the pub `build_rules` assembly seam in
   stack-authored conform-cli. Falsified → stop; a discipline-core
   bump enters the plan explicitly with the owner's eyes on it.
4. The product diff is additive-small: the language tables, the
   recipe parameterisation, the skill-template line, tests — no new
   tools, no adapter-logic change, no vibe-workspace change.
5. The TS line is FROZEN by this campaign: zero edits under
   `tools/ts-oracle`, `tcg-oracle-bridge`, `tcg-cli-typescript`, and
   the TS mechanisms docs.
6. rust-demo's conform baseline freezes EMPTY (the newtype needs no
   cast — Rust's privacy does what TS needed a sanctioned `as` for).
   Falsified → the irreducible finding is frozen and named in the WAL.

## 5. Phase 0 — spikes (no commits; gates for everything after)

1. **LSP-handshake spike** (scratch crate + throwaway driver, deleted
   after): against a minimal cargo project — `initialize` (record the
   GRANTED capabilities: positionEncoding? pull diagnostics?
   serverStatus?), `didOpen` an overlay with a seeded `E0308`,
   `textDocument/diagnostic` → the diagnostic WITHOUT any disk write,
   `hover` on a known symbol, `completion` inside a fn body,
   `shutdown`/`exit` → child gone (no zombie). Measure cold-to-
   quiescent and three warm validates. THE gating spike: it proves
   the D3 op set against 1.93.1 and records the first latency facts.
2. **Quiescence + degraded semantics**: how serverStatus/progress
   behave on the scratch project; decide eager-init-at-serve-start
   (TS relay lesson) vs lazy with degraded-first-answer; record.
3. **Pull-diagnostics fallback need**: if `textDocument/diagnostic`
   is absent or non-deterministic on 1.93.1, prototype the
   version-keyed publishDiagnostics wait — D3's fallback becomes the
   design, not the option.
4. **cargo-check mapping sanity**: one seeded file per corpus class
   through `cargo check --message-format=json`; record the exact
   E-codes and r-a ids for the mapping table.
5. Red spikes rewrite the affected decision IN THIS FILE before
   Phase 1 commits anything; findings land in the WAL session
   section.

## 6. Phase 1 — the version bump + the D13 renames + all specs

1. D12 ritual first (git mv, manifest, requires, sync-engines,
   re-materialise, INDEX regen).
2. The D13 rename sweep in the same boundary (its ripple checklist is
   normative): the three CLI crates → `-rust` names + idents; the
   token brief → `vibe-tcg-rust.md`; GUIDE §2 policy paragraph;
   vibevm root path-deps + xtask shim imports; boot/GUIDE/README
   recipe paths; package specmap.toml roots/exempt; binary NAMES and
   behaviour unchanged.
3. Author `vibe-agentic-tcg-rust.md`, `TCG-ORACLE-RUST-v0.1.md`,
   `TCG-PROTOCOL-RUST-v0.1.md`; the vibe-tcg-rust.md pointer + GUIDE
   §13 move + generation-time subsection + SKILL twins + boot row +
   README row; PROP-026 rust rows + history; ROADMAP M1.25 (+ the
   M1.23 tool-line mention updates to `vibe-tcg-rust`).
4. Acceptance: `bash tools/self-check.sh` exit 0 (13 steps, incl.
   sync-engines over the renamed dir AND the renamed crates — the
   xtask shims compile against the new idents); `cargo run -q -p
   vibe-cli -- bin list` still shows 7 binaries with UNCHANGED names;
   `cargo xtask specmap --check` 0 dangling; `vibe check` clean;
   ts-demo floor untouched 7/7.
5. Commits (Rule 3): `build(packages): bump rust-ai-native to 0.5.0`;
   `refactor(rust-ai-native): rename the cli crates to the -rust
   convention` (body carries the D13 policy + the root-shim ripple);
   `docs(rust-ai-native): the agentic tcg brief + mechanisms`;
   `docs(spec): PROP-026 - the rust rows + roadmap M1.25`.

## 7. Phase 2 — `research/rust-demo`

1. The D7 tree: crate, cells, `GuestName` newtype + parse
   constructor + REQ-citing error, doctests on the seams, tests;
   vibe.toml (`^0.5`) + `vibe install`; conform.toml + frozen (empty)
   baseline; specmap.toml + committed index; Cargo.lock; AGENTS.md.
2. Acceptance: `discipline-rust floor` (slot-built via
   `vibe bin exec`) ALL green on the demo; `specmap-rust --check`
   0 dangling through the external specs; vibevm floor untouched.
3. Commit: `feat(research): rust-demo - the committed Rust consumer
   testbed`.

## 8. Phase 3 — `tcg-oracle-bridge-rust`

1. The D3 cells: framing, handshake, position, overlay, ops,
   quiescence, taxonomy; replay suite from the Phase-0 recorded
   transcripts; position-cell unit tests incl. non-ASCII; the
   kill-on-drop/zombie assertion; hermetic e2e against REAL r-a on a
   fixture tree (D11 posture: absent component = recipe-carrying
   FAIL).
2. Acceptance: package tests green; no vendored-crate diffs
   (`sync-engines --check`); vibevm floor untouched.
3. Commit: `feat(rust-ai-native): tcg-oracle-bridge-rust - the
   rust-analyzer client seam`.

## 9. Phase 4 — the `build_rules` seam + bin `tcg-rust`

1. `conform_cli_rust::build_rules` pub seam + doctest (behaviour
   identical; the gate tests hold).
2. `tcg-cli-rust` per D4/D5/D6: serve (self-init + boot stderr line;
   enrichment in-process; findings/baselined/advice), the four
   one-shot forms (TS exit contract), bench (runnable frame; Phase 6
   adds the corpus), the finding-parity test vs `conform-rust check`
   on the demo, the e2e serve test.
3. Manifest 4th `[[binary]]`; package specmap.toml scan_roots += the
   two crates, exempt += the driver; re-materialise vibedeps.
4. Acceptance: `vibe bin list` shows EIGHT binaries; `vibe bin exec
   tcg-rust -- validate src/cells/greeting.rs --root
   research/rust-demo` → 0 diagnostics / 0 findings / exit 0; a
   seeded-overlay validate → the E0308-class diagnostic + exit 1;
   `07`-shaped content → the non-baselined finding + exit 1; package
   suite + vibevm floor green.
5. Commits: `refactor(conform): export the rust rule-set assembly
   seam`; `feat(rust-ai-native): tcg-rust - serve, one-shot ops,
   bench frame`; `docs(packages): declare the tcg-rust binary`;
   `build(deps): re-materialise vibedeps at 0.5.0+tcg`.

## 10. Phase 5 — product wiring (PROP-026 cashed)

1. D9: the language tables, recipe parameterisation, ProcessLink
   de-hardcode; skill_template rust line; vibe-tcg + vibe-mcp tests
   (dispatch doubles, per-language recipes, absent-stack fixture);
   `live_chain_on_rust_demo` (ignored).
2. Acceptance: vibe-tcg/vibe-mcp/vibe-cli suites green; the manual
   `vibe mcp serve` probe — `tools/list` still carries FOUR tools,
   `tcg_validate {language:"rust"}` on a demo file answers enriched
   (recorded in the WAL; the owner's live MCP sessions see it after
   their restart — the standing R6 note); specmap 0 dangling; floor
   green.
3. Commits: `feat(tcg): the rust language value across the family`;
   `test(mcp): the rust live chain + absent-stack recipes`.

## 11. Phase 6 — the corpus + bench baseline

1. `corpus-rust/` cases + content per D10; the mapping table; bench
   fills in (agreement + latency, JSON + human table).
2. Run against rust-demo → `REPORT-<date>-rust-baseline.md` with real
   numbers; predictions §4.1/§4.2 checked (falsified → rewrite here +
   WAL note, the campaign form's honesty rule).
3. Acceptance: corpus green as a package/bench run; REPORT committed;
   floor green.
4. Commit: `test(research): the rust differential corpus + bench
   baseline`.

## 12. Phase 7 — campaign close

1. Final re-materialise if needed; the full panel: self-check 13
   steps exit 0, specmap --check 0 dangling, conform 0, ts-demo floor
   7/7 (untouched), rust-demo floor green, both live chains green,
   `fresh_ts_project` green.
2. WAL standing line + session section; this plan flips to EXECUTED
   with the commit map; CONTINUE.md follows the session-end contract,
   not this plan.
3. Mirror + registry publish stay owner-held (0.5.0 joins the
   publish-court item).

## 13. Risks & fallbacks

- **R1 — r-a latency/cold-init**, especially beyond demo-class trees.
  Ladder: one LS per root, never per request; single-document pull
  diagnostics (never whole-workspace); eager init at serve start
  (the relay boots before the host's first frame); the degraded flag
  on pre-quiescent answers; targets move only with a recorded REPORT
  reason. The product's 60 s first-request cap is the hard ceiling —
  documented in ORACLE-RUST for large-workspace consumers. And per
  §17.7: a missed target never cancels the campaign — it is carried
  to the owner in the report, prominently.
- **R2 — diagnostic fidelity** (r-a ≠ rustc). D2 is the posture; the
  curated corpus is the detector; the floor stays the truth; the
  mapping table makes the delta inspectable instead of vibes.
- **R3 — position encoding** (UTF-16 vs utf-8). The position cell +
  the non-ASCII corpus case; prefer negotiated utf-8; conversion is
  unit-tested against multi-byte lines.
- **R4 — r-a version/capability variance across consumer toolchains.**
  Capability-gated features + B5 degradation with recipes; the
  handshake records what was granted; ORACLE-RUST names the minimum
  useful capability set.
- **R5 — zombie r-a children on Windows.** shutdown/exit dance +
  kill-on-drop backstop + the e2e zombie assertion (the proven TS/
  PROP-019 lesson chain).
- **R6 — proc-macro/build-script-heavy consumers** (slow init,
  partial analysis until proc-macro srv warms). The demo avoids them
  by design; ORACLE-RUST names the limit honestly; not chased in
  v0.1.
- **R7 — outer-protocol drift between the two relays.** Wire parity
  is product-enforced (one client, two live chains) + per-package
  frame goldens; PROTOCOL-RUST names every deliberate delta.
- **R8 — scope creep toward an LSP relay.** PROP-026 §6 stands: four
  ops + lifecycle, full stop; rename/code-actions/references go
  through the owner.
- **R9 — the `build_rules` export tempting broader conform surface.**
  Assembly only; rule semantics stay in the vendored core; §4.3 gates
  it mechanically.

## 14. Non-goals (named, so they stay visible)

- The agent battery / delivery arms — BACKLOGGED with
  TCG-STAGE-B-DELIVERY-PLAN (owner, 2026-07-07); rust-demo ships
  battery-ready so the future cross-language Stage-B pays nothing.
- Blanket rustc-parity claims (D2); borrow-checker/trait-solver
  completeness.
- `ra_ap_*` library embedding — NOT here, but not dead: the owner's
  FAR BACKLOG (ROADMAP.md carries the entry and the picking-up
  conditions; D1 carries the note).
- An LSP relay surface (R8), reasoning ops, affinity involvement.
- Token-level TCG — VERY-FAR-FUTURE, owner disposition stands; the
  clean-room rule re-binds there.
- Any TS-side edit (§4.5); any vendored-engine edit (§4.3).
- Registry publish, mirror — standing separate items.

## 15. Quick-start for the executing session

```sh
bash tools/self-check.sh; echo "EXIT=$?"        # 13 steps, 0 — before anything
rust-analyzer --version                          # 1.93.1 on this box (component installed 2026-07-07)
rustup which rust-analyzer                       # the resolution path D1 uses
cargo run -q -p vibe-cli -- bin list             # 7 binaries before, 8 after
# then Phase 0, in order; record spike findings in the WAL session section
```

## 16. Whole-campaign acceptance (what "done" looks like)

```sh
bash tools/self-check.sh; echo "EXIT=$?"                          # 0
cargo run -q -p vibe-cli -- bin list                               # 8 binaries; tcg-rust listed
cargo run -q -p vibe-cli -- bin exec tcg-rust -- \
    validate src/cells/greeting.rs --root research/rust-demo       # 0 diagnostics; 0 findings; exit 0
# seeded-overlay validate → the E0308-class diagnostic named, exit 1
# 07-shaped content → the non-baselined no-unwrap finding + advice, exit 1
printf '…initialize/tools-list/tcg_validate(language:"rust")…' \
  | cargo run -q -p vibe-cli -- mcp serve --path research/rust-demo
                                   # four tools; the rust call answers enriched
cargo run -q -p vibe-cli -- bin exec tcg-rust -- \
    bench --corpus research/tcg-bench/corpus-rust \
    --report /tmp/r.json --root research/rust-demo                 # agreement 7/7, latencies in REPORT
cargo test -p vibe-mcp --test tcg_tools -- --ignored               # both live chains
# rust-demo floor green; ts-demo floor 7/7 UNTOUCHED; specmap 0 dangling;
# conform 0; sync-engines --check green; REPORT-…-rust-baseline.md committed
```

All commits local; mirror and registry publish stay held for the
owner's word, per standing policy.

## 17. Review points — RESOLVED by the owner (2026-07-07)

1. **Substrate** — approved: the consumer's rust-analyzer over LSP.
   Amendment: the `ra_ap_*` embedding goes to the FAR BACKLOG
   («сильно-сильно позже») as a capability upgrade with a short
   description of what it is — recorded in ROADMAP.md's Far backlog
   section and in D1; the brief's staged ambition carries it too.
2. **The fidelity posture** — approved as framed (D2).
3. **Names** — superseded by a STANDING POLICY (the owner's words:
   «всё, что относится к Rust и имеет аналоги в других языках,
   заканчивалось на Rust … это касается и имен крейтов и модулей, но
   в особенности — исполняемых файлов и внешне доступных библиотек»).
   D13 codifies it: the bridge is `tcg-oracle-bridge-rust`, the brief
   `vibe-agentic-tcg-rust.md`, the token brief renames to
   `vibe-tcg-rust.md`, and the existing unsuffixed CLI crates
   (`conform-cli` / `discipline-cli` / `specmap-cli` — «а не просто
   cli как сейчас») are renamed in Phase 1.
4. **The testbed** — approved: experiment on `research/rust-demo`
   first.
5. **The prerequisite** — resolved as a STACK obligation: whoever
   installed ai-native-rust is obliged to have rust-analyzer on the
   machine; without the stack there is no obligation and skipping is
   clean. D11 reworded accordingly.
6. **No battery** — confirmed.
7. **Latency targets** — approved as INITIAL targets; a miss cancels
   nothing — the campaign proceeds and the miss is reported to the
   owner in the campaign report (§4.2 and R1 carry the rule).
