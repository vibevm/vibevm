# PROP-013: Periodic health audit — vibevm's instance {#root}

<status stage="spec" state="done" comment="B0 2026-07-24: accepted 2026-05-23, in force; living audit-category checklist; fact grain 2026-07-24"/>

##status-line **Status:** accepted 2026-05-23 — owner-requested; in force. The audit-category checklist (§2) is **living** — it grows as new defect classes surface.
**Related:** [PROP-000](PROP-000.md) (the per-commit gate this audit complements), [`CLAUDE.md`](../../CLAUDE.md), [PROP-006](PROP-006-operating-modes.md) (the `move fast and break things` posture an audit-driven fix-up often runs under), `vibe check` (the automated *subset* of what this audit does by hand), [`spec/WAL.md`](../WAL.md) (Known issues — active findings), [`AUDIT.md`](../../AUDIT.md) (the inventory this process writes). @spec/done

- ##model-pointer The **general methodology** — why a green per-commit gate is not enough (its four blind spots), what a periodic breadth-first judgment sweep inventories, the `AUDIT.md` append-only home and why it is not the volatile checkpoint, the five-field finding record, severity, disposition and carry-forward, the living checklist, and the once-per-milestone cadence — is the `health-audit` flow this project depends on: `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#root`. @spec/done
- ##prop-scope This PROP records vibevm's **instance** of it: the concrete gate, the known findings per category, vibevm's added discipline-depth category, and the open questions. @spec/done

## 1. Why vibevm runs it — the M1.19 proof {#motivation}

- ##GATE-DEF vibevm's per-commit gate is `tools/self-check.sh` (`cargo fmt --check`, `cargo test --workspace`, `cargo clippy -D warnings`, `vibe check`). @spec/done
- ##GATE-BLIND-SPOTS It is a regression detector, blind by construction to uncovered code, out-of-gate trees, drift, and slow debt (the four blind spots: `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#blind-spots`). @spec/done

##m119-lead This is not hypothetical for vibevm. @spec/done

- ##M119-INIT-DEFECT The M1.19 session shipped a milestone — eight phases, ~800 hermetic tests green, the gate passing on every commit — in which `vibe init` scaffolded **broken projects**: it wrote `naming = "kind-name"`, so a freshly-initialised project could not install any package at all. @spec/done
- ##M119-TEST-ASSERTED-BROKEN The defect survived the entire milestone; a `cli_init` test even *asserted the broken value as correct*, staying green the whole time. @spec/done
- ##M119-CAUGHT-BY-SMOKE It was caught only by a live smoke run during the registry migration. @spec/done
- ##VIBE-INDEX-ROT Earlier, the `vibe-index` crate — then a separate Cargo workspace, outside `cargo test --workspace` — rotted unnoticed until a state review found its suite red. @spec/done
- ##AUDIT-NON-OPTIONAL These are the concrete failures that made the audit non-optional here. @spec/done

## 2. vibevm's checklist — the known instances {#instances}

##CHECKLIST-WALK vibevm walks the flow's category checklist (`spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#categories`) breadth-first. The categories below carry vibevm's **known instances** — the findings that made each line permanent (a discovered defect class becomes a standing row, `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#living`): @spec/done

##CAT-A **A — Test integrity.** @spec/done

- ##A1-COVERAGE-GAPS **A1 · Coverage gaps.** *Known:* install e2e tests overwhelmingly drive `LocalRegistry`, shadowing the real `GitPackageRegistry` + `NamingConvention` path; the `vibe init` default-config path had no e2e at all. @spec/done
- ##A2-QUARANTINED **A2 · Quarantined tests.** `#[ignore]`d tests — red? stale? (`grep -rn '#\[ignore\]'`.) *Known:* `cli_live_e2e.rs` — ignored and red. @spec/done
- ##A3-WRONG-BEHAVIOR **A3 · Tests that encode the wrong behavior.** Detectable only by reading the assertion against the spec. *Known:* `cli_init` asserted `naming == kind-name`. @spec/done

##CAT-B **B — Rot outside the gate.** @spec/done

- ##B1-UNREACHED-TREES **B1 · Unreached trees.** Separate workspaces, scripts, `fixtures/**` and `manual-tests/**` no test parses. *Known:* `fixtures/manual-test-packages/` carries retired schema across two milestones. @spec/done
- ##B2-GATE-COMPLETENESS **B2 · Gate completeness.** A new crate, a `[lib] test = false`, or a moved file can quietly carve a hole in what the gate covers. @spec/done

##CAT-C **C — Drift.** @spec/done

- ##C1-DOC-DRIFT **C1 · Doc drift.** `docs/**` versus `VIBEVM-SPEC.md` versus the code's actual behavior. @spec/done
- ##C2-SPEC-DRIFT **C2 · Spec drift.** A PROP self-contradicting or contradicting another; dead `spec://` references. *Known:* PROP-008 §3 said lockfile v4 while §7 said v5; PROP-005 references a `crates/vibe-index/schemas/` directory that does not exist. @spec/done
- ##C3-WAL-DRIFT **C3 · WAL / CONTINUE drift.** Does the checkpoint match the tree, the branch, the commit chain? @spec/done
- ##C4-OUTWARD-DRIFT **C4 · Outward drift.** Live registry orgs and other external state versus what the tool now expects. @spec/done

