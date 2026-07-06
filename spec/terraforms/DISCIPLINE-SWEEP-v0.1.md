# vibevm Discipline Sweep v0.1 — the standing guardian that holds the tree inside the Discipline
**status: STANDING · recurring (daily / weekly) · vibevm-specific · the recurring instrument that keeps the codebase conformant as it grows**

*Origin: five terraform plans (SHRINK v0.1/v0.2, CONVERT v0.1, PUBDOC-DRAIN v0.1, and the TERRAFORM-v0.3 adoption) each drove the tree a step deeper into the AI-Native Discipline, and each left a residue of hard-won lessons in its execution record. Those plans were one-shot campaigns; this one is not. It is the **standing sweep** — the recurring run that, executed daily or weekly, keeps the codebase inside the Discipline instead of letting it drift back out between campaigns. It synthesises every check we learned to care about and pairs each with the concrete, experience-derived signal to look for — file:line patterns, the falsifications we recorded, the idioms that worked.*

*Discipline Law 2 governs the shape of this plan: explanation capital must be runnable capital — a check that could be a checker is a WISH until it becomes one. So the sweep is **collector-first**: a no-LLM fact-gatherer (`cargo xtask health`) computes the objective state, and the human/agent acts on facts, not vibes. Automated fact-gathering is itself the Discipline value here. Where a check genuinely cannot be mechanised yet (the WISH rules — §4), the sweep names it and routes it to judgment, but it never pretends prose is a gate.*

*The two truths to hold throughout: (1) **the gates are the floor, this sweep is the ceiling** — `conform check` / `specmap --check` / `self-check.sh` say pass/fail and must be green before any sweep work; the collector's advisory facts (coverage, danger-band, backlog) sit *above* that floor and say what to harden next. (2) **The gate is truth, the collector is a guide** — when the collector says a crate is ready to gate, the gate confirms or refutes it; trust the gate.*

---

## 0. Snapshot at authoring (2026-06-14, tree `91bc763`)

`cargo xtask health` at authoring time — the baseline this guardian starts from:

- **Gating:** 16 crates gated / 4 exempt-with-reason / 1 under `pub-doctest` (`vibe-core`). *(The count comes from conform.toml's `gated_crates` list, read by the collector — never from memory. An LLM inventorying this during authoring miscounted it as 18; the list said 16 (it is 13 since the conform toolchain itself relocated into the package, PROP-024). This is the "instrument discipline — count the list, not the record" lesson, SHRINK v0.1 §0, made mechanical.)*
- **conform baseline: 0 frozen** — empty for the first time (PUBDOC-DRAIN v0.1 drained the last 55).
- **`file-length` danger band: 14 files in `[540, 600]`, 0 over budget.** `crates/vibe-workspace/src/boot.rs` sits at **exactly 600** — the standing landmine; one added line trips the gate. Then `vibe-resolver/src/features.rs` 591, `vibe-registry/src/index_client.rs` 590, `vibe-core/src/manifest/package.rs` 582, `vibe-resolver/src/activation.rs` 582, …
- **`pub-doctest` promotion candidates (gated, 0 type-coverage gap — ready to widen the gate, zero drain): `conform-core`, `conform-frontend-rust`, `env-audit`, `specmark-grammar`.**
- **`pub-doctest` drain backlog (ranked, smallest gap first):** vibe-install 9 · specmap-core 11 · vibe-check 11 · vibe-publish 13 · vibe-workspace 15 · vibe-mcp 16 · vibe-resolver 16 · vibe-registry 30 · vibe-cli 54 · vibe-index 85 (≈260 public types to document across the remaining gated crates).
- **Deviation debt: 9 fn-grain `#[spec(deviates)]` sites.**

These numbers are not pinned in this document — they live in `terraform/health/latest.json`, regenerated each sweep; this section is the worked example. Re-read the live file, not this paragraph.

## 1. How to run (the operator contract)

This machine has quirks that have bitten before; honour them or the sweep corrupts the tree it is meant to guard:

- **Edits go through editor tools only.** PowerShell 5.1 corrupts UTF-8-no-BOM round-trips (`Get-Content`/`Set-Content` mangle non-ASCII); recover a mangled file with `git restore`.
- **`self-check.sh` runs through Git Bash, not WSL** (`bash` in PowerShell resolves to WSL). Check the real exit code — `$?` or `${PIPESTATUS[0]}`, never a `| tail`'d pipe (the tail masks the script's exit).
- **Commits via `git commit -F - <<'MSG'` heredoc only** — `-m` with backticks has twice corrupted messages through command substitution.
- **Windows UAC blocks test executables named `*install*`** (os-740): a crate whose integration tests touch installation uses `[lib] test = false` + a safely-named test binary (the vibe-install pattern, SHRINK v0.2).

