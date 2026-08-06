# MARKUP-B15 — the git family + `wal-specspaces` + `dev-runtime-docs` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.world/{git-atomic-commits, git-autonomy,
git-conventional-commits, git-practices, wal-specspaces, dev-runtime-docs}/v0.1.0/`.
`git-attribution-policy` is **already marked** (B13) and out of scope.

**All fifty-six locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked) bind
this batch.** Two are struck (18, 19). **Rulings 53–56 are new from B14** — and
53 is the one that will decide calls in this batch: ruling 35's "a set the lead
names" **covers a condition, an instruction or a bare predicate**, measured
against 69 landed sites. Do not keep a "When X: do A, B, C" whole on the
grounds that a condition names no set.

## Seven marked siblings, and one genre with exactly one precedent {#siblings}

@fact:B15-SEVEN-SIBLINGS B8 through B14 are landed `world` batches of this shape. Read the nearest
sibling's corresponding file before marking yours.

@fact:B15-UMBRELLA-PRECEDENT **`git-practices` is a family aggregator and there is exactly one marked
one in the corpus: `packages/org.vibevm.ai-native/rust-ai-native/v0.7.0/README.md`
(the Phase A pilot).** It uses `##AGG-ROLE` for the role sentence, `##AGG-MEMBER-*`
for each member entry, `##AGG-HOW-TO-REQUIRE` for the requirement line, and
`doc/done audience="user"` as the document marker. **Follow it.** `git-practices`
is 7 units in one README and ships no boot snippet by design (PROP-028).

@fact:B15-COMPOSITION-STAGES **Ruling 37 governs the member entries**: a Composition entry takes the
stage its predicate asserts, never a genre-uniform one. A member that exists and
is pinned → `@impl/done`; a positioning claim or a statement about what the
family will grow to include → `@spec/done`. `rust-ai-native` marks all three of
its member entries `@impl/done` because all three exist.

## Scope {#scope}

**17 files, 300 units** — measured 2026-07-28 from a live
`check --exhaustive --no-cache` run, and matching `BATCH-PLAN.md` exactly.

| file | units | cells | items | paras | terminators |
|---|---|---|---|---|---|
| `git-conventional-commits/…/spec/flows/conventional-commits/conventional-commits.md` | 60 | 40 | 10 | 10 | 21 |
| `wal-specspaces/…/spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.md` | 40 | 6 | 22 | 12 | 19 |
| `git-atomic-commits/…/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md` | 37 | 0 | 15 | 22 | 46 |
| `git-atomic-commits/…/spec/flows/atomic-commits/splitting-large-changes.md` | 35 | 0 | 14 | 21 | 37 |
| `git-atomic-commits/…/boot/30-flow-atomic-commits.md` | 17 | 0 | 10 | 7 | 11 |
| `wal-specspaces/…/spec/boot/11-flow-wal-specspaces.md` | 17 | 0 | 10 | 7 | 12 |
| `git-atomic-commits/…/README.md` | 12 | 0 | 6 | 6 | 11 |
| `git-autonomy/…/spec/flows/autonomy/AUTONOMY-PROTOCOL.md` | 12 | 0 | 5 | 7 | 18 |
| `dev-runtime-docs/…/spec/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.md` | 10 | 0 | 4 | 6 | 9 |
| `git-autonomy/…/boot/32-flow-autonomy.md` | 10 | 0 | 5 | 5 | 7 |
| `dev-runtime-docs/…/README.md` | 8 | 0 | 2 | 6 | 8 |
| `git-conventional-commits/…/boot/31-flow-conventional-commits.md` | 8 | 0 | 5 | 3 | 7 |
| `wal-specspaces/…/README.md` | 8 | 0 | 2 | 6 | 8 |
| `git-autonomy/…/README.md` | 7 | 0 | 3 | 4 | 6 |
| `git-conventional-commits/…/README.md` | 7 | 0 | 3 | 4 | 6 |
| `git-practices/…/README.md` | 7 | 0 | 2 | 5 | 7 |
| `dev-runtime-docs/…/boot/58-flow-dev-runtime-docs.md` | 5 | 0 | 2 | 3 | 4 |

## Sizing {#sizing}

