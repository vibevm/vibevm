# DRIFT-031 — the parser stops swallowing two units it can already see {#root}

```
<status stage="impl" state="plan" ref="DRIFT-031"/>
```

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** common (`progress-core`)
**Findings:** F-083 and F-084 (campaign LOG, 2026-07-26).

## 1. Goal {#goal}

Two markup shapes that PROP-043's grammar allows in principle become
expressible in fact: a GFM task-list item can carry a fact anchor, and a unit
whose text quotes a fenced-code backtick can carry a trailing marker.

## 2. Contract {#contract}

```
> **Fact anchors — the anchored-when-marked law**. A stable fact address is
> written `##<ID>` as the **first token** of a paragraph or list item.
> — spec://vibevm/modules/vibe-progress/PROP-043#FACT-ANCHOR-SYNTAX
```

```
> every paragraph, list item, and non-empty table body cell carries its own
> marker — these are the **countable units** the exhaustive counter enforces
> — spec://vibevm/modules/vibe-progress/PROP-043#COUNTABLE-UNITS
```

```
> Inside fenced code blocks, inline code spans, and URLs the element and the
> shorthand (§3.7) are **not recognized** — the scanner is fence-aware.
> — spec://vibevm/modules/vibe-progress/PROP-043#FENCE-AWARE
```

```
> A shorthand is recognized only as a standalone token at the start or end of
> a paragraph's text, never mid-sentence, never inside code or links.
> — spec://vibevm/modules/vibe-progress/PROP-043#SHORTHAND-STANDALONE
```

`##COUNTABLE-UNITS` makes **every** list item a countable unit, with no
carve-out for task lists — so the grammar already intends them to be markable,
and only the parser disagrees. `##FENCE-AWARE` suppresses markers **inside** a
code span; a marker *outside* one, at the end of the paragraph, is exactly what
`##SHORTHAND-STANDALONE` blesses.

## 3. Current state {#current}

Both reproduced 2026-07-26 by running the gate, not by reading. **Do not
re-discover; do reproduce before fixing, so you can watch each go green.**

**F-083.** In `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/02-EXECUTABLE-SCAFFOLDS.md`
§3, four `- [ ]` items were marked in the canonical shape:

```
- [ ] ##CHECKLIST-RUNS-OR-CHECKS **Runs/checks:** emits pass/fail or typed signal, not prose. @impl/done
```

`progress check` reported each as
`Error [MissingAnchor] marked unit has no ##<ID> fact anchor` — the parser reads
the checkbox as the item's first token, so the anchor that follows is invisible.
There is **no legal placement today**: putting `##ID` before `[ ]` stops it being
a task list, which is a content edit. The four were reverted and are unmarked in
the tree now; restoring them is part of this task's acceptance.

**F-084.** In the same package's `spec/01-PATTERN-CARD-FORMAT.md`, the
`##band-three-fields-lead` paragraph carries an inline code span holding a
**triple backtick** — `` ` ```card-ops ` ``. With the marker as the **last**
token the unit reported `Error [unmarked] Para unit carries no marker`; moving
the marker to position 1 cleared it, with no other change. The marker is
currently in position 1 as a workaround, and should stay valid either way.

**The failure mode is the expensive part of F-084**: the unit reports as
*unmarked*, not as a diagnostic about the code span, so a session "fixing" it by
re-adding the marker in the same position gets the identical message and loops.

Start at `crates/progress-core/src/parse/` — `anchors.rs`, `facts.rs`,
`markers.rs` — and `doc.rs`. Name the two sites in §9 with file:line before
changing them.

## 4. Required behavior {#behavior}

```
1. A GFM task-list checkbox is STRUCTURE, exactly like the `-` list
   marker and the `1.` ordinal that precede it. When locating a list
   item's first token, skip a leading `[ ]`, `[x]` or `[X]` (with its
   trailing space) the same way the list marker is already skipped.
2. An inline code span must not leave the scanner believing a fenced
   block is open. A span's backticks are span delimiters and its
   CONTENTS are inert — a triple backtick inside a span opens nothing.
   Markers inside a span stay unrecognised (##FENCE-AWARE is not
   being relaxed); markers outside one become visible again.
3. Neither change may make a marker inside a real fenced code block
   or inside a code span recognisable. That suppression is the point
   of ##FENCE-AWARE and there are goldens on it — keep them green.
