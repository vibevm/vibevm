# PROP-013: Periodic health audit — vibevm's instance {#root}

**Status:** accepted 2026-05-23 — owner-requested; in force. The audit-category checklist (§2) is **living** — it grows as new defect classes surface.
**Related:** [PROP-000](PROP-000.md) (the per-commit gate this audit complements), [`CLAUDE.md`](../../CLAUDE.md), [PROP-006](PROP-006-operating-modes.md) (the `move fast and break things` posture an audit-driven fix-up often runs under), `vibe check` (the automated *subset* of what this audit does by hand), [`spec/WAL.md`](../WAL.md) (Known issues — active findings), [`AUDIT.md`](../../AUDIT.md) (the inventory this process writes).

The **general methodology** — why a green per-commit gate is not enough (its four blind spots), what a periodic breadth-first judgment sweep inventories, the `AUDIT.md` append-only home and why it is not the volatile checkpoint, the five-field finding record, severity, disposition and carry-forward, the living checklist, and the once-per-milestone cadence — is the `health-audit` flow this project depends on: `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#root`. This PROP records vibevm's **instance** of it: the concrete gate, the known findings per category, vibevm's added discipline-depth category, and the open questions.

## 1. Why vibevm runs it — the M1.19 proof {#motivation}

vibevm's per-commit gate is `tools/self-check.sh` (`cargo fmt --check`, `cargo test --workspace`, `cargo clippy -D warnings`, `vibe check`) — a regression detector, blind by construction to uncovered code, out-of-gate trees, drift, and slow debt (the four blind spots: `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#blind-spots`).

This is not hypothetical for vibevm. The M1.19 session shipped a milestone — eight phases, ~800 hermetic tests green, the gate passing on every commit — in which `vibe init` scaffolded **broken projects**: it wrote `naming = "kind-name"`, so a freshly-initialised project could not install any package at all. The defect survived the entire milestone; a `cli_init` test even *asserted the broken value as correct*, staying green the whole time. It was caught only by a live smoke run during the registry migration. Earlier, the `vibe-index` crate — then a separate Cargo workspace, outside `cargo test --workspace` — rotted unnoticed until a state review found its suite red. These are the concrete failures that made the audit non-optional here.

## 2. vibevm's checklist — the known instances {#instances}

vibevm walks the flow's category checklist (`spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#categories`) breadth-first. The categories below carry vibevm's **known instances** — the findings that made each line permanent (a discovered defect class becomes a standing row, `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#living`):

**A — Test integrity.**

- **A1 · Coverage gaps.** *Known:* install e2e tests overwhelmingly drive `LocalRegistry`, shadowing the real `GitPackageRegistry` + `NamingConvention` path; the `vibe init` default-config path had no e2e at all.
- **A2 · Quarantined tests.** `#[ignore]`d tests — red? stale? (`grep -rn '#\[ignore\]'`.) *Known:* `cli_live_e2e.rs` — ignored and red.
- **A3 · Tests that encode the wrong behavior.** Detectable only by reading the assertion against the spec. *Known:* `cli_init` asserted `naming == kind-name`.

**B — Rot outside the gate.**

- **B1 · Unreached trees.** Separate workspaces, scripts, `fixtures/**` and `manual-tests/**` no test parses. *Known:* `fixtures/manual-test-packages/` carries retired schema across two milestones.
- **B2 · Gate completeness.** A new crate, a `[lib] test = false`, or a moved file can quietly carve a hole in what the gate covers.

**C — Drift.**

- **C1 · Doc drift.** `docs/**` versus `VIBEVM-SPEC.md` versus the code's actual behavior.
- **C2 · Spec drift.** A PROP self-contradicting or contradicting another; dead `spec://` references. *Known:* PROP-008 §3 said lockfile v4 while §7 said v5; PROP-005 references a `crates/vibe-index/schemas/` directory that does not exist.
- **C3 · WAL / CONTINUE drift.** Does the checkpoint match the tree, the branch, the commit chain?
- **C4 · Outward drift.** Live registry orgs and other external state versus what the tool now expects.

**D — Debt.**