One-command facts and the binary floor:

```sh
cargo xtask health                       # the advisory snapshot → terraform/health/latest.json + stdout summary
cargo xtask conform check                # the ratchet floor: 0 new findings against conform-baseline.json
cargo xtask specmap --check              # the index floor: 0 suspects / warnings / gated-orphans
cargo xtask test-gate                    # nextest, xfail-strict against terraform/registry/tests-baseline.json
cargo xtask fast-loop --enforce-budget   # every cell builds+tests < 60s
cargo xtask tripwire                     # which debt.json entries the change set touches (warn-only)
bash tools/self-check.sh                 # fmt + workspace tests + doctests + clippy -D + vibe check
```

## 2. Tier 0 — the hard floor (EVERY run, binary, before anything else)

**Never sweep on a red tree.** Run the floor first; if any gate is red, the only work is making it green. Order and meaning:

1. `bash tools/self-check.sh` — fmt-clean, all workspace tests + doctests green, `clippy --all-targets -D warnings`, `vibe check` 0/0/0. The cheapest signal first (fmt), the spec linter last.
2. `cargo xtask conform check` — **0 new** findings against the (currently empty) baseline. A new finding means a Discipline regression landed; drain it, do not freeze it. `conform freeze` is legal **only** on a reviewed shrink (a deletions-only `git diff conform-baseline.json`) or a new-rule landing — never to paper over a fresh violation.
3. `cargo xtask specmap --check` — clean: 0 suspects, 0 warnings, 0 gated orphans, 0 dispositioned drift. Doc-only edits that shift line numbers require a `cargo xtask specmap` regen committed alongside (the index pins line numbers).
4. `cargo xtask test-gate` — xfail-strict: no newly-failing tests, no unexpectedly-passing-unpromoted.
5. `cargo xtask fast-loop --enforce-budget` — no cell exceeds the 60s first-signal budget (a slow cell is hidden coupling).

*Exit:* all five green. This is the precondition for everything below.

## 3. Tier 1 — the ratchet (EVERY run; act on `cargo xtask health`)

Run the collector, read `terraform/health/latest.json`, and work its facts in this order. Each item below is driven by a field the collector emits, so the work-list is objective.

- **1a · Split the danger band before it trips (`summary.danger_band_files`).** Any file approaching 600 is a landmine — `boot.rs` at 600 is one edit from red. **Measure with the rule, not the eye:** `file-length` counts *physical* lines (`text.lines().count()`), not non-blank — the SHRINK v0.1 §0 falsification (three files eyeballed at 556/566 were really 609/612). Two levers, in order:
  - **Tests-out** (cheap, structure-preserving): move an inline `#[cfg(test)] mod tests` to a sibling `foo/tests.rs` declared `#[cfg(test)] #[path = "foo/tests.rs"] mod tests;`. The cell's single-registration-point property is untouched. Gotchas the campaigns paid for: the conform frontend parses files **standalone**, so a non-`#[test]` helper in the tests-out file needs its own `#[cfg(test)]` or its `unwrap`s read as domain; `pub(super)` items cannot be re-exported wider (E0364).
  - **Responsibility split** (when the production half alone exceeds budget): split along the file's seam into module-grain cells; **every new module carries the parent's `scope!` URI** so it stays in the retrieval index (no gated orphan).
