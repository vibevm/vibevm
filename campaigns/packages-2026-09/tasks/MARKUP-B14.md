# MARKUP-B14 — `sync-from-code` + `licensing` + `manual-tests` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `vibevm/vibepacks/org.vibevm.world/{sync-from-code/v0.1.0,
licensing/v0.1.0, manual-tests/v0.1.0}/`.

**All fifty-two locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked) bind
this batch.** Two are struck (18, 19). **Rulings 51–52 are new from B13** — the
pointer unit is `##sibling-document-pointers` uniformly, and an enumeration
standing *before* its colon with the claim after it stays whole.

## Six marked siblings, and three genres that have one each {#siblings}

@fact:B14-SIX-SIBLINGS B8 through B13 are landed `world` batches of this shape. Read the nearest
sibling's corresponding file before marking yours. Three files in this batch
belong to genres the flow documents do not cover, and each has exactly one or
two landed precedents — **read the precedent, do not invent**:

| this file | its genre | read this, already marked |
|---|---|---|
| `licensing/…/spec/skills/draft-eula/SKILL.md` | agent skill | `wal/…/spec/skills/wal-status/SKILL.md` (B9), `health-audit/…/spec/skills/health-audit/SKILL.md` (B10) |
| `licensing/…/eula-template.xml` | copy-ready template | `comparative-research/…/research-template.xml` (B13), `decision-records/…/record-template.xml` (B8) |
| `manual-tests/…/test-template.xml` | copy-ready template | the same two |

@fact:B14-SKILL-DOC-MARKER **Both marked `SKILL.md` siblings carry
`<status stage="impl" state="done"/>` on the line after the frontmatter**, then
mark the prose normally. The frontmatter itself is structure and owes nothing
(DRIFT-037) — do not touch it, and do not put an anchor in it.

@fact:B14-BOOT-SNIPPET-PATH `sync-from-code` keeps its boot snippet at
**`boot/20-flow-sync-from-code.md`**, not `vibevm/vibespecs/boot/`. That is a family trait,
not a defect — `dev-runtime-docs` and the three `git-*` packages do the same,
and all four are B15's. **Ruling 7 is about the genre, not the path:** an
installed boot snippet takes `impl/done` wherever it sits, with `audience`
absent (ruling 8).

## Scope {#scope}

**16 files, 327 units** — measured 2026-07-28 from a live
`check --exhaustive --no-cache` run, *after* the F-102 fix. `LICENSE.xml` is out
of corpus in both packages that ship one (F-070, verbatim third-party text);
`eula-template.xml` is **in** corpus and is ours.

| file | units | cells | items | paras | terminators |
|---|---|---|---|---|---|
| `sync-from-code/…/spec/flows/sync-from-code/SYNC-PROTOCOL.md` | 37 | 0 | 17 | 20 | 34 |
| `manual-tests/…/spec/flows/manual-tests/MANUAL-TESTS-PROTOCOL.md` | 37 | 18 | 10 | 9 | 20 |
| `licensing/…/spec/flows/licensing/LICENSING-PROTOCOL.md` | 31 | 9 | 9 | 13 | 29 |
| `sync-from-code/…/spec/flows/sync-from-code/review-workflow.md` | 30 | 0 | 14 | 16 | 33 |
| `licensing/…/spec/flows/licensing/dependency-licenses.md` | 28 | 12 | 12 | 4 | 11 |
| `manual-tests/…/spec/flows/manual-tests/authoring-rules.md` | 20 | 0 | 6 | 14 | 24 |
| `licensing/…/spec/flows/licensing/eula-template.md` | 20 | 8 | 10 | 2 | 4 |
| `sync-from-code/…/spec/flows/sync-from-code/when-to-apply.md` | 19 | 0 | 5 | 14 | 31 |
| `sync-from-code/…/README.md` | 16 | 0 | 9 | 7 | 12 |
| `licensing/…/README.md` | 16 | 0 | 9 | 7 | 13 |
| `sync-from-code/…/boot/20-flow-sync-from-code.md` | 15 | 0 | 9 | 6 | 10 |
| `manual-tests/…/spec/flows/manual-tests/test-template.md` | 14 | 0 | 11 | 3 | 6 |
| `licensing/…/spec/boot/60-flow-licensing.md` | 13 | 0 | 10 | 3 | 6 |
| `manual-tests/…/README.md` | 13 | 0 | 7 | 6 | 12 |
| `manual-tests/…/spec/boot/44-flow-manual-tests.md` | 11 | 0 | 7 | 4 | 9 |
| `licensing/…/spec/skills/draft-eula/SKILL.md` | 7 | 0 | 5 | 2 | 5 |

