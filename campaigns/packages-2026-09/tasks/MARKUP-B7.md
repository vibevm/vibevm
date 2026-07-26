# MARKUP-B7 — `rust-ai-native-lang` v0.7.0 {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/`.

**All thirty-two locked conventions in
[`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this batch.** Two are struck (18, 19
— DRIFT-031 closed the findings they encoded). **Rulings 30–32 are new from B6
and this is the first batch to run under them:** the lazy-continuation repair,
ruling 23 generalised past tables, and the card's labelled field as the atom.

## Scope {#scope}

**18 files, 312 units** — measured 2026-07-26 by `progress check --exhaustive`
against the live tree, not estimated. Pre-markup count; at the measured ×1.7 the
batch finishes near **530 units**.

| file | units |
|---|---|
| `spec/rust/GUIDE-AI-NATIVE-RUST.md` | 55 |
| `spec/cards/scaffold-d-differential-oracle.md` | 47 |
| `spec/rust/tools/vibe-agentic-tcg-rust.md` | 34 |
| `spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md` | 34 |
| `spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md` | 29 |
| `spec/rust/tools/rust-ai-native-tcg.md` | 28 |
| `spec/skills/rust-ai-native-terraform/SKILL.md` | 20 |
| `spec/skills/rust-ai-native-sweep/SKILL.md` | 19 |
| `README.md` | 13 |
| `spec/boot/20-stack-rust-ai-native-lang.md` | 9 |
| `spec/cards/scaffold-{a,b,c,e,f,g,h,i}` — 3 each | 24 |

Already out by the excludes and **not yours**: `LICENSE.md` (file name),
`spec/cards/INDEX.md` (derived index). **Out of scope:** `crates/`, `tools/`,
`target/`, `Cargo.*`, and every other package.

## The three predictions this batch is checked against {#predictions}

Stated here so the review can be run mechanically
(`tools/batch-review.py`). A brief that does not state them has not predicted
anything.

- ##B7-EXPECT-RESIDUAL **Residual after the batch: exactly 2 unmarked units**, one in each
  `SKILL.md`, and zero everywhere else.
- ##B7-EXPECT-RESIDUAL-FILES **The residual files are** `spec/skills/rust-ai-native-sweep/SKILL.md`
  and `spec/skills/rust-ai-native-terraform/SKILL.md`.
- ##B7-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked after the batch: 3 630** — it stands at 3 940
  today and this batch owes 310 of it (312 units less the 2 that cannot be
  marked).

## Two twins now, and this one is the original {#twins}

##B7-TWO-REFERENCES `go-ai-native-lang` (B5, 665 units) and `typescript-ai-native-lang`
(B6, 581 units) are both marked, reviewed and landed. **Read the go and the
TypeScript sibling of each file before marking it.** Where the two twins agree,
follow them; where they disagree, that disagreement is itself information and it
goes in your report.

##B7-RUST-IS-THE-SOURCE-NOT-A-PROJECTION **This package is the original the other two were projected from**,
not a third copy. So where its wording differs from both twins, the likely
reading is that **rust is the source and the twins paraphrased** — do not
"correct" toward the twins. `##FORM-ONLY` binds: reuse the author's words, and
the author is here.

##B7-README-RETURNS **The README is back.** B6 had none, so no file in it took `doc` or an
`audience`; B7 has exactly one such file. Ruling 7: `doc/done` +
`audience="user"`. Two marked precedents, both landed:
`packages/org.vibevm.ai-native/rust-ai-native/v0.7.0/README.md` (the Phase A
pilot, the aggregator README) and the go twin's. Everything else in the batch
takes `impl` (boot snippet, both `SKILL.md`) or `spec` (guide, nine cards, two
`mechanisms/`, two `tools/`) — measured off the landed go stack, not derived.

## Ruling 30 fires here, once, and it is already located {#ruling-30}

##B7-LAZY-CONTINUATION-AT-GUIDE-63 `spec/rust/GUIDE-AI-NATIVE-RUST.md:63` — «A ban with no escape hatch is
a discipline bug; a deviation with no reason is a code bug.» sits at column 0
directly after a bullet, so markdown folds it into that bullet and the bullet
ends up asserting something it does not assert. **This is the same sentence, in
the same position, that B6 repaired in the TypeScript guide.** Insert the blank
line, give it its own anchor, mark it. Ruling 30 governs; ruling 14 is the
whitespace licence.

##B7-LOOK-FOR-MORE That is the one this brief located by measurement. **Look for others** —
the class is «section-level sentence at column 0 immediately after a list item»
— and report any you repair, with the line number.

## Two things this batch is likely to surface {#expect}

##B7-EXPECT-VERSION-CLAIMS The guide and the tool briefs state toolchain versions, crate names and
licence facts. Those are checkable against `vibe.toml` and the tree and may be
**stale**. That is a finding, reported, never fixed.

##B7-EXPECT-HOST-ONLY-CITATION `spec/rust/tools/vibe-agentic-tcg-rust.md:156` cites
`spec/boot/90-user.md` — a path that exists in the **host** repository and not
in the package a consumer installs. The TypeScript twin carries the identical
citation and B6 reported it; **the go twin does not**. Confirm the shape here
and report it. Mark stage and state and move on — whether a document may cite
outside itself is a verdict question, Phase C's, not a marker question.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors.
  A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.** The reviewer reads the diff and commits.

## Acceptance {#acceptance}

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

The three predictions above, exactly — **2 unmarked in the batch, in the two
named files, and 3 630 corpus-wide.** Every marked unit anchored; no id collides
with another in its file across the **one case-sensitive address space** shared
with heading anchors; `git diff` shows markers, splits, anchors and ruling-30
blank lines, and nothing else.

## Report back {#report}

Per-file counts · every `@unknown` with its text and why · every semantic
problem seen and not fixed · every ruling-30 repair with its line number · any
case the thirty-two conventions did not decide · **every place the two twins
disagreed with each other**, which is new to this batch and is worth more than
either twin alone. Twelve batches have run; **eleven found a factual error in
their own brief by measuring, and B6 was the first that did not.** If this one
is wrong, say so with the measurement.
