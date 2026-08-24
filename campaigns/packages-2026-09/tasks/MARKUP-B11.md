# MARKUP-B11 — `source-mirrors` + `tool-design-lessons` + `qualified-naming` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.world/{source-mirrors/v0.1.0,
tool-design-lessons/v0.1.0, qualified-naming/v0.1.0}/`.

**All forty-six locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked) bind
this batch.** Two are struck (18, 19). **Rulings 42–46 are new, and all five come
from B10's own open cases** — the arrow chain, what «sequence» means in ruling
34, ruling 20 gating ruling 34, the colon-segment tiebreaker, and ruling 32's
scope. Read them before you start; they were written to answer questions you
would otherwise re-ask.

## Three marked siblings {#siblings}

@fact:B11-THREE-SIBLINGS B8 (`e654c86f`), B9 (`b1689359`) and B10 are all landed `world` batches
of this shape. **Read the corresponding file in the nearest one before marking
yours.** B10 is closest in genre — protocol documents plus a catalogue — and its
`rejected-designs.xml` is the reference for anything that reads as a record of
what was *not* built. `tool-design-lessons` is a strong candidate for that shape.

@fact:B11-AGREEMENT-IS-SETTLED **Where two or more siblings agree, the case is settled** — follow it and
do not re-derive. **Where they disagree, report with all references.** B10
checked thirteen shapes across B8 and B9 and found zero disagreements; that is
the coverage bar for this section, not a formality.

## Scope {#scope}

**15 files, 451 units** — measured 2026-07-27 by `progress check --exhaustive`.
This is the first batch whose measurement matched `BATCH-PLAN.md` exactly; no
correction was needed.

| file | units |
|---|---|
| `tool-design-lessons/…/spec/flows/tool-design-lessons/TOOL-DESIGN-LESSONS.md` | 55 |
| `source-mirrors/…/spec/flows/source-mirrors/fanout-mechanics.md` | 47 |
| `qualified-naming/…/spec/flows/qualified-naming/ref-grammar.md` | 44 |
| `source-mirrors/…/spec/flows/source-mirrors/daily-loop.md` | 42 |
| `qualified-naming/…/spec/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md` | 40 |
| `qualified-naming/…/spec/flows/qualified-naming/naming-forks.md` | 39 |
| `source-mirrors/…/spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md` | 38 |
| `tool-design-lessons/…/spec/flows/tool-design-lessons/self-updating-tools.md` | 36 |
| `tool-design-lessons/…/spec/flows/tool-design-lessons/packaging-lessons.md` | 36 |
| `qualified-naming/…/README.md` | 15 |
| `tool-design-lessons/…/README.md` | 14 |
| `source-mirrors/…/README.md` | 14 |
| `qualified-naming/…/spec/boot/67-flow-qualified-naming.md` | 11 |
| `tool-design-lessons/…/spec/boot/70-flow-tool-design-lessons.md` | 10 |
| `source-mirrors/…/spec/boot/62-flow-source-mirrors.md` | 10 |

## Sizing — the rule was falsified, and this batch tests what replaced it {#sizing}

@fact:B11-NO-POINT-PREDICTION **There is no point prediction, deliberately.** B9 produced a paragraph
multiplier of ×2.13 from two measurements and `BATCH-PLAN.md` locked it as
«stable to 0.7 %»; B10 came in at **×2.365**, 11.6 % high, and the rule is now a
**range: ~2.2 ± 15 %**. Two points cannot show stability, and this brief will
not repeat the mistake by quoting one number as if it were reliable.

@fact:B11-COMPOSITION **Measured composition: 119 cells, 145 items, 186 paragraphs** — 41 %
paragraphs, against B10's 32 % and B8's 25 %. **The most paragraph-heavy batch
so far.**

@fact:B11-THE-REAL-HYPOTHESIS B10 proposed that the multiplier **tracks paragraph density rather than
package or genre** — its dense protocol documents ran ×2.5–2.9 and its READMEs
×1.71–2.00. **B11 is the test of that hypothesis, and it is a sharp one:** if
density drives the multiplier, this batch should land **at or above ×2.3**; if
the multiplier is really noise around ~2.2, it should land near it regardless.
Range at ~2.2 ± 15 % is **612–734 units**. Report your total, your
paragraph/item/cell split, and your per-file multipliers — the per-file spread
is what tests the hypothesis, not the batch total.

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You are not asked to
re-run anything**, and nothing here contradicts the boundaries below.

- @fact:B11-EXPECT-RESIDUAL **Residual: ZERO.** No file in scope ships a `SKILL.md` or opens with
  frontmatter.
- @fact:B11-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B11-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 1 651** — it stands at 2 102 and this
  batch owes all 451.

## F-097 reaches exactly one file {#f097}

@fact:B11-F097-ONE-FILE `source-mirrors/…/README.md` cites a dead package name. **F-097 was
widened on 2026-07-27 after a sweep of every package reference against every
declared name: four names are dead** — `atomic-commits`, `attribution-policy`,
`conventional-commits`, `autonomy`, all renamed to `git-*` — across 21 files and
33 references, six of which are literal `vibe install` command lines.

@fact:B11-DO-NOT-REFILE **Mark it, do not fix it, do not re-file it.** A *fifth* dead name would
be a new finding; these four are not. The fix is one wave-level DRIFT under
sync-from-code.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines, ruling-33 re-wraps and ruling-12 emphasis re-application.
  A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

The reviewer runs `progress check --exhaustive`. **Zero unmarked units in the
batch's 15 files**, and 1 651 corpus-wide. Every marked unit anchored; no id
collides with another in its file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits, anchors
and the licensed whitespace and emphasis repairs, and nothing else.

## Report back {#report}

Per-file counts · **your total, your para/item/cell split, and your per-file
multipliers**, which are what test B10's density hypothesis · every `@unknown`
with its text and why · every semantic problem seen and not fixed, excluding the
four F-097 names · every ruling-30, ruling-33 and ruling-12 repair with its line
number · every place the siblings disagreed. Eighteen batches have run;
**thirteen found a factual error in their own brief by measuring.** B10 found
that this plan's sizing rule was wrong, which is the most valuable brief error
the campaign has had. If this one is wrong, say so with the measurement.
