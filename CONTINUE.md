# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-10 at the terraform merge checkpoint. The Big
Refactoring is **complete**: every phase of
[`spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`](spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md)
executed (−1/0/1/2/3/4/5/6), owner-declared done, `new` merged to
`main` with `--no-ff`. This file carries what a cold session needs._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and
> the WAL disagree, the WAL wins. Boot first (`CLAUDE.md` →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

The terraform delivered, on top of the Phase −1/0/1 base from earlier
the same day: the **vibe-resolver item-grain backfill** (54 approved
proposals, six affirmation commits, three honest deviates edges), the
**orphan ratchet** (blocking `specmap --check` for ten crates, 8
reasoned exemptions, per-symbol debt dispositions), **cells v0**
(`#[cell]` manifests, the vibe-cli selection registry, a hermetic
differential oracle over real `file://` git repos), the **conform
engine MVP** (facts store with 1-file-diff incrementality, three rules,
SARIF, frozen-findings baseline), the **local ledger** (epoch-keyed
`--prose` cache with provenance lines), **scope-grade backfill of 98
modules**, intent reconciliation to **0 unaccounted**, an instrumented
category-C audit, and [`terraform/REPORT.md`](terraform/REPORT.md) —
the close-out with metrics vs BASELINE and the eight-item
what-went-wrong list.

## Where work stands

- **`main`** carries the merge commit (the `--no-ff` of ~30 terraform
  commits from `new`); pushed to `origin/main`. `new` retained, pushed.
- Gate panel, all green at merge: `cargo xtask specmap --check`
  (489 units / 170 items / 177 edges / 0 suspects + orphan ratchet 0
  gated / 6 dispositioned), `cargo xtask conform check` (0 new, 6
  frozen unsafe), `cargo xtask test-gate` (1075, xfail-strict), golden
  byte-identical, `tools/self-check.sh` all four steps.

## Quick-start (the gate panel, in run order)

```sh
cargo xtask specmap --check     # index + orphan ratchet
cargo xtask conform check       # facts → rules → SARIF → baseline
cargo xtask test-gate           # nextest, xfail-strict
bash tools/self-check.sh        # fmt, tests, clippy -D warnings, vibe check
cargo xtask trace explain <symbol|uri> [--text|--json|--prose]
cargo xtask tripwire            # debt watches over the change set
```

## What the next session most likely picks up (owner's call)

1. **PROP-010 design session** (INT-0003) — the local package cache's
   five §5 open questions; M1.20.
2. **The SAT solver** (DBT-0011) — now staring back from deviates
   edges on `DepSolver` and `NaiveDepSolver`; the differential-oracle
   harness is ready to take the naive/sat pair.
3. **`VIBEVM-SPEC.md` unit-ification** (DBT-0019) — unblocks
   vibe-cli's item-grain backfill (the root spec is owner-frozen and
   outside `spec/**`, so this is an owner decision).
4. **The full PROP-013 audit** (INT-0001) — category C is now
   machine-fed; the manual breadth sweep remains.
5. **Discipline package v0.2** — REPORT.md's honest list is the input;
   `spec/neworder/` edits are the owner's to make.

## Non-obvious findings (this session)

- **The ratchet caught what the sweep missed** within minutes of being
  flipped (PredicateError, PRP-0054) — exclude *items*, never *files*.
- **scope! cannot tag the bootstrap pair**: specmark/specmark-grammar
  would need a dependency cycle; tagging them is a scanner-level
  feature. They stay exempt with that reason recorded.
- **Doc-strings lie quietly**: `expand_features` claims cycles are
  "rejected" (they terminate silently — AUD-0014); `ResolvedNode`
  cites PROP-008 §2.3 where the identity anchor is §2.2 (AUD-0015).
- **`git -c init.defaultBranch=main` + bare clones** make a fully
  hermetic multi-registry fixture; `package_repo_url` happily composes
  plain Windows paths (`<dir>/org.vibevm.<name>.git`).
- **538 → 0 orphans in one pass** because module docs in this repo
  already cite their PROP homes — the scope!-grade sweep just made the
  claims machine-checkable.
- Owner drops kept arriving mid-session (Go, 4× Java, Kotlin guides);
  the DBT-0016 watch caught each one — commit them on sight or
  `--check` drifts.

## Repository map (delta vs pre-terraform)

```
vibevm/
├── specmap.json                  ← committed index (489/170/177)
├── specmap-ratchet.json          ← orphan gate: exemptions + dispositions
├── conform-baseline.json         ← frozen conform findings (6 unsafe)
├── schemas/specmap.jtd.json
├── crates/
│   ├── specmark-grammar/ specmark/ specmap-core/   ← tags, index, ratchet, ledger
│   ├── conform-core/ conform-frontend-rust/        ← the conform engine (Phase 4)
│   ├── vibe-cli/src/registry.rs                    ← cell-selection registry (R-001)
│   └── vibe-resolver/                              ← fully item-grain backfilled
├── xtask/                        ← specmap/test-gate/tripwire/trace/conform
├── terraform/                    ← BASELINE, PHASE1-PILOT, LOG, REPORT, registries, golden
├── .ledger/                      ← (git-ignored) interpretations cache + telemetry
└── spec/neworder/                ← discipline package + 6 new language guides
```

## Decisions in force (terraform legacy, beyond the four rules)

- Gates are the merge criterion: specmap --check (with ratchet) +
  conform check + test-gate + self-check, all green before a commit
  lands.
- Proposals-then-affirmation for every backfill tag (PROP-014 §2.7);
  the proposals file records APPROVEs as the audit trail.
- xfail-strict semantics; tests-baseline shrinks only via promotion.
- Tripwires are read, not muted; owner spec-drops are committed on
  sight.
- No CI by standing owner decision (INT-0017 rejected); every gate is
  a local command.
- `.ledger/` and `target/conform/` are derived, never committed.

Session-resume phrase: `восстанови сессию`. The WAL supersedes this
snapshot wherever they diverge.