## Sizing {#sizing}

@fact:B14-COMPOSITION **Measured composition: 47 cells, 150 items, 130 paragraphs, 259
terminators.** The terminator figure is the quantity `BATCH-PLAN.md` records the
regex for — **not a sentence count**, which runs about 17 % higher; the
coefficient is fitted to that undercount, so do not repair the counter toward
its name.

@fact:B14-COUNTER-CALIBRATED **The counter used here was calibrated against three landed batches before
it was pointed at this one**, and reproduces **B13 at 274, B12 at 320 and B11 at
381** against their published 274 / 320 / 382. Its paragraph count also equals
the gate's own `Para` unit count exactly, file by file — which is the check that
matters, since the recorded rule defines the universe as progress-core's own
`Para`/`Lead` facts and not as whatever a second parser thinks a paragraph is.

@fact:B14-PREDICTION **Predicted: 474–494 units** (`1.07–1.15 × 259 + 150 + 47`). Five realised
coefficients now span 1.068–1.153 (B9 1.068, B10 1.092, B11 1.094, B13 1.113,
B12 1.153); quoting that full envelope instead of the recorded band moves the
prediction by one unit at each end, so the band is used as written. **Report
your own terminator count under the recorded rule**; if it disagrees with 259,
the rule is still under-specified, and that is worth more than the batch total.

@fact:B14-PROSE-SHARE **This batch is proportionally more prose than B13**: 197 of its 327
pre-state units (60 %) are items or cells already at fact grain and cannot
deconstruct, against B13's 66 %. Cells are only 14 % here (B13: 22 %), and they
sit in four files — `MANUAL-TESTS-PROTOCOL.xml` 18, `dependency-licenses.xml` 12,
`LICENSING-PROTOCOL.xml` 9, `eula-template.xml` 8. **The other twelve files have
no table at all.**

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You are not asked to
re-run anything.**

- @fact:B14-EXPECT-RESIDUAL **Residual: ZERO.** The one `SKILL.md` in scope has its frontmatter
  exempted (DRIFT-037), and F-102 removed the only other class that could not be
  satisfied.
- @fact:B14-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B14-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 532** — it stands at 859 and this
  batch owes all 327.

## F-102 landed under you this morning — do not try to mark inside a fence {#f102}

@fact:B14-F102-WHAT-CHANGED Until commit `c813b849` the gate closed a fenced block on any line opening
with three backticks. Both `manual-tests` templates quote fenced markdown inside
**four**-backtick blocks, so each outer block was closed by its own first inner
opener and the parse ran inverted: eleven shell commands —
`export SCRATCH="$(mktemp -d)"`, `acme init`, `rm -rf "$SCRATCH"` — were counted
as paragraphs owing a marker, while the `**Expected.**` prose beside them
vanished from the count.

@fact:B14-F102-NOTHING-TO-DO **Nothing is asked of you here; the count in the table above is already
post-fix.** It is written down because the two files still *look* like they
contain markable prose inside their skeletons, and they do not:
`test-template.xml` owes **14** units and `authoring-rules.xml` **20**, all of
them outside the quoted blocks. **If you find yourself about to put an anchor or
a marker inside a four-backtick block, stop and report it** — a marker inside a
fence is not read as one, and the file is a skeleton consumers copy verbatim.

## F-097 reaches three files, and the install lines here are live {#f097}

@fact:B14-F097-FIVE-REFS Five references to dead package names, all in this batch's three
packages: `sync-from-code/README.md` lines 53 and 58, `when-to-apply.xml` lines
114 and 116 (`flow:atomic-commits`, renamed to `flow:git-atomic-commits`), and
`licensing/README.md` line 68 (`flow:attribution-policy` →
`flow:git-attribution-policy`).

