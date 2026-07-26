# MARKUP-B1 — `core-ai-native` v0.8.0, the guiding and operating layer {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus, per package.
**Reviewer:** Fable — reviews **every** diff and owns sense-preserving splits,
anchor names and `audience` (owner ruling 2026-07-26, hybrid markup).
**Corpus:** `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/`.

## Why this batch is not the whole package {#split}

`BATCH-PLAN.md` allocates `core-ai-native` to **B1–B2** — 16 files, still the
largest single package in Phase B. The split is by **genre**, because genre
decides the marker vocabulary and a batch is a review unit:

| batch | what | files | lines |
|---|---|---|---|
| **B1** | guiding + operating layer | 9 | 689 |
| B2 | mechanisms + appendix | 7 | 950 |
| ~~B3~~ | ~~`legacy-projections/`~~ | ~~11~~ | ~~1 264~~ |

**B3 is retired.** Owner ruling 2026-07-26 (F-080) makes
`spec/legacy-projections/` frozen history — the same category as a superseded
version slot — so it leaves the corpus rather than being marked. That is 11
files and 1 264 lines, the fourth subtraction this campaign has made by asking
what the corpus is made of.

B1 itself runs in two passes: **B1a** calibrates the conventions on three
files, the reviewer locks them, **B1b** takes the remaining six. The point is
not caution for its own sake — it is that re-reviewing 689 lines of
wrong-shaped markup costs more than reviewing 183 twice.

- **B1a (calibration):** `README.md`, `spec/boot/10-flow-core-ai-native.md`,
  `spec/00-MANIFESTO.md` — 183 lines.
- **B1b:** `spec/01-PATTERN-CARD-FORMAT.md`, `spec/02-EXECUTABLE-SCAFFOLDS.md`,
  `spec/03-RAID-PLAYBOOK.md`, `spec/04-SWEEP-PLAYBOOK.md`,
  `spec/05-CAMPAIGN-FORM.md`, `spec/06-WAL-CONVENTION.md` — 506 lines.

`LICENSE.md` is **not** in the corpus (verbatim third-party text, F-070).

## The grammar, compiled in {#grammar}

