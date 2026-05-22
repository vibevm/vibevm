# PROP-013: Periodic health audit — a recurring inventory of defects, rot, and drift {#root}

**Status:** accepted 2026-05-23 — owner-requested; in force. The audit-category checklist (§2.2) is **living** — it grows as new defect classes surface.
**Related:** [PROP-000](PROP-000.md) (process foundation — the four rules, the per-commit gate), [`CLAUDE.md`](../../CLAUDE.md) (the per-commit discipline), [PROP-006](PROP-006-operating-modes.md) (operating modes — the `move fast and break things` posture an audit-driven fix-up often runs under), `vibe check` (the spec linter, `VIBEVM-SPEC.md` §12 — the automated *subset* of what this audit does by hand), [`spec/WAL.md`](../WAL.md) (Known issues — where active findings surface), [`AUDIT.md`](../../AUDIT.md) (the inventory this process writes).

---

## 1. Motivation {#motivation}

The per-commit gate (`tools/self-check.sh` — `cargo fmt --check`, `cargo test --workspace`, `cargo clippy -D warnings`, `vibe check`) is a **regression detector**: it proves, on every commit, that *covered* code still behaves. It is indispensable, and it is structurally blind to four things:

1. **Uncovered code.** A path no test exercises can break with the gate fully green.
2. **Code outside the gate.** Anything `cargo test --workspace` does not reach — a separate workspace, an unparsed fixture, a manual-test recipe — rots silently.
3. **Drift.** Docs, spec, the WAL, and external state fall out of step with the code without any test failing.
4. **Slow debt.** `#[allow(...)]` escape hatches, aging `TODO`s, deferred items, quarantined `#[ignore]`d tests — each individually invisible, collectively corrosive.

This is not hypothetical. The M1.19 session shipped a milestone — eight phases, ~800 hermetic tests green, the gate passing on every commit — in which `vibe init` scaffolded **broken projects**: it wrote `naming = "kind-name"`, so a freshly-initialised project could not install any package at all. The defect survived the entire milestone; it was caught only by a live smoke run during the registry migration. A `cli_init` test even *asserted the broken value as correct*, staying green the whole time. Earlier, the `vibe-index` crate — then a separate Cargo workspace, outside `cargo test --workspace` — rotted unnoticed until a state review found its suite red.

The fix is not "more of the same gate". It is a **different kind of check**: a deliberate, periodic, breadth-first sweep, run with human / agent judgment, that inventories what the gate cannot see — plus a durable record so the project can tell, over time, whether it is getting healthier or worse. This PROP defines that process. It operationalises the owner's "base-machinery-first" principle (`spec/WAL.md`): the audit is the recurring *measurement* of whether the base is in fact stable enough to build the next layer on.

---

## 2. Decisions {#decisions}

### 2.1 The audit is a deliberate periodic sweep, complementary to the gate {#nature}

The audit is **not** part of `self-check.sh` and does not run per commit. It is a distinct activity: a session — human- or agent-driven — that works through the §2.2 checklist breadth-first, judges each area, and records findings in [`AUDIT.md`](../../AUDIT.md). Where the gate answers *"did this commit regress covered code?"*, the audit answers *"what is wrong, rotting, or drifting that no commit will ever flag?"*. The two are complements; neither replaces the other.

### 2.2 Audit scope — the category checklist {#checklist}

An audit run walks these categories. The checklist is **living** (§2.5) — every run may add a category a finding revealed. Each category names what to look for and, where one exists, a mechanical aid.

**A — Test integrity.**

- **A1 · Coverage gaps.** Production code paths exercised only through a proxy, or not at all. *Known instance:* install e2e tests overwhelmingly drive `LocalRegistry` (a directory layout), shadowing the real `GitPackageRegistry` + `NamingConvention` path; the `vibe init` default-config path had no e2e at all.
- **A2 · Quarantined tests.** `#[ignore]`d tests — are they red? stale? has the quarantine become permanent and forgotten? (`grep -rn '#\[ignore\]'`.) *Known instance:* `cli_live_e2e.rs` — ignored and currently red.
- **A3 · Tests that encode the wrong behavior.** A test asserting *current* output rather than *intended* behavior stays green while the behavior is a defect. Detectable only by reading the assertion against the spec. *Known instance:* `cli_init` asserted `naming == kind-name`.

