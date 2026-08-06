# MARKUP-B10 — `health-audit` + `conflict-protocol` + `managed-blocks` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.world/{health-audit/v0.1.0,
conflict-protocol/v0.1.0, managed-blocks/v0.1.0}/`.

**All forty-one locked conventions in [`MARKUP-B1.md`](MARKUP-B1.md#locked) bind
this batch.** Two are struck (18, 19). **Rulings 39–41 are new from B9** — ruling
35 applied literally where it collides with 34, the bare-connector colon, and
the multi-sentence blockquote.

## Two marked siblings now {#siblings}

@fact:B10-TWO-SIBLINGS B8 (`discovery-prompt` + `decision-records`, `e654c86f`) and B9
(`spec-genres` + `wal` + `addressable-specs`, `b1689359`) are both landed
`world` flows of the same shape — README, boot snippet, `spec/flows/<name>/`
documents, and in B9's case a `SKILL.md` too. **Read the corresponding file in
both before marking yours.**

@fact:B10-WHERE-THEY-AGREE-IS-SETTLED **Where B8 and B9 agree, the case is settled and no ruling is needed** —
that is what two independent batches converging means. **Where they disagree,
say so with both file references**; that is the most useful thing you can
report, and it is how rulings 39–41 were found.

@fact:B10-B9-FOLLOWED-B8-EXPLICITLY B9's report named the places it followed B8 on cases no ruling covered:
example cells take their row's stage, a semicolon-joined pointer pair stays one
unit, a normative colon lead-in takes UPPER. **Those three now have two batches
behind them.** Treat them as settled and do not re-derive.

## Scope {#scope}

**16 files, 487 units** — measured 2026-07-27 by `progress check --exhaustive`.

| file | units |
|---|---|
| `health-audit/…/spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.md` | 60 |
| `managed-blocks/…/spec/flows/managed-blocks/adoption-guide.md` | 54 |
| `health-audit/…/spec/flows/health-audit/audit-checklist.md` | 53 |
| `managed-blocks/…/spec/flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL.md` | 50 |
| `conflict-protocol/…/spec/flows/conflict-protocol/uncertainty-protocol.md` | 48 |
| `conflict-protocol/…/spec/flows/conflict-protocol/failure-modes.md` | 42 |
| `conflict-protocol/…/spec/flows/conflict-protocol/CONFLICT-PROTOCOL.md` | 42 |
| `managed-blocks/…/spec/flows/managed-blocks/rejected-designs.md` | 28 |
| `health-audit/…/spec/flows/health-audit/running-an-audit.md` | 23 |
| `conflict-protocol/…/README.md` | 16 |
| `health-audit/…/README.md` | 15 |
| `conflict-protocol/…/spec/boot/35-flow-conflict-protocol.md` | 15 |
| `managed-blocks/…/README.md` | 13 |
| `health-audit/…/spec/skills/health-audit/SKILL.md` | 12 |
| `health-audit/…/spec/boot/42-flow-health-audit.md` | 10 |
| `managed-blocks/…/spec/boot/65-flow-managed-blocks.md` | 6 |

## Sizing, and an honest note on how weak this test is {#sizing}

@fact:B10-COMPOSITION **Measured composition: 130 cells, 201 items, 156 paragraphs** — a
26.7 % cell share.

@fact:B10-THREE-CONSTANT-PREDICTION B9 corrected the sizing rule to **three constants — paragraphs × 2.13,
pre-existing list items × 1.00, table cells × 1.00** — because only paragraphs
multiply. Applied here: `156 × 2.13 + 201 + 130 = ` **≈ 663 units**.

@fact:B10-THIS-TEST-DISCRIMINATES-WEAKLY **Say plainly that this is a weak test, unlike B9's.** B9's composition
made the competing models disagree widely (779 / 797 / 739 / 981 against a
measured 776). Here the three-constant rule says 663 and B8's superseded
two-constant rule says 676 — **2 % apart**, which no single measurement can
separate. So B10 tests the new rule's **stability**, not its superiority. Report
your total either way; a third point near 2.13 is worth having even when it
cannot arbitrate.

## The three predictions {#predictions}

Verified by the reviewer against a gate run at dispatch. **You do not need to
re-run anything** — the boundaries below forbid it, and there is no instruction
here that contradicts them.

- @fact:B10-EXPECT-RESIDUAL **Residual: ZERO.** `health-audit/…/SKILL.md` is in scope and fully
  markable — DRIFT-037 closed F-092 on 2026-07-27, so its frontmatter is no
  longer a countable unit.
- @fact:B10-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B10-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 2 102** — it stands at 2 589 and this
  batch owes all 487.

## What this batch is likely to surface {#expect}

@fact:B10-F097-DOES-NOT-REACH-HERE **F-097 does not reach this batch.** None of the sixteen files cites
`flow:atomic-commits` — checked. If you meet a dead `flow:` reference it is a
**new** finding and should be reported as one.

@fact:B10-EXPECT-CHECKLIST-GENRE `audit-checklist.md` is a **checklist** and `rejected-designs.md` is a
**catalogue of things deliberately not built** — two shapes the campaign has not
marked before. A rejected design is a decision record's «considered and
rejected» field at document scale: ruling 10 sends rationale to `@spec/done`,
and ruling 26 sends a rule in force to `@impl/done` even when its checker is a
human habit. **Report where that line was hard to draw** rather than stretching
a stage to fit; ruling 11 says `@unknown` is cheap.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines and ruling-33 re-wraps. A semantic problem is
  **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

The reviewer runs:

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

**Zero unmarked units in the batch's 16 files**, and 2 102 corpus-wide. Every
marked unit anchored; no id collides with another in its file across the **one
case-sensitive address space** shared with heading anchors; `git diff` shows
markers, splits, anchors and the two licensed whitespace repairs, and nothing
else.

## Report back {#report}

Per-file counts · **your measured total against the 663 prediction, with the
paragraph/item/cell split that produced it** · every `@unknown` with its text
and why · every semantic problem seen and not fixed · every ruling-30 and
ruling-33 repair with its line number · **every place B8 and B9 disagreed with
each other**, with both references. Seventeen batches have run; **thirteen found
a factual error in their own brief by measuring, most recently B9, which found
two.** If this one is wrong, say so with the measurement.
