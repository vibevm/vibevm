# Task index — campaign `packages-2026-09`

| id | title | executor | status |
|---|---|---|---|
| DRIFT-031 | the parser stops swallowing two units it can already see | opus | queued (F-083 + F-084) |
| MARKUP-B1 | `core-ai-native` v0.8.0, guiding + operating layer | opus | **done** — B1a + B1b, 417 units over 9 files |
| DRIFT-030 | the hoist counter learns that the root is a consumer too | opus | returned (§8 stop) — counter necessary, not sufficient; design call parked |
| DRIFT-029 | a slot's boot artifacts are spec, not drift | opus | returned (§8 stop) — superseded by DRIFT-030 |
| DRIFT-028 | a unit can be void; the names query stopped on its own rule | opus | partial (§4.1 done, §4.2 stopped) |
| DRIFT-027 | one config home, the way there is one credential home | opus | done |
| DRIFT-024 | the scope stops observing what it must not mark | opus | done |
| DRIFT-025 | the progress adapter splits before it is forced to | opus | done |
| DRIFT-026 | sealing a verdict stops depending on memory | opus | done |

DRIFT-024 is the exception to "Phase A is not tasks": it comes out of the
Phase A pilot, where `check --exhaustive` turned out to demand fact markup on
33 files of verbatim licence text (F-070) and on three derived indexes that
call hand edits a defect (F-071). Neither can be expressed in `progress.toml`,
because §4 is include-only by design — so the exclusions need code, and one of
them needs a §4 amendment. Until it lands, Phase B's exit gate cannot be
reached honestly.

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