**B — Rot outside the gate.**

- **B1 · Unreached trees.** Anything `cargo test --workspace` / `self-check.sh` does not run: separate workspaces, scripts, `fixtures/**` and `manual-tests/**` that no test parses. *Known instance:* `fixtures/manual-test-packages/` carries retired schema across two milestones.
- **B2 · Gate completeness.** Does the gate still cover every crate and every target? A new crate, a `[lib] test = false`, or a moved file can quietly carve a hole.

**C — Drift.**

- **C1 · Doc drift.** `docs/**` versus `VIBEVM-SPEC.md` versus the code's actual behavior.
- **C2 · Spec drift.** A PROP self-contradicting or contradicting another; `VIBEVM-SPEC.md` versus shipped reality; dead `spec://` references (`vibe check` covers some). *Known instances:* PROP-008 §3 said lockfile v4 while §7 said v5; PROP-005 references a `crates/vibe-index/schemas/` directory that does not exist on disk.
- **C3 · WAL / CONTINUE drift.** Does the checkpoint match the tree, the branch, the commit chain?
- **C4 · Outward drift.** Live registry orgs and other external state versus what the tool now expects.

**D — Debt accumulation.**

- **D1 · Deferred & parked items.** Walk every "deferred" / "parked" / "Known issues" entry in the WAL and the PROPs: still valid? still wanted? silently overtaken by later work?
- **D2 · Aging markers.** `<!-- REVIEW … -->`, `TODO`, `FIXME`, `HACK` (`grep -rn`). `vibe check` ages REVIEW markers; the audit reviews the rest.
- **D3 · Escape hatches.** `#[allow(dead_code)]`, `#[allow(clippy::…)]`, and similar — each justified once, never revisited (`grep -rn '#\[allow'`).
- **D4 · Dependency staleness.** Outdated crates and security advisories (`cargo update --dry-run`; `cargo audit` / `cargo outdated` when installed).

### 2.3 The inventory — `AUDIT.md` {#inventory}

Each run records its findings in **`AUDIT.md`** at the repository root — a curated, append-only chronicle, one dated section per run (the shape of `CHANGELOG.md`). Every finding carries:

- an **ID** — `<run-date>-NN`;
- the **category** it came from (`A1` … `D4`);
- a one-line **description** with enough of a locator (file, crate) to act on;
- a **severity** — `P1` / `P2` / `P3`;
- a **disposition** — `fixed` / `filed` / `accepted` / `open` (§2.4).

Severity:

- **P1 — blocker.** A correctness gap, or a defect that can ship wrong behavior. Must be resolved before the next milestone is declared shipped.
- **P2 — debt.** Real and scheduled — fixed in a dedicated pass, or opportunistically when the area is next touched.
- **P3 — note.** Low cost of leaving; recorded so the next run re-judges it rather than re-discovering it.

`AUDIT.md` is committed to git. Its history *is* the project's health trend.

### 2.4 Disposition and carry-forward {#disposition}

Every finding is dispositioned:

- **fixed** — resolved within the audit run itself. Small findings are fixed on the spot; the fix is a normal commit, and the finding records its hash.
- **filed** — too large to fix in the run. It becomes tracked work: a WAL "Known issues" entry (for active items), a `TASKS.md` line, or — if it needs design — a new PROP. The `AUDIT.md` finding records where it was filed.
- **accepted** — a deliberate decision not to act, recorded with the reason. Re-judged next run.
- **open** — not yet dispositioned. An `open` finding, or a `filed` one whose work has not landed, **carries forward**: the next run re-lists it and re-judges its severity. A finding that recurs across runs without progress is itself a signal.

### 2.5 The checklist is living; mechanical checks migrate to `vibe check` {#living}

