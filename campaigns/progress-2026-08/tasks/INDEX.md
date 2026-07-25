# Task index — campaign `progress-2026-08`

| id | title | executor | status |
|---|---|---|---|
| DRIFT-001 | cache prunes records that leave the observed scope | opus | done |
| DRIFT-002 | split progress-core parse.rs to hold the file budget (floor red) | opus | done |
| DRIFT-003 | campaign phase hardcoded "A" in the progress adapter | opus | done |
| DRIFT-004 | specmap learns ##<ID> fact anchors (owner commission) | opus | done |
| DRIFT-005 | spec compiler learns fact inheritance R1-R4 | opus | done |
| DRIFT-006 | the evidence provider reaches the report | opus | done |
| DRIFT-007 | `progress check` verifies that a fold was lossless | opus | done |
| DRIFT-008 | `campaign.json` carries the gate panel | opus | done |
| DRIFT-009 | baseline invalidation gets its other two rules | opus | done |
| DRIFT-010 | the subcommands take the incremental path | opus | done |
| DRIFT-011 | a blockquote can carry a fact anchor | opus | done |
| DRIFT-012 | the e2e harness stops drinking the developer's settings | opus | done |
| DRIFT-013 | `--plain`'s help stops describing a Phase 2 that shipped | opus | done |
| DRIFT-014 | three deviate reasons stop denying the shipped resolver | opus | done |
| DRIFT-015 | the progress adapter goes back under its file budget | opus | superseded |
| DRIFT-016 | the irreplaceable stays in git, the acceleration leaves the repo | opus | done |
| DRIFT-017 | a run that changes nothing writes nothing | opus | queued |
| DRIFT-018 | the last two test files stop touching the real user home | opus | queued |
| DRIFT-019 | three module docs stop describing the pre-port world | opus | done |

DRIFT-006…011 are Phase E, opened by the owner's 2026-07-25 ruling on the
seven F-046 parity rows (wire, not demote) plus F-015. They close the last
drift rows of wave 1's ledger — the spec already promises each behaviour, so
these tasks make the promise true rather than softening it.

DRIFT-012…014 are the code-side findings the verification phases raised
against the tree rather than against the prose: a test that reads the
developer's real settings (F-055), a `--help` string describing a phase that
shipped (F-036), and three `#[spec(deviates)]` reasons that deny the resolver
actually running in production (F-047). The last of those is the sharpest —
a stale reason is a defect wearing the discipline's own badge.

DRIFT-016 and DRIFT-017 come from the owner's 2026-07-25 ruling on what
DRIFT-010 surfaced against itself. 016 separates the two things that had
been sharing one file — irreplaceable verdicts and an erasable parse
payload — and sends the payload out of the repository entirely, keyed by
branch, so repository weight stops being a constraint on how much the
campaign may cache. 017 takes the lever DRIFT-010 identified and declined
to pull: parsing was never the expensive part, the JSON writes are, and a
run that changes nothing should write nothing. 017 is blocked on 016
because they rewrite the same paths.

DRIFT-NNN (coding, Opus) and SPEC-NNN (spec stitching, budget-dependent)
task files live beside this index. Formats:
`spec/modules/vibe-progress/templates/impl-task.md` /
`templates/spec-task.md`. Statuses mirror into `run/state/tasks.json`.