- **D1 · Deferred & parked items** — walk every "deferred" / "parked" / "Known issues" entry in the WAL and the PROPs. **D2 · Aging markers** — `<!-- REVIEW … -->`, `TODO`, `FIXME`, `HACK` (`grep -rn`; `vibe check` ages REVIEW markers). **D3 · Escape hatches** — `#[allow(dead_code)]`, `#[allow(clippy::…)]` (`grep -rn '#\[allow'`). **D4 · Dependency staleness** — `cargo update --dry-run`; `cargo audit` / `cargo outdated`.

**E — Discipline depth (AI-Native).** vibevm's own category, added 2026-06-12: it measures how deep the Discipline v0.2 adoption actually goes (the flow's corollary — audit *depth* of adoption, not merely that it exists), against vibevm's specific machinery:

- **E1 · Spec granularity & typing.** Units at REQ grain with kind/revision/status lines, not merely heading-anchored — an untyped unit cannot carry revision discipline. *Aid:* `specmap.json`. *Known:* at the category's birth, 347 of 352 units were untyped.
- **E2 · Edge coverage.** Which crates and specs carry `implements`/`verifies` edges, which are bare; implemented features whose PROP has zero inbound edges (PROP-012 at birth); suites with no `#[verifies]`. *Aid:* `specmap.json` counts; a `#[verifies]` census.
- **E3 · Cell & seam structure.** Seam traits without `#[cell]` manifests; god-files; single-impl speculative seams; hardcoded dispatch where a seam belongs; test monoliths. *Aid:* `grep '#\[cell('`, the `pub trait` inventory, a file-length census.
- **E4 · Checker-vs-card gaps.** Conform rules implemented weaker than the card they cite; guide-mandated checkers that do not exist (a rule with no checker is a WISH); committed gate artifacts that have silently rotted. *Aid:* read each rule's `check()` against its card's ops block; probe gates empirically on a clean tree.

## 3. vibevm's inventory and cadence {#record}

Findings live in **`AUDIT.md`** at the repo root — the append-only chronicle whose history is vibevm's health trend (the flow's `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#audit-md`; the five-field record `…#finding`; disposition and carry-forward `…#disposition`). Cadence is **owner-triggered, floor once per milestone** (`…#cadence`) — a vibevm milestone is never declared done on an un-audited base. Each run reconciles the WAL "Known issues" against `AUDIT.md` before it closes.

## 4. Open questions {#open}

1. **Trigger phrase.** Add `АУДИТ` / `RUN AUDIT` to `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` as a recognised command (mirroring the `ЗАВЕРШИ СЕССИЮ` session-end command), or keep the audit purely owner-narrated?
2. **`vibe audit` aggregator.** Once enough §2 categories are mechanical, a `vibe audit` subcommand could run them and pre-fill the `AUDIT.md` skeleton — a FEAT worth opening when that threshold is reached.
3. **Health metric.** Should a run compute one coarse number — open P1 / P2 counts, the trend versus the previous run — so the project's direction is visible at a glance?

## 5. Version history {#history}

- **2026-05-23 — draft 1, in force.** Owner-requested after the M1.19 session surfaced a milestone-grade defect — `vibe init` scaffolding broken projects — that the per-commit gate and ~800 hermetic tests missed. The process, the category checklist, the `AUDIT.md` inventory, the severity / disposition model, and the per-milestone cadence floor were defined here. The first (seed) run is recorded in [`AUDIT.md`](../../AUDIT.md).
- **2026-06-12 — category E (discipline depth) added** by that day's owner-requested full sweep — the first post-adoption depth audit. Permanent per the living-checklist law: the same gap (surface adoption mistaken for depth) is never re-missed. The run also demonstrated E4's empirical-probe clause — a merge-panel gate believed green was red on a clean tree.
- **2026-07-14 — general methodology extracted to the `health-audit` flow.** The gate-vs-audit argument, the four blind spots, the category framework, the `AUDIT.md` model, the finding / severity / disposition machinery, the living-checklist law, and the cadence moved into the installable `health-audit` package (reaching vibevm through the redbook dependency); this PROP was thinned to vibevm's gate, its known findings, its discipline-depth category E, and its open questions. No process changed.