@fact:B14-F097-INSTALL-LINES-ARE-FINE **Unlike B13, none of the six `vibe install` / `vibe uninstall` lines in
scope is broken** — each names its own package (`flow:sync-from-code`,
`flow:licensing`, `flow:manual-tests`) and all three are live. The dead names
are all in Composition / cross-reference prose.

@fact:B14-STILL-DO-NOT-FIX **Mark them, do not fix them, do not re-file.** Four names are dead
corpus-wide. A **fifth** would be new, and the review checks for one
mechanically.

## Heading anchors: sixteen owed, and they sit in three files {#anchors}

@fact:B14-ANCHORS-OWED Ruling 5 makes missing heading `{#anchor}`s part of what a batch owes.
Counted outside fenced blocks: **`licensing/README.md` owes 7 (it has none),
`sync-from-code/README.md` owes 6 (none), `boot/20-flow-sync-from-code.md` owes
3.** Every other file in the batch is fully anchored, including all three
templates.

@fact:B14-README-ASYMMETRY **`manual-tests/README.md` is already 6/6 anchored and the other two
READMEs are 0/N** — three sibling READMEs of the same generation, one done and
two not. Follow the marked one's anchor style. Watch `##FACT-ID-GRAMMAR`:
heading anchors and fact ids share one case-sensitive address space, so do not
mint `{#composition}` beside a `##COMPOSITION` fact.

⚠️ @fact:B14-ANCHOR-COUNT-TRAP **A naive `grep` reads 32 owed, not 16.** It counts
`#` headings inside the templates' fenced skeletons — `test-template.xml` alone
has 17 heading-shaped lines of which 12 are quoted. **Count outside fences.**

## What this batch is likely to surface {#expect}

@fact:B14-EXPECT-LICENCE-TEXT `licensing` is about licence text and ships two kinds of it. `LICENSE.xml`
is verbatim third-party text and **out of corpus** (F-070) — do not open it as
work. `eula-template.xml` is **ours and in corpus**; its skeleton sits in a fenced
block and costs the markup nothing, so its 20 units are the commentary, the
adapting table and the summary around it.

@fact:B14-EXPECT-TABLE-STAGES Four files carry tables, and two of them mix our posture with
outside-world claims — the licence allow/deny table in `dependency-licenses.xml`
most of all. **Ruling 16** puts one uniform stage per row; **ruling 23** sends a
mixed row to `@spec/done`. Prefer the list form only where the source already
uses one; turning a table into a list is not a sense-preserving split.

@fact:B14-EXPECT-PROCEDURES `sync-from-code`'s `review-workflow.xml` and `when-to-apply.xml` are
procedure documents, and `manual-tests`' `authoring-rules.xml` is four numbered
rules. **Ruling 26** is the discriminator: a scheduled, unexecuted procedure step
is `@spec/done`; a timeless rule in force is `@impl/done`, even when its checker
ships elsewhere (ruling 10).

@fact:B14-EXPECT-TWO-SEGMENT-COLONS **Ruling 45's two-segment colon is still undecided and rulings 3, 20, 35,
48 and 49 all outrank the segment count (ruling 50).** Report every
two-segment case rather than settling it silently.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines, ruling-33 re-wraps, ruling-47 hyphen repairs, ruling-12
  emphasis re-application. A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

The reviewer runs `progress check --exhaustive`. **Zero unmarked units in the
batch's 16 files**, and 532 corpus-wide. Every marked unit anchored; no id
collides with another in its file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits, anchors
and the licensed repairs, and nothing else.

## Report back {#report}

Per-file counts · **your terminator count under the recorded rule, and any place
that rule was ambiguous** · every `@unknown` with its text and why · every
semantic problem seen and not fixed, excluding the five F-097 names · every
ruling-30, -33, -47 and -12 repair with its line number · every two-segment
colon and how you called it · every place the six siblings disagreed.

**Twenty-one batches have run; fifteen found a factual error in their own
brief by measuring.** This brief's own numbers moved twice before it was
written — `BATCH-PLAN.md` said 339, then 338, and the live gate says 327. **If
this one is wrong, say so with the measurement.**
