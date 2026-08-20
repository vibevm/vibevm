# The WAL Convention — session-durable project state {#root}
**Discipline v0.2 · status: BETA · T1 · language-neutral · OPTIONAL but preferred**

*Agent sessions end, compact, and crash; a project outlives all of them. This
convention makes session boundaries cheap by keeping the project's living
state in two repository files. It is **optional**: every Discipline procedure
that touches it ([Sweep §4](04-SWEEP-PLAYBOOK.md#output),
[Campaign Form §4](05-CAMPAIGN-FORM.md#resume), the shipped terraform/sweep
skills) carries an explicit without-WAL branch. It is **preferred** because
the alternative — resumption state scattered across commit messages and
plan-status lines — degrades as a project grows. Adopt it when more than one
session (or more than one operator) will ever touch the tree.*

## 1. The two files {#files}

**`spec/WAL.md` — the living checkpoint.** Describes the *current* state:
a dated standing line (what landed, gate-panel state, what's next, known
issues), plus a per-session section for the active campaign. Two hard rules:

- **Rewrite, not append.** The WAL is a checkpoint, not a log — its lead
  always describes *now*. History lives in git; an append-only WAL rots into
  an archive nobody reads. (Prior standing lines may be demoted into a
  PRIOR-tail as they age; the git log is the authoritative per-item record.)
- **The WAL is canonical.** Where the WAL and any snapshot (CONTINUE, a plan's
  status line, a README) disagree, the WAL wins.

**`CONTINUE.md` (repo root) — the cold-resume snapshot.** Written at session
end for whoever picks up cold: TL;DR, where work stands (branch, sync,
tree state), the active blocker and the exact action that unblocks it, the
next-steps recipe with paths and commands, non-obvious findings, and the
recent commit chain. Overwritten wholesale each time — staleness compounds
otherwise. It is a *snapshot*; the WAL supersedes it.

## 2. The freshness rule {#freshness}

A WAL older than **24 hours** is stale: verify the recorded state against
reality (branch, gates, tree) before any destructive work, and say so to the
owner when the divergence matters. Tooling may enforce this advisorily (the
pilot's project linter warns on a stale WAL); the sweep's Tier-2 drift pass
checks it weekly regardless.

## 3. Session boundaries {#boundaries}

- **Session end (wind-down):** update the WAL's standing line + session
  section; rewrite CONTINUE.md; commit both as their own topic commits.
  The test: a stranger with only the repository resumes without asking.
- **Session resume:** boot per the project's boot sequence, read the WAL,
  read CONTINUE.md, verify empirically, **report and wait** — a recorded
  "next step" is the candidate, not an authorisation; the owner steers.
- **Mid-work checkpoints:** campaigns bump the WAL at phase boundaries
  ([Campaign Form §3–4](05-CAMPAIGN-FORM.md#gates)); sweeps bump it at
  milestone moves ([Sweep §4](04-SWEEP-PLAYBOOK.md#output)).

## 4. Without a WAL {#without}

A project that opts out still owes the same invariant — **resumption state
lives in the repository, never in a session**. The fallbacks the procedures
use:

- a campaign PLAN carries a status line at its top, updated with each
  phase's commits;
- a sweep's closing commit message carries the summary, and the committed
  health snapshot is the trend record;
- the terraform skill's inventory registries (BROWNFIELD §3) hold what a WAL
  would have held about debt and intent.

These fallbacks work; they are simply weaker — three places instead of one,
no single canonical "now". When a without-WAL project notices it keeps
re-deriving its own state, that is the signal to adopt §1.

## 5. Scope discipline {#scope}

The WAL records *project* facts. Machine-scoped quirks (shell behaviors, OS
footguns of one contributor's box) belong in that machine's user-scoped
notes or the project's boot user-override file — not in the WAL and not in
the Discipline's method documents. Keep the three layers apart: method
(this package), project (WAL/CONTINUE), machine (user-owned boot snippet).