@fact:B15-COMPOSITION **Measured composition: 46 cells, 120 items, 134 paragraphs, 237
terminators.** The terminator figure is the quantity `BATCH-PLAN.md` records the
regex for — **not a sentence count**, which runs about 17 % higher; the
coefficient is fitted to that undercount, so do not repair the counter toward
its name.

@fact:B15-COUNTER-CLAUSES **The counting rule gained two clauses after B14 and both bite here.**
YAML frontmatter is structure and is not a `Para` (no `SKILL.md` in this batch,
so it is inert); and **fences are matched by RUN, not by prefix** — a block
opened with four backticks is not closed by a three-backtick line inside it. A
counter that gets the second wrong reads *fewer* paragraphs than the gate.

@fact:B15-PREDICTION **Predicted: 419–438 units** (`1.07–1.15 × 237 + 120 + 46`). Six realised
coefficients span 1.068–1.153 and the band has now held twice running (B13
1.113, B14 1.089). **Report your own terminator count under the recorded rule**;
if it disagrees with 237, say so with the measurement.

@fact:B15-CELL-DENSITY **One file is 67 % table cells** —
`conventional-commits.md`, 40 cells of 60 units, the densest in the campaign so
far. Sixteen of the seventeen files have no table at all, so `SPECSPACES-PROTOCOL.md`
(6 cells) and that one file carry every cell in the batch.

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You are not asked to
re-run anything.**

- @fact:B15-EXPECT-RESIDUAL **Residual: ZERO.** No `SKILL.md`, no frontmatter and no nested fence in
  scope — the two classes that produced residuals in this campaign are both absent.
- @fact:B15-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B15-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 232** — it stands at 532 and this
  batch owes all 300. That leaves exactly B16.

## This batch is F-097's epicentre — fourteen dead names, three of them titles {#f097}

@fact:B15-F097-FOURTEEN-SITES **Three of the six packages here ARE the renamed ones**, and they still
name themselves by the names they lost. Fourteen occurrences across six files,
counted 2026-07-28:

| file | sites | what |
|---|---|---|
| `git-atomic-commits/README.md` | 4 | the **H1 title**, one prose reference, `vibe install`, `vibe uninstall` |
| `git-conventional-commits/README.md` | 4 | the **H1 title**, two prose references, `vibe install` |
| `git-autonomy/README.md` | 2 | the **H1 title**, `vibe install` |
| `git-practices/README.md` | 2 | both member entries, written bare rather than `flow:`-prefixed |
| `git-conventional-commits/boot/31-…` | 1 | prose |
| `git-atomic-commits/…/splitting-large-changes.md` | 1 | prose |

@fact:B15-F097-FOUR-BROKEN-COMMANDS **Four of those are command lines a consumer cannot run** — three
`vibe install`, one `vibe uninstall`. The live names are `flow:git-atomic-commits`,
`flow:git-autonomy`, `flow:git-conventional-commits`.

⚠️ @fact:B15-PATHS-ARE-NOT-NAMES **A path is not a name, and this batch is where that distinction
costs something.** The *package* was renamed to `git-atomic-commits`; the *flow
directory inside it* is still `spec/flows/atomic-commits/`. So
`spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md` is **correct** and
`flow:atomic-commits` is **dead**. A grep that does not anchor on the `flow:`
prefix or a backtick-delimited bare name reports the correct paths as findings.

@fact:B15-STILL-DO-NOT-FIX **Mark them, do not fix them, do not re-file.** Four names are dead
corpus-wide and one wave-level DRIFT is queued for all of them. A **fifth** name
would be new, and the review checks for one mechanically.

## F-103 is universal in this batch — all five links broken {#f103}

@fact:B15-F103-ALL-FIVE **Four of the six packages keep their boot snippet at `boot/`, not
`spec/boot/`, and every relative link in all four is broken in-package**:
`git-atomic-commits` 2, `git-autonomy` 1, `git-conventional-commits` 1,
`dev-runtime-docs` 1. They point at `../flows/…`, which from `boot/` resolves to
`<pkg>/flows/…`; the files are at `<pkg>/spec/flows/…`. They work only once the
snippet is installed into a consuming project. `wal-specspaces` uses `spec/boot/`
and its links resolve correctly.

