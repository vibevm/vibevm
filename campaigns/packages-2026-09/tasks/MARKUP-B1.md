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

**Fifty-eight rulings now, of which two are struck (18, 19).** They accumulate in
one place on purpose: a batch reads this list and nothing else to know what was
already decided. **The list is a derived thing too and nothing recomputes it** —
B5 caught two rulings that had outlived their findings by a day, and B6 caught a
justification in its own report that did not survive being checked.

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

### Added by the B1b pass (2026-07-26) {#locked-b1b}

16. **Tables.** `##ROW-*` anchor on the **first** body cell of each row, a marker
    on **every** non-empty body cell, uniform stage per row; header and
    delimiter rows untouched. This is the host corpus's own live pattern
    (`PROP-037`, `PROP-017`, `PROP-003`) and it puts visible `@impl/done` text
    into rendered tables — that is accepted, not an oversight.
17. **Empty table cells stay bare.** Verified: `--exhaustive` honours
    `##COUNTABLE-UNITS`' "non-empty" literally. Do not insert an em-dash to
    make a blank markable — that is a content edit.
18. ~~Do not mark GFM task-list items — blocked by F-083.~~ **OBSOLETE since
    DRIFT-031 (`4f9143b4`).** `parse/facts.rs::task_box_len` now treats the
    checkbox as structure, so `- [ ] ##ID … @impl/done` is legal and a test
    asserts it. **Mark task-list items normally.** *(Caught stale by B5, which
    read the ruling against the code rather than trusting it — the ruling had
    outlived its finding by a day.)*
19. ~~Marker position when the text carries a fenced-code backtick — F-084.~~
    **OBSOLETE since DRIFT-031 (`75009f8c`).** `parse/blocks.rs::blank_inline_code`
    was rewritten to run-matching, so a trailing marker survives beside a quoted
    fence and a test asserts it. **Place markers last-token as everywhere else.**
    *(Also caught stale by B5. Two rulings out of twenty-nine had outlived the
    findings that produced them, and neither bit that batch — the go corpus has
    no task lists and no quoted fences — but both bound B6 onward as written.
    A convention list is a derived thing too, and nothing recomputes it.)*
20. **Colon-split tiebreaker: the verdict test.** When ruling 3 leaves a colon
    ambiguous, ask whether **Phase C could assign a separate verdict to each
    item**. "What is unusual is everything around it: types, contracts,
    metadata, the verification loop" splits — each is separately checkable.
    "Keep the three layers apart: method, project, machine" does not — the claim
    is the separation, and the three names are its terms. This reconciles B1a's
    aggressive split with B1b's conservative keeps; both were right under it.
21. **Ruling 7's "normative root document" meant *normative document*.** Every
    normative document takes `stage="spec" state="done"`, root or not.

### Added by the B2 pass (2026-07-26) {#locked-b2}

22. **An enumerating colon beats the semicolon rule.** Ruling 3 says a
    semicolon-joined pair of parallel clauses does not split; ruling 20 says
    split when Phase C could verdict each item. They collide when a colon
    introduces semicolon-separated items. **Resolution: an enumerating colon
    plus a passing verdict test splits, whatever the separator; a semicolon
    pair with no colon lead stays whole.** B2 called this the one most worth
    locking — it decides roughly a dozen splits per batch.
23. **A mixed table row takes the outside-world stage.** Ruling 16 forces one
    stage per row; where a row mixes outside-world facts with our own posture,
    `@spec/done` governs the row. The same content as a **list** keeps per-item
    stages, which is better — prefer the list where the source allows it.