- **1b · Widen the gate for free (`summary.pub_doctest_promotion_candidates`).** A gated crate at **0 type-coverage gap** is ready to enter conform.toml's `gated_pub_doctest` list with zero drain. Add the crate to that list, run `cargo xtask conform check` — expect **0 new** (the collector predicted it; the gate confirms it). If a finding does appear, the collector slightly under-counted; drain that one type and proceed. This is the cheapest ratchet win there is. *(The live ready-set is the collector's `pub_doctest_promotion_candidates`; the four named at authoring have since been gated, and three of them — `conform-core`, `conform-frontend-rust`, `env-audit` — relocated into `stack:org.vibevm/rust-ai-native` per PROP-024.)*
- **1c · Drain the smallest backlog (`summary.pub_doctest_drain_backlog`).** Take the smallest-gap gated crate not yet under `pub-doctest`, document its public types, then promote it (1b). The PUBDOC-DRAIN v0.1 idiom set, proven across 55 types: a **TOML round-trip** doctest for serde sections (`toml::from_str::<T>(r#"…"#)` — the wire form is the canonical use); a **parse** one-liner for newtypes (`T::parse("…")`, asserting via the `Deref<str>` / `PartialEq<str>` ergonomics); a **variant / `Default`** assert for bare enums; a **construct-and-Display** assert for error enums (the Class-F message already cites its REQ, so the example doubles as a navigability demo). Per crate: doctests → `cargo test -p <c> --doc` → `fmt` → `conform check` → **deletions-only** `conform freeze` diff → `specmap` regen for the line shifts → topic commit. Any crate is a safe stop; the backlog is the resume pointer.
- **1d · Re-justify the deviation debt (`crates[].deviations`, `summary.deviation_debt`).** Every `#[spec(deviates = …, reason = …)]` is a recorded escape from a ban (unwrap / unsafe / ambient-env / a structural rule). Walk each: does its reason still hold? A deviation whose invariant has since been encoded in a type should be removed and restructured — *a deviation with no live reason is a defect*. The canonical deviates target is `spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#rules` (the grammar admits only resolvable `spec://` units; the ban itself lives in the package guide, outside the specmap).
- **1e · Catch regressions in the censuses (`crates[].error_enums_missing_req` / `unwrap_domain` / `env_nonroot` / `unsafe_nonaudit`).** For a **gated** crate these must stay zero — the gates already enforce them, so a non-zero here is a fresh violation that slipped a local check; drain it the way the campaigns did: unwrap → the layer's error enum (restructure beats testify — types carry the invariant: split-first tuples, `let-else`, `next_if`, read-then-advance counters, parser early-returns; `from_validated` beats a fake-fallible signature; a structural `semver::Comparator` beats `VersionReq::parse("={v}")` which panics on build metadata); a new error enum → `#[spec(implements = "spec://…")]` on the enum and the Class-F `(violates spec://…; fix: …)` tail in every `#[error("…")]`; an ambient env read → thread it from the composition root or record it in `ENV_ROOTS`. For an **ungated** crate these are the conversion backlog that precedes its eventual gate flip — **a crate flips into conform.toml's `gated_crates` only after it drains to zero** (the expand-as-you-conform rhythm; a flip must never widen the baseline, SHRINK v0.2).

## 4. Tier 2 — drift (WEEKLY; the slow rot the gates don't see)

- **2a · Debt registry.** `cargo xtask tripwire` lists which `terraform/registry/debt.json` entries the week's change set touched (BROWNFIELD §3 — debt as a first-class object, `touch:` globs as tripwires). Re-disposition any touched-and-`open` entry; a fixed one records `fixed_in`/`adjudication`. New deficiencies discovered during the sweep are *filed* here, not left as prose.
- **2b · Doc/code drift.** WAL freshness (if `spec/WAL.md` is older than 24h, verify state before destructive work — `spec/boot/00-core.md`); `CONTINUE.md` staleness (it lags the WAL by design — the WAL wins on conflict); `docs/architecture.md` vs the real crate layout; ROADMAP self-staleness (the DBT-0017 pattern — shipped items unticked, retired filenames cited). Stale doc that governs code is filed to debt.json (`kind: stale-doc`).
- **2c · Marker census.** `rg -n 'TODO|FIXME|REVIEW|XXX|HACK' crates/ xtask/` — each marker is a WISH or a deferred decision. Load-bearing ones (a real deferred requirement) graduate to `debt.json` / `intent.json`; trivial ones are resolved or deleted. Prose that promises and doesn't deliver is false training signal.
- **2d · Golden transcripts.** `terraform/golden/*.transcript.md` are characterization oracles for the CLI surface. They must fail **loudly** and be re-captured **deliberately** (`terraform/golden/capture.sh`), never auto-updated (R-040) — an auto-updated golden is a test that always passes.
- **2e · specmap kind hygiene.** When code gains an `implements` edge into a spec unit that carries no kind/revision marker, specmap raises `pin-into-unmarked-unit` — mark the unit `req r1` (or the appropriate kind) in the *same* change (the CONVERT-PLAN Phase 0.3 lesson). Unmarked = informative; a unit that code points at is not informative.

## 5. Tier 3 — deep judgment (WEEKLY/BIWEEKLY; the WISH rules with no checker yet)

These are Discipline rules the conform engine does not yet mechanise (the §7 gap list). They need human or strong-agent review — the sweep names them so they are not silently skipped, and each is a candidate to graduate into a real checker (which is how the rule set has always grown).

