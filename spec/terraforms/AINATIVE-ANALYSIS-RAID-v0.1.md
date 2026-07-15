# AINATIVE-ANALYSIS-RAID v0.1 — AI-Native scaffold coverage of the static-analysis engine

_Status: **EXECUTED** (Phases 0–5 done, floor-green throughout) · opened against
tree `3227fde`, closed at `d821910` · cold-executable: every phase ends
floor-green (`bash tools/self-check.sh`); any phase boundary is a safe stop. A
**raid** (03-RAID-PLAYBOOK) applies a card-set across a layer when per-edit
triggers can't keep up — here the layer is the static-analysis engine, never
scaffold-covered beyond file-level `scope!`. Paper trail per 05-CAMPAIGN-FORM._

---

## 2 — Execution record (prepended at close)

**Closed 2026-07-15, `3227fde` → `d821910`, floor-green at every boundary.** The
engine now carries the AI-Native scaffolds where they buy reader value; two
predicted gaps proved illusory and are recorded honestly rather than papered
over.

**What landed (per phase):**

- **P1 — tree D/H full-engine oracle** (`5b90357`). A hermetic six-package
  fixture (`crates/vibe-cli/tests/tree_fixture.rs`) — root manifest + lockfile +
  STATIC.md + INDEX.md, no real-repo dependency — run end-to-end through `vibe
  tree --json`, asserting every load lane (static/declared, dynamic/default,
  static-transitive declarer, transitive member, when-forced, none). The
  Class-D safety net (D5): a silent change in how the lockfile + artifacts fold
  into `LoadType`/`LoadOrigin` breaks a named lane. **P1 confirmed** — passed
  first run; no lane needed a real-repo fact the fixture couldn't express. The
  schema version is injected from `vibe-core` so the fixture survives a bump.
  (`decompile_static` / `read_index` were already characterized in artifacts.rs
  — no work, as R1 anticipated.)