A new defect class a run discovers is added to §2.2 as a permanent category, so the same gap is never re-missed — the M1.19 `vibe init` defect is what made A1's "untested default path" a permanent line. Conversely, a category that *can* be checked mechanically should, over time, migrate **into `vibe check`** (or `self-check.sh`, or CI) — converting a manual audit category into an automatic per-commit guard. The audit is the judgment-heavy *superset*; `vibe check` is the automated *subset* the audit keeps feeding. The long-run goal: each run finds *fewer* things the gate could have caught and *more* that genuinely need judgment.

---

## 3. Cadence {#cadence}

The audit is **owner-triggered**, with a recommended floor of **once per milestone** — run as part of, or immediately after, a milestone close-out, so a milestone is never declared "done" on an un-audited base. The owner re-runs it at will between milestones. The PROP deliberately fixes no calendar cron: the trigger is the owner's judgment plus the per-milestone floor.

A future refinement may add a trigger phrase (`АУДИТ` / `RUN AUDIT`) to the boot files, mirroring the `ЗАВЕРШИ СЕССИЮ` session-end command — recorded as an open question (§5.1), not wired by this PROP.

---

## 4. Running an audit {#running}

One run, start to finish:

1. **Open a section** in `AUDIT.md` dated today.
2. **Walk §2.2** category by category, breadth-first. Use the mechanical aids; read with judgment where there is no aid (A3 especially).
3. **Record every finding** — ID, category, description, severity, disposition.
4. **Carry forward** every still-`open` finding, and every `filed` one whose work has not landed, from the previous run; re-judge each.
5. **Fix the cheap P1 / P2 findings in-run** (normal commits); **file the rest**.
6. **Reconcile** the WAL "Known issues" with the run's `open` / `filed` findings, so the living checkpoint and the durable inventory agree.
7. **Commit** `AUDIT.md` as `docs(audit): <run-date> audit`, plus any in-run fixes as their own commits.

A run need not finish every fix — it must finish the *inventory*. Fixing is the work the inventory schedules.

---

## 5. Open questions {#open}

1. **Trigger phrase.** Add `АУДИТ` / `RUN AUDIT` to `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` as a recognised command, or keep the audit purely owner-narrated? (§3.)
2. **`vibe audit` aggregator.** Once enough §2.2 categories are mechanical, a `vibe audit` subcommand could run them and pre-fill the `AUDIT.md` skeleton — a FEAT worth opening when that threshold is reached.
3. **Health metric.** Should a run compute one coarse number — open P1 / P2 counts, the trend versus the previous run — so the project's direction is visible at a glance?

---

## 6. Rejected alternatives {#rejected}

- **A one-time test-hardening pass instead of a recurring process.** Rejected — a one-shot pass decays the day after it lands; rot resumes. The value is the *recurrence* and the *trend record*.
- **Rely on the per-commit gate / `vibe check` alone.** Rejected — the gate is a regression detector, blind by construction to uncovered code, out-of-gate trees, and drift. The M1.19 `vibe init` defect passed the gate on every commit.
- **Fully automate the audit now (ship `vibe audit` first).** Deferred, not rejected (§5.2). The audit's value is breadth plus judgment — "this test encodes a bug" (A3) is not mechanically detectable. Automation grows category by category (§2.5); it does not precede the process.
- **Track findings only in the WAL.** Rejected — the WAL is volatile, rewritten each session. The audit needs a durable, append-only history to show whether the project trends healthier or worse. Hence `AUDIT.md`.

---

## 7. Version history {#history}

- **2026-05-23 — draft 1, in force.** Owner-requested after the M1.19 session surfaced a milestone-grade defect — `vibe init` scaffolding broken projects — that the per-commit gate and ~800 hermetic tests missed. The process, the §2.2 checklist, the `AUDIT.md` inventory, the severity / disposition model, and the per-milestone cadence floor are defined here. The first (seed) run is recorded in [`AUDIT.md`](../../AUDIT.md).