- **3a · Newtypes at seams (Scaffold B).** Did a new public surface land stringly-typed where a value has an invariant (a name, a URL, a hash, an id)? Wrap it. The cascade is small if the newtype is ergonomic: `Deref<str>` + `PartialEq<str>` + `Display` + `From` collapse the read-site churn to a handful of construction sites; `from_validated` is the infallible reconstruction seam, `parse` the validating input boundary.
- **3b · Cells: oracle, isolation, no stamping (Scaffolds D/E, R-002).** A new `#[cell]` carries a differential oracle **referenced from an integration test under `crates/<c>/tests/`** (an inline unit test does **not** satisfy `cell-has-oracle` — the import/construction must live in the integration test). No cell imports a sibling cell (extract the shared logic to a seam or core module — the `org_walk.rs` move). And do not mint a cell where there is one production variant — cells grow only where real variance exists (no cell-stamping).
- **3c · Uniformity (R3-006, WISH).** One idiom per operation across the crate; an exception is marked `#[spec(deviates)]` or it propagates as false training signal (a model copies the nearest example).
- **3d · Contract-first ordering (R3-002, WISH).** Intent before body — signature, invariants, error contract, and the doctest example precede the implementation, so the file conditions a reader top-down.
- **3e · Lying prose (R2C-004 / H4, WISH).** A prose claim adjacent to code that a doctest or `#[spec(documents = "…")]` edge *could* verify but doesn't is worse than no comment — convert it to runnable capital or delete it.
- **3f · Closed-vocabulary naming (R3-004, WISH).** Identifiers drawn from the project's closed vocabulary (the four installable kinds; the spec's terms); no shadowing on contract surfaces.
- **3g · RAID sweep (`03-RAID-PLAYBOOK.md`).** Pick one scaffold dimension and sweep it workspace-wide — scope+freeze → card-set+order → per-layer phases → batch+checkpoints → differential safety → REPORT. This is how a Tier-1 backlog (e.g. the `pub-doctest` drain across all ten crates) becomes a focused campaign when it is large enough to warrant one.

## 6. The collector — `cargo xtask health`

`xtask/src/health.rs`. Reuses the conform fact frontend (`Store::extract_workspace`) so its numbers cannot drift from what the gates parse, and reads `gated_crates` / `gated_pub_doctest` / `env_roots` straight from `conform.toml` (single source of truth — no hardcoded counts). It emits, per crate: public-type count and doctest coverage (`typed_gap`), error-enum REQ-edge coverage, cell count, the unwrap / ambient-env / unsafe censuses, deviation count, and the file danger/over-budget lists; and at the workspace level: the gated/exempt counts, the conform baseline by rule, the ranked drain backlog, the zero-gap promotion candidates, and the full danger band. It is **pure of the source tree** (re-running on an unchanged tree writes a byte-identical file — its git diff is the health delta), **advisory** (never fails the build — the gates do that), and **no-LLM**. The stdout summary is the at-a-glance; the JSON is the work-list.

Extending it is itself ratchet work: when a Tier-3 WISH rule becomes mechanisable, add the census here first (cheap, advisory), then promote it to a conform `Rule` once it is proven (gated, blocking) — the same path `pub-doctest`, `no-unwrap-in-domain`, and `ambient-env` all walked.

## 7. Cadence

| Tier | Daily | Weekly |
|---|:---:|:---:|
| 0 — hard floor (self-check, conform, specmap, test-gate, fast-loop) | ✓ | ✓ |
| 1 — ratchet (split danger band, widen gate, drain smallest backlog, re-justify deviations) | ✓ | ✓ |
| 2 — drift (debt tripwire, doc/code drift, markers, goldens, kind hygiene) | | ✓ |
| 3 — deep judgment (newtypes, cells, the WISH rules, a RAID sweep) | | ✓ |

The daily run is light: floor green, then one or two cheapest Tier-1 wins (a promotion candidate, or one danger-band split, or one small backlog crate). The weekly run adds the drift and judgment tiers and may launch a RAID. Any single item is a safe stop — the sweep is incremental by construction, and the collector re-derives the remaining work each run.

## 8. Output of a sweep

Each sweep that changes the tree lands topic commits per Rule 3 (one logical unit each), citing this plan and the specific item (`spec://vibevm/terraforms/DISCIPLINE-SWEEP-v0.1#<tier>`). Refresh `terraform/health/latest.json` in the same run (its git diff records the health trend over time). At a sweep that moves a milestone — a gate widened, a backlog crate cleared — bump the `spec/WAL.md` standing line. The health snapshot plus the WAL is the resume pointer for the next sweep.

## 9. What this plan deliberately does NOT do

- It does **not** replace the gates. Tier 0 is the floor; this plan hardens what sits above it. A green sweep on a red floor is a contradiction — fix the floor.
- It does **not** auto-fix. The collector gathers facts with no LLM; the human/agent acts on them. Mechanising a fix is a separate, deliberate promotion (a new `Rule` or a `codemod`).
- It does **not** build measurement infrastructure (deferred by owner decision, TERRAFORM-PLAN v0.3 §6) — it records objective state, not effectiveness metrics.
- It does **not** touch owner-frozen surfaces (`spec/boot/00-core.md`, `90-user.md`, `VIBEVM-SPEC.md`) or owner-court items without sanction; drift it finds in them is *filed* to debt.json, not fixed.
