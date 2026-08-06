# MARKUP-B12 — `campaign-plans` + `two-process-model` + `operating-modes` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.world/{campaign-plans/v0.1.0,
two-process-model/v0.1.0, operating-modes/v0.1.0}/`.

**All forty-six locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked) bind
this batch.** Two are struck (18, 19).

## Four marked siblings {#siblings}

@fact:B12-FOUR-SIBLINGS B8, B9, B10 and B11 are all landed `world` batches of this shape. **Read
the corresponding file in the nearest sibling before marking yours.** B11 is the
most recent and its report is the most useful: it settled how a labelled-field
genre is treated (ruling 46 confines ruling 32 to pattern cards, so a
four-field *lesson* deconstructs normally) and it found the one place the
siblings disagreed.

@fact:B12-THE-RECONCILED-DISAGREEMENT **That disagreement is settled and you should not re-open it.** B9 split
a colon with three post-colon segments; B10 kept one with five. Ruling 44
reconciles them: **ruling 20's verdict test gates the colon rules.** A
post-colon item that is a *table-of-contents entry* cannot carry an independent
Phase-C verdict, so a **scope enumeration stays whole however many segments it
has**; a post-colon item that is a *verdictable claim* splits. Five siblings now
carry a single `##scope-of-this-document` unit on that reading.

## Scope {#scope}

**15 files, 403 units** — measured 2026-07-27 by `progress check --exhaustive`,
and matching `BATCH-PLAN.md` exactly; no correction was needed.

| file | units |
|---|---|
| `campaign-plans/…/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md` | 42 |
| `operating-modes/…/spec/flows/operating-modes/OPERATING-MODES-PROTOCOL.md` | 40 |
| `two-process-model/…/spec/flows/two-process-model/cognitive-load-split.md` | 39 |
| `two-process-model/…/spec/flows/two-process-model/files-as-ipc.md` | 38 |
| `campaign-plans/…/spec/flows/campaign-plans/phase-gates.md` | 37 |
| `operating-modes/…/spec/flows/operating-modes/writing-a-codeword.md` | 35 |
| `campaign-plans/…/spec/flows/campaign-plans/execution-ledger.md` | 32 |
| `two-process-model/…/spec/flows/two-process-model/TWO-PROCESS-MODEL.md` | 29 |
| `operating-modes/…/spec/flows/operating-modes/mfbt-mode.md` | 29 |
| `two-process-model/…/README.md` | 15 |
| `operating-modes/…/README.md` | 14 |
| `campaign-plans/…/README.md` | 14 |
| `two-process-model/…/spec/boot/05-flow-two-process-model.md` | 13 |
| `operating-modes/…/spec/boot/45-flow-operating-modes.md` | 13 |
| `campaign-plans/…/spec/boot/40-flow-campaign-plans.md` | 13 |

## Sizing — and read the caveat before the number {#sizing}

@fact:B12-COMPOSITION **Measured composition: 96 cells, 159 items, 148 paragraphs.**

@fact:B12-TWO-PREDICTIONS-NOT-ONE B11 replaced the paragraph rule with a mechanism: deconstruction makes
roughly **one unit per sentence**, so `1.08 × sentences + items + cells`. Sizing
this batch with it produced **two different answers**, and that is the finding
rather than a nuisance:

- a reviewer-side counter reads **426 sentences** → **715 units**;
- **the same counter reads B11's pre-state at 515 where B11's own method
  reported 382** — a 35 % disagreement — and 426 corrected by that measured bias
  is ≈316 → **596 units**.

@fact:B12-STATE-YOUR-COUNTING-RULE **So there is no single prediction here, deliberately, and your job is to
make the next one possible.** Report your sentence count **together with the
rule you counted by** — what you do with abbreviations, inline code, list text,
and what you treat as a paragraph. The coefficient is bound to a counter that
nothing specifies; two batches that count differently cannot use each other's
number. **A measurement is only as portable as its definition**, and that is
this campaign's own recurring lesson turning up in its planning arithmetic.

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You are not asked to
re-run anything.**

- @fact:B12-EXPECT-RESIDUAL **Residual: ZERO.** No `SKILL.md` and no frontmatter in scope.
- @fact:B12-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B12-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 1 248** — it stands at 1 651 and this
  batch owes all 403.

## F-097 reaches four of your files {#f097}

@fact:B12-F097-FOUR-SITES `campaign-plans/README.md`, `two-process-model/README.md`,
`operating-modes/README.md` and
`two-process-model/…/spec/flows/two-process-model/files-as-ipc.md` cite dead
package names. Four names are dead corpus-wide — `atomic-commits`,
`attribution-policy`, `conventional-commits`, `autonomy` — all renamed to
`git-*` by `520e7478`, all declared correctly under the new name.

@fact:B12-MARK-DO-NOT-FIX **Mark them, do not fix them, do not re-file.** A **fifth** dead name
would be a new finding, and the review now checks for one mechanically.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines, ruling-33 re-wraps, ruling-12 emphasis re-application.
  A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

The reviewer runs `progress check --exhaustive`. **Zero unmarked units in the
batch's 15 files**, and 1 248 corpus-wide. Every marked unit anchored; no id
collides with another in its file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits, anchors
and the licensed whitespace and emphasis repairs, and nothing else.

## Report back {#report}

Per-file counts · **your sentence count AND the rule you counted by** · every
`@unknown` with its text and why · every semantic problem seen and not fixed,
excluding the four F-097 names · every ruling-30, -33 and -12 repair with its
line number · every place the four siblings disagreed. Nineteen batches have
run; **fourteen found a factual error in their own brief by measuring.** B11
found one in its composition line. If this one is wrong, say so with the
measurement.