Governing contract: [PROP-043 §3](../../../spec/modules/vibe-progress/PROP-043-progress-markup.md#markup).
Everything below is quoted from it; where this file and PROP-043 disagree,
PROP-043 wins and the disagreement is a finding.

**Exhaustive at fact grain** (`##COUNTABLE-UNITS`): every paragraph, every list
item at every nesting level, and every non-empty table **body** cell is a unit
of its own and carries its own marker. Header and delimiter rows are structure,
not units.

**Anchored when marked** (`##ANCHORED-WHEN-MARKED`): every marked paragraph or
list item MUST carry a `##<ID>` anchor as its **first token**. A marked,
anchor-less unit is a `check` error. **Table cells are exempt**
(`##CELLS-ANCHOR-EXEMPT`) — mint a cell id only where something cites it.

**Two registers** (`##DECISION-TWO-REGISTERS`) — this is the one that carries
meaning at zero syntax cost, so get it right:

- `##UPPER-SLUG` — a **normative fact**: a law, a rule, a carrier, a changelog
  entry. Content with binding weight.
- `##kebab-case` — a **service unit**: status lines, lead-ins, connective prose.

**Id grammar** (`##FACT-ID-GRAMMAR`): `[A-Za-z][A-Za-z0-9_-]*`. The unit becomes
addressable as `spec://…/<doc>#<ID>`, sharing one address space with heading
`{#anchor}`s — **a duplicate across both forms is a `check` error**. Ids are
unique **per file**, not corpus-wide (measured in wave 1: 316 names live in
more than one file).

**Marker position** (`##ANCHOR-MARKER-POSITIONS`): immediately after the anchor,
or as the unit's **last** token. Prefer last-token for prose units, which is
what the host corpus does and what the Phase A pilot did.

**Shorthand** (`##SHORTHAND-FORMS`, `##SHORTHAND-STANDALONE`): `@<stage>/<state>`
is the macro-equivalent of a point marker and is recognised **only** as a
standalone token at the start or end of a unit's text — never mid-sentence,
never inside code or links. Use the shorthand; reserve the XML form for
document/section markers and for fragments.

**Document and section markers** (`##PLACE-DOCUMENT`, `##PLACE-SECTION`): every
file here opens with its heading, so the standalone marker immediately after
that **first** heading is the **document** marker. For every other heading, a
standalone marker on its own line immediately after the heading line is that
**section**'s marker. A standalone marker anywhere else is a `check` error
(`##NO-ORPHAN-MARKER`) — there is no nearest-paragraph heuristic.

**Fence-aware** (`##FENCE-AWARE`): markers are **not** recognised inside fenced
code blocks, inline code spans, or URLs. Do not put them there, and do not
count a fenced block as a unit.

**Fragments** (`##INLINE-FRAGMENT`): a fact that cannot leave its sentence is
wrapped `<status …>the fact</status>`. Use sparingly — a fragment is the tool
of last resort, after splitting has been tried and failed.

### The deconstruction law — the heart of this pass {#deconstruction}

`##DECONSTRUCTION-LAW`: a paragraph carrying **more than one fact is
deconstructed** — rewritten, **sense-preserving and wording-preserving**, into
a bulleted or numbered list with one fact per item, each item marked. Most
prose here is expected to become lists. A paragraph stays prose only when it
truly carries one fact, or none (connective tissue).

`##FORM-ONLY`: **deconstruction changes form only.** Reuse the author's words.
Do not improve, compress, modernise, re-order for elegance, or fix what looks
like a mistake. If a sentence reads badly, it stays reading badly.

This corpus is dense — `README.md`'s opening paragraph carries at least four
facts and the boot snippet's carries about the same. Expect heavy splitting.

## Stage and state — the convention this repo already uses {#vocabulary}

Vocabularies are **closed** (`##VOCAB-CLOSED`); a value outside the tables is a
`check` error.

- A fact asserting that something **exists or behaves** — a shipped rule, a
  mechanism, a described behaviour → `@impl/done`.
- A fact that is **rationale**: a *why*, a *considered and rejected*, a *revisit
  when*, or a rule stated with no machinery behind it → `@spec/done`.
- A fact you **cannot classify with confidence** → `@unknown` (which is
  `state="hold"`, an explicit triage demand) **and list it in your report**.

That last line is not a formality. Wave 1's most expensive lesson, recorded in
its own close-out, was writing *measured* about things that had been *inferred*.
**A wrong `@impl/done` is worse than an honest `@unknown`**, because Phase C
will verify the marker and a confident wrong one costs a verdict cycle. Guess
nothing.

`audience` (`##AUDIENCE-VALUES`): `user` · `author` · `dev`; absent means `dev`.
Set it only where it is **obvious** — a README addressed to a consumer is
`audience="user"`. When in doubt leave it absent; the reviewer owns this axis.

## Conventions LOCKED by the B1a calibration (2026-07-26) {#locked}

B1a marked 141 units over 183 lines and surfaced thirteen cases the grammar
above did not decide. These are the reviewer's rulings; **they bind B1b and
every batch after it.** Where a ruling contradicts the prose earlier in this
file, the ruling wins — the earlier text was written before the cases were known.

1. **Deconstruction grain is the paragraph, not the list item.** A
   *pre-existing* list item stays whole even when it carries several facts.
   `##DECONSTRUCTION-LAW` binds "a **paragraph** that carries more than one
   fact"; `##COUNTABLE-UNITS` is satisfied by one marker per item; and PROP-043's
   own live markup keeps multi-fact items whole. **This overrides the "one fact
   per item" phrasing in this file's target-shape section, which was looser than
   the law it paraphrased.**
2. **Exception, and it is forced by the grammar, not by taste:** an item whose
   facts need **different markers** must split, because `##MULTI-MARKERS` allows
   at most one status marker per node. An unsplittable two-stage item is
   unmarkable. This is what resolved `##MAP-RUST-TCG`.
3. **Split guard — no invented words.** A colon splits when the lead and every
   item stand in the author's own wording. An *explanatory* colon does not, and
   a semicolon-joined pair of parallel clauses does not (`##SEP-NEVER`).
4. **When the facts are not parallel, split into sibling paragraphs** rather
   than forcing a bulleted list. `##DECONSTRUCTION-LAW`'s end state is "one fact
   per unit"; "most prose becomes lists" is its expectation, not a requirement,
   and a forced list over non-parallel sentences is less sense-preserving.
5. **Heading `{#anchor}`s ARE owed.** `##FORM-ONLY` says campaign passes still
   add missing ones, and `BATCH-PLAN.md` lists them in what a batch owes. B1a
   shipped without them and the reviewer added them; **B1b adds its own.**
   Kebab-case, short, derived from the heading text. Watch
   `##FACT-ID-GRAMMAR`: heading anchors and fact ids share one address space, so
   do not mint `{#central-law}` beside a `##CENTRAL-LAW` fact — vary the wording.
6. **No section markers.** `##PLACE-SECTION` defines the position, but sections
   are not countable units and nothing requires one.
7. **Document-marker stage by genre:** a README → `doc/done` with
   `audience="user"` (the pilot shape); an installed boot snippet →
   `impl/done`; a normative root document → `spec/done`. **No `comment=`** —
   the pilot carries none and provenance is in git.
8. **`audience` only where obvious.** README `user`; absent everywhere else,
   including boot snippets, whose real reader is a *consuming project's* session
   and fits none of `user`/`author`/`dev` cleanly. Leaving it absent is the
   honest answer; do not stretch a value to fit.
9. **Register case law.** `##UPPER-SLUG` for laws, axioms, vocabulary and legend
   entries, package-map entries, and enumerated limitations. `##kebab-case` for
   status lines, colon lead-ins, connective sentences, and rationale or
   evidence prose. This is PROP-043's own live pattern.
10. **Stage discriminator.** A claim verifiable against this corpus or this
    repository → `@impl/done`. A claim about the outside world, about
    motivation, or about the future → `@spec/done`. A stated rule is
    `@impl/done` **even when its checker ships in another package** — the fact
    realized here is that the rule exists and is in force.
11. **`@unknown` is the honest answer and it is cheap.** Prefer it to a guess and
    list it in the report; the reviewer adjudicates. B1a left two, both correct
    to leave.
12. **Splitting an italic paragraph re-applies its emphasis.** Those `*`
    characters are the only text bytes a markup pass may add beyond anchors and
    markers.
13. **Blockquotes are anchored and marked** — verified: `progress check` reads a
    marked blockquote as a unit, not as an orphan marker.
14. **A title with no preamble gains one blank line** before its document
    marker, so the marker does not read as an orphan. Whitespace only.
15. **Anchor names: descriptive over short.** Do not shorten for taste; the
    correction contract cites these, and a long clear name beats a short cryptic
    one. Keep them consistent within a file.

## Boundaries {#boundaries}

- **Semantic edits are forbidden** (plan §5-B). The diff contains **markers,
  sense-preserving splits and anchors only**. A semantic problem you find —
  drift, a contradiction, a broken cross-reference, a stale version — is
  **reported, never fixed**. It becomes a finding.
- **Do not touch `spec/**`** anywhere in the repository. That is the reviewer's
  exclusive lane in this campaign.
- **Do not touch** `crates/`, `vibedeps/`, `vibe.lock`, `progress.toml`, or any
  other package.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.** A
  concurrent task holds the floor, and both write the real `~/.vibe`; a
  concurrent write turns the floor red for a reason that has nothing to do with
  either task. The reviewer runs the gate.
- **Do not commit.** The reviewer reads the diff and commits.

## Acceptance {#acceptance}

The reviewer runs, after the batch:

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
```

- every unit in the batch's files carries a marker; zero unmarked paragraphs,
  list items, or non-empty table body cells in them;
- every marked unit carries an anchor, and no anchor collides with another in
  the same file (including heading `{#anchor}`s);
- `git diff` shows markers, splits and anchors — **and nothing else**. A
  reworded sentence is a review rejection for the whole file.

## Report back {#report}

1. Per file: units marked, paragraphs deconstructed into lists, anchors minted.
2. **Every `@unknown` you left, with the unit text and why you could not
   classify it.**
3. **Every semantic problem you saw and did not fix** — this is the findings
   harvest and it is as valuable as the markup.
4. Anything where the grammar above did not decide the case. Those are the
   conventions the reviewer locks before B1b.

## Open findings touching this batch {#findings}

- **F-069** — the aggregator grammar gap. **Not this phase's problem.** A marker
  records a fact's stage and state; whether a document can be the *source of
  truth* for a fact about another package is a question about its **verdict**,
  which is Phase C's. Mark stage and state and move on. Do not stall.
- **F-080 — RULED 2026-07-26, closed.** `spec/legacy-projections/` is 11 files
  and 1 264 lines of substantive normative prose that **nothing in the living
  corpus cites**; the go stack's guide says GUIDE-GO-v0.1 «stays, **untouched**»
  there, and the typescript stack declares GUIDE-TYPESCRIPT-v0.1 superseded.
  Owner: «замороженная история. Мы когда-нибудь покроем эти языки, но еще не
  сейчас. Сейчас у нас есть активные rust, typescript и go.» It leaves the
  corpus via `progress.toml`'s exclude list, genre-shaped so a future version
  slot cannot re-admit it. **Nothing in `legacy-projections/` is marked.**