```

Edge cases: a task-list item with no anchor and no marker stays unmarked and
uncounted, as today; `- [x]` and `- [ ]` behave identically; a nested task-list
item at any depth behaves like its flat sibling; a code span with one, two or
four backticks must behave like the three-backtick case; an *unterminated* span
must not hang or silently swallow the rest of the file.

Error paths: none new. Both changes make previously-invisible units visible; no
diagnostic is removed.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** PROP-043 needs an amendment recording that a
  checkbox is structure — **the reviewer writes it.** If you believe the spec
  contradicts §4, that is a §8 stop.
- **Do not edit `packages/**`** except the one restoration §6 requires, which is
  re-applying the four reverted anchors verbatim from §3.
- **Do not touch** `campaigns/**` except §9 of this file.
- Never edit a golden to make it pass. The `##FENCE-AWARE` suppression goldens
  are load-bearing here — a fix that greens by weakening them is the failure
  this task is most likely to produce.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo test -p progress-core
bash tools/self-check.sh ; echo "EXIT=$?"
```

Read the floor's **real** exit code; never judge it from a piped `tail`.

Then restore the four task-list anchors in
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/02-EXECUTABLE-SCAFFOLDS.md`
§3 exactly as §3 above quotes them (`##CHECKLIST-RUNS-OR-CHECKS`,
`##CHECKLIST-CARRIES-COGNITION`, `##CHECKLIST-FAST-ENOUGH`,
`##CHECKLIST-CANNOT-SILENTLY-LIE`), and run:

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
```

- plain `check` → **clean, 264 files, 0 warnings** (it is clean today; it must
  stay clean with the four restored);
- `--exhaustive` → **zero** hits in
  `core-ai-native/v0.8.0/spec/0[1-6]-*` (there are zero today only because the
  four are unmarked; they must now be marked *and* silent);
- move `01-PATTERN-CARD-FORMAT.md`'s `##band-three-fields-lead` marker back to
  the **last** position and confirm `--exhaustive` still reports nothing —
  then leave it in whichever position you verified, and say which in §9.

New tests in `progress-core`: one asserting a `- [ ]` item's `##ID` is found as
its anchor; one asserting a paragraph containing a triple-backtick inline code
span accepts a trailing shorthand; one asserting a marker **inside** a code span
and inside a fenced block is still *not* recognised. Name them for the
behaviour, not for the bug ids.

Discipline: `#[spec(implements = "spec://…#anchor")]` citing the §2 anchors,
`cargo fmt --all`, clippy clean, atomic commits — the two findings are two
logical changes and want **two** commits, **no AI attribution anywhere**.

## 7. Analogies {#analogies}

Whatever already skips the `-` / `1.` list marker when finding an item's first
token is the shape to extend for F-083 — the checkbox joins that set rather than
getting its own branch.

## 8. Stop rule {#stop}

- If a PROP-043 unit says a task-list item is **not** a countable unit, or that
  a code span's contents open a fence: **STOP.** `<!-- REVIEW: … -->` at the
  code point, question in §9, status `returned`.
- If F-084 turns out to be a general inline-code-span defect rather than the
  triple-backtick case, **report the wider blast radius before fixing** — the
  fix may then need to be scoped differently and that is a reviewer call.
- **Budget signal:** past **8 files / 250 lines**, stop and return.

## 9. Log {#log}

*(appended by executor / reviewer)*

### 2026-07-26 — executor: both findings fixed, floor green

**The two sites, named before changing them.**

1. **F-083** — `crates/progress-core/src/parse/facts.rs:10-26`,
   `list_item_content`. It returns the byte offset of a list item's content
   after `- ` / `* ` / `+ ` / `N. ` / `N) ` and stops there, so for
   `- [ ] ##ID …` the item's content begins at `[`. `take_fact_id`
   (`facts.rs:88`) then finds no `##` at the first token and mints no anchor,
   while the trailing shorthand still marks the unit — the exact pair that
   `check_anchor_laws` (`parse/anchors.rs:33-42`) reports as `MissingAnchor`.
2. **F-084** — `crates/progress-core/src/parse/blocks.rs:185-199`,
   `blank_inline_code`, reached from `collect_blocks`'s `flush` at
   `blocks.rs:26`. It toggled an `in_code` flag on **every individual
   backtick**. A code span whose contents hold a longer backtick run inverts
   the flag an odd number of times, so everything after the span is treated as
   code and blanked — including the paragraph's trailing marker.

**Reproduced first, from the gate.** With the four anchors restored and
`##band-three-fields-lead`'s marker moved to last position, on the unfixed
parser:

```
02-EXECUTABLE-SCAFFOLDS.md:63: Error [MissingAnchor] marked unit has no `##<ID>` fact anchor (anchored-when-marked, PROP-043 §3.8)
02-EXECUTABLE-SCAFFOLDS.md:64: …  :65: …  :66: …
error: progress check: 4 error(s), 0 warning(s)
01-PATTERN-CARD-FORMAT.md:41: Error [unmarked] Para unit carries no marker (--exhaustive)
```

Both cleared, each by its own change, with nothing else in the corpus moving.

**The changes.** `facts.rs`: `list_item_content` now composes
`list_marker_len` (the old body, unchanged) with a new `task_box_len` — the
checkbox joins the set of things skipped when locating an item's first token,
per §7, rather than getting a branch of its own. `blocks.rs`:
`blank_inline_code` matches spans by backtick **run** (a run of N opens a span
only a run of exactly N closes; a run with no matching closer is literal text
and opens nothing), blanking span contents and leaving delimiters. The
suppression is untouched — see the new
`markers_inside_code_spans_and_fences_stay_unrecognised`, and the
`foreign-grammars.md` golden, which is unchanged and green.

**§8, second bullet — F-084 IS the general defect, and here is its radius.**
The triple backtick is one instance of two mechanisms: (a) any backtick run of
length ≥ 2 desynchronises the toggle; (b) any block with an **odd** total of
backticks leaves the flag set and blanks everything to the end of the block. A
probe replicating `collect_blocks`' segmentation over all 723 `.md` files in
the tree found **23 text blocks in 21 files** where naive and run-aware
blanking disagree — e.g. `` `` `req r1` `` `` in the terraform SKILLs
(double-backtick span), `docs/glossary.md:217` (a stray tick, everything after
it in the block blanked), and the v0.7.0 twin of this very paragraph. §4's
required behaviour already prescribes the general fix (its edge cases name the
one-, two- and four-backtick cases and the unterminated span explicitly), so
the scope was not widened beyond what this task authorises.

Empirically the wider radius costs nothing today: `--exhaustive` over the whole
corpus differs from the pre-change run by exactly the four restored items
(5482 → 5478 errors), **no line added anywhere**. The 23 blocks are re-blanked
differently but none of them held a marker or anchor in the affected region.

**Marker position.** `##band-three-fields-lead` is left in the **last**
position. Both were verified against `--exhaustive` after the fix (0 hits in
`core-ai-native/v0.8.0/spec/0[1-6]-*` either way); last is the position F-084
actually broke, so leaving it there makes the file a live witness that would go
red again if the run matching regressed.

**Deviation, one, and it is forced.** §6 asks for `#[spec(implements = …)]`
citing the §2 anchors, but those are fact anchors (`##COUNTABLE-UNITS`,
`##FACT-ANCHOR-SYNTAX`, `##FENCE-AWARE`, `##SHORTHAND-STANDALONE`) and the
specmark address grammar rejects them at compile time — *"anchor must be
kebab-case `[a-z0-9]+(-[a-z0-9]+)*`"*. The tags therefore cite the **sections
those facts live in** (`#granularity`, `#placement`, `#element`, `#shorthand`)
and name the fact anchor in the doc comment beside it. Nothing in the repo
cites a fact anchor from code today; whether the grammar should accept one is a
reviewer question, not something this task settles.

**Open items for the reviewer.**

- **The `list_item_content` twins now disagree.** `crates/vibe-spec/src/facts.rs:29`
  and the package twin `core-ai-native-specmap::mdspec` carry deliberate copies
  of this function across the separability seam (the convention is held by
  tests on both sides — `crates/vibe-spec/src/facts.rs:10-15`). Only the
  progress-core copy learned the checkbox here, because §5 puts the other two
  out of bounds. A follow-up task should decide whether the spec linter's
  fact-leaf recogniser gains the same rule.
- **A parser change is invisible to a warm parse cache.** `run/cache.json` is
  keyed on file content hash only, so the first verification run after this
  change reported the *old* parse for every unchanged file. `--no-cache` was
  used throughout; anyone re-verifying should do the same or wipe the cache.
- **PROP-043 amendment still owed** (reviewer's, per §5): a checkbox is
  structure. `##COUNTABLE-UNITS` already admits task items with no carve-out,
  so §4 was implemented without a spec contradiction, but the placement rule
  says nothing about the box today.
- **Budget:** 7 files, ~280 changed lines — inside the 8-file signal, slightly
  past the 250-line one. The overrun is tests and doc comments; the production
  change is ~60 lines across the two functions.
