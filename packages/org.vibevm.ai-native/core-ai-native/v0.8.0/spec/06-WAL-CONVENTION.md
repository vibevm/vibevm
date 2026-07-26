# The WAL Convention — session-durable project state {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · status: BETA · T1 · language-neutral · OPTIONAL but preferred** @impl/done

##sessions-end-a-project-outlives-them *Agent sessions end, compact, and crash; a project outlives all of them.* @spec/done

##CONVENTION-KEEPS-STATE-IN-TWO-FILES *This
convention makes session boundaries cheap by keeping the project's living
state in two repository files.* @impl/done

##WAL-IS-OPTIONAL *It is **optional**: every Discipline procedure
that touches it ([Sweep §4](04-SWEEP-PLAYBOOK.md#output),
[Campaign Form §4](05-CAMPAIGN-FORM.md#resume), the shipped terraform/sweep
skills) carries an explicit without-WAL branch.* @impl/done

##WAL-IS-PREFERRED *It is **preferred** because the alternative — resumption
state scattered across commit messages and
plan-status lines — degrades as a project grows.* @impl/done

##ADOPT-WHEN-MORE-THAN-ONE-SESSION *Adopt it when more than one
session (or more than one operator) will ever touch the tree.* @impl/done

## 1. The two files {#files}

##WAL-FILE-DESCRIBES-THE-CURRENT-STATE **`spec/WAL.md` — the living checkpoint.** Describes the *current* state:
a dated standing line (what landed, gate-panel state, what's next, known
issues), plus a per-session section for the active campaign. @impl/done

##two-hard-rules-lead Two hard rules: @impl/done

- ##RULE-REWRITE-NOT-APPEND **Rewrite, not append.** The WAL is a checkpoint, not a log — its lead
  always describes *now*. History lives in git; an append-only WAL rots into
  an archive nobody reads. (Prior standing lines may be demoted into a
  PRIOR-tail as they age; the git log is the authoritative per-item record.) @impl/done
- ##RULE-THE-WAL-IS-CANONICAL **The WAL is canonical.** Where the WAL and any snapshot (CONTINUE, a plan's
  status line, a README) disagree, the WAL wins. @impl/done

##CONTINUE-FILE-IS-THE-COLD-RESUME-SNAPSHOT **`CONTINUE.md` (repo root) — the cold-resume snapshot.** Written at session
end for whoever picks up cold: TL;DR, where work stands (branch, sync,
tree state), the active blocker and the exact action that unblocks it, the
next-steps recipe with paths and commands, non-obvious findings, and the
recent commit chain. @impl/done

##CONTINUE-IS-OVERWRITTEN-WHOLESALE Overwritten wholesale each time — staleness compounds
otherwise. @impl/done

##CONTINUE-IS-A-SNAPSHOT-THE-WAL-SUPERSEDES-IT It is a *snapshot*; the WAL supersedes it. @impl/done

## 2. The freshness rule {#freshness}

##WAL-OLDER-THAN-24-HOURS-IS-STALE A WAL older than **24 hours** is stale: verify the recorded state against
reality (branch, gates, tree) before any destructive work, and say so to the
owner when the divergence matters. @impl/done

##FRESHNESS-ENFORCEMENT-IS-ADVISORY Tooling may enforce this advisorily (the
pilot's project linter warns on a stale WAL); the sweep's Tier-2 drift pass
checks it weekly regardless. @impl/done

## 3. Session boundaries {#boundaries}

- ##BOUNDARY-SESSION-END **Session end (wind-down):** update the WAL's standing line + session
  section; rewrite CONTINUE.md; commit both as their own topic commits.
  The test: a stranger with only the repository resumes without asking. @impl/done
- ##BOUNDARY-SESSION-RESUME **Session resume:** boot per the project's boot sequence, read the WAL,
  read CONTINUE.md, verify empirically, **report and wait** — a recorded
  "next step" is the candidate, not an authorisation; the owner steers. @impl/done
- ##BOUNDARY-MID-WORK-CHECKPOINTS **Mid-work checkpoints:** campaigns bump the WAL at phase boundaries
  ([Campaign Form §3–4](05-CAMPAIGN-FORM.md#gates)); sweeps bump it at
  milestone moves ([Sweep §4](04-SWEEP-PLAYBOOK.md#output)). @impl/done

## 4. Without a WAL {#without}

##OPT-OUT-STILL-OWES-THE-SAME-INVARIANT A project that opts out still owes the same invariant — **resumption state
lives in the repository, never in a session**. @impl/done

##the-fallbacks-lead The fallbacks the procedures
use: @impl/done

- ##FALLBACK-PLAN-STATUS-LINE a campaign PLAN carries a status line at its top, updated with each
  phase's commits; @impl/done
- ##FALLBACK-SWEEP-CLOSING-COMMIT a sweep's closing commit message carries the summary, and the committed
  health snapshot is the trend record; @impl/done
- ##FALLBACK-TERRAFORM-REGISTRIES the terraform skill's inventory registries (BROWNFIELD §3) hold what a WAL
  would have held about debt and intent. @impl/done

##fallbacks-work-but-are-weaker These fallbacks work; they are simply weaker — three places instead of one,
no single canonical "now". @spec/done

##SIGNAL-TO-ADOPT-THE-TWO-FILES When a without-WAL project notices it keeps
re-deriving its own state, that is the signal to adopt §1. @impl/done

## 5. Scope discipline {#scope}

##WAL-RECORDS-PROJECT-FACTS The WAL records *project* facts. @impl/done

##MACHINE-SCOPED-QUIRKS-BELONG-ELSEWHERE Machine-scoped quirks (shell behaviors, OS
footguns of one contributor's box) belong in that machine's user-scoped
notes or the project's boot user-override file — not in the WAL and not in
the Discipline's method documents. @impl/done

##KEEP-THE-THREE-LAYERS-APART Keep the three layers apart: method
(this package), project (WAL/CONTINUE), machine (user-owned boot snippet). @impl/done
