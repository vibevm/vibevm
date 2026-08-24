# MARKUP-B8 — `discovery-prompt` + `decision-records` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `vibevm/vibepacks/org.vibevm.world/discovery-prompt/v0.1.0/` and
`vibevm/vibepacks/org.vibevm.world/decision-records/v0.1.0/`.

**All thirty-three locked conventions in
[`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this batch.** Two are struck (18, 19).
Rulings 30–32 came from B6, **ruling 33 from B7 and this is the first batch to
run under it** — the wrapped-bullet repair, which the mechanical checker cannot
see and which must therefore be reported by name.

## This batch has no twin, and that is what makes it different {#no-twin}

@fact:B8-FIRST-WORLD-BATCH B5, B6 and B7 were three projections of one skeleton, and by B7 two
marked siblings decided most cases before they were asked. **B8 has none.** It
is the first batch in the `world` namespace: flow packages, whose shape is a
README, a boot snippet, and protocol documents under `spec/flows/<name>/`. There
are no cards, no `mechanisms/`, no `tools/`, and no sibling to read first.

@fact:B8-IS-THE-GENRE-CALIBRATION **So this batch is a calibration, and its report is worth more than its
markup.** The nine batches after it are all `world` flows. Cases the
thirty-three conventions do not decide here will be locked and will bind
B9–B16, exactly as B1a's fifteen bound everything after it. **Report every case
you had to think about, including the ones you resolved confidently** — a
convention that felt obvious to you is one the next batch may resolve the other
way.

@fact:B8-MULTIPLIER-IS-UNMEASURED-HERE **The ×1.7 sizing multiplier does not apply and must not be assumed.**
It was measured three times and all three were language stacks (×1.62, ×1.72,
×1.75). `BATCH-PLAN.md` says to re-measure at the first `world` batch rather
than trust a constant drawn from three similar things. **Your final unit count
is a measurement this campaign does not have yet** — report it plainly.

## Scope {#scope}

**8 files, 286 units** — measured 2026-07-27 by `progress check --exhaustive`.

| file | units |
|---|---|
| `discovery-prompt/…/spec/flows/discovery-prompt/usage.md` | 62 |
| `decision-records/…/spec/flows/decision-records/revisit-triggers.md` | 62 |
| `decision-records/…/spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md` | 58 |
| `decision-records/…/spec/flows/decision-records/record-template.md` | 50 |
| `decision-records/…/spec/boot/25-flow-decision-records.md` | 22 |
| `decision-records/…/README.md` | 15 |
| `discovery-prompt/…/README.md` | 11 |
| `discovery-prompt/…/spec/boot/50-flow-discovery-prompt.md` | 6 |

@fact:B8-DISCOVERY-PROMPT-IS-EXCLUDED **`DISCOVERY-PROMPT.xml` is NOT yours** — owner ruling F-096, 2026-07-27.
It left the corpus as a **prompt payload, not a claim**: `confirmed` has no
meaning applied to a line addressed to another model. Do not mark it, do not
open it to «check consistency», do not report drift in it.

@fact:B8-CITATIONS-INTO-IT-ARE-FINE **`usage.xml` cites the excluded file throughout, and that is correct.**
An exclusion is about *measurement*, not about links — a document in the corpus
may cite a document outside it. A broken-looking cross-reference into
`DISCOVERY-PROMPT.xml` is **not** a finding on those grounds alone.

## The three predictions {#predictions}

Stated for `cargo xtask batch-review`. **Re-verify all three against a fresh
gate run at dispatch time** — DRIFT-037 and the F-096 exclusion both move the
corpus total, and a number carried from a document instead of a command is this
campaign's most repeated defect.

- @fact:B8-EXPECT-RESIDUAL **Residual after the batch: ZERO.** Neither package has a `SKILL.md`
  and no file in scope opens with `---`, so F-092 does not reach this batch.
  **This is the first batch in the campaign that can finish at zero unmarked**,
  and «2» is therefore not an acceptable outcome here.
- @fact:B8-EXPECT-RESIDUAL-FILES **No file may carry a residual.** There is no exempt list.
- @fact:B8-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 3 630 − 9 (DRIFT-037) − 169 (F-096)
  − 286 = 3 166**, if both land first. Confirm the starting number with a gate
  run; do not trust this arithmetic.

## Two things this batch is likely to surface {#expect}

@fact:B8-EXPECT-TEMPLATE-GENRE `record-template.xml` is a **template with worked examples** — a genre
the campaign has not marked before. A filled-in example inside a template is not
obviously a fact about this project; it may be closer to the F-096 shape than to
a contract. **Mark it and report the discomfort** rather than deciding: do not
invent an exclusion, and do not stretch a stage to fit. Ruling 11 says
`@unknown` is cheap and honest.

@fact:B8-EXPECT-SELF-DESCRIBING-FLOWS These two packages describe **how this project records decisions and
runs research** — so their prose is about our own practice, which makes ruling
10's discriminator (checkable against this repository → `@impl`; outside world,
motivation or future → `@spec`) do more work than usual. A flow that states its
own law is `@impl/done` even where its checker is a human habit rather than
code, per ruling 26.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines and ruling-33 re-wraps. A semantic problem is
  **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.** The reviewer reads the diff and commits.

## Acceptance {#acceptance}

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

**Zero unmarked units in the batch's 8 files.** Every marked unit anchored; no
id collides with another in its file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits, anchors
and the two licensed whitespace repairs, and nothing else.

## Report back {#report}

Per-file counts and **your measured multiplier** · every `@unknown` with its
text and why · every semantic problem seen and not fixed · every ruling-30 and
ruling-33 repair with its line number · **every case the thirty-three
conventions did not decide, including ones you were confident about** — this
batch calibrates the genre for nine more. Fourteen batches have run; **twelve
found a factual error in their own brief by measuring, and the last two did
not.** If this one is wrong, say so with the measurement.
