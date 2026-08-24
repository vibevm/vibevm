# MARKUP-B16 — three `-mcp` packages, two family umbrellas, and `redbook` {#root}

**Phase:** B (markup, facts pass) — **the last batch.** **Executor:** Opus.
**Reviewer:** the boss, who owns sense-preserving splits, anchor names and
`audience`.
**Corpus:** `vibevm/vibepacks/org.vibevm.world/redbook/v0.2.0/` and
`vibevm/vibepacks/org.vibevm.ai-native/{rust-ai-native-mcp/v0.7.0,
typescript-ai-native-mcp/v0.6.0, go-ai-native-mcp/v0.1.0,
typescript-ai-native/v0.6.0, go-ai-native/v0.1.0}/`.

**All fifty-eight locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked)
bind this batch.** Two are struck (18, 19). **Rulings 57–58 are new from B15**,
and 57 is the one that will decide calls here: a colon introducing **instances,
members or cases** splits; a colon introducing the **steps of a named procedure**
or an **appositive gloss** does not. A definition *by extension* is still an
enumeration.

@fact:B16-CLOSES-PHASE-B **This batch closes Phase B.** The corpus stands at 232 and this batch owes
all of it. There is no next batch to fix an inconsistency in, so where a
convention is ambiguous, **report it rather than absorbing it quietly**.

## Eight marked siblings, and this batch is mostly siblings {#siblings}

@fact:B16-STACK-AGGREGATOR-PRECEDENT **Two of your ten files are stack aggregators, and their third sibling is
already marked**: `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native/v0.7.0/README.md`
(the Phase A pilot). It uses `##AGG-ROLE`, `##AGG-MEMBER-*`,
`##AGG-HOW-TO-REQUIRE`, and `doc/done audience="user"`. `git-practices` (B15)
added `##AGG-CLOSURE-*`. **The three stack aggregators are uniform in their H1
already** — all read `# AI-Native <Lang> (stack:…)` — so mark
`typescript-ai-native` and `go-ai-native` to match `rust-ai-native` exactly.
Both are 5 units.

@fact:B16-MCP-TRIPLET **The three `spec/tools/discipline-mcp-*.md` files are a triplet** — 73, 67
and 78 lines, the same document per language. Mark them **consistently with each
other**; a divergence between them that is not a language difference is a
finding. They are 43, 39 and 41 units, and each is ~85 % table cells.

@fact:B16-COMPOSITION-STAGES **Ruling 37 governs every aggregator and roster entry**: it takes the
stage its predicate asserts, never a genre-uniform one. A member that exists and
is pinned → `@impl/done`; a positioning claim, or an entry naming something not
yet built → `@spec/done` (or `@idea/plan` per ruling 29, if the entry *is* the
pointer to an unbuilt deliverable rather than a claim about one).

## Scope {#scope}

**10 files, 232 units** — measured 2026-07-28 from a live
`check --exhaustive --no-cache` run, matching `BATCH-PLAN.md` exactly.

| file | units | cells | items | paras | terminators |
|---|---|---|---|---|---|
| `redbook/v0.2.0/README.md` | 57 | 42 | 4 | 11 | 20 |
| `rust-ai-native-mcp/…/spec/tools/discipline-mcp-rust.md` | 43 | 36 | 0 | 7 | 15 |
| `go-ai-native-mcp/…/spec/tools/discipline-mcp-go.md` | 41 | 34 | 0 | 7 | 17 |
| `typescript-ai-native-mcp/…/spec/tools/discipline-mcp-typescript.md` | 39 | 34 | 0 | 5 | 15 |
| `redbook/v0.2.0/spec/boot/03-flow-redbook.md` | 30 | 0 | 23 | 7 | 12 |
| `go-ai-native/v0.1.0/README.md` | 5 | 0 | 3 | 2 | 2 |
| `rust-ai-native-mcp/v0.7.0/README.md` | 5 | 0 | 3 | 2 | 3 |
| `typescript-ai-native-mcp/v0.6.0/README.md` | 5 | 0 | 3 | 2 | 3 |
| `typescript-ai-native/v0.6.0/README.md` | 5 | 0 | 3 | 2 | 3 |
| `go-ai-native-mcp/v0.1.0/README.md` | 2 | 0 | 0 | 2 | 4 |

## Sizing — the batch where the coefficient barely matters {#sizing}

@fact:B16-COMPOSITION **Measured composition: 146 cells, 39 items, 47 paragraphs, 94
terminators.** The terminator figure is the quantity `BATCH-PLAN.md` records the
regex for — **not a sentence count**.

@fact:B16-CELL-SHARE **This batch is 63 % table cells, and no previous batch exceeded 48 %.**
The three MCP briefs are ~85 % cells each and carry **no list items at all**;
`redbook/README.md` is 42 cells of 57. Only `03-flow-redbook.xml` (23 items, 0
cells) looks like the batches before it.

@fact:B16-PREDICTION **Predicted: 285–293 units** (`1.07–1.15 × 94 + 39 + 146`). **The band is
8 units wide because 185 of the 232 pre-state units are already at fact grain
and cannot deconstruct** — so the multiplier applies to the smallest base of any
batch, and the whole observed coefficient range (1.025–1.153, seven points) only
widens the prediction to **281–293**. B15 broke the band and the plan now records
why: the coefficient measures how aggressively a batch splits colons, not a
property of the corpus. **Here it has almost no leverage, which makes this the
one batch whose total is genuinely predictable.**

