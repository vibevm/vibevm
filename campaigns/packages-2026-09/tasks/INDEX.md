# Task index — campaign `packages-2026-09`

| id | title | executor | status |
|---|---|---|---|
| MARKUP-B15 | the git family + `wal-specspaces` + `dev-runtime-docs` | opus | **done** — 409 units, **0 unmarked**; **band broken** (1.025) and explained; rulings 57–58 |
| MARKUP-B14 | `sync-from-code` + `licensing` + `manual-tests` | opus | **done** — 479 units, **0 unmarked**; band held twice running (1.089); rulings 53–56; F-102 fixed under it |
| MARKUP-B13 | `git-attribution-policy` + `secrets-hygiene` + `comparative-research` | opus | **done** — 555 units, **0 unmarked**; band held (1.113); rulings 51–52 |
| MARKUP-B12 | `campaign-plans` + `two-process-model` + `operating-modes` | opus | **done** — 624 units, **0 unmarked**; counting rule written down; coefficient band 1.07–1.15 |
| MARKUP-B11 | `source-mirrors` + `tool-design-lessons` + `qualified-naming` | opus | **done** — 682 units, **0 unmarked**; **falsified density**, found the sentence mechanism |
| MARKUP-B10 | `health-audit` + `conflict-protocol` + `managed-blocks` | opus | **done** — 700 units, **0 unmarked**; **falsified the sizing constant** (×2.365); F-097 widened |
| MARKUP-B9 | `spec-genres` + `wal` + `addressable-specs` | opus | **done** — 776 units, **0 unmarked**; sizing rule corrected to 3 constants; rulings 39–41 |
| MARKUP-B8 | `discovery-prompt` + `decision-records` | opus | **done** — 366 units, **0 unmarked** (a first); ×1.28 measured; rulings 34–38 locked; F-097 |
| MARKUP-B7 | `rust-ai-native-lang` v0.7.0 | opus | **done** — 544 of 546 units over 18 files; all 3 predictions held; ruling 33 locked |
| DRIFT-037 | a skill's frontmatter stops being mistaken for prose | opus | **done** — F-092 closed; −9 exactly, three stacks reach 0; §2 review point still owner's |
| MARKUP-B6 | `typescript-ai-native-lang` v0.6.0 | opus | **done** — 579 of 581 units over 18 files; 2 F-092 frontmatter units left; rulings 30–32 locked |
| MARKUP-B5 | `go-ai-native-lang` v0.1.0 | opus | **done** — 663 of 665 units over 19 files; 2 F-092 frontmatter units left unmarked |
| MARKUP-B2 | `core-ai-native` v0.8.0, mechanisms + appendix | opus | **done** — 526 units over 7 files; the live slot closed at 943 |
| DRIFT-036 | both package gates learn their own denominator | opus | **done** — F-086 closed; sync 6/6, floor 7/7, guards fired |
| DRIFT-035 | the boot lane carries each statically-linked package once | opus | queued (F-078 c, measure first) |
| DRIFT-034 | a heading anchor may be written the way a fact id may | opus | re-scoped, queued — widening only; fold ruled out |
| DRIFT-033 | campaign state stops being believed on its own word | opus | **done in part** — F-075 was already closed; F-077's `counters` stopped on spec |
| DRIFT-032 | a `spec://` URI can address a normative fact | opus | **done** — F-085 a, floor green, 4 copies synced |
| DRIFT-031 | the parser stops swallowing two units it can already see | opus | **done** — F-083 + F-084, floor green |
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
