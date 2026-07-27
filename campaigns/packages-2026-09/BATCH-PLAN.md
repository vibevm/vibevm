# Phase B batch plan — campaign `packages-2026-09`

_Recomputed 2026-07-26 against **live version slots only**. The first version
of this plan measured 250 files / 8 463 facts and was wrong by a quarter: it
counted two superseded slots that nothing resolves to._

## The workload, after three rounds of finding out what was in it

**202 files** across 37 packages; the observed corpus is **260 files**
(58 host + 202 packages), confirmed by `progress scan`.

**Sizing: count SENTENCES, not paragraphs — the mechanism, found at B11.**

```
predicted units  ≈  1.08–1.15 × sentences  +  pre-existing list items  +  table cells
```

**⚠️ The quantity is TERMINATORS UNDER THE REGEX BELOW, not sentences.** B13
measured the gap: over its fifteen files the rule reads **274** where a true
sentence count is **≈320**, a structural **+17 %**. 34 paragraphs end in a colon
before a fence or a list and are never counted; 13 terminators are swallowed by
a following `**` or backtick because the lookahead wants whitespace.

**The coefficient is bound to that undercount, so do not "fix" the counter.**
Anyone who repairs it toward real sentences reads ≈320 on B13, derives a
coefficient near **0.95**, and sizing with 1.08 then over-predicts by an eighth.
If the counter is ever corrected, **every coefficient in the table below must be
re-derived in the same commit.** The word «sentences» is kept below only because
the table's numbers were produced under it; read it as shorthand for the regex.

