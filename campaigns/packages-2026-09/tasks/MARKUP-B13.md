# MARKUP-B13 — `git-attribution-policy` + `secrets-hygiene` + `comparative-research` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.world/{git-attribution-policy/v0.1.0,
secrets-hygiene/v0.1.0, comparative-research/v0.1.0}/`.

**All fifty locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this
batch.** Two are struck (18, 19). **Rulings 47–50 are new from B12** — the
wrapped hyphen, the trailing-coda blocker, ruling 40 gating from the lead side,
and the one that matters most: **ruling 45's segment count is a tiebreaker that
rulings 3, 20, 35, 48 and 49 all outrank.**

## Five marked siblings {#siblings}

##B13-FIVE-SIBLINGS B8, B9, B10, B11 and B12 are landed `world` batches of this shape. Read
the nearest sibling's corresponding file before marking yours. B12's report is
the most useful on colons — it worked eight two-segment cases and two
three-segment ones and stated its reasoning for each.

##B13-NAMING-VARIANCE-TO-SETTLE **One naming variance is open and you should close it by following the
majority.** Two names coexist for the same unit: `##sibling-document-pointers`
(source-mirrors, qualified-naming, and B12 throughout) and
`##companion-document-pointers` (decision-records, health-audit). **Use
`##sibling-document-pointers`** — three batches to two, and it is the more
recent. Say in your report if a file's wording makes it wrong.

## Scope {#scope}

**15 files, 378 units** — measured 2026-07-27, matching `BATCH-PLAN.md` exactly.

| file | units |
|---|---|
| `secrets-hygiene/…/spec/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL.md` | 48 |
| `comparative-research/…/spec/flows/comparative-research/from-research-to-roadmap.md` | 44 |
| `git-attribution-policy/…/spec/flows/attribution-policy/disclosure-alternative.md` | 40 |
| `comparative-research/…/spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md` | 38 |
| `git-attribution-policy/…/spec/flows/attribution-policy/enforcement-checklist.md` | 37 |
| `secrets-hygiene/…/spec/flows/secrets-hygiene/scope-discipline.md` | 26 |
| `secrets-hygiene/…/spec/flows/secrets-hygiene/third-party-code-consent.md` | 25 |
| `git-attribution-policy/…/spec/flows/attribution-policy/ATTRIBUTION-POLICY.md` | 25 |
| `comparative-research/…/spec/flows/comparative-research/research-template.md` | 19 |
| `secrets-hygiene/…/spec/boot/57-flow-secrets-hygiene.md` | 14 |
| `secrets-hygiene/…/README.md` | 14 |
| `git-attribution-policy/…/README.md` | 13 |
| `comparative-research/…/README.md` | 13 |
| `git-attribution-policy/…/spec/boot/55-flow-attribution-policy.md` | 12 |
| `comparative-research/…/spec/boot/52-flow-comparative-research.md` | 10 |

## The first prediction from a reproducible counter {#sizing}

##B13-COMPOSITION **Measured composition: 82 cells, 168 items, 128 paragraphs, and 274
sentences.**

##B13-THE-COUNTER-IS-NOW-REPRODUCIBLE **The sentence figure is the first one this campaign can trust across
implementations.** B12 wrote its counting rule down and it went into
`BATCH-PLAN.md` verbatim; a second, independent implementation at review now
reproduces **B11 at 381 against its published 382, and B12 at 320 exactly**. The
single defect in the reviewer's earlier counter — the one that produced the
35 % disagreement — was that it read **indented continuation lines of list
items** as paragraph prose.

##B13-PREDICTION **Predicted: 543–565 units** (`1.07–1.15 × 274 + 168 + 82`). This is a
band because the coefficient has moved twice — 1.068, 1.092, 1.094, 1.153 across
four batches — and the plan says to quote the band and treat an outlier as
informative. **Report your own sentence count under the recorded rule**; if it
disagrees with 274, the rule is still under-specified and that is worth more
than the batch total.

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You are not asked to
re-run anything.**

- ##B13-EXPECT-RESIDUAL **Residual: ZERO.** No `SKILL.md` and no frontmatter in scope.
- ##B13-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- ##B13-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 870** — it stands at 1 248 and this
  batch owes all 378.

## F-097 reaches five files, including the renamed package itself {#f097}

##B13-F097-FIVE-SITES `git-attribution-policy`'s own `README.md` and its three flow documents,
plus `secrets-hygiene/README.md`, cite dead names. **This is the package whose
own `vibe install` / `vibe uninstall` lines name `flow:attribution-policy`** —
the sharpest instance in the whole finding, in the file a consumer reads first.

##B13-STILL-DO-NOT-FIX **Mark them, do not fix them, do not re-file.** Four names are dead
corpus-wide, all renamed to `git-*` by `520e7478`. A **fifth** would be new, and
the review checks for one mechanically.

## What this batch is likely to surface {#expect}

##B13-EXPECT-ATTRIBUTION-SELF-REFERENCE `attribution-policy` is the flow that forbids naming AI tooling
anywhere in the repository, and it is also the one document family permitted to
discuss the topic. **Mark it exactly as you would any other flow** — the policy
governs the repository's authored surface, not the markup grammar, and a marker
is not authorship. If you find yourself reasoning about whether a marker
violates the policy, that is the answer: it does not.

##B13-EXPECT-SECRETS-DISCIPLINE `secrets-hygiene` documents token handling. **Its prose names env-var
names and file paths, which is exactly what the flow says is safe to print.**
Do not treat a path or a variable name as a secret; do not quote any value you
encounter anywhere in the repository. Nothing in this batch should require you
to read a credential file.

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
batch's 15 files**, and 870 corpus-wide. Every marked unit anchored; no id
collides with another in its file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits, anchors
and the licensed repairs, and nothing else.

## Report back {#report}

Per-file counts · **your sentence count under the recorded rule, and any place
that rule was ambiguous** · every `@unknown` with its text and why · every
semantic problem seen and not fixed, excluding the four F-097 names · every
ruling-30, -33, -47 and -12 repair with its line number · every place the five
siblings disagreed. Twenty batches have run; **fifteen found a factual error in
their own brief by measuring.** B12 found that this brief's predecessor quoted
an uncorrected sentence count. If this one is wrong, say so with the measurement.
