# MARKUP-B9 — `spec-genres` + `wal` + `addressable-specs` {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `vibevm/vibepacks/org.vibevm.world/{spec-genres/v0.1.0, wal/v0.2.0,
addressable-specs/v0.1.0}/`.

**All thirty-eight locked conventions in
[`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this batch.** Two are struck (18, 19).
**Rulings 34–38 are new from B8's `world` calibration and this is the first
batch to run under them** — the comma rule, the enumerating-colon rule, em-dash
appositives, Composition entries by predicate, and `spec/flows/**` → `spec/done`.

## You have a marked `world` sibling, and B8 did not {#sibling}

@fact:B9-B8-IS-THE-REFERENCE B8 marked `discovery-prompt` + `decision-records` — the same genre, the
same shape (README, boot snippet, `spec/flows/<name>/` documents), reviewed and
landed at `e654c86f`. **Read the corresponding B8 file before marking each of
yours.** It is not a twin in the way go/typescript/rust were twins — different
subject matter, not a projection — but the *genre decisions* are settled there:
document-marker stage, register for colon lead-ins, how a Composition section is
staged, how a table row is handled.

@fact:B9-WHERE-B8-STOPPED-DECIDING B8 left twelve cases in its report and five became rulings. **The
seven that did not are still open**, and you will meet them: example-bearing
tables (its case C10), two-pointer paragraphs (C12), and the register of a
colon lead-in that is itself normative (C6). Where B8's file shows a choice and
no ruling covers it, **follow B8 and say that you did** — a second batch
agreeing is evidence; a second batch diverging silently is a defect.

## Scope {#scope}

**17 files, 577 units** — measured 2026-07-27 by `progress check --exhaustive`.

| file | units |
|---|---|
| `spec-genres/…/spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md` | 70 |
| `addressable-specs/…/spec/flows/addressable-specs/authoring-rules.md` | 57 |
| `spec-genres/…/spec/flows/spec-genres/when-to-write-what.md` | 54 |
| `addressable-specs/…/spec/flows/addressable-specs/spec-tree-layout.md` | 52 |
| `wal/…/spec/flows/wal/cold-resume.md` | 46 |
| `addressable-specs/…/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md` | 45 |
| `wal/…/spec/flows/wal/WAL-PROTOCOL.md` | 40 |
| `wal/…/spec/flows/wal/session-end-hook.md` | 33 |
| `spec-genres/…/spec/boot/17-flow-spec-genres.md` | 32 |
| `wal/…/spec/flows/wal/morning-routine.md` | 31 |
| `spec-genres/…/spec/flows/spec-genres/design-docs.md` | 29 |
| `wal/…/README.md` | 23 |
| `addressable-specs/…/README.md` | 15 |
| `wal/…/spec/boot/10-flow-wal.md` | 14 |
| `spec-genres/…/README.md` | 14 |
| `addressable-specs/…/spec/boot/15-flow-addressable-specs.md` | 13 |
| `wal/…/spec/skills/wal-status/SKILL.md` | 9 |

@fact:B9-PLAN-SAID-578 `BATCH-PLAN.md` says 578 and the measurement says **577**. The missing
one is accounted for exactly: DRIFT-037 taught the parser that YAML frontmatter
is structure, and `wal-status/SKILL.md` went 10 → 9. Nothing else moved.

## This batch is sized by B8's constants, and is the first test of them {#sizing}

@fact:B9-CELL-SHARE-MEASURED **Measured composition: 162 cells, 236 items, 179 paragraphs** — a
**28.1 % cell share**, against B8's 47.6 %.

@fact:B9-TWO-CONSTANT-PREDICTION B8 produced two constants instead of one because cells are already at
fact grain and cannot deconstruct: **prose × 1.53, cells × 1.00**. Applied here:
`415 × 1.53 + 162 = ` **≈ 797 units**. A blended B8 constant would say 739 and
a language-stack constant 981.

@fact:B9-THE-PREDICTION-IS-THE-POINT **This is the first batch sized by a rule rather than by a precedent,
so the number is a falsifiable prediction and your measured total is the test.**
Report it plainly whether it lands near 797 or nowhere near. A miss is a
correction to the rule and is worth more than a hit.

## The three predictions {#predictions}

- @fact:B9-EXPECT-RESIDUAL **Residual: ZERO.** `wal-status/SKILL.md` is in scope and it is the
  first `SKILL.md` any batch can take to zero — DRIFT-037 closed F-092 on
  2026-07-27, so its frontmatter is no longer a countable unit. There is no
  exempt file in this batch.
- @fact:B9-EXPECT-RESIDUAL-FILES **No file may carry a residual.**
- @fact:B9-EXPECT-CORPUS-TOTAL **Corpus-wide unmarked afterwards: 3 166 − 577 = 2 589.** Confirm the
  starting number with a gate run rather than trusting this line.

## F-097 reaches two of your files — do not re-file it {#f097}

@fact:B9-F097-IS-ALREADY-FILED `addressable-specs/…/README.md` and `wal/…/README.md` both cite
**`flow:atomic-commits`**, a package that does not exist — it was renamed to
`git-atomic-commits` by `520e7478`. This is **F-097**, filed 2026-07-27 against
the whole wave: sixteen canonical files carry the dead name, and the sharpest
are the `vibe install` / `vibe uninstall` lines in that package's own README.

@fact:B9-MARK-IT-DO-NOT-FIX-IT **Mark those units and move on.** Do not fix the name — it is a fact
correction under sync-from-code, not a markup edit — and **do not report it as
a new finding**; report only if you meet a *different* dead reference.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors,
  ruling-30 blank lines and ruling-33 re-wraps. A semantic problem is
  **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit and do not stage.**

## Acceptance {#acceptance}

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

**Zero unmarked units in the batch's 17 files**, and 2 589 corpus-wide. Every
marked unit anchored; no id collides with another in its file across the **one
case-sensitive address space** shared with heading anchors; `git diff` shows
markers, splits, anchors and the two licensed whitespace repairs, and nothing
else.

## Report back {#report}

Per-file counts · **your measured total against the 797 prediction, and the
cell/prose split that produced it** · every `@unknown` with its text and why ·
every semantic problem seen and not fixed, excluding F-097 · every ruling-30 and
ruling-33 repair with its line number · **every place you followed B8 where no
ruling covered the case**, named, because a second batch agreeing is what turns
a choice into a convention. Sixteen batches have run; **twelve found a factual
error in their own brief by measuring, and the last three did not.**
