# MARKUP-B6 — `typescript-ai-native-lang` v0.6.0 {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/`.

**All twenty-nine locked conventions in
[`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this batch** — fifteen from B1a, six
from B1b, eight from B2. They were paid for; do not re-litigate them.
**Rulings 18 and 19 are struck**: DRIFT-031 closed the findings they encoded
(`4f9143b4` made task-list checkboxes structure, `75009f8c` made inline-code
blanking run-matching). Task-list items are marked normally and markers go
last-token beside a quoted fence, like everywhere else.

## Scope {#scope}

**18 files, 338 units** — measured 2026-07-26 by
`progress check --exhaustive`, not estimated. This is the *pre-markup* count;
at B5's measured ×1.6 the batch finishes near **540 units**. Per file:

| file | units |
|---|---|
| `spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` | 80 |
| `spec/cards/scaffold-d-differential-oracle.md` | 48 |
| `spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md` | 39 |
| `spec/typescript/tools/vibe-agentic-tcg-ts.md` | 35 |
| `spec/typescript/mechanisms/TCG-ORACLE-v0.1.md` | 26 |
| `spec/typescript/tools/conform-frontend-typescript.md` | 20 |
| `spec/skills/typescript-ai-native-terraform/SKILL.md` | 20 |
| `spec/typescript/tools/typescript-ai-native-tcg.md` | 18 |
| `spec/skills/typescript-ai-native-sweep/SKILL.md` | 15 |
| `spec/boot/20-stack-typescript-ai-native-lang.md` | 13 |
| `spec/cards/scaffold-{a,b,c,e,f,g,h,i}` — 3 each | 24 |

Already out by the excludes and **not yours**: `LICENSE.md` (file name),
`spec/cards/INDEX.md` (derived index), `tools/ts-extract/test/fixtures/**`
(always-on `fixtures` exclusion).

**Out of scope and not yours:** anything under `crates/`, `tools/`, `target/`,
or `Cargo.*`.

## The go twin decides most of this batch {#twin}

##B6-SAME-SKELETON `go-ai-native-lang` v0.1.0 — batch B5, 665 units, landed in
`d3242f99` — is **the same package skeleton in another language**: the same boot
snippet, the same nine cards, one guide, two `mechanisms/` protocol specs, three
`tools/` briefs, two `skills/SKILL.md`. Measured file-for-file, B6 is **B5 minus
its `README.md`**, which this package does not have. Read the go twin of each
file before marking its TypeScript sibling. **Anchor names, split shape and
stage assignment are already decided there**; mirror them and diverge only where
the TypeScript text genuinely differs.

##B6-DOCUMENT-MARKERS Document markers, measured off the marked go stack rather
than derived: boot snippet → `impl`; both `SKILL.md` → `impl`; the guide, all
nine cards, both `mechanisms/`, all three `tools/` → `spec` (ruling 21). **With
no README in this package, no file in B6 takes `doc` and no file takes an
`audience`** — ruling 8's "absent everywhere else" governs all eighteen.

## The two SKILL.md files will not reach zero, and that is correct {#f083}

##B6-F092-IS-EXPECTED Both `SKILL.md` files open with YAML frontmatter. The
parser reads the `name:`/`description:` block as a paragraph unit that **cannot
carry an anchor** — YAML would break. This is **F-092**, open, nine files
corpus-wide, and its fix is one exemption in `blocks.rs`, not a markup move.
*(Filed as F-083 until 2026-07-26; that id was already spent on the GFM
task-list gap DRIFT-031 closed — see the plan's §7 entry for F-092.)*

##B6-F092-EXPECTED-RESIDUE **Expected end state for this batch: exactly one
unmarked unit in each of the two `SKILL.md` files, and zero everywhere else.**
Measured precedent: the go twins sit at exactly that today. Mark the body of
both files normally; the document marker goes after the frontmatter's closing
`---`, as the go twins do.

##B6-NO-IN-FILE-DODGE **Do not invent an in-file workaround.** A previous worker
found one and correctly declined to apply it. Do not edit, reorder, or annotate
the frontmatter. Report the two residual units in your report and stop there.

*(B5's brief did not mention this and B5's acceptance clause therefore asked for
something unreachable. Corrected here.)*

## The cards are two shapes, not one {#cards}

##B6-EIGHT-COMPACT-CARDS Eight cards (`a`, `b`, `c`, `e`, `f`, `g`, `h`, `i`)
are the **compact** form: Band 1 and Band 2 are each a **single unsplit
paragraph** carrying every labelled fact (`Classification:` … `Intent:` … `Also
Known As:` … ), which is why they scan at only 3 units each. `##DECONSTRUCTION-LAW`
bites hardest here. **The go twins already show the target shape**: one sibling
paragraph per bold-lead label, each with an UPPER anchor named for its label
(`##CLASSIFICATION`, `##INTENT`, `##ALSO-KNOWN-AS`, …). That is ruling 4 plus
rulings 20/22, already adjudicated on this exact text — follow it.

##B6-CARD-D-IS-ALREADY-EXPANDED `scaffold-d-differential-oracle.md` is the
package's declared *reference instance* and the author already wrote it in the
expanded form — hence 48 units against the others' 3. It needs anchors and
markers, **not** re-splitting into a shape it is already in.

##B6-BAND-THREE-IS-CODE Band 3 is a fenced `card-ops` block in the eight compact
cards and **carries no markers** (`##FENCE-AWARE`); it is not a unit. In
`scaffold-d` Band 3 is prose and **is** marked.

## Two things this batch is likely to surface {#expect}

##B6-EXPECT-VERSION-CLAIMS The guide states tool versions, ecosystem facts and
licences — node ≥ 22.6, TS 7 / "Corsa", `erasableSyntaxOnly`, the PLDI'25 74.8 %
figure, Apache-2.0 for the TypeScript Compiler API — and names a dozen crates
and binaries. Those are checkable against `vibe.toml` and the tree, and may be
**stale**. That is a finding, reported, never fixed.

##B6-EXPECT-HOST-ONLY-CITATION A document in this package cites a path that only
exists in the *host* repository, not in the package a consumer installs. Where a
citation's target is outside this package, **check whether it resolves for a
consumer** before marking, and report it if it does not. Mark stage and state
and move on — this is a verdict question, not a marker question.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors.
  A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit.** The reviewer reads the diff and commits.

## Acceptance {#acceptance}

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

**Two unmarked units remain in the batch's 18 files — the two `SKILL.md`
frontmatter paragraphs (F-092) — and nothing else.** Every marked unit
anchored; no id collision within a file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits and
anchors and nothing else.

## Report back {#report}

Per-file counts · every `@unknown` with its text and why · every semantic
problem seen and not fixed · any case the twenty-nine conventions did not
decide · every place the go twin was **not** a safe guide, and why. Eleven
batches have run; **every one found a factual error in its own brief by
measuring.** If this one is wrong, say so with the measurement.
