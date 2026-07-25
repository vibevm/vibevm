# vibevm Convert Plan v0.1 — declare the surfaces, armor the core, drain the periphery

<status stage="spec" state="done" action="continue" actionstage="impl" comment="B0 2026-07-24: status line says AUTHORED 2026-06-12, NOT STARTED"/>

**status: AUTHORED 2026-06-12 · NOT STARTED · vibevm-specific · the full-depth conversion queue behind the 2026-06-12 audit**

*Origin: the same-day full-depth audit (post-SHRINK-v0.2) found the codebase split into
three maturity strata. **Stratum A — discipline by spirit**: vibe-resolver (41 `#[spec]`,
4 cells, naive-vs-sat differential oracle, runnable `FixpointModel`), vibe-check (12 cells,
one registration point), vibe-registry (3 Registry cells + seam oracle), vibe-index/scanner,
vibe-wire (generated, determinism-checked). **Stratum B — formally green, surfaces
undeclared**: vibe-core, vibe-workspace, vibe-publish, vibe-index (outside scanner/),
vibe-install — gates pass, but ~zero cells, ~zero seam doctests, invariants live in prose,
hot surfaces are stringly-typed. **Stratum C — not converted**: vibe-cli (23k LOC, the
largest crate, outside `CONFORM_GATED`, ~2.5–2.8k LOC of domain logic stranded in a binary),
vibe-mcp (zero metadata of any kind), conform-frontend-rust / specmark / specmark-grammar /
env-audit (the discipline's own toolchain, ungated), vibe-graph / vibe-llm (unmarked stubs).*

*The systemic finding this plan exists to kill: **the five-gate panel is green because the
gates only see declared surfaces.** `SeamHasDoctest` checks declared seams — vibe-core
declares none. `CellIsolation` checks `#[cell]` items — vibe-index declares two, so
`cli/add.rs:20` imports `server::lock` across an undeclared boundary unchallenged.
`CONFORM_GATED` lists 10 crates of 19. Green measures conformance of the declared, not
completeness of the declaration. The codebase is the few-shot prompt (guide §2, R3-006):
a model adding a feature copies the nearest example, and today the nearest example is
stratum-dependent. This plan converts strata B and C to full depth so that every example
a reader can land on teaches the discipline.*

*Owner scope decision carried forward unchanged: **DBT-0020 (the MCP pair) stays parked**
until the owner opens an MCP spec home. Phase 7 below specifies that endgame completely
but is **owner-gated** — it does not run on the strength of this plan alone.*

*Rhythm: per-crate gated batches, exactly the SHRINK-v0.1/v0.2 cadence — each batch lands
as a topic commit series, ends with build + crate tests + clippy + `conform check` (crate
scope) + a shrink-only freeze diff where the baseline is touched + specmap regen on any
unit/tag/line move. Any batch is a safe stopping point; this file plus the panel is the
resume pointer.*

---

## 0. Baseline survey and target arithmetic

Panel at plan time (2026-06-12, tree `9886419`): specmap 442 units / 407 items / 417
edges / 0 suspects / **6 warnings** / 0 gated orphans (10 dispositioned DBT-0020, 7 exempt
crates); conform 2 frozen (`file-length`: `vibe-cli/src/commands/mcp.rs`,
`vibe-mcp/src/tools.rs`) / 0 new; test-gate 1132/0/3 xfail-strict; fast-loop 20/20 < 60s;
self-check green.

`CONFORM_GATED` (`xtask/src/conform.rs:15`) lists **10** crates: vibe-core, vibe-index,
vibe-install, vibe-resolver, conform-core, specmap-core, vibe-registry, vibe-workspace,
vibe-check, vibe-publish. (The v0.2 execution record said 11 — count the list, not the
record; instrument discipline per SHRINK-v0.1 §0's lesson.) Outside the list: vibe-cli,
vibe-mcp, conform-frontend-rust, specmark, specmark-grammar, env-audit, vibe-graph,
vibe-llm, vibe-wire, plus xtask. Unlike `specmap-ratchet.json`, **the conform gate records
no reasons for its exemptions** — that asymmetry is itself a Phase-0 item.

Exit state of the main plan (Phases 0–6):

- `CONFORM_GATED` **10 → 15** (+ conform-frontend-rust, env-audit, specmark,
  specmark-grammar, vibe-cli), each flip preceded by a drain so the baseline never widens
  (the v0.2 recipe). Remaining exemptions (vibe-mcp until Phase 7; vibe-wire generated;
  vibe-graph / vibe-llm stubs; xtask tooling) are **recorded with reasons in a
  `CONFORM_EXEMPT` table beside the list**.
- Two new ratcheted rules, both born with checkers (A5): **`pub-doctest`** (Class G,
  gated on vibe-core) and **`ambient-env`** (the R-001 projection onto `env::var`,
  gated on `CONFORM_GATED`). Frontend v6 supplies the facts.
- specmap warnings **6 → 0**; suspects stay 0; gated orphans stay 0.
- Cells **20 → 23** (+3 vibe-publish `RepoCreator` variants — cells grow only where real
  variance exists; no cell-stamping, see §10).
- Compiled doctests on public seams: **+25 to +35** across vibe-core (≥8),
  vibe-workspace (≥4), vibe-publish (≥3), vibe-install (≥2), vibe-index (≥6).
- vibe-cli sheds **~2.5–2.8k LOC of domain** into gated lib crates and joins
  `CONFORM_GATED` itself.
- conform baseline stays **2** (the parked MCP pair) — it reaches **0** only via Phase 7.

Phase 7 (owner-gated) adds: `CONFORM_GATED` 15 → 16, cells 23 → 26, baseline 2 → 0,
specmap dispositioned 10 → 0, specmap exempt 7 → 6.

## 1. Phase 0 — hygiene and honest ledgers (one sitting)

Small, verified, zero-design items; everything here was confirmed at file:line during
the audit.

- **0.1 `CONFORM_EXEMPT` reasons table** beside `CONFORM_GATED` in `xtask/src/conform.rs`:
  every non-gated crate gets one line of why, mirroring the honesty of
  `specmap-ratchet.json`'s note. A silent exemption reads as a bug; a recorded one reads
  as a decision.
- **0.2 Stub markers**: `vibe-graph/src/lib.rs` (9 lines) and `vibe-llm/src/lib.rs`
  (6 lines) gain an explicit status header — `STATUS: M0 stub; providers land in M1.5
  per VIBEVM-SPEC §10.4` — so a cold reader (human or model) cannot mistake deferred
  scope for forgotten work.
- **0.3 PROP-014 heals itself**: all 6 specmap warnings are `pin-into-unmarked-unit`
  against `spec/discipline/PROP-014-specmap-bidirectional-traceability.md` §2.1–2.6
  (`#addressing-spec`, `#spec-units`, `#addressing-code`, `#edges`, `#index`,
  `#queries`) — the specmap spec's own sections carry no kind/revision lines. Add them.
  The discipline's index spec must be indexable.
- **0.4 R-001 in search**: `commands/search.rs:414` (`VIBEVM_GITHUB_API_BASE`) and
  `commands/search_cache.rs:76` (cache-root env) read `env::var` inside domain commands.
  Parameterize both from the composition root (`main.rs` reads, call-chain carries).
  This also pre-cleans Phase 5's `ambient-env` rule.
- **0.5 `short_name.rs:80,89`**: the two `expect("len checked == 1")` sites restructure
  to `match candidates.as_slice()` — the len-checked-expect pattern the SHRINK-v0.1
  Phase 2 already drained from every gated crate.
- **0.6 The lock seam**: `vibe-index/src/cli/add.rs:20` and `cli/remove.rs:14` import
  `crate::server::lock::ServerLock` across an undeclared subsystem boundary. `ServerLock`
  is a process-level primitive, not server plumbing — move it to a top-level module
  (`src/lock.rs`) with its own `scope!`, consumed by both `cli/` and `server/`.
- **0.7 Two cheap gate flips**: conform-frontend-rust and env-audit enter
  `CONFORM_GATED`. Verified during the audit: frontend production code carries no
  `unwrap()`/`expect()` findings (its 21 grep hits are `unwrap_or` combinators and test
  code), env-audit's 2 are in tests. Drain is ~zero; flip lands with **zero frozen
  entries** (the v0.2 invariant: a flip never widens the baseline).

*Exit:* specmap warnings = 0; `CONFORM_GATED` = 12 with a reasons table for the rest;
zero new frozen findings; panel green.
*Prediction:* no test expectations change anywhere in Phase 0; both flips freeze nothing.

## 2. Phase 1 — vibe-core armor: the foundation pass (three to four sittings)

vibe-core is what every other crate consumes; every Phase after this one builds on its
types. Today its hot surfaces are stringly-typed (`PackageRef.name: String`,
`CapabilityRef.namespace/name: String`, lockfile `source_url`/`content_hash`/`trace_id`
as bare `String`), its invariants live in comments, and it carries ~zero compiled
doctests on 7k LOC. This is the Scaffold-B and Scaffold-C debt of the whole workspace
concentrated in one crate (guide §3, cards `scaffold-b-typed-builders`,
`scaffold-c-runnable-contracts`).

- **1.1 Newtypes at the seams** (Scaffold B): `PackageName`, `CapabilityNamespace`,
  `CapabilityName`, `SourceUrl`, `ContentHash`, `TraceId`, and `RelPath` (consumed by
  vibe-workspace's `WorkspaceMember.rel_path`). Each: parse-validating constructor,
  `serde(transparent)`, `Display`, one doctest showing the canonical construction and
  the rejection. Wire format and lockfile bytes do not change — these are in-memory
  contracts. Migration cascades through consumers in per-crate batches (registry,
  resolver, workspace, publish, install, index, cli) — mechanical per batch, compiler-led.
- **1.2 Witnessed invariants** (Scaffold C): the invariants currently documented in
  prose get runnable witnesses at use sites — `project ⊕ package` XOR on `Manifest`,
  kebab-case package names, group-segment grammar. `debug_assert!` where relied upon,
  a doctest where claimed (R3-009: redundancy is ground truth for a paged reader).
- **1.3 Doctests on every public seam** (Scaffold G): `PackageRef`, `Group`,
  `CapabilityRef`, `Manifest` (parse → validate → read), `Lockfile` (load → entry →
  save), `UserConfig` — ≥8 compiled examples of canonical use. A doctest that lies
  fails the gate; a comment that lies ships (H4) — this converts vibe-core's prose
  capital into runnable capital.
- **1.4 The `pub-doctest` rule** (new, Class G, ratcheted): frontend v6 emits
  `PubItem` facts with a `has_doc_example` flag; the rule — every top-level `pub` item
  in a gated-G crate carries a compiled doc example or a `#[spec(documents)]` edge —
  activates **on vibe-core only** at first (the foundation earns the strictest
  envelope). Pre-existing misses freeze into the baseline, shrink-only from there;
  1.3 drains the freeze in the same phase.
- **1.5 `#[spec]` edges**: vibe-core's 6 in-source edges grow to cover the key types —
  `PROP-008#pkgref` on `PackageRef`/`Group`, `VIBEVM-SPEC#lockfile-schema` on
  `Lockfile`, `PROP-002#capability` on `CapabilityRef` — so the retrieval index
  (guide §7, R3-012) reaches the foundation items a reader pages in first.

*Exit:* `pub-doctest` active on vibe-core with 0 new findings and a drained (empty or
single-digit frozen) ratchet; all consumer crates compile and their suites pass; panel
green.
*Prediction:* the newtype cascade surfaces **≥1 latent misuse** at a boundary (the
SHRINK-v0.1 precedent: two "invariants" that weren't, plus the `VersionReq` build-metadata
panic — typed constructors find what review didn't); the cascade touches 60–80 files
but no batch requires judgment beyond the compiler's error list.

## 3. Phase 2 — declare the surfaces: workspace, publish, install (two sittings)

The stratum-B crates pass every gate because they declare nothing the gates can see.
This phase declares what already exists — it is mostly attribution, not redesign.

- **2.1 vibe-publish cells** — the one stratum-B crate with *real variance*:
  `GitHubCreator` (`github.rs`), `GitVerseCreator` (`gitverse.rs`), `DirectGitCreator`
  (`direct_git.rs`) become three `#[cell(seam = "RepoCreator", variant = …)]` manifests;
  `creator.rs` carries the seam doctest (canonical `&dyn RepoCreator` use);
  `creator_for_url` selection already sits at one match — R-001 shape confirmed, tag it.
  A seam-driving oracle test exercises all three variants through the trait object
  (`cell-has-oracle` then gates them). The token-redaction invariant — today a comment
  at `token.rs:17–19` backed by six unit tests — gets the `#[spec(verifies)]` edge tying
  tests to `PROP-000#token-secrecy` so the claim is machine-anchored.
- **2.2 vibe-publish doctests**: `extract_host_segment` (`lib.rs:255`) and
  `creator_for_url` (`lib.rs:305`) — the two public functions the audit found bare —
  plus a contract-first doc block on `Publisher::publish` (signature → invariants →
  error contract → example; R3-002 ordering).
- **2.3 vibe-workspace, honestly cell-free**: workspace has no variance — no fake cells
  (§10). Instead, full Scaffold C+G: doctests on `Workspace::discover`,
  `Workspace::load`, `compute_effective_boot`, `vibedeps_slot`; witnessed ordering
  invariants in `boot.rs` ("an authored boot file is always static",
  "inherited foundation precedes own", "`i` requires `j` → `j` precedes `i`" — each
  currently a comment, each becomes a `debug_assert!` at the sort/merge sites plus one
  doctest asserting observable order); `RelPath` newtype from 1.1 lands here.
- **2.4 vibe-install rounding**: born conforming in v0.2 but metadata-thin (2 edges).
  `#[spec]` edges on `plan`/`apply`/`InstallSource`/`PlanEvent` (≥6 total), one
  doctest on the `InstallSource` seam and one on `PlanEvent` consumption.

*Exit:* cells 20 → 23, `cell-has-oracle` and `seam-has-doctest` green on the expanded
set; fast-loop 23/23 < 60s; panel green.
*Prediction:* the publish oracle needs **one new test file** (the existing creator tests
drive concrete types, not the seam); ≥1 of the boot-ordering doctests catches its prose
overstating the code (the H4 lying-prose hazard made falsifiable).

## 4. Phase 3 — vibe-index to full depth (two to three sittings)

Outside scanner/, vibe-index is gated-green with undeclared interiors: server subsystems
coupled concretely, an index API without examples, a 15-struct type file.

- **3.1 Server seams + fakes** (Scaffold H): traits for the server's swappable
  dependencies — `TokenStore`, `RateLimiter` (and the auth check if it is separable) —
  with in-memory fakes; `AppState` holds seam objects, handlers consume seams. The
  existing e2e suites (`server_e2e.rs`, `rate_limit_e2e.rs`) stay as the behavioral
  oracles; new unit tests drive handlers through fakes. No `#[cell]` here unless a
  second production variant actually exists (§10) — the seam + fake is the deliverable.
- **3.2 RateLimiter runnable model**: the window/refill behavior as a small stepping
  model with a doctest (the CRUXEval argument: execution prediction is where weak
  readers are weakest; give them a model to run, not a paragraph to simulate).
- **3.3 `types/entry.rs` split**: 15 nested structs in 499 lines become a module family
  (`types/entry/{mod,compat,provides,features,subskills,…}.rs`), every child carrying
  the parent's `scope!` URI — the Phase-4 SHRINK recipe applied at type grain.
- **3.4 Index API doctests**: `upsert`, `remove_version`, `get`, `load_from`, `save` —
  the in-memory index is the crate's primary seam and currently teaches by signature
  alone; ≥6 compiled examples.
- **3.5 `ApiError` Class-F tails**: the RFC-7807 responses keep their wire shape; the
  human-facing `detail` strings gain the `(violates spec://…; fix: …)` tail where a
  spec unit exists (PROP-005 serve sections), matching the product-error grammar fixed
  in SHRINK-v0.1 §4.

*Exit:* server handlers unit-testable through fakes; entry family ≤150 lines per file;
index seam documented by example; panel green.
*Prediction:* zero behavioral diffs — the e2e suites pass unmodified (the fakes are
*additive* test surface, not replacements); the entry split moves no test expectations.

## 5. Phase 4 — vibe-cli: the facade diet (four to five sittings)

The structural fact that drives this phase: vibe-cli is a pure binary (no `[lib]`
target) — **rustdoc doctests cannot even run there**. The discipline's doctest and seam
obligations are not waived for the CLI; they are *unmeetable inside it*. The only path
to full depth is the one the audit named: domain moves to gated lib crates where the
envelope works; the CLI keeps argument parsing, dispatch, confirmation dialogs, and
rendering. Stratum-A precedent: v0.2 already did exactly this for install
(`vibe-install` extraction) — this phase finishes the job for the remaining domain.

- **4.1 Search domain → vibe-registry**: `commands/search_full_scan.rs` (560 lines:
  GitHub org walk, repo scoring) and `commands/search_cache.rs` (385 lines: TTL cache,
  key derivation) plus the scoring half of `commands/search.rs` move into a
  `vibe-registry::search` module family beside `index_client.rs` (the index-consuming
  client side already lives there; the full-scan fallback is its degraded mode —
  one home, not two). Born gated: `scope!` + `#[spec]` onto `PROP-005`'s search/serve
  units and `PROP-002`'s registry-model units, errors born in Class-F grammar
  (`FullScanError` moves and converts), doctests on the public query API. The CLI
  keeps a thin `run()` per subcommand.
- **4.2 Vendor + redirect-sync domain → vibe-registry**: `commands/registry/vendor.rs`
  (474 lines: mirror generation, clone orchestration) and the tag-sync half of
  `commands/registry/redirect/sync.rs` (310 lines) follow the same route into
  `vibe-registry` (vendor beside the cache/mirror machinery it drives; sync beside
  `redirect_follow`).
- **4.3 Manifest mutation → vibe-install**: `commands/install/resolver.rs`'s
  `apply_git_source_flag` writes manifest state from the CLI. It becomes a typed
  entry on the orchestrator (`InstallRequest::with_git_source(…)` or equivalent),
  carrying its `#[spec]` edge; the CLI translates args to the typed request and
  nothing else.
- **4.4 Templates become data**: `commands/init.rs`'s ~300 lines of embedded template
  strings move to `crates/vibe-cli/templates/*` consumed via `include_str!` (or into
  vibe-workspace if a second consumer appears — decide at the joint, not before).
  Code renders; data carries content.
- **4.5 Class-F on the CLI's own errors**: `exit_code.rs::InstallError` and every
  remaining thiserror enum left in the binary after 4.1–4.3 adopt the grammar
  («human text (violates spec://…; fix: hint)») — the CLI is the user's first reader;
  its errors are agent food too (R3-011).
- **4.6 The gate flip**: with the domain out, `CONFORM_GATED` += vibe-cli. Drain first
  (the audit expects the drain to be small: unwrap-wise the binary is already clean —
  the 945 grep hits resolved to test code and combinators on inspection, with 0.5's
  two expects already gone), then flip with zero frozen entries.

*Exit:* every `commands/*` file ≤ ~300 lines of dispatch/render; the moved domain lives
in gated crates with edges, grammar, and doctests; `CONFORM_GATED` = 13; `vibe --help`
surface and every e2e transcript byte-identical except where messages gained Class-F
tails; panel green.
*Predictions:* the search/vendor/sync extraction changes **zero e2e expectations**
beyond message tails (behavior moves, it does not change); ≥2 `unwrap`s in the moving
code convert to honest error variants during the drain (lib context is stricter than
binary context); vibe-cli loses 20–30% of its LOC to the moves.

## 6. Phase 5 — the toolchain under its own law (one to two sittings)

A checker the discipline's own code ignores is adversarial few-shot signal — worse, it
is the *first* code a contributor to the discipline reads.

- **5.1 specmark + specmark-grammar gate flip**: audit-verified — specmark-grammar's
  10 unwraps all sit below `#[cfg(test)]` (first at `lib.rs:424`, test mod at 415),
  specmark's are zero. The specmap-ratchet exemption (the `scope!` bootstrap cycle —
  the tag pair cannot tag itself) **does not extend** to the unwrap ban or file budget,
  which need no specmark dependency; the pair joins `CONFORM_GATED` at zero drain.
  The error-grammar rules hold vacuously (no thiserror enums there today) and start
  guarding the day one appears. The `scope!` cycle stays recorded in
  `specmap-ratchet.json` — that half of the exemption is real and keeps its note.
- **5.2 The `ambient-env` rule** (new, the R-001 projection, ratcheted): frontend v6
  emits `EnvRead` facts (`env::var`/`var_os`/`env::set_var` outside `#[cfg(test)]`);
  the rule fires on gated crates for reads outside the recorded composition roots
  (`vibe-cli/src/main.rs`, vibe-index's reindex root) and outside env-audit (the
  designated mutation crate from v0.2). Escape hatch as always: fn-grain
  `#[spec(deviates, reason)]`. Phase 0.4 pre-cleaned the two known violations;
  the freeze should be near-empty.
- **5.3 xtask stays exempt, on the record**: internal tooling, panics acceptable at
  the developer's own console — but the reason line lands in the Phase-0 exemption
  table, and its own messages already speak Class-F (audit-verified), so the exemption
  costs nothing.

*Exit:* `CONFORM_GATED` = 15; `ambient-env` active with a near-empty frozen set;
every exemption everywhere carries a recorded reason; panel green.
*Prediction:* both flips freeze zero; `ambient-env`'s initial scan finds **≤6** sites
beyond the two already fixed, and every one either moves to a root or testifies — none
needs a rule change.

## 7. Phase 6 — the spec layer truth pass (one sitting)

The audit found the spec corpus 94% traceable with healthy density where it matters
(PROP-002 at 3.4 edges/unit, PROP-005 at 2.0, PROP-003 at 1.5). The debt is
classification honesty, not coverage: REQ-shaped units that nothing implements should
either gain their edge or stop claiming REQ-hood.

- **6.1 PROP-000 kind audit**: 24 units, 2 incoming edges. Mark each section honestly —
  `req` where code must comply (then add the missing edge or file the debt),
  informative where it is rationale. Expect most of the foundation document to be
  policy/rationale, which is fine *once it says so*.
- **6.2 PROP-011 edges**: 14 units, 2 edges — but the freshness fast path and
  held-pin fallback *are implemented* (vibe-install plan(); vibe-workspace freshness).
  Add the `implements` edges the code already earns; `deviates`-mark the §5.3 churn
  items that are genuinely deferred.
- **6.3 PROP-006 / PROP-013 / LEDGER-INTENT / BROWNFIELD**: process documents — mark
  informative (no code edges expected, none missing). **No file moves**: PROP-006 is
  addressed from boot files; relocation breaks anchors for zero truth gained.
- **6.4 PROP-010 deferral header**: `[DEFERRED — M2 scope]` at the top, so 18 unitized
  sections stop reading as unimplemented requirements.

*Exit:* zero REQ-marked units without an edge or a filed debt across
`spec/common/` and `spec/modules/`; specmap regenerated; panel green.
*Prediction:* ≥70% of PROP-000's units classify as informative; PROP-011 gains ≥4
implements edges without writing a line of code.

## 8. Phase 7 — the MCP endgame (OWNER-GATED behind DBT-0020)

**Precondition, restated from the owner's 2026-06-12 instruction: this phase does not
start until the owner opens the MCP spec home.** Everything below is specified now so
the gate-opening decision is the only remaining input.

- **7.1 The spec home**: `spec/modules/vibe-mcp/PROP-0xx` — units for the tool surface
  (query/read/materialise), the serve loop, agent detection, per-agent config formats,
  skill materialisation. Spec-first; owner signs the design.
- **7.2 vibe-mcp conversion**: `tools.rs` (720 lines, three independent tools sharing
  one contract — describe + run) becomes a real seam: `McpTool` trait + three
  `#[cell(seam = "McpTool", variant = …)]` cells with one registration point;
  `ToolError`/`ServerError` adopt Class-F with edges into PROP-0xx; `scope!` lands
  per file; seam doctest + per-cell oracles through the existing tool tests.
- **7.3 vibe-cli/commands/mcp.rs drained**: the 2638-line god-file's domain — agent
  detection, config writers (JSON/TOML per agent), skill template generation — moves
  into vibe-mcp behind typed surfaces; templates become data (the 4.4 pattern);
  the CLI keeps dispatch + rendering, under the 600 budget.
- **7.4 The ledgers close**: vibe-mcp exits `specmap-ratchet.json` exempt and enters
  `CONFORM_GATED` (drain-then-flip); both `file-length` baseline entries drain;
  the 10 DBT-0020 dispositioned orphans resolve.

*Exit:* conform baseline **empty**; specmap dispositioned 0, exempt 6; `CONFORM_GATED`
= 16; cells 26; the panel green with nothing parked anywhere.
*Prediction:* mcp.rs decomposes into ≥6 modules along the Scope/What/Agent axes already
visible in its enums; its comments yield ≥3 lying-prose findings (it is the oldest
unreviewed surface in the tree).

## 9. Order, sizing, cadence

**0 → 1 → 2 → 3 → 4 → 5 → 6**, Phase 7 whenever the owner opens it (it depends only on
Phase 0's hygiene, not on 1–6).

The order is load-bearing: 1 before 2–4 so the newtypes exist before surfaces are
declared around them and before domain moves consume them (each extraction in Phase 4
lands on typed seams, not on `String`); 2–3 before 4 so the moved domain arrives into
crates that already model the discipline; 5 after 4 so `ambient-env` scans the
post-move tree; 6 last because it follows the code edges that 1–5 mint.

Estimated sittings: 0:1 · 1:3–4 · 2:2 · 3:2–3 · 4:4–5 · 5:1–2 · 6:1 — **~15–18 gated
batches** in the main plan, plus 3–4 in the owner-gated Phase 7. Every batch ends panel-
green; the baseline only ever shrinks; any batch is a checkpoint the WAL can record.

## 10. What this plan deliberately does NOT do

- **Does NOT touch DBT-0020 before the owner does** — Phase 7 is fully specified and
  fully locked; the two MCP files and their 10 dispositioned orphans park exactly as
  the 2026-06-12 instruction left them until then.
- **Does NOT stamp `#[cell]` where there is no variance.** vibe-workspace's subsystems
  and vibe-index's server internals get seams, doctests, witnesses — not decorative
  cell manifests. A cell without a second variant or a replacement story is uniformity
  theater, and uniformity theater is exactly the false few-shot signal the discipline
  exists to prevent (R3-006).
- **Does NOT invent checker-less rules.** Every new obligation this plan adds ships as
  a ratcheted conform rule with frontend facts (`pub-doctest`, `ambient-env`) or it is
  not added (A5). Prose-only obligations remain WISHes and are not pretended otherwise.
- **Does NOT change wire formats or CLI behavior.** Newtypes are `serde(transparent)`;
  lockfile bytes, manifest schema, exit codes, and e2e transcripts stay fixed except
  for Class-F message tails.
- **Does NOT implement vibe-graph / vibe-llm** (M1.5 scope; they get status headers
  only), does NOT reform the WAL's format (process surface, owner's), does NOT touch
  CI/signing (none exists, by owner decision), and does NOT edit `VIBEVM-SPEC.md`
  (owner-frozen).
- **Does NOT relocate spec documents** — classification over relocation (Phase 6.3);
  anchors are load-bearing.
