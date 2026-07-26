# Phase B batch plan — campaign `packages-2026-09`

_Measured 2026-07-26 from the corpus projection, after DRIFT-024's exclusions.
Counts are **provisional** until that task is reviewed and committed; the
**shape** is not, and the shape is what this plan is for._

## The workload, measured rather than estimated

**250 files · 8 463 unmarked facts** across 37 packages. Two facts about the
distribution decide the batching, and neither is visible from the file count:

- **`core-ai-native` is 35 % of the entire campaign in one package** — 54 files,
  **2 967 facts**. The plan's §5-B guess was "as much as five of them"; measured,
  it is as much as the *twenty smallest packages put together*.
- **File count lies about cost.** `redbook` is 12 files but 898 facts (75/file);
  `go-ai-native-lang` is 19 files but 411 (22/file). Batching by file count
  would put a 4× cost difference in the same-sized box.

## Batches, largest first

Largest-first is deliberate: the expensive package sets the pattern while
attention is freshest, and every later batch is cheaper than the last.

| # | Batch | Files | Facts | Notes |
|---|---|---|---|---|
| B1–B5 | `core-ai-native` | 54 | 2 967 | **split into five**; one package, five sittings |
| B6 | `redbook` | 12 | 898 | two version slots — §3.3: judge the live one, mark v0.1.0 as frozen history |
| B7 | `go-ai-native-lang` | 19 | 411 | |
| B8 | `typescript-ai-native-lang` | 18 | 338 | |
| B9 | `rust-ai-native-lang` | 18 | 312 | pilot genre — the pattern is already set |
| B10 | `discovery-prompt` + `decision-records` | 9 | 455 | |
| B11 | `spec-genres` + `wal` + `addressable-specs` | 17 | 578 | |
| B12 | `health-audit` + `conflict-protocol` + `managed-blocks` | 16 | 488 | |
| B13 | `source-mirrors` + `tool-design-lessons` + `qualified-naming` | 15 | 451 | |
| B14 | `campaign-plans` + `two-process-model` + `operating-modes` | 15 | 403 | |
| B15 | `git-attribution-policy` + `secrets-hygiene` + `comparative-research` | 15 | 378 | |
| B16 | `sync-from-code` + `licensing` + `manual-tests` | 16 | 339 | |
| B17 | the git family + `wal-specspaces` + `dev-runtime-docs` | 17 | 300 | small, uniform, fast |
| B18 | the three `-mcp` packages + two family umbrellas | 7 | 145 | `rust-ai-native` already marked (pilot) |

**Eighteen batches**, against the plan's estimated 12–15 — the difference is
entirely `core-ai-native` needing five.

## What each batch owes

Per plan §5-B, and no more: paragraph-exhaustive fact-grain markers,
sense-preserving re-splits, missing `{#anchor}`s, `audience` where obvious.
**Semantic edits are forbidden** — a semantic problem found becomes a finding,
not a fix. Batch diffs contain markers, splits and anchors only.

Two rules this campaign adds to wave 1's:

- **§3.3 — superseded slots are marked, never verified.** `redbook` v0.1.0 and
  `core-ai-native` v0.7.0 get a single document-level marker recording that they
  are frozen history. Do not mark them fact-by-fact; it costs the same as a live
  contract and buys nothing, and fresh-looking verdicts on a dead slot invite a
  reader to act on it.
- **F-069 is Phase C's problem, not this phase's.** An aggregator's facts are
  about other packages, and whether this document can be their source of truth
  is a question about the *verdict*, not the *marker*. Mark stage and state
  normally and move on.

## Gate

`check --exhaustive` green over **both** corpora, and the host's 58 files must
stay at 0 unmarked — wave 2 does not un-measure wave 1. Write `baseline.json`
at the batch boundary (amendment A6): the writer exists now, so a crash costs
O(delta) rather than the batch.
