# DRIFT-011 — a blockquote can carry a fact anchor {#root}

<status stage="impl" state="plan" ref="DRIFT-011"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core parser)
**Unit-stability check:** F-015 carries the owner's 2026-07-25 ruling —
«F-015 — научи срезать префикс».

## 1. Goal {#goal}

A blockquote paragraph, which can already carry a marker, can also carry a
`##<ID>` fact anchor — so a quoted normative statement is addressable like
any other fact.

## 2. Contract {#contract}

> A `##<ID>` at the start of a countable unit (any nesting depth, outside
> fenced/inline code) mints a **fact unit**.
> — `spec://vibevm/modules/vibe-progress/PROP-043#granularity`

> Anchored-when-marked: a marked fact must be anchored.
> — `spec://vibevm/modules/vibe-progress/PROP-043#granularity`

Finding realised: **F-015** — blockquote paragraphs are countable units (they
can carry a marker) but cannot carry a `##<ID>`, because the parser never
strips the `> ` prefix. Found while marking PROP-024's package-shape formula,
which had to be demoted to a plain bold paragraph as the workaround.

## 3. Current state {#current}

From Phase B/C evidence — do not re-discover:

- `crates/progress-core/src/parse/facts.rs:61` — `take_fact_id(text, s, e)`
  trims leading whitespace (line 63) and then requires the segment to start
  with `##` (line 65). A blockquote line begins `> ##ID …`, so the check
  fails and the anchor is silently not minted.
- Everything else about blockquotes already works: they are segmented into
  countable units and accept markers.

## 4. Required behavior {#behavior}

1. `take_fact_id` strips a leading blockquote prefix before looking for
   `##`: any run of `>` characters each optionally followed by a single
   space, repeated for nested quotes (`> `, `>> `, `> > `), after the
   existing leading-whitespace trim.
2. The returned offset must still point **after** the consumed `##<ID>` in
   the original `text`, so callers slicing the remainder are unaffected. Get
   this wrong and every downstream span shifts — the returned index is the
   contract of this function.
3. Nothing else about blockquote handling changes: the same id grammar (first
   char alphabetic, then alphanumerics / `-` / `_`, terminated by whitespace
   or end), the same one-anchor-per-unit rule, the same behaviour inside
   fenced code (a `> ##x` inside a fence is still not an anchor).
4. A `>` that is not a blockquote marker — inside inline code, or mid-line —
   must not be stripped. Only a prefix at the start of the trimmed segment
   counts.

Edge cases: `>##ID` with no space (valid markdown) mints the anchor. `> >
##ID` (nested, spaced) mints it. `>` alone on the line is not a unit and
never reaches this function. A blockquote line whose text begins `> > ` but
whose content is `#hash` is not an anchor (one `#`, not two).

Error paths: none — the function returns `(None, s)` for anything it does
not recognise, exactly as today.

## 5. Boundaries {#boundaries}

- Touch `parse/facts.rs` and its tests. Do not change segmentation, marker
  extraction, or the `check` rules.
- Do not re-mark any spec file as part of this task. Making PROP-024's
  formula a blockquote again is a **separate** decision and belongs to
  stitching, not to the parser change.
- Never edit spec text or golden tests.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress scan   # 58 files, 4975 markers, 0 errors — UNCHANGED
cargo run -q -p vibe-cli --bin vibe -- progress check  # must stay 0
bash tools/self-check.sh
```

- New test: `blockquote_fact_anchor_is_taken` — `> ##MY-FACT text @spec/done`
  yields `Some("MY-FACT")` and an offset pointing just past `##MY-FACT`.
- New test: `nested_blockquote_anchor_is_taken` — `> > ##MY-FACT` and
  `>> ##MY-FACT` both work.
- New test: `no_space_blockquote_anchor_is_taken` — `>##MY-FACT`.
- New test: `gt_inside_text_is_not_stripped` — `a > b ##NOT-AN-ANCHOR`
  yields `None`.
- New test: `blockquote_in_fence_is_not_an_anchor` — unchanged fenced-code
  behaviour.
- **The live corpus must not move**: the scan line stays 58 files / 4975
  markers / 0 unmarked. If the count changes, an existing `> ##…` in the
  tree was being silently ignored — that is a finding to surface (§8), and a
  welcome one, but it is not something to absorb quietly.
- Discipline: `#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#granularity")]`,
  `cargo fmt --all`, clippy clean, atomic commits, no AI attribution.

## 7. Analogies {#analogies}

`crates/progress-core/src/parse/facts.rs` itself: the existing
leading-whitespace trim at line 63 is precisely the shape to extend, and the
file's own unit tests are the style to imitate.

## 8. Stop rule {#stop}

If stripping the prefix turns out to change any existing marker's span in
the live corpus: STOP, mark `<!-- REVIEW: … -->`, record it here, set status
`returned`. Silently moving spans would corrupt the campaign's verdict
addressing.

Budget signal: past ~3 files or ~200 lines, stop and return — this is a
small, surgical change.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling «научи срезать префикс».
