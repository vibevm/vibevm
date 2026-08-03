# E12-B035-PARITY — parity-audit loop, pass after batch 2

Read-only parity audit for `BACKLOG.md {#b-035}` (owner principle, now
discipline law: manifesto `##PARITY-ACROSS-PROJECTIONS` — “no language
projection enforces the discipline more weakly than another; a gap carries a
recorded reason, never silent”). This pass re-cuts the table **by the fact of
the tree** after wave-B batch 2 landed. The previous cut is pass №1
`harvest/e10-b035-parity-pass.md`; only the CHANGED rows are re-stated here,
the rest hold at pass №1.

**Headline.** Batch 2 closed the **two content asymmetries** pass №1 named as
the real parity debt (row 1 seam-error REQ-citation, row 7
conformance-assertion) and **the one inversion** (row 13 floor-disable, where
Rust was the weaker floor). The parity principle itself is now a manifesto law
the three guides cite. **M-PARITY is NOT yet reached** — two Go-only gaps
survive (row 6 the Go flag/registry rule; rows 8/12 the Go
`vet`/`tests`/`staticcheck` `./...` residual, B-048's Go analogue), both
straight builds routed to a later batch, both recorded not silent.

## Rows that CHANGED against pass №1

| # | Mechanism | Rust | TypeScript | Go | Delta vs pass №1 | Verdict |
|---|---|---|---|---|---|---|
| 1 | REQ-citation of seam errors (rule + both halves) | **yes (two rules)** — `error-enum-cites-req` + `error-message-cites-req` (`diagnostics.rs`) | **yes (NEW)** — `ts-seam-error-cites-req` (`rules/typescript_parity.rs`) flags a discriminated-union error alias `E` with no `spec://` REQ; mounted `typescript-ai-native-conform/src/lib.rs`. Honest limits recorded (Form-1 union, name-based error position, closed `{kind,tag,_tag}` discriminant) | **yes (NEW, both halves)** — `go-seam-error-cites-req` (`rules/go_parity.rs`), one rule, per-half fingerprints; structure half (no `Spec` field) + message half (`Error()` renders no `spec://`/`violates REQ`, anchored at the method line); mounted `go-ai-native-conform/src/lib.rs`; the `seam_error_missing_req` arm removed from `GoUnsafeInDomain` | **CLOSED.** Pass №1 verdict was “build TS twin + Go message half” — both built (B-033). Rust holds the bar; Go and TS now check both halves | **PARITY ACHIEVED** |
| 7 | Conformance-assertion presence (`var _ seams.X = (*Impl)(nil)`; B-030) | **recorded reason** — the compiler at the use site IS the assertion (`GUIDE-AI-NATIVE-RUST.md:68`); no gate-checked written assertion is promised, so nothing to build | **routed (recorded)** — the guide promises type-level tests on public surfaces (`:237`), a distinct rule over a distinct surface from the Go `var _` scan; named parity debt carried by this loop, not silently dropped | **yes (NEW, gated)** — `go-conformance-assertion` (`rules/go_parity.rs`) polices **gated** cells for the assertion; the extractor emits `go_conformance` from `var _ Seam = (*Impl)(nil)`; seam-less / exempt cells are out (never falsely flagged); mounted conditional on `cells_dir` | **changed.** Pass №1 “none at all three” → Go built, Rust reason recorded, TS routed. The absence is no longer symmetric-neutral; it is a built rule where the idiom can drift and a recorded reason where it cannot | **Go BUILT; Rust/TS RECORDED** (the TS routed build is the only open thread — a parity-debt row, not a silent gap) |
| 13 | Floor policy-disable (`[[<lang>.floor_disable]]` — disable a step with a recorded reason) | **yes (NEW)** — `RustConfig.floor_disable: Vec<FloorDisable>` (`config.rs`); enforced `rust-ai-native-cli/src/floor.rs` (`STEPS` dictionary, prints each disable + reason, hard-fails an unknown step) — textually mirrors the twins (B-049) | **yes** (unchanged) | **yes** (unchanged) | **changed.** Pass №1’s one inversion (Rust the weaker floor) is CLOSED — B-049 built the Rust twin exactly the Go/TS shape | **PARITY ACHIEVED** |

## Rows unchanged, still OPEN (the remaining parity debt)

- **Row 6 — the Go flag/registry rule.** Rust `FlagSites` (R-001) + TS
  `ts-flag-sites` exist; Go has `registry_pkg` (a dead config key) and no
  `GoFlagSites`. Batch 2 did not touch it. **Straight build, routed to a later
  batch** — the guide's §6 promise is the remaining large asymmetry. Recorded
  not silent (the `registry_pkg` field says “carries no rule”).
- **Rows 8 / 12 — the Go floor `./...` residual.** `gofmt` is
  `exclude_substrings`-scoped (B-003); `vet`/`tests`/`staticcheck` still walk
  `./...` unfiltered. B-048 is the TS twin of B-003; the Go residual is its
  sibling. **Verify/build routed** (whether it bites the dirty fixtures is the
  measured question). Recorded not silent.
- **Rows 9 / 10 / 11 — the record-reason rows** (cell-isolation shapes,
  Rust-only rules with no twin, the three exterior-read semantics). These are
  load-bearing language differences, not slack; pass №1's verdict stands —
  **record the reason in the guides** so the asymmetry is not read as silence.
  (S4 lifted the parity principle into the manifesto and the three guides cite
  it — the frame that makes “recorded reason” a first-class disposition.)

## M-PARITY status

M-PARITY = “the B-035 table shows no language cell weaker than Rust without a
recorded reason.” After batch 2: the seam-error, conformance and floor-disable
mechanisms are at parity or recorded. **The two Go-only content gaps (row 6,
rows 8/12) are the only cells still weaker WITHOUT a full build** — both are
recorded (not silent) and routed as straight builds. So M-PARITY is
**recorded-honest but not yet build-complete**: reaching it needs the Go flag
rule (row 6) and the Go floor `./...` scoping (rows 8/12), both later batches.
The TS routed conformance build (row 7) is a named parity-debt thread, not a
blocker for M-PARITY (Rust's own conformance is a recorded reason, so “no
weaker than Rust without a reason” holds for TS here).
