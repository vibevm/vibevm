# MARKUP-B2 — `core-ai-native` v0.8.0, mechanisms and appendix {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/`.

**All twenty-one locked conventions in
[`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this batch.** They were paid for by
B1a and B1b and they are not re-litigated here. Read that section before the
files.

## The seven files {#files}

| file | lines |
|---|---|
| `mechanisms/PROP-014-specmap-bidirectional-traceability.md` | 360 |
| `appendix/ATLAS.md` | 203 |
| `mechanisms/BROWNFIELD-PROTOCOL-v0.1.md` | 106 |
| `mechanisms/MCP-CORE-v0.1.md` | 97 |
| `mechanisms/ENGINE-CONFORM-v0.1.md` | 77 |
| `mechanisms/LEDGER-INTENT-v0.1.md` | 66 |
| `appendix/CONTRADICTION-MAP.md` | 41 |
| **total** | **950** |

This closes `core-ai-native` — B1's nine files plus these seven are the whole
live slot, after F-080 retired `legacy-projections/`.

## What is different about this batch {#different}

B1 was the guiding layer: prose about the discipline. These are the **mechanism
specs the shipped checkers implement**, and `spec://org.vibevm.ai-native/core-ai-native/mechanisms/…`
is what code tags actually cite. Three consequences:

- @fact:B2-ANCHORS-ARE-LOAD-BEARING **An anchor minted here may be cited from code.** PROP-014 is the
  document that defines the citation grammar itself. Anchor names in this batch
  are held to a higher bar than B1's: read the surrounding heading anchors and
  match their idiom before minting a new fact id.
- @fact:B2-PROP-014-JUST-CHANGED **`PROP-014`'s anchor clause was amended by the reviewer on
  2026-07-26**, immediately before this batch, to record the widened anchor
  grammar and case-sensitivity. Mark what is there now; do not be surprised that
  it disagrees with older statements elsewhere in the corpus — that disagreement
  is a **finding**, not something to fix.
- @fact:B2-ATLAS-IS-A-LEDGER **`ATLAS.xml` is a findings ledger, not a contract.** Its entries are
  dated research results with evidence classes (`_benchmark · high · refines:H4_`).
  Treat each entry as one fact, keep its evidence tag inside the unit, and
  expect `@spec/done` to dominate — these are claims about the outside world,
  which ruling 10 sends to the spec register.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors.
  A semantic problem found is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root** — the reviewer's lane. Your
  seven files live under `packages/…/v0.8.0/spec/`, which is a different tree
  and is your scope.
- **Do not touch** B1's nine files, any other package, `crates/`, or
  `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.** The
  reviewer runs the gate.
- **Do not commit.** The reviewer reads the diff and commits.

## Acceptance {#acceptance}

The reviewer runs:

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --no-cache --campaign campaigns/packages-2026-09
```

- zero unmarked units in the seven files under `--exhaustive`;
- every marked unit anchored; no id collides with another in the same file,
  **including heading `{#anchor}`s** — one address space, and it is
  case-sensitive, so `##FOO` beside `{#foo}` is legal and `##FOO` beside
  `{#FOO}` is an error;
- `git diff` shows markers, splits and anchors and **nothing else**.

## Report back {#report}

1. Per file: units marked, anchors minted, paragraphs deconstructed, heading
   anchors added.
2. Every `@unknown`, with the unit text and why.
3. Every semantic problem seen and not fixed.
4. Any case the twenty-one conventions did not decide — B1a produced fifteen
   and B1b six, and each was worth more than the markup it came with.