@fact:B16-REPORT-YOUR-COUNT **Report your own terminator count under the recorded rule**; if it
disagrees with 94, say so with the measurement. Two clauses were added to the
rule after B14 — frontmatter is structure, and fences are matched by **run**, not
prefix — and one caveat after B15: a quoted question and an `(e.g. …)` both fire
the terminator regex, so a single-sentence unit can legitimately count two.

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You are not asked to
re-run anything.**

- @fact:B16-EXPECT-RESIDUAL **Residual: ZERO.** No `SKILL.md`, no frontmatter, no nested fence.
- @fact:B16-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B16-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: ZERO.** It stands at 232 and this batch
  owes all of it. **This is the number that closes Phase B**, so a residual of
  any size is a phase-level fact, not a batch-level one — report it loudly.

## Tables are the work here, so read ruling 16 and 23 first {#tables}

@fact:B16-TABLE-RULES **Ruling 16:** `##ROW-*` anchor on the **first** body cell of each row, a
marker on **every** non-empty body cell, uniform stage per row; header and
delimiter rows untouched. **Ruling 17:** an empty body cell stays bare — do not
insert an em-dash to make it markable, that is a content edit.

@fact:B16-TABLE-STAGE **Ruling 23:** a row mixing our posture with an outside-world claim takes
`@spec/done` for the whole row. B10 marked 39 such cells that way and it is 23
working as written. **The list form ruling 23 prefers is not available here** —
turning a table into a list is not a sense-preserving split.

@fact:B16-CELLS-EXEMPT `##CELLS-ANCHOR-EXEMPT` still holds: mint a cell id only where something
cites it. With 146 cells this is the difference between a readable diff and an
unreadable one.

## Three findings already taken from this batch {#known}

@fact:B16-KNOWN-F112 **F-112** — `go-ai-native-mcp/README.md:9` cites
`spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md`. The path is stale, the file it means
now lives in `legacy-spec/terraforms/` where the WAL forbids citing as a
normative source, and it is a host path inside a consumer's package. **Wrong
three ways in one line, and the only such reference that fails for the author
too.**
@fact:B16-KNOWN-STUB **The same README is a stub** («This README is finalized at campaign
close»), which is why it carries 2 units where its two MCP siblings carry 5 —
and its H1 is the only one of the three written `# <name> (mcp:…)` rather than
`# mcp:…`.
@fact:B16-KNOWN-DO-NOT-REFILE **All three are filed. Mark the file as it stands and do not re-file
them** — but do report anything else in it.

@fact:B16-F097-FOUR **Four dead package names remain in scope.** Mark them, do not fix them,
do not re-file. A **fifth** name would be new and the review checks for one
mechanically.

## Nine heading anchors owed {#anchors}

@fact:B16-ANCHORS-OWED Counted outside fenced blocks: **one each** in the five READMEs that have
no `{#root}` (`rust-ai-native-mcp`, `typescript-ai-native-mcp`,
`go-ai-native-mcp`, `typescript-ai-native`, `go-ai-native`), **one** in
`redbook/README.md`, and **two** in `redbook/…/03-flow-redbook.xml`. The three
`discipline-mcp-*.md` files are fully anchored at 4/4 each.

@fact:B16-ANCHOR-SPACE Watch `##FACT-ID-GRAMMAR`: heading anchors and fact ids share one
**case-sensitive** address space.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines, ruling-33 re-wraps, ruling-47 hyphen repairs, ruling-12
  emphasis re-application. A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` or `legacy-spec/**` at the repository root**,
  `crates/` anywhere, any other package, or `campaigns/**`.
- **Do not touch any `vibe.toml`.**
- **`redbook/v0.1.0` is a superseded slot and out of corpus** — only `v0.2.0`
  is yours. `redbook`'s `spec/book/**` left the corpus at F-091 and is not
  marked either.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

The reviewer runs `progress check --exhaustive`. **Zero unmarked units in the
batch's 10 files, and ZERO corpus-wide.** Every marked unit anchored; no id
collides with another in its file across the one case-sensitive address space
shared with heading anchors; `git diff` shows markers, splits, anchors and the
licensed repairs, and nothing else.

## Report back {#report}

Per-file counts · **your terminator count under the recorded rule** · every
`@unknown` with its text and why · every semantic problem seen and not fixed,
excluding F-112, the stub facts and the four F-097 names · every ruling-30, -33,
-47 and -12 repair with its line number · every two-segment colon and how you
called it · **every place the three `discipline-mcp-*.md` triplets diverge from
each other in a way that is not a language difference** · **how ruling 57 landed
in practice**, since B15 derived it from a single control and you are the second
batch under it.

**Twenty-three batches have run; fifteen found a factual error in their own
brief by measuring.** B14 found none; B15 found one — this reviewer's dead-name
count was short by two, because it was built from a delimiter-anchored grep.
**If this one is wrong, say so with the measurement.**

@fact:B16-LAST-WORD **This is the last batch, so say what you would tell B17 if there were
one.** The conventions list has grown to 58 rulings across sixteen batches;
anything you had to re-derive because it was not written down is worth more than
the markup.