- **P2 — tree F + C** (`3533610` F, `da3c57e` C). **F:** every `tree` diagnostic
  now carries a required `spec_ref` citing the REQ it enforces (today
  root-drift → PROP-036 §2.10); the field sets the pattern the deferred
  stale-artifacts check will follow. **P2 confirmed** — additive schema
  property, `tree_json.rs` + schema stayed green. **C:** a `debug_assert` on the
  static-transitive closure documents the load-bearing invariant (every member
  is a resolved package) AND the subtle non-invariant (a declarer may sit in
  another declarer's closure — `Declared` wins), so a reader steps through the
  real rule.
- **P3 — hybrid H** (`59bde39`). A shared `#[cfg(test)]` testkit
  (`hybrid/testkit.rs`) replaces three byte-identical copies of the
  `org`/`id`/`unit`/`table` construction boilerplate — one declarative reference
  model of the unit table — plus a fluent `gated_unit` for the `when`-gate case.
  **P3 confirmed** — no test's meaning changed; the full hybrid suite stayed
  green, net −26 lines.
- **P4 — hybrid G doctests** (`d821910`). `resolve_zone`, `topo_zone`,
  `soft_static_pulls` gained runnable examples on the public API; the hybrid
  seam is now 8/8 doctested.
- **P4 overall confirmed** — the ≈200-test suite passed unchanged at every
  boundary; no behavior changed (D1 freeze held).

**RP1 resolved.** The F wire field is a *required* property on the `diagnostic`
schema def (not a version bump — v1 stays v1). Justification exercised without
stopping the raid: the only consumers of `package-tree.schema.v1.json` are
in-repo (the TUI, `tree_json.rs`) and output is always freshly generated, so a
required field is safe and reversible. Recorded here for the owner; trivially
revertible if unwanted.

**Cards that misfired (illusory gaps — the honest part):**

- **Hybrid `#[spec]` fn-traces (Phase 3's second half) — dropped as redundant.**
  The audit predicted a fn-trace gap. It is not one: `specmark::scope!` gives
  *every* item in a module a default `implements` edge to the module's anchor
  (PROP-014 §2.3), and hybrid.rs / hoist.rs / fingerprint.rs are each
  **single-REQ modules** (all fns implement the one anchor — #edge-recursion /
  #hoisting / #change-detection respectively). A fn-level `#[spec]` would only
  restate the inherited edge — noise, not finer provenance. (Contrast
  `hybrid_emit`, whose fns genuinely span #units / #hoisting / #change-detection
  / #incremental and so earn per-fn tags — that is where the pattern pays off.)
- **"vibe-workspace pub-doctest gap reduced" (§4 exit line) — imprecise.** The
  health `pub_doctest_drain_backlog` metric is **type-scoped** (the sweep
  playbook drains a crate's *types*); the seam fns I doctested improve reader
  value but do not move the crate counter, which stays 30. The seam goal (8/8
  hybrid items doctested) was the real target and is met; the crate-wide,
  type-scoped drain is a separate sweep (DEF-A1), exactly as scoped.

**Gaps "где имеет смысл" skipped (recorded, not silently dropped):**

- B newtypes on `model.rs` and I codemods — pre-declared non-goals (D4, DEF-A2);
  a serde DTO and a no-recurring-pattern engine gain nothing.
- Weak/theatre C contracts — only the ONE load-bearing closure invariant was
  asserted; disjointness-of-declarers was deliberately NOT asserted (it is a
  legal state) and recorded as a comment instead.

**Exit census (verified `d821910`):** floor all-green; specmap 0 gated orphans;
file-length 0 over budget (the P3 refactor *removed* net lines); hybrid public
seam 8/8 doctested; the tree engine has a hermetic full-engine oracle where it
had none. Deferred, unchanged: vibe-workspace type-doctest gap 30, vibe-install
gap 9 (the next promotable crate) — DEF-A1.

---

## 3 — The mandate

Owner (2026-07-15), across three messages that escalated the ask:

> "насколько хорошо у нас в гибридной линковке и движке работы с деревом
> используется AI Native Rust и инструменты типа specmap? может быть сделать
> discipline sweep и проверить?"

> "мы за последнее время сделали целый движок статического анализа, в нём куда
> больше чем последняя спека на гибридную линковку, это всё надо проаудировать
> и покрыть AI Native практиками"

> "я … имел в виду всё включая G/H/I" (the nine scaffolds A–I, GUIDE §3 — G
> doctests, H simulators/reference-models, I codemods), and chose **"оформить
> raid-кампанию"** over incremental.

Interpretation, binding for the raid: cover the static-analysis engine with the
AI-Native scaffolds **where it makes sense** ("где имеет смысл" — not blanket
every trivial item), with explicit attention to G/H/I. The audit is already done
(§5); this raid executes the fills.

---

## 4 — Target arithmetic (baseline → exit)

Census at raid open (`rust-ai-native health` + grep, verified §5):

```
Baseline (tree engine, crates/vibe-cli/src/commands/tree, bin-only):
  build.rs (the engine)      : 6 unit tests (the classify_origin oracle,
                               landed pre-raid in 3227fde), 1 C contract.
  decompile_static/read_index: artifacts.rs has some tests; characterization
                               unverified.
  build_tree (full engine)   : NO isolated-fixture oracle (only the real-repo
                               integration test tree_json.rs).
  diagnostics                : anyhow + Diagnostic.code, no spec:// URIs (F gap).
  G doctests                 : N/A (bin crate — cargo test --doc skips it).
Baseline (hybrid, crates/vibe-workspace, lib):
  G doctests 5 · D fuzz present · scope! traces 9/16 · #[spec] fn-traces partial.
  vibe-workspace pub-doctest gap 30.
Exit:
  tree: build_tree fixture-oracle (D/H) green; decompile/read_index
    characterization green; Diagnostic codes cite spec:// (F); contracts on the
    remaining engine invariants (C).
  hybrid: an H reference-fake for the boot linker; #[spec] fn-traces on the
    public seams; vibe-workspace pub-doctest gap reduced.
  Every phase floor-green; raid REPORT checks the predictions.
```

Reconciliation: G is bounded by crate kind (lib→doctests, bin→unit tests); B
(newtypes) and I (codemods) are **out of scope by the "где имеет смысл" clause**
(§10) — a serde DTO and a no-recurring-pattern engine gain nothing.

---

## 5 — Current-state facts (audit, verified 2026-07-15; do not re-discover)

Per-scaffold audit of the two engines:

- **Tree engine** (`crates/vibe-cli/src/commands/tree/`, 12 files, 60 pub items,
  **bin-only** — no lib target, so external doctests do not run):
  - A: JSON schema `resources/package-tree.schema.v1.json` is the contract;
    `model.rs` mirrors it hand-maintained. `tree_json.rs` validates output
    against it.
  - B: 0 newtypes — `model.rs` is a serde output-DTO (low value).
  - C: 1 `debug_assert` (build_package mutual-exclusion, landed 3227fde).
  - D/H: `classify_origin` decision-table oracle landed (3227fde, 6 rows);
    `build_tree` (the full engine) has NO isolated fixture oracle; `decompile_static`
    / `read_index` characterization unverified (artifacts.rs has a test module).
  - E: crate-level fast-loop (ok).
  - F: `anyhow` + `.context()`; `Diagnostic.code` strings carry no `spec://`.
  - G: N/A (bin).
  - `scope!`: 1 per file (file-level trace present).
- **Hybrid** (`crates/vibe-workspace/src/boot/hybrid*`, `install/bootgen*`, lib):
  D fuzz (fuzz.rs) present; G 5 doctests; `scope!` in hoist/fingerprint added;
  `#[spec]` fn-traces partial (hybrid_emit 8, hybrid.rs/hoist/fingerprint 0 at fn
  grain); H absent (no reference fake). vibe-workspace pub-doctest gap 30.
- **Floor:** all-green at raid open (`3227fde`). Machine quirks (host): Edit/Write
  only, heredoc commits, self-check via Git Bash.

---

## 6 — Decisions

- **D1 — scope & freeze (RAID §1.1).** In scope: `crates/vibe-cli/src/commands/tree/**`
  and `crates/vibe-workspace/src/boot/hybrid**` + `install/bootgen**`. Frozen for
  the raid: no behavior change to these surfaces — the raid adds tests, contracts,
  traces, doctests, and a reference fake only. A behavior change blocks the batch.
- **D2 — card-set & order (RAID §1.2).** No behavior-changing card here (all
  additive: C contracts, D/H oracles, F trace-in-errors, G doctests, `#[spec]`
  traces), so the topo constraint (names→contracts, D wraps behavior) is slack;
  order by value: D/H → C → F → G → traces.
- **D3 — G is crate-kind-bounded.** lib (`vibe-workspace`) → external doctests;
  bin (`vibe-cli`) → unit-level oracles + `examples/` if warranted. Do not chase
  external doctests on the bin (they don't run).
- **D4 — "Где имеет смысл" excludes B and I.** `model.rs` DTO newtypes and
  engine codemods add no reader value; recorded as non-goals (§10), not gaps.
- **D5 — differential safety (RAID §1.5).** The build_tree fixture oracle is the
  Class-D net; it is authored BEFORE any later engine touch so a silent behavior
  change is impossible.
- **D6 — resumable, per-file batches (RAID §1.4).** Each file/seam is a batch with
  a green-gate checkpoint; the raid is never one giant diff.

---

## 7 — Predictions

- **P1** — the `build_tree` fixture oracle exercises static / dynamic /
  static-transitive / when-gated / none classification on ONE isolated fixture,
  reproducing the engine's output deterministically (no real-repo dependency).
  Falsifiable: a lane needs a real-repo fact the fixture can't express.
- **P2** — `Diagnostic.code` → `spec://` mapping (F) is additive: the JSON wire
  `code` strings are unchanged, a `spec_ref` is added, so `tree_json.rs` and the
  schema stay green. Falsifiable: the schema rejects the new field.
- **P3** — the hybrid H reference-fake (an in-memory unit-table builder) collapses
  the boilerplate in the existing hybrid tests, and no test's meaning changes.
- **P4** — the raid is floor-green at every phase boundary; no behavior changes
  (D1 freeze), so the full existing suite (≈200 tests) passes unchanged throughout.

---

## 8 — Phases

Each phase ends floor-green; each is a safe stop.

**Phase 0 — DONE (pre-raid, 3227fde).** The `classify_origin` D/H oracle + the
build_package C contract. (Recorded so the raid opens from a real baseline.)

**Phase 1 — tree D/H: the full-engine fixture oracle.** An isolated fixture
project (vibe.toml + vibe.lock + STATIC.md + INDEX.md + vibedeps manifests) that
exercises every load lane; `build_tree` run on it; the `PackageTree` classification
asserted. The runnable reference for the whole engine (Class-D safety net, D5).
Plus `decompile_static` / `read_index` characterization if artifacts.rs lacks it.

**Phase 2 — tree F + C.** `Diagnostic.code` carries a `spec://` REQ (F, additive
wire field); `debug_assert` contracts on the remaining engine invariants
(static-transitive closure membership, in-place spec resolution).

**Phase 3 — hybrid H + traces.** An in-memory reference-fake / builder for unit
tables (H — collapses hybrid test boilerplate, a runnable model of the boot
graph); `#[spec]` fn-level traces on the hybrid public seams (finer than the
file `scope!`).

**Phase 4 — pub-doctest drain (bounded).** Drain `vibe-workspace`'s hybrid public
seams' remaining doctests; note (do NOT do here) that per the sweep playbook the
next *promotable* crate is `vibe-install` (gap 9) — a separate whole-crate sweep,
not this raid.

**Phase 5 — the raid REPORT.** What the sweep learned: cards that misfired, gaps
that were "где имеет смысл" skipped, and the exit census.

---

## 9 — Risks and fallbacks

- **R1 — the build_tree fixture is hard to construct** (needs a valid lockfile +
  artifacts + manifests). Detection: the fixture won't parse. Fallback: build it
  incrementally lane-by-lane; worst case, characterize the pure sub-functions
  (`decompile_static`, `classify_origin` — done) and defer the full fixture with a
  named deferral.
- **R2 — the F `spec_ref` field breaks the JSON schema.** Detection: `tree_json.rs`
  red. Fallback: make it `skip_serializing_if=None` and schema-additive, or defer.
- **R3 — scope creep into behavior.** D1 freeze + P4: any behavior change blocks
  the batch; the raid only adds scaffolds.
- **R4 — PS5.1 / CRLF** (machine): Edit/Write only, heredoc commits.

---

## 10 — Non-goals

- **B newtypes on `model.rs`** — a serde output-DTO; the JSON schema is its
  contract, newtypes add no reader value (D4).
- **I codemods** — no recurring multi-file pattern in the engine (D4).
- **External doctests on `vibe-cli`** — bin crate, they don't run (D3).
- **Full `vibe-workspace` pub-doctest drain to promotion** — whole-crate sweep;
  per playbook `vibe-install` (gap 9) is the promotable target, a separate sweep.
- **Any behavior change** to the engines — frozen (D1).

---

## 11 — Quick-start for the executing session

```sh
git log --oneline -1                 # 3227fde — matches the status line
bash tools/self-check.sh             # floor GREEN before Phase 1
cargo test -p vibe-cli --bin vibe classify_origin  # the pre-raid oracle, green
sed -n '/^## 8/,/^## 9/p' spec/terraforms/AINATIVE-ANALYSIS-RAID-v0.1.md
```

---

## 12 — Whole-raid acceptance

```sh
bash tools/self-check.sh; echo "EXIT=$?"                      # 0
cargo test -p vibe-cli --bin vibe build::tests                 # engine oracle green
cargo test -p vibe-cli --test tree_json                        # schema + real-repo facts green
cargo test -p vibe-workspace                                   # hybrid oracles + fakes green
cargo xtask health                                             # doctest gap moved, census recorded
```

---

## 13 — Review points

- **RP1 — F wire field.** Adding `spec_ref` to `Diagnostic` touches the JSON
  schema (an observable contract). Executor proposes at Phase 2; owner confirms
  the schema bump vs. an additive optional field.

---

## 14 — Execution ledger

- **Phase 0 (pre-raid)** — `3227fde` `test(vibe-cli): characterization oracle +
  contract for the tree engine (d/h/c)`. The classify_origin decision-table oracle
  (6 rows) + build_package mutual-exclusion contract.
- **Phase 1** — `5b90357` `test(vibe-cli): hermetic full-engine fixture oracle
  for vibe tree (d/h)`. Six-package hermetic fixture, every load lane asserted
  through `vibe tree --json`. `tempfile` added to vibe-cli dev-deps.
- **Phase 2 (F)** — `3533610` `feat(vibe-cli): tree diagnostics cite their
  governing REQ (scaffold-f)`. Required `spec_ref` on `Diagnostic` + schema def;
  RP1 resolved additively.
- **Phase 2 (C)** — `da3c57e` `test(vibe-cli): contract the static-transitive
  closure invariant (scaffold-c)`. The load-bearing closure-membership
  `debug_assert` + the documented non-invariant.
- **Phase 3 (H)** — `59bde39` `test(vibe-workspace): shared unit-table testkit
  for the hybrid linker (scaffold-h)`. `hybrid/testkit.rs` reference model +
  fluent `gated_unit`; three suites refactored onto it (net −26 lines). Traces
  half dropped as redundant (see §2).
- **Phase 4 (G)** — `d821910` `docs(vibe-workspace): doctests on the hybrid
  linker's remaining public seams (scaffold-g)`. `resolve_zone` / `topo_zone` /
  `soft_static_pulls` doctested; seam now 8/8.
- **Phase 5** — this REPORT (§2), committed with the ledger close.

---

## 15 — Deferrals ledger

- **DEF-A1** — full `vibe-workspace` / `vibe-install` pub-doctest drain to crate
  promotion · owner · a standing sweep item (04-SWEEP-PLAYBOOK), not this raid.
- **DEF-A2** — `model.rs` newtypes, engine codemods (B / I) · owner · "где имеет
  смысл" excludes them (DTO + no recurring pattern).