24. **A registry record cited by id takes UPPER**, even when its content is
    evidence. The kebab register is for text nothing cites; UPPER is for
    content that gets cited. (ATLAS's 87 entries are UPPER on this reading.)
25. **Appendix document markers go by content, not by folder.** An appendix
    recording decisions and their rationale → `spec/done`. An appendix that is
    derived and rendered for humans → `doc/done`.
26. **A scheduled, unexecuted procedure step is `@spec/done`; a timeless rule
    in force is `@impl/done`** — the latter even when its checker ships in
    another package (ruling 10). Measure before choosing: B2 marked PROP-014
    §4's migration playbook `spec` after verifying the crate it plans does not
    exist.
27. **A `` `req rN` `` kind line is a countable unit** and takes a kebab
    service anchor named for its section (`##kind-line-<section>`).
28. **A lead-less split is legal.** A bare comma-separated list with no colon
    may become bullets; bullet characters are not invented words.
29. **Future work: what the unit *is* decides the register.** A claim *about*
    future work is `@spec/done` (ruling 10). A map entry that *names a
    deliverable* which does not exist yet is `@idea/plan` — B1's
    `##MAP-RUST-TCG`. The test is whether the unit asserts a fact about the
    plan, or is itself a pointer to an unbuilt thing.

### Added by the B6 pass (2026-07-26) {#locked-b6}

30. **A lazy continuation at column 0 gets its blank line back.** A sentence
    written at column 0 immediately after a list item is folded by markdown into
    that item, so the item ends up asserting something it does not assert and
    the sentence cannot be marked as itself. **Insert the blank line.** Ruling
    14 already licenses whitespace-only insertion for exactly this class of
    parser accident. **Scope it narrowly:** only at column 0, only where the
    sentence is section-level (a `*Rule:*`, a closing summary over the list),
    and only where the same file shows the author's own paragraph-level form of
    the same construct. *(B6 did this twice in the TypeScript guide. Its report
    justified it as «every other section states its `*Rule:*` at paragraph
    level», which is **false** — checked: the guide has exactly two `*Rule:*`
    lines at column 0, and only one of them has its blank line. The move is
    right and the evidence for it is one internal precedent, not a pattern.
    B7's Rust guide carries the same sentence in the same shape, once.)*
31. **Ruling 23 generalises past tables.** Where prose mixes an outside-world
    claim with our own posture and rulings 3 and 20 forbid splitting it, the
    **outside-world stage governs the whole unit** — `@spec/done`. Ruling 23
    said this for a table row; nothing said it for a semicolon-joined pair with
    no colon lead, which is where B6 met it twice.
32. **In a pattern card, the labelled field is the atom.** A card field
    (`Evidence & Transfer-strength:`, `Risks & Assumptions:` …) stays one unit
    even when it carries several separately-verdictable facts, and even when a
    language-specific sentence is appended inside it. The go twins do this and
    the deconstruction law's grain is the paragraph (ruling 1). **A strict
    reading of ruling 20's verdict test would split these; it does not win
    here** — the card format defines the field as the unit, and a format's own
    grain beats a general test.

### Added by the B7 pass (2026-07-27) {#locked-b7}

33. **A wrapped line whose first token is a bullet character gets re-wrapped.**
    When prose contains `-`, `+` or `*` as a word and an author's line wrap puts
    it at the start of a continuation line, markdown reads it as a list marker:
    the character **vanishes from the rendered text** and one unit becomes two.
    **Move one word across the newline so it sits mid-line.** No text byte
    changes — only which side of a newline a word is on.
    *This codifies what two batches had already done without a rule.* B5 made
    exactly this repair to «the surface is the four queries **+** lifecycle» in
    the go package and recorded it in its landing commit; B7 met the same
    sentence in rust and repaired it the same way. Ruling 30 is the same
    rationale through a different mechanism — there a blank line was missing,
    here a word is on the wrong line — and both are the parser mis-reading
    layout as structure.
    **The alternative is worse and is why this is a repair, not an edit:**
    marking both fragments puts a marker mid-sentence and mints a `spec://`
    address for a phrase that is not a fact.
    ⚠️ **The batch checker cannot see this.** `cargo xtask batch-review` drops
    standalone bullet characters from its word stream — a blind spot declared
    at the function, because reflow is legal and a position-sensitive rule
    would red-light every legal one. So a ruling-33 repair passes C3 silently
    and **must be reported by the executor and read by the reviewer**.

### Added by the B8 pass — the `world` genre calibration (2026-07-27) {#locked-b8}

34. **A comma-joined sequence of complete clauses splits; a compound object does
    not.** Rulings 3, 20 and 22 are all keyed to colons and semicolons, and
    nothing covered the comma — which B8 met ten times. Split when each element
    is a **complete clause** with its own subject and verb, separately
    verdictable, and standing in the author's own words. Keep whole when the
    elements are a **verb's compound object**: splitting would leave a dangling
    lead («…replaces the frame **with**»), and manufacturing one is exactly what
    ruling 3's no-invented-words guard forbids. *Consequence, accepted: the same
    three facts sit at grain 3 in one file and grain 1 in another, and the
    difference is pure syntax.* Ruling 28 stays narrow — a lead-less split is
    legal for a bare list that **is** the content, not for a list that is a
    verb's object.
35. **An enumerating colon introduces members of a set the lead names. Anything
    else is explanatory and stays whole — even when ruling 20's verdict test
    passes.** A colon introducing the lead's *contrast*, *reason* or *definition*
    does not split.
    **The control that decides it, and it is internal to the corpus:** the same
    three facts appear in `25-flow-decision-records.md` as a semicolon triple
    with no colon, where rulings 3 and 22 unambiguously keep them whole, and in
    `DECISION-RECORDS-PROTOCOL.md` behind a colon. Reading that colon as
    enumerating would split one and not the other **for a punctuation difference
    between two files saying the same thing**. Reading it as explanatory puts
    both at the same grain. That is the strongest evidence this campaign has
    produced for any convention.
36. **Em-dash appositives stay whole.** Nothing in the rulings mentioned `—`;
    treat it as ruling 3's explanatory colon, even when the clause after it
    reads as a standalone rule.
37. **A Composition entry takes the stage its predicate asserts, never a
    genre-uniform one.** A rule in force or a checkable property of a shipped
    package → `@impl/done`; typical practice («often precedes»), a
    recommendation, or a positioning claim → `@spec/done`. **Uniformity was
    considered and rejected:** it produces visibly different stage mixes in two
    adjacent READMEs, which looks uneven and is correct — the two files assert
    different kinds of thing, and forcing one stage would make one of them
    wrong. Register is UPPER either way (ruling 9, package-map entries).
38. **A flow's `spec/flows/**` documents take `spec/done`, and `audience` stays
    absent on all of them** — including a consumer-facing `usage.md`. Rulings 7,
    21 and 25 named README, boot snippet, normative document and appendix but
    not this genre. The precedent is uniform across the 74 already-marked
    package files: every file under `spec/` that is not a boot snippet or a
    `SKILL.md` carries `spec/done`, and the language stacks' `spec/*/tools/*.md`
    are the same genre. Ruling 8's «absent everywhere but a README» holds.

### Added by the B9 pass (2026-07-27) {#locked-b9}

39. **Ruling 35 is applied literally, and it beats ruling 34 where they
    collide.** B9 met the same three facts twice inside one package — «rots
    **three ways**: A, B, C» in a README and «rots: A, B, C» in a boot snippet.
    Ruling 35's test splits the first (the lead names a set) and keeps the
    second (it does not), which is exactly the cross-file grain divergence
    ruling 35's own control argument was written to avoid. **Apply 35 anyway.**
    Ruling 34 already accepts that grain can differ on syntax alone, and a test
    that bends when its own output looks uneven is not a test. *(4 units in the
    README, 1 in the snippet.)*
40. **A bare connector colon is not a unit; a lead-in with content is.** A
    trailing lead of ≤3 words carrying no independent claim — «Compare:»,
    «Then:», «Watch for:», «Short version:» — is connective tissue and rides
    with the sentence before it. A lead that carries its own content — «Two ways
    in:», «The body includes, at minimum:» — is its own unit. **Exception:** a
    connector already standing as its own paragraph in the source stays one
    (B8's `##side-by-side-lead`).
41. **A multi-sentence blockquote stays one unit.** Ruling 13 makes a marked
    blockquote a unit but does not exempt it from deconstruction; splitting one
    needs blank-separated `>` blocks, which renders as several quote boxes.
    That is a form change beyond a sense-preserving split, so it is not made.
    *Also settled, needing no ruling of its own: a compound predicate sharing
    one subject («This package sorts…, fixes…, and pins…») matches neither pole
    of ruling 34 and stays whole; and a `**Scope of this document.**` paragraph
    carrying a second fact still splits, with the bold label riding on the
    first heir.*

### Added by the B10 pass (2026-07-27) {#locked-b10}

42. **An arrow chain `→` does not split.** Nothing covered `→`, and B10 met a
    four-rung ladder compressed into one arrow-separated sentence. Ruling 36's
    principle generalises: **an unruled separator is non-splitting.** *Accepted
    consequence, and it is the same shape ruling 39 accepts: the four rungs are
    four table rows in one document and one unit in another.*
43. **«Sequence» in ruling 34 means three or more.** A two-clause «X, and Y»
    compound sentence stays whole. All ten instances that produced ruling 34
    had three or more elements, so the ruling was written from a population that
    never tested two — this settles it rather than leaving each batch to guess.
44. **Ruling 20's verdict test gates ruling 34, not only ruling 3.** A
    comma-joined sequence that passes 34's grammatical test but fails 20's
    verdict test stays whole: «It is boring, it is unskippable, and it does not
    delegate» is three terms of one claim, exactly like ruling 20's own
    «keep the three layers apart: method, project, machine». Ruling 20 was
    stated as a tiebreaker for ruling 3 alone; it binds 34 as well.
45. **The colon-segment tiebreaker, from a measured corroboration.** B10
    measured every colon decision B8 and B9 made: **every split had three or
    more post-colon segments; every keep had one.** So a colon with 3+ segments
    splits and a colon with 1 does not, which matches rulings 22 and 35 without
    contradicting either. **Two segments is genuinely undecided** — B10's two
    such calls went opposite ways and both are defensible. Report a
    two-segment colon rather than settling it silently.
46. **Ruling 32 is confined to the pattern-card format.** A `**Bold label.**`
    paragraph in ordinary prose splits when it carries a second fact (ruling 41),
    and the label rides with the first heir. Ruling 32's «the labelled field is
    the atom» governs card Band-1/Band-2 fields and nothing else.
    *Also recorded, needing no ruling: ruling 23 sends an outside-world «why»
    column to `@spec/done` even when the table's rules are in force — B10 marked
    39 such cells that way. It is 23 working as written, and the list form it
    recommends is not available, because turning a table into a list is not a
    sense-preserving split.*

### Added by the B12 pass (2026-07-27) {#locked-b12}

47. **A hyphen wrapped to the end of a line is repaired, the way a bullet is.**
    A trailing `-` before a newline renders as `word- rest` — the parser reading
    layout as content, ruling 33's family exactly. **Move one word across the
    newline.** Ruling 33 licensed this only for a bullet character because a
    bullet was the only case anyone had met; B12 met four hyphens in one package
    and correctly left them, having no licence. It has one now.
48. **An enumerating colon does not split when a substantive clause trails the
    enumeration.** Attaching the coda to the last item changes what that item
    claims. **A bracketed source pointer is not a coda** — a citation changes
    nothing the item asserts and rides with it. This decided five sites in B12
    and is the third clause of the split test, after «the lead stands alone» and
    «each item is separately verdictable».
49. **Ruling 40 gates the colon rules from the lead side, as ruling 20 gates
    them from the item side.** If the only available lead is a ≤3-word connector
    with no independent claim, and no preceding sentence in the same paragraph
    can carry it, then **the colon cannot split** — there is nowhere legal to put
    the lead. B12 met the identical shape three times and it split once, purely
    because a sentence was available to ride with.
50. **Ruling 45's «every split had 3+ segments» is a correlation, not a law.**
    B12 kept a three-segment colon whole with a stated mechanical reason: one
    item carried an em-dash parenthetical whose closing dash would leave the
    bullet ending on nothing, so it could not stand in the author's own wording
    (ruling 3). **Segment count is a tiebreaker, and rulings 3, 20, 35, 48 and 49
    all outrank it.**

*Reported and deliberately not ruled:* B12 drew a stage line for a package whose
subject is human and AI cognition rather than this repository — a stated rule or
described mechanism `@impl/done`, a claim about how humans and models behave
`@spec/done`. It is ruling 10 read literally and it produced that batch's
largest block of `@spec`. **Left as the executor drew it**; if a future batch on
outside-world subject matter disagrees, that disagreement is the evidence a
ruling would need.

### Added by the B13 pass (2026-07-27) {#locked-b13}

51. **The pointer unit is `##sibling-document-pointers`.** Measured across the
    landed corpus: **7 packages to 2** — `addressable-specs`, `spec-genres`,
    `wal`, `qualified-naming`, `source-mirrors`, `campaign-plans`,
    `operating-modes` against `decision-records` and `health-audit`. *(B13's
    brief said «three batches to two», counting batches and omitting three B9
    packages; the correction strengthens the ruling rather than reversing it.)*
    **Known cost, accepted:** `secrets-hygiene/…/scope-discipline.md` calls
    itself «the **companion** to the four laws» in the author's own words, so
    its id now contradicts its sentence. Uniform ids beat locally apt ones —
    the id is an address, not a description.
52. **An enumeration BEFORE the colon, with the claim after it, stays whole.**
    «Publish tokens, registry credentials, provider keys: none of them are
    exported into a process running third-party code.» Every colon ruling
    (22, 35, 45, 48, 49) assumes lead-then-items and none reaches this shape.
    Ruling 3 decides it: splitting leaves the claim with nowhere to live.

### Added by the B14 pass (2026-07-28) {#locked-b14}

53. **Ruling 35's "a set the lead names" covers a condition, an instruction or a
    bare predicate — the colon may supply the lead's complement.** What 35
    excludes is a colon introducing the lead's *contrast*, *reason* or
    *definition*, which is what its own control was about.
    **Measured, and this is why it outranks the literal reading:** since ruling
    35 was locked at B8, **69 lead-then-manufactured-list sites have landed**
    across the marked corpus, and they routinely carry leads that name no set —
    «When the channel degrades, the symptoms are always the same:»
    (`two-process-model/…/files-as-ipc.md`), «Emit a short end-of-session report
    in the chat:» (`wal/…/session-end-hook.md`), «The repair is never
    mysterious:» (same file), «Concretely:»
    (`spec-genres/…/SPEC-GENRES-PROTOCOL.md`). Reading 35 literally enough to
    keep those whole would contradict the majority of the corpus it governs.
    *B14 met the question twice and flagged rather than settled it; the count is
    what decided it, not the argument.*
54. **A `SKILL.md`'s document marker stands in the preamble, before the
    `# Title`.** `##PLACE-DOCUMENT` reasons from "every file here opens with its
    heading", which is false for this genre — a skill opens with YAML
    frontmatter — so the rule does not reach it and there is no contradiction to
    resolve. Both landed siblings (`wal-status`, `health-audit`) put the marker
    after the frontmatter and before the title, and the gate reads it as the
    document marker.
55. **The trailing sentence of a deconstructed paragraph is not a ruling-30
    candidate.** `batch-review`'s C2 surfaces "a paragraph sitting directly after
    a list item", which is true **by construction** of every manufactured list
    whose source paragraph continued past the enumeration — the blank line is
    part of the split, not a separate repair. B14 produced four and all four were
    the split's own tail. Check them against the pre-state anyway; the point is
    that a C2 queue of this size is expected, not suspicious.
56. **Ruling 51's uniform id names the *primary* pointer unit; a second pointer
    paragraph in the same file takes a descriptive name.** Ids are unique per
    file, so `##sibling-document-pointers` cannot repeat. B14 met two such files
    and used the landed style for the second (`##full-protocol-pointer`,
    `##review-checklist-pointer`). Ruling 51 measured 7 packages to 2 and never
    considered the two-pointer case.

### Added by the B15 pass (2026-07-28) {#locked-b15}

57. **The colon boundary ruling 53 was missing: instances split, procedure steps
    and glosses do not.** A colon introducing **instances, members or cases** of
    what the lead names → **split**. A colon introducing the **steps of a named
    procedure**, or an **appositive gloss** of the lead, → **keep whole**.
    **The control is internal and same-file, which is the strongest kind this
    campaign recognises.** `wal/…/spec/flows/wal/session-end-hook.md` keeps
    «**Scope of this document.** The procedure every session ends with: confirm a
    good stopping state, rewrite `spec/WAL.md`, overwrite `CONTINUE.md`, report.»
    whole at four comma-separated steps behind a colon (line 5) — and splits
    «Emit a short end-of-session report in the chat:» into four bullets 120 lines
    later. One file, one author, one batch, opposite calls, and the difference is
    exactly steps-of-a-procedure against members-of-a-set.
    This bounds ruling 53, which was the widest colon ruling on the list and was
    touching ruling 35's definition carve-out. **A definition *by extension* is
    still an enumeration** — «Routine means: A, B, C, D» splits, because the
    lead literally names the set and the items are literally its members.
58. **When a file carries more than one pointer paragraph, ruling 51's uniform id
    goes to the one pointing at the flow's *sibling documents*; a pointer whose
    target is a single named document takes a descriptive id.**
    `##full-protocol-pointer`, `##review-checklist-pointer`,
    `##splitting-procedure-pointer` are the landed descriptive names.
    Ruling 56 said "primary" and never said how to pick it; measured across the
    corpus, **no positional rule survives** — of the three files with two pointer
    paragraphs, the uniform id is last in two and first in the third. Function
    decides it, position does not. *(Ruling 51 is untouched for the ordinary
    case: a file with ONE pointer paragraph uses the uniform id whatever it
    points at, which 35 landed files do — 16 of them at a single link.)*

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