**Count by THIS rule, or the coefficient means nothing** (B12's rule, which
reproduces B11's published figure exactly):

- **Universe:** paragraph units only — progress-core's own `Para`/`Lead` facts.
  List items and table cells are excluded; the formula adds each back at 1.
- **Fenced code excluded entirely; inline code blanked** by progress-core's own
  run-matching `blank_inline_code`, so a full stop inside `` `a-file.md` ``
  cannot terminate a sentence.
- **Terminator:** `[.!?]["\)]?` followed by whitespace or end of unit.
- **Not terminators:** em-dash, semicolon, colon. **Abbreviations:** no special
  case — measured across 148 paragraphs, zero occurrences of `e.g.`/`i.e.`/etc.
- **A paragraph with no terminator counts as 1.** A multi-line blockquote is
  one paragraph.
- **YAML frontmatter is structure, not a paragraph** (DRIFT-037). The rule said
  "progress-core's own `Para`/`Lead` facts" and left this implicit; B14's
  executor wrote a counter from the text alone, read the frontmatter of
  `draft-eula/SKILL.md` as prose, and came out **+3 (262 against 259)** before
  finding the omission. Any batch containing a `SKILL.md` is mis-sized by a
  counter written from the recorded words alone.
- **Fences are matched by RUN, not by prefix** (F-102). A block opened with four
  backticks is not closed by a three-backtick line inside it. A counter that
  gets this wrong reads *fewer* paragraphs than the gate, not more — it was an
  eleven-unit gap on B14, and it is what surfaced the parser bug.

Deconstruction produces roughly **one unit per sentence**, which is what it is
for; paragraphs were only ever a proxy for sentences and a bad one. Measured
back over three batches:

| batch | sentences | items | cells | predicted | measured | error |
|---|---|---|---|---|---|---|
| B9 | 354 | 236 | 162 | 780 | 776 | **+0.6 %** |
| B10 | 338 | 201 | 130 | 696 | 700 | **−0.6 %** |
| B11 | 382 | 145 | 119 | 677 | 682 | **−0.8 %** |
| B12 | 320 | 159 | 96 | 601 | 624 | **−3.7 %** |
| B13 | 274 | 168 | 82 | 546 | 555 | **−1.6 %** |
| B14 | 259 | 150 | 47 | 476 | 479 | **−0.6 %** |

**B13 was the first batch sized with the band before dispatch, and the band
held**: predicted 543–565, measured 555, realised coefficient **1.113**.
**B14 held it a second time**: predicted 474–494, measured 479, realised
**1.089**. Six points now span **1.068–1.153**, and the band has stopped being
news — which is the point at which a rule is allowed to be boring.

**The coefficient has now moved twice.** B12 realised **1.153** units per
paragraph-sentence against B11's 1.08, and 1.153 reproduces its 624 exactly.
Four batches span 1.068–1.153. **Size with the band, quote no point**, and treat
the next batch that lands outside it as informative rather than as a surprise —
that is the third time this file has been told a small-n constant is not one.

**±0.8 % against the ±15 % the paragraph rule needed.**

**The operational claim needs one caveat, found while sizing B12 and worth more
than the coefficient.** «Countable by regex» is true, but **two regexes disagree
by 35 %**: a reviewer-side counter read B11's pre-state at **515** sentences
where the batch's own method reported **382**. Abbreviations, code spans, list
text and what counts as a paragraph all move the number. So:

- **The 1.08 is bound to the counter that produced 354 / 338 / 382**, and it is
  meaningless with any other. A batch reporting a sentence count **must state
  its counting rule**, or the next batch cannot use it.
- **Sizing before dispatch works only after calibrating your counter** against a
  landed batch's reported figure, **and the calibration works.** B12's
  reviewer-side count of 426 became ≈316 once divided by the measured 1.348
  bias; the batch's own rule read **320** — the correction landed within
  **1.3 %**. Against the realised 624 units the corrected branch was 4.5 % low
  and the uncorrected one 14.6 % high. *(B12 hypothesised that the 515 figure
  was a B12 measurement mislabelled as B11's; checked at review against the
  commit the calibration actually read — it was B11's files. The cause is
  simply that the reviewer-side counter over-counts paragraphs by about a
  fifth, which is what the bias measures.)*
- **RESOLVED 2026-07-27 at B13: the rule above is reproducible.** A second,
  independent implementation reads **B11 at 381 against its published 382, and
  B12 at 320 exactly**. The single defect in the reviewer's earlier counter — the
  one that produced the 35 % gap — was that it read **indented continuation
  lines of list items** as paragraph prose. Written down, the measurement now
  transfers; that was the whole claim, and it is now demonstrated rather than
  asserted.

**The controlled experiment, which is why this is a mechanism and not another
curve fit.** `self-updating-tools.md` and `packaging-lessons.md` are the same
author, the same package, the same four-field lesson genre, and *identical*
pre-composition: 29 paragraphs, 7 items, 0 cells each. They produced **×2.45 and
×1.93** — a 27 % spread with paragraph density, package and genre all held
constant. Counted independently at review they carry **68 sentences against 55**,
a ratio of 1.24 against the multipliers' 1.27. Paragraph share correlates with
the multiplier at **r = −0.171** across B11's fifteen files; sentences per
paragraph correlate at **r = +0.886**.

**Read this as a mechanism with a provisional coefficient, and do not lock the
coefficient.** This is the *third* version of this rule. The first was a blended
constant, the second was «paragraphs ×2.13, stable to 0.7 %» promoted from two
measurements and falsified by the third, and the plan recorded that as its own
error. Three points is not proof either. **What is now well-supported is the
mechanism — one unit per sentence — because a controlled pair demonstrates it;
the 1.08 is three measurements and will move.**

**Superseded — kept for the reasoning, which still holds:**
The paragraph multiplier read ×2.127 (B8), ×2.112 (B9) and ×2.365 (B10). Only
paragraphs multiply — items and cells are already at fact grain, and ruling 1
keeps a pre-existing item whole — and that structural half survives intact; it
is the *rate* that paragraphs were never the right unit for.

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
| ~~B10~~ | `health-audit` + `conflict-protocol` + `managed-blocks` — **DONE**, 700 units, **0 unmarked** | 16 | 487 |
| ~~B11~~ | `source-mirrors` + `tool-design-lessons` + `qualified-naming` — **DONE**, 682 units, **0 unmarked** | 15 | 451 |
| ~~B12~~ | `campaign-plans` + `two-process-model` + `operating-modes` — **DONE**, 624 units, **0 unmarked** | 15 | 403 |
| ~~B13~~ | `git-attribution-policy` + `secrets-hygiene` + `comparative-research` — **DONE**, 555 units, **0 unmarked** | 15 | 378 |
| ~~B14~~ | `sync-from-code` + `licensing` + `manual-tests` — **DONE**, 479 units, **0 unmarked** | 16 | 327 |
| B15 | the git family + `wal-specspaces` + `dev-runtime-docs` | 17 | 300 |
| B16 | three `-mcp` packages + two family umbrellas + `redbook`'s remaining 2 | 10 | 232 |

**Thirteen batches remain** (B5–B16). The count has read 18, 16, then 15: F-080
retired `core-ai-native`'s third, F-091 dissolved `redbook`'s into B16, and B1–B2
are done. `rust-ai-native` is already marked — it was the
Phase A pilot.

**B14 reads 327. This row has now been corrected twice in one day, by two
parser fixes, and both times the row was stale rather than wrong when written.**

- **339 → 338 (DRIFT-037).** The row was last written 2026-07-26 13:38; the
  frontmatter fix landed 2026-07-27 01:06 and took the YAML envelope of
  `licensing`'s `draft-eula/SKILL.md` out of the count — DRIFT-037 measured that
  file at 8 units, the gate then read 7. **No other remaining row moves:** the
  nine in-corpus `SKILL.md` files are the three language stacks' six,
  `health-audit`'s, `wal`'s and this one, and only this one was still unmarked.
  B15 and B16 ship none.
- **338 → 327 (F-102).** The fence scanner matched by prefix, so a
  four-backtick block quoting three-backtick ones was closed by its own inner
  opener and eleven shell commands inside `manual-tests`' two template files
  were counted as prose. They could not have been marked — a marker inside a
  fence is not read as one — so this is F-092's genre, not a miscount. Fixed by
  `c813b849`; **corpus-wide 870 → 859**, and the two files are the only ones in
  the corpus carrying the construct.

**The pattern in both is the same and it is worth more than either number:**
this table is a derived thing and nothing recomputes it. Measure the batch from
a live gate log at dispatch; never quote this column into a brief.

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
