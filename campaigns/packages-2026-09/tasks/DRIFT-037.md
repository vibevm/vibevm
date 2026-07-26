# DRIFT-037 — a skill's frontmatter stops being mistaken for prose {#root}

```
<status stage="impl" state="plan" ref="DRIFT-037"/>
```

**Status:** queued — **do not dispatch while a markup batch is in flight.**
Both this task and the batch's gate write the real `~/.vibe`.
**Executor:** Opus. **Reviewer:** the boss, against §6 verbatim.
**Cluster:** progress-core / parse.
**Finding:** F-092 (campaign §7 LOG, 2026-07-26).

## 1. Goal {#goal}

The nine `SKILL.md` files that open with YAML frontmatter stop reporting an
unmarkable countable unit, because the parser learns that a leading
`---`-delimited block is structure — the way it already knows a comment, a
thematic break and a fence are.

## 2. Contract {#contract}

```
> every paragraph, list item, and non-empty table body cell carries its
> own marker — these are the **countable units** the exhaustive counter
> enforces
> — `spec://vibevm/modules/vibe-progress/PROP-043#COUNTABLE-UNITS`
```

```
> every marked paragraph or list item MUST carry a `##<ID>` anchor as its
> first token
> — `spec://vibevm/modules/vibe-progress/PROP-043#ANCHORED-WHEN-MARKED`
```

**The two are unsatisfiable together on a frontmatter block, and that is the
whole finding.** The block is a paragraph by `##COUNTABLE-UNITS`, so it owes a
marker; its first token is the `---` fence, so `##ANCHORED-WHEN-MARKED` has no
legal placement. Neither unit is wrong; the boundary between prose and
structure is what is unstated.

**REVIEW POINT — OPEN, owner's.** `##COUNTABLE-UNITS` names what a unit **is**
and never names what structure is. The carve-outs all live in code:
`is_comment_only`, `is_thematic_break_only`, the fence state machine, and
`task_box_len` — the last one added by **DRIFT-031, which moved that boundary
without amending the spec**. This task proposes the same move a second time and
therefore surfaces the omission rather than repeating it: PROP-043 §3.9 should
gain a structure clause naming comments, thematic breaks, fenced blocks, list
markers and ordinals, GFM checkboxes, and leading YAML frontmatter as
non-countable. **The executor does not write that clause.** The reviewer holds
it for the owner's ruling; the code change below is correct under either
outcome, because it makes the parser agree with what the campaign has been
doing since B5.

## 3. Current state {#current}

Verified 2026-07-26 by reading the parser, not by recall:

- `crates/progress-core/src/parse/blocks.rs:53` — the fence state machine
  recognises exactly ` ``` ` and `~~~`. `---` opens nothing.
- `blocks.rs:73` — a **blank line is the only thing that ends a text block**, so
  a frontmatter header, its keys and its closing `---` join **one** block.
- `blocks.rs:117` — reclassification to the non-countable `BlockKind::Comment`
  runs through `is_comment_only` / `is_thematic_break_only` only.
- `blocks.rs:155-165` — `is_thematic_break_only` requires **every line** of the
  block to be a break rule. A block carrying `name:` and `description:` fails
  it, stays `BlockKind::Text`, and becomes a countable paragraph.
- Corpus effect, measured by `check --exhaustive` on 2026-07-26: **nine files
  across six packages**, one unmarked unit each. `go-ai-native-lang`'s two are
  the entire residue of batch B5 — 663 of 665.

## 4. Required behavior {#behavior}

```
1. In `collect_blocks`, before the main loop: if line 1 of the document is
   exactly `---` (trimmed), scan forward for the next line that is exactly
   `---` (trimmed).
2. If one is found, emit lines 1..=N as a single `BlockKind::Comment` block
   and begin the main loop at line N+1.
3. If none is found, emit nothing special — the document parses exactly as
   it does today.
4. The rule fires ONLY at line 1. A `---` anywhere else keeps its present
   meaning (thematic break, or setext underline in a text block).
```

**The narrowness is the design, not caution.** It is `task_box_len`'s guard
shape from DRIFT-031 — accept the structure only in the one position where it
cannot mean anything else. F-084 is the standing lesson about the other
choice: a delimiter scanner that resynchronises loosely blanked whole files and
reported it as `unmarked`, which is a failure that reads like an absence.

**Edge cases**, each owed a test: an unterminated leading `---`; a leading `---`
whose closer is the last line of the file; a `---` on line 1 followed
immediately by a heading; a document that is *only* frontmatter; frontmatter
whose body contains a line that is itself `---`-like but indented; a thematic
break at line 1 of a file that has no frontmatter at all (three dashes, blank
line, prose) — **that last one must keep parsing as a thematic break**, and it
is the case most likely to regress.

**Error paths:** none new. This adds no diagnostic and no config.

## 5. Boundaries {#boundaries}

- Do not touch `facts.rs`, `markers.rs`, `anchors.rs`, or `units.rs`. The
  change is `blocks.rs` and its tests.
- Do not edit any `SKILL.md`, and do not add markup anywhere. The nine files
  are the *measurement*; editing them destroys the acceptance.
- Do not edit `PROP-043`. The amendment is a review point, §2.
- Never edit spec text or golden tests. Spec doubts → §8.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core parse::blocks
bash tools/self-check.sh ; echo "EXIT=$?"
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

- **Record the unmarked total immediately before the change and immediately
  after. The difference must be exactly 9 — not "about 9", and not "9 or more".**
  A larger drop means the rule is eating prose somewhere and is a failure, not a
  bonus.
- **`go-ai-native-lang` v0.1.0 reaches 0 unmarked**, 19 files of 19 — the first
  batch in this campaign to be completed retroactively by a parser fix.
- **No other file's per-file unmarked count changes.** This is the real gate;
  produce the before/after per-file lists and diff them.
- New tests, one per §4 edge case, named for the case. At least one must be a
  **negative control**: assert that a line-1 thematic break in a
  frontmatter-less document still parses as a thematic break — write it, watch
  it fail against the naive implementation, then fix. DRIFT-032's shape.
- Discipline: `#[spec(implements = "spec://…#anchor")]` on new items,
  `cargo fmt --all`, clippy clean, atomic commits, no AI attribution anywhere.

## 7. Analogies {#analogies}

`crates/progress-core/src/parse/facts.rs::task_box_len` — DRIFT-031's checkbox
rule. Same problem (structure read as the unit's first token), same solution
shape (a narrowly guarded recogniser), same corpus-count acceptance. Read it
before writing this one.

## 8. Stop rule {#stop}

- If the spec is silent or ambiguous on a point you need: STOP, mark
  `<!-- REVIEW: <question> -->` at the code point, record it here, set status
  `returned`. Do not invent semantics — and in particular **do not decide the
  §2 review point yourself.**
- Budget signal: past **2 files or 150 lines**, stop and return. A frontmatter
  rule that needs more than that is not the rule described here.

## 9. Log {#log}

*(appended by executor/reviewer)*
