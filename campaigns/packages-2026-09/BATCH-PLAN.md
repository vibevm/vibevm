# Phase B batch plan — campaign `packages-2026-09`

_Recomputed 2026-07-26 against **live version slots only**. The first version
of this plan measured 250 files / 8 463 facts and was wrong by a quarter: it
counted two superseded slots that nothing resolves to._

## The workload, after three rounds of finding out what was in it

**217 files · 6 561 facts** across 37 packages; the observed corpus is
**275 files** (58 host + 217 packages).

Getting to that number took three corrections, and the pattern in them is worth
more than the number:

| what came out | files | facts | why |
|---|---|---|---|
| `vibedeps/`, `.vibe/cache`, `refs`, `fixtures` | 970 | — | machine-copied; ~71 % of all markdown under `packages/` |
| `LICENSE.md` × 33 (F-070) | 33 | 264 | verbatim third-party text |
| three derived indexes (F-071) | 3 | 265 | "hand edits are a defect", their own words |
| `core-ai-native/v0.7.0`, `redbook/v0.1.0` | 33 | 1 908 | superseded slots — frozen history |

**Every one of these was found by asking what the corpus is made of, not by
estimating how big it is.** The first two rounds were prompted by the owner
asking whether generated content had really been kept out; the third by
measuring §3.3 against the exhaustive gate. A number quoted before that
decomposition is a guess wearing a decimal point.

## Shape

Two facts decide the batching, and neither is visible from the file count:

- **`core-ai-native` is 23 % of the work** — 27 files, 1 487 facts. It was 35 %
  until its superseded slot left. Still the largest by a wide margin.
- **File count lies about cost.** `redbook` is 6 files but 470 facts (78/file);
  `go-ai-native-lang` is 19 files but 411 (22/file). Batching by file count
  would put a 3.5× cost difference in the same-sized box.

## Batches, largest first

| # | Batch | Files | Facts |
|---|---|---|---|
| B1–B3 | `core-ai-native` (live slot) | 27 | 1 487 |
| B4 | `redbook` (live slot) | 6 | 470 |
| B5 | `go-ai-native-lang` | 19 | 411 |
| B6 | `typescript-ai-native-lang` | 18 | 338 |
| B7 | `rust-ai-native-lang` | 18 | 312 |
| B8 | `discovery-prompt` + `decision-records` | 9 | 455 |
| B9 | `spec-genres` + `wal` + `addressable-specs` | 17 | 578 |
| B10 | `health-audit` + `conflict-protocol` + `managed-blocks` | 16 | 488 |
| B11 | `source-mirrors` + `tool-design-lessons` + `qualified-naming` | 15 | 451 |
| B12 | `campaign-plans` + `two-process-model` + `operating-modes` | 15 | 403 |
| B13 | `git-attribution-policy` + `secrets-hygiene` + `comparative-research` | 15 | 378 |
| B14 | `sync-from-code` + `licensing` + `manual-tests` | 16 | 339 |
| B15 | the git family + `wal-specspaces` + `dev-runtime-docs` | 17 | 300 |
| B16 | three `-mcp` packages + two family umbrellas | 8 | 145 |

**Sixteen batches** against the plan's estimated 12–15, and against this file's
own earlier 18. `rust-ai-native` is already marked — it was the Phase A pilot.

## What each batch owes

Per plan §5-B and no more: paragraph-exhaustive fact-grain markers,
sense-preserving re-splits, missing `{#anchor}`s, `audience` where obvious.
**Semantic edits are forbidden** — a semantic problem found becomes a finding.
Batch diffs contain markers, splits and anchors only.

Two rules that are easy to lose mid-batch:

- **Superseded slots are out of scope, not marked.** §3.3 rules them "marked,
  never verified", and `--exhaustive` cannot express that — it demands a marker
  per paragraph and one document marker does not satisfy it. They leave the
  corpus instead. Owner policy (2026-07-26) keeps this rare: **a package
  version is not bumped on every change** — the source text is edited in place
  and `vibe update` re-materialises consumers.
- **F-069 is Phase C's, not this phase's.** An aggregator's facts are about
  other packages; whether this document can be their source of truth is a
  question about the *verdict*, not the *marker*. Mark stage and state and
  move on.

## Gate

`check --exhaustive` green over **both** corpora, and the host's 58 files stay
at 0 unmarked — wave 2 does not un-measure wave 1. Write `baseline.json` at the
batch boundary (amendment A6).

**Unverified assumption, flagged rather than relied on:** `vibe update` as the
re-materialisation path is the owner's account of his own tool and has not been
exercised in this campaign. Run it once before Phase B leans on it.
