# Phase B batch plan — campaign `packages-2026-09`

_Recomputed 2026-07-26 against **live version slots only**. The first version
of this plan measured 250 files / 8 463 facts and was wrong by a quarter: it
counted two superseded slots that nothing resolves to._

## The workload, after three rounds of finding out what was in it

**202 files** across 37 packages; the observed corpus is **260 files**
(58 host + 202 packages), confirmed by `progress scan`.

**Sizing: THREE constants, measured twice and stable to 0.7 % — corrected at B9.**
Size a `world` batch as **paragraphs × 2.13 + pre-existing list items × 1.00 +
table cells × 1.00**. Only paragraphs multiply: items and cells are already at
fact grain, and ruling 1 keeps a pre-existing item whole. Measured paragraph
heirs — B8 **71 → 151 (×2.127)**, B9 **179 → 378 (×2.112)**. On B9's real
composition the three constants predict **779** against a measured **776**
(+0.4 %), where B8's two-constant rule said 797 (+2.7 %), a blended ×1.28 said
739 (−4.8 %) and the language-stack ×1.7 said 981 (+26 %).

**Why two constants were not enough, and it is worth keeping:** the old rule's
single «prose» constant lumped paragraphs and list items, which behave nothing
alike. B9's prose is 57 % items against B8's 53 %, and that 4 points is the
whole 2.6 % miss. **`progress check` reports only a total today** — the
para/item/cell split has to be counted from the gate log's «Para unit» / «Item
unit» / «Cell unit» text, and a subcommand that reported it directly would make
this sizing a command rather than a ritual.

**Superseded — kept for the reasoning, which still holds:**
The language stacks came in at ×1.62 / ×1.72 / ×1.75, but B8's `world` flows
measured **×1.28**, and the cause is structural: **47.6 % of their units are
table body cells**, which are already at fact grain and cannot deconstruct. On
prose alone B8 was **×1.53**. So size a `world` batch as
**prose × 1.53 + cells × 1.00**, not by a blended constant — per file B8 ranged
×1.10 (a template that is 72 % cells) to ×1.73 (a README with no tables), and a
single number would mis-size an individual batch by up to 35 %. **Count the
cells before sizing B9–B16.**

**The language-stack figure, for those batches only — multiply by ~1.7.**
The `facts` column below is the **pre-markup scan count**, and the deconstruction
law then turns paragraphs into units. B5's `go-ai-native-lang` scanned at 411 and
finished at **665** (×1.62); B6's `typescript-ai-native-lang` scanned at 338 and
finished at **581** (×1.72). The scan number is exact as a starting measurement
and under-predicts the batch by **62–72 %**. Size B7–B16 with **×1.7**, and treat
that as a range rather than a constant — two points do not make a law, and both
were language stacks. **The `world` flows of B8–B16 are a different genre and may
land outside it**; re-measure at the first of them rather than assuming.

**Unmarked facts remaining: 4 685**, re-measured 2026-07-26 at the B2 boundary,
after F-091 excluded the book. Earlier figures in this file's history — 6 561,
then 5 685, then 5 068 — were each replaced by a **measurement**, never by
subtracting an estimate, because the campaign's own rule about numbers quoted
before their decomposition binds its own numbers first. **Re-measure at every
batch boundary rather than decrementing.**

Getting to that number took **six** corrections, and the pattern in them is
worth more than the number:

| what came out | files | facts | why |
|---|---|---|---|
| `vibedeps/`, `.vibe/cache`, `refs`, `fixtures` | 970 | — | machine-copied; ~71 % of all markdown under `packages/` |
| `LICENSE.md` × 33 (F-070) | 33 | 264 | verbatim third-party text |
| three derived indexes (F-071) | 3 | 265 | "hand edits are a defect", their own words |
| `core-ai-native/v0.7.0`, `redbook/v0.1.0` | 33 | 1 908 | superseded slots — frozen history |
| `spec/legacy-projections/` (F-080) | 11 | ~729 | frozen history — owner ruling 2026-07-26 |
| `spec/book/**` (F-091) | 4 | 383 | reference depth, not a contract — owner ruling 2026-07-26 |
| `DISCOVERY-PROMPT.md` (F-096) | 1 | 169 | a prompt payload, not a claim — owner ruling 2026-07-27 |

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
| ~~B5~~ | `go-ai-native-lang` — **DONE**, 665 units | 19 | 411 |
| ~~B6~~ | `typescript-ai-native-lang` — **DONE**, 581 units (×1.72) | 18 | 338 |
| ~~B7~~ | `rust-ai-native-lang` — **DONE**, 546 units (×1.75) | 18 | 312 |
| ~~B8~~ | `discovery-prompt` + `decision-records` — **DONE**, 366 units (×1.28), **0 unmarked** | 8 | 286 |
| ~~B9~~ | `spec-genres` + `wal` + `addressable-specs` — **DONE**, 776 units, **0 unmarked** | 17 | 577 |
| B10 | `health-audit` + `conflict-protocol` + `managed-blocks` | 16 | 488 |
| B11 | `source-mirrors` + `tool-design-lessons` + `qualified-naming` | 15 | 451 |
| B12 | `campaign-plans` + `two-process-model` + `operating-modes` | 15 | 403 |
| B13 | `git-attribution-policy` + `secrets-hygiene` + `comparative-research` | 15 | 378 |
| B14 | `sync-from-code` + `licensing` + `manual-tests` | 16 | 339 |
| B15 | the git family + `wal-specspaces` + `dev-runtime-docs` | 17 | 300 |
| B16 | three `-mcp` packages + two family umbrellas + `redbook`'s remaining 2 | 10 | 232 |

**Thirteen batches remain** (B5–B16). The count has read 18, 16, then 15: F-080
retired `core-ai-native`'s third, F-091 dissolved `redbook`'s into B16, and B1–B2
are done. `rust-ai-native` is already marked — it was the
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

**The mechanical half of the review is `cargo xtask batch-review`** (added at
the B6 boundary, after the same throwaway checker had been written from scratch
four times). Run the gate, then hand it the log and the batch's predictions:

```bash
cargo xtask batch-review --gate-log gate.log --scope scope.txt --expect-unmarked <n> --expect-residual residual.txt --expect-total <n> --campaign campaigns/packages-2026-09
```

**Always pass `--campaign`** — it is what enables C11, the check that every task
file has a row in `tasks/INDEX.md` and every row names a task that exists. That
ledger has already gone silent twice in this campaign: `MARKUP-B2` and
`MARKUP-B5` both ran, landed, and were never entered, so a cold reader would
have seen `MARKUP-B1` at the top of the table and concluded nothing else had
run. C11 was verified by running it against the tree as it stood at B5's landing
commit, where it names both missing tasks.

Its negative controls are `#[test]`s, so **the floor runs them on every commit**
rather than when someone remembers a flag; `cargo xtask batch-review --selftest`
additionally replays landed batches out of git history, which the hermetic tests
cannot.

It checks scope containment, word-stream identity, the gate delta against the
brief's prediction, error classes, the closed vocabulary, anchor collisions,
encoding, and markers-in-fences; it surfaces every `@unknown` and every
ruling-30 candidate. **It does not judge**, and its output ends with the list of
what it did not check — which is the review. Three of its checks read the
brief's predictions, so **a brief that does not state them cannot be checked
mechanically**: state the expected residual, the residual's files, and the
expected corpus total in every batch brief from B7 on.

**Unverified assumption, flagged rather than relied on:** `vibe update` as the
re-materialisation path is the owner's account of his own tool and has not been
exercised in this campaign. Run it once before Phase B leans on it.