@fact:B15-F103-FILED **Already filed and widened — do not re-file.** Ruling 7 is about the
genre, not the path: an installed boot snippet takes `impl/done` wherever it
sits, with `audience` absent (ruling 8).

## Two findings already taken from `git-practices` {#known}

@fact:B15-KNOWN-F108 **F-108** — its `vibe.toml` declares **four** members and, three lines
above the declaration, says the family «grows to include human-authored
attribution and commit autonomy **as those members land**». They landed. The
`description` field repeats it, and the README lists **two** members where the
closure pulls four.
@fact:B15-KNOWN-F109 **F-109** — the same manifest cites `neworder2/memory/BACKLOG.md`, a path
that exists in this repository and in no consumer's.

@fact:B15-KNOWN-DO-NOT-REFILE **Both are filed. Mark the README as it stands and do not re-file
either** — but *do* report anything else you find in that file, which is the
smallest and the most self-contradictory in the batch.

## Thirty-nine heading anchors owed, across ten files {#anchors}

@fact:B15-ANCHORS-OWED Ruling 5 makes missing heading `{#anchor}`s part of what a batch owes, and
**this is the largest anchor debt of any batch so far**. Counted outside fenced
blocks: `git-atomic-commits/README` 6, `git-atomic-commits/boot/30-…` 5,
`git-conventional-commits/README` 4, `git-conventional-commits/boot/31-…` 4,
`conventional-commits.md` 5, `git-autonomy/README` 4, `dev-runtime-docs/README` 5,
`dev-runtime-docs/boot/58-…` 2, `git-practices/README` 3,
`wal-specspaces/README` 1. **The other seven files are fully anchored.**

@fact:B15-ANCHOR-SPACE Watch `##FACT-ID-GRAMMAR`: heading anchors and fact ids share **one
case-sensitive address space**, so do not mint `{#atomicity}` beside an
`##ATOMICITY` fact. `conventional-commits.md` already has 12 heading anchors and
owes 5 more — read the existing ones before minting.

## What this batch is likely to surface {#expect}

@fact:B15-EXPECT-THE-FAMILY-CROSS-REFERENCES These four git packages describe each other constantly — the format
flow points at the atomicity flow and back, and the umbrella points at both.
**Ruling 37 decides each entry on its own predicate**, so expect a visible mix of
`@impl` and `@spec` inside single lists. That is correct, not uneven.

@fact:B15-EXPECT-PROSE-HEAVY-PROTOCOLS `ATOMIC-COMMITS-PROTOCOL.md` and `splitting-large-changes.md` carry 46
and 37 terminators over 22 and 21 paragraphs — the two most prose-dense files in
the batch and where most of the deconstruction work is. Ruling 4 (sibling
paragraphs where the facts are not parallel) did 81 of B14's 88 splits.

@fact:B15-EXPECT-TWO-SEGMENT-COLONS **Ruling 45's two-segment colon is still undecided and rulings 3, 20, 35,
48 and 49 all outrank the segment count (ruling 50).** Report every
two-segment case rather than settling it silently.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines, ruling-33 re-wraps, ruling-47 hyphen repairs, ruling-12
  emphasis re-application. A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`. `git-attribution-policy` is marked already —
  do not touch it either.
- **Do not touch any `vibe.toml`.** F-108 and F-109 are manifest findings and
  the manifest is not in the corpus.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

The reviewer runs `progress check --exhaustive`. **Zero unmarked units in the
batch's 17 files**, and 232 corpus-wide. Every marked unit anchored; no id
collides with another in its file across the one case-sensitive address space
shared with heading anchors; `git diff` shows markers, splits, anchors and the
licensed repairs, and nothing else.

## Report back {#report}

Per-file counts · **your terminator count under the recorded rule, and any place
that rule was ambiguous** · every `@unknown` with its text and why · every
semantic problem seen and not fixed, excluding the fourteen F-097 names, the
five F-103 links, and F-108/F-109 · every ruling-30, -33, -47 and -12 repair
with its line number · every two-segment colon and how you called it · every
place the seven siblings disagreed · **how ruling 53 landed in practice**, since
this is the first batch to run under it.

**Twenty-two batches have run; fifteen found a factual error in their own brief
by measuring.** B14 found none — the first to do so — and said which two
wordings it would still have changed. **If this one is wrong, say so with the
measurement.**
