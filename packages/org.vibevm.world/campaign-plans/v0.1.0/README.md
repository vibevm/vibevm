# `flow:campaign-plans` — cold-executable campaign plans

A vibevm `flow` package that installs the **campaign plan** format
into a project: any multi-commit change too big for one session is
planned as a single document that a fresh session — or a different
person — can execute with no memory of the planning conversation.
Frozen baseline arithmetic, falsifiable predictions, phases gated on
the project's green floor, an execution ledger of hash-level
evidence, and a deferrals ledger that seeds the next campaign's
mandate.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md` — what a
  campaign is, the five artifact roles (PLAN / BASELINE /
  PREDICTIONS / LOG / REPORT), and the canonical fifteen-section
  plan skeleton, each section with a spec and a worked mini example.
- `spec/flows/campaign-plans/phase-gates.md` — Phase 0 spike
  discipline (no commits; a red spike rewrites the affected decision
  before anything lands), phase anatomy, the safe-stop law,
  resumability, review points, discovered-necessary work.
- `spec/flows/campaign-plans/execution-ledger.md` — the record half:
  status-line lifecycle, the prepended execution record, per-phase
  commit maps, honesty rules, the closing report, the deferrals
  ledger and the lineage law.
- `spec/boot/40-flow-campaign-plans.md` — boot snippet loaded at
  session start: when to propose a campaign, the phase-boundary
  checklist, and the never-do list.

## Install

```bash
vibe install flow:campaign-plans
```

## Uninstall

```bash
vibe uninstall flow:campaign-plans
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Canonical home

This package is the canonical home of the campaign-plan format. The
AI-Native Discipline (`flow:core-ai-native`) ships a campaign-form
document that defers to this package from its next release.

## Composition

- `flow:wal` — a campaign checkpoint updates the WAL's standing line
  at every phase boundary; the plan file, not the WAL, carries the
  campaign detail.
- `flow:atomic-commits` — each phase's commit set follows it:
  subjects are spelled in the plan, one idea per commit, and the
  ledger binds hashes to the planned subjects.
- `flow:decision-records` — the plan's D-sections are decision
  records inline; rejected options carry reasons so nobody re-opens
  a settled question mid-campaign.
- `flow:conflict-protocol` — review points are the campaign-scale
  REVIEW marker: OPEN with options, then RESOLVED with the owner's
  ruling verbatim.

## Philosophical background

The format crystallized over a dozen real campaigns in the origin
project — a debt drain that took a frozen baseline from 130 findings
to 10, a package-family rename across five packages, a six-wave
subsystem landing whose execution corrected two of the plan's own
decisions and said so in the ledger. Each was executed cold, several
across session boundaries, and the format kept what survived contact
with execution. Its spirit is the book's "write for the system, not
for yourself": the plan is a program for whoever runs it next, and
the ledger is that program's output (*AI-native development*, ships
in Russian inside `flow:redbook` at `spec/book/ru/`).

## License

UPL-1.0. See [LICENSE.md](LICENSE.md).
