# Phase B batch plan — campaign `packages-2026-09`

_Recomputed 2026-07-26 against **live version slots only**. The first version
of this plan measured 250 files / 8 463 facts and was wrong by a quarter: it
counted two superseded slots that nothing resolves to._

## The workload, after three rounds of finding out what was in it

**206 files** across 37 packages; the observed corpus is **264 files**
(58 host + 206 packages), confirmed by `progress check`.

**Unmarked facts remaining: 5 685**, measured 2026-07-26 with
`check --exhaustive` after B1a. The figure this file carried before was 6 561,
quoted before F-080's eleven files left and before anything was marked; it was
replaced by a measurement rather than by subtraction, because the campaign's own
rule is that a number quoted before its decomposition is a guess wearing a
decimal point — and that rule binds its own numbers first. Re-measure at each
batch boundary rather than decrementing.

Getting to that number took **four** corrections, and the pattern in them is
worth more than the number:

| what came out | files | facts | why |
|---|---|---|---|
| `vibedeps/`, `.vibe/cache`, `refs`, `fixtures` | 970 | — | machine-copied; ~71 % of all markdown under `packages/` |
| `LICENSE.md` × 33 (F-070) | 33 | 264 | verbatim third-party text |
| three derived indexes (F-071) | 3 | 265 | "hand edits are a defect", their own words |
| `core-ai-native/v0.7.0`, `redbook/v0.1.0` | 33 | 1 908 | superseded slots — frozen history |
| `spec/legacy-projections/` (F-080) | 11 | *recount* | frozen history — owner ruling 2026-07-26 |

**Every one of these was found by asking what the corpus is made of, not by
estimating how big it is.** The first two rounds were prompted by the owner
asking whether generated content had really been kept out; the third by
measuring §3.3 against the exhaustive gate. A number quoted before that
decomposition is a guess wearing a decimal point.

## Shape

Two facts decide the batching, and neither is visible from the file count:

- **`core-ai-native` is still the largest single package** — 16 files after
  F-080, down from 27, down from 56 before its superseded slot left. Three
  subtractions have now hit this one package; each time the remainder was the
  part a consumer actually resolves.
- **File count lies about cost.** `redbook` is 6 files but 470 facts (78/file);
  `go-ai-native-lang` is 19 files but 411 (22/file). Batching by file count
  would put a 3.5× cost difference in the same-sized box.

## Batches, largest first

| # | Batch | Files | Facts |
|---|---|---|---|
| ~~B1–B2~~ | `core-ai-native` (live slot) — **DONE**, 943 units | 16 | — |
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

**Fifteen batches** against the plan's estimated 12–15, against this file's own
earlier 18, and against the sixteen it read before F-080 retired
`core-ai-native`'s third batch. `rust-ai-native` is already marked — it was the
Phase A pilot.

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
- **F-082 — OPEN, owner's call, and it recurs across the whole wave.** Marking a
  package's **boot snippet** grew `10-flow-core-ai-native.md` from 29 to 44
  lines, +52 %. That file is installed into every *consuming project's* boot
  lane and read at every session start, and its own
  `##DO-NOT-READ-ALL-AT-BOOT` rule is minimal sufficiency — so our campaign's
  markup rides into every consumer's context window permanently. Roughly thirty
  packages ship one. The host's own authored boot snippets (`00-core.md`,
  `90-user.md`) *are* marked and in the corpus, which is the precedent pointing
  the other way; the difference is that those are read by host sessions only.
  **Marking proceeds meanwhile** — B1a's snippet is marked and the cost is
  measured rather than assumed, which is what makes the ruling decidable.

- **Frozen history leaves the corpus; it is not marked.** Owner ruling
  2026-07-26 (F-080) puts `spec/legacy-projections/` in the same category as a
  superseded version slot: «замороженная история… Сейчас у нас есть активные
  rust, typescript и go.» The three active languages are covered by their own
  `-lang` stacks. The exclusion is genre-shaped in `progress.toml`, so a future
  version slot cannot silently re-admit it, and the languages return as
  *includes* when a stack for them lands.

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
