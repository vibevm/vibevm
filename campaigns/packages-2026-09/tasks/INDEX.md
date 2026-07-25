# Task index — campaign `packages-2026-09`

| id | title | executor | status |
|---|---|---|---|

*(empty — Phase A is scope and prerequisite work, not tasks. The first entries
arrive from Phase C's ledger.)*

Wave-2 DRIFT tasks differ from wave 1's in one way that must appear in every
task's acceptance: **a fix inside a package's crates has to be vendored forward**
to every family member that copies it (`cargo xtask sync-engines`), or the fix
ships to one consumer and not the others. A task whose acceptance does not say
so is incomplete (plan §5-E).

The second wave-2 rule, from §5-D: **a finding that spans a package boundary is
a release event.** Fixing `core-ai-native`'s prose may require a version bump
and a re-vendor into three family members — such a finding is not closed by an
edit, it is closed by a published version.

DRIFT-NNN (coding, Opus) and SPEC-NNN (spec stitching) task files live beside
this index. Formats: `spec/modules/vibe-progress/templates/impl-task.md` /
`templates/spec-task.md`. Statuses mirror into `run/state/tasks.json` — and
wave 1 learned that nothing refreshes that file automatically, so it drifts
unless a session updates both. It sat 18 tasks stale for a week.