##CAT-D **D — Debt.** @spec/done

- ##D1-DEFERRED **D1 · Deferred & parked items** — walk every "deferred" / "parked" / "Known issues" entry in the WAL and the PROPs. @spec/done
- ##D2-AGING-MARKERS **D2 · Aging markers** — `<!-- REVIEW … -->`, `TODO`, `FIXME`, `HACK` (`grep -rn`; `vibe check` ages REVIEW markers). @spec/done
- ##D3-ESCAPE-HATCHES **D3 · Escape hatches** — `#[allow(dead_code)]`, `#[allow(clippy::…)]` (`grep -rn '#\[allow'`). @spec/done
- ##D4-DEP-STALENESS **D4 · Dependency staleness** — `cargo update --dry-run`; `cargo audit` / `cargo outdated`. @spec/done

##CAT-E **E — Discipline depth (AI-Native).** vibevm's own category, added 2026-06-12: it measures how deep the Discipline v0.2 adoption actually goes (the flow's corollary — audit *depth* of adoption, not merely that it exists), against vibevm's specific machinery: @spec/done

- ##E1-SPEC-GRANULARITY **E1 · Spec granularity & typing.** Units at REQ grain with kind/revision/status lines, not merely heading-anchored — an untyped unit cannot carry revision discipline. *Aid:* `specmap.json`. *Known:* at the category's birth, 347 of 352 units were untyped. @spec/done
- ##E2-EDGE-COVERAGE **E2 · Edge coverage.** Which crates and specs carry `implements`/`verifies` edges, which are bare; implemented features whose PROP has zero inbound edges (PROP-012 at birth); suites with no `#[verifies]`. *Aid:* `specmap.json` counts; a `#[verifies]` census. @spec/done
- ##E3-CELL-SEAM **E3 · Cell & seam structure.** Seam traits without `#[cell]` manifests; god-files; single-impl speculative seams; hardcoded dispatch where a seam belongs; test monoliths. *Aid:* `grep '#\[cell('`, the `pub trait` inventory, a file-length census. @spec/done
- ##E4-CHECKER-CARD-GAPS **E4 · Checker-vs-card gaps.** Conform rules implemented weaker than the card they cite; guide-mandated checkers that do not exist (a rule with no checker is a WISH); committed gate artifacts that have silently rotted. *Aid:* read each rule's `check()` against its card's ops block; probe gates empirically on a clean tree. @spec/done

## 3. vibevm's inventory and cadence {#record}

- ##INVENTORY-AUDIT-MD Findings live in **`AUDIT.md`** at the repo root — the append-only chronicle whose history is vibevm's health trend (the flow's `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#audit-md`; the five-field record `…#finding`; disposition and carry-forward `…#disposition`). @spec/done
- ##CADENCE-FLOOR Cadence is **owner-triggered, floor once per milestone** (`…#cadence`) — a vibevm milestone is never declared done on an un-audited base. @spec/done
- ##RECONCILE-WAL Each run reconciles the WAL "Known issues" against `AUDIT.md` before it closes. @spec/done

## 4. Open questions {#open}

<status stage="spec" state="work" comment="B1 2026-07-24: three questions still open, no owner ruling yet"/>

1. ##open-trigger-phrase **Trigger phrase.** Add `АУДИТ` / `RUN AUDIT` to `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` as a recognised command (mirroring the `ЗАВЕРШИ СЕССИЮ` session-end command), or keep the audit purely owner-narrated? @spec/work
2. ##open-vibe-audit **`vibe audit` aggregator.** Once enough §2 categories are mechanical, a `vibe audit` subcommand could run them and pre-fill the `AUDIT.md` skeleton — a FEAT worth opening when that threshold is reached. @spec/work
3. ##open-health-metric **Health metric.** Should a run compute one coarse number — open P1 / P2 counts, the trend versus the previous run — so the project's direction is visible at a glance? @spec/work

## 5. Version history {#history}

- ##HIST-DRAFT-1 **2026-05-23 — draft 1, in force.** Owner-requested after the M1.19 session surfaced a milestone-grade defect — `vibe init` scaffolding broken projects — that the per-commit gate and ~800 hermetic tests missed. The process, the category checklist, the `AUDIT.md` inventory, the severity / disposition model, and the per-milestone cadence floor were defined here. The first (seed) run is recorded in [`AUDIT.md`](../../AUDIT.md). @spec/done
- ##HIST-CATEGORY-E **2026-06-12 — category E (discipline depth) added** by that day's owner-requested full sweep — the first post-adoption depth audit. Permanent per the living-checklist law: the same gap (surface adoption mistaken for depth) is never re-missed. The run also demonstrated E4's empirical-probe clause — a merge-panel gate believed green was red on a clean tree. @spec/done
- ##HIST-EXTRACTED **2026-07-14 — general methodology extracted to the `health-audit` flow.** The gate-vs-audit argument, the four blind spots, the category framework, the `AUDIT.md` model, the finding / severity / disposition machinery, the living-checklist law, and the cadence moved into the installable `health-audit` package (reaching vibevm through the redbook dependency); this PROP was thinned to vibevm's gate, its known findings, its discipline-depth category E, and its open questions. No process changed. @spec/done
