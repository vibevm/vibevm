# DRIFT-002 — split progress-core `parse.rs` to hold the file budget {#root}

<status stage="impl" state="done" ref="DRIFT-002"/>

**Status:** done — executed by Opus 2026-07-24, reviewed and accepted by
Fable the same day (motion verified by spot-diff, behavior by the §6
differential oracle — corpus identical modulo timestamp; 31 tests green;
conform 0 new findings; `self-check` all green, exit 0). Accepted
deviation: one-line `//!` module docs per new file, the crate's own
convention.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core scanner)
**Unit-stability check (release precondition):** every anchor cited in §2 has
no open obligation in the findings ledger and no `unknown` marker.
**Priority:** HIGH — this finding is the one red on the standing floor
(`self-check` → conform); every campaign batch commit is currently made
against a red gate.

## 1. Goal {#goal}

`crates/progress-core/src/parse.rs` returns under the 600-line conform
budget by splitting along its responsibility seams, with byte-identical
scanner behavior.

## 2. Contract {#contract}

> NEW file-length crates/progress-core/src/parse.rs:1 — violates
> REQ discipline://rust-ai-native-lang/guide#surface-form: 809 lines
> exceeds the 600-line file budget; fix surface: split along the file's
> responsibility seams into module-grain cells
> — `cargo xtask conform check`, 2026-07-24

> The **core** (parser, model, rollup, renderers, cache) is its own crate …
> — `spec://vibevm/modules/vibe-progress/PROP-043#separability`

## 3. Current state {#current}

- `parse.rs` grew to 809 lines when the fact-grain amendment landed
  (list items at every nesting level, lead lines, table body cells,
  `##<ID>` fact anchors, anchored-when-marked) — commit `b67fa97`.
- Its responsibilities are separable today: block collection
  (`collect_blocks` + line classifiers), heading/unit segmentation
  (`collect_units`, `split_anchor`), fact segmentation (`segment_facts`,
  `take_fact_id`, `list_item_content`, table-row/cell recognition),
  marker scanning (`scan_markers`), and the anchor laws
  (`check_anchor_laws`).
- 31 unit tests in the crate cover the scanner; the floor is otherwise
  green.

## 4. Change {#change}

1. Split `parse.rs` into module-grain files under
   `crates/progress-core/src/parse/` (e.g. `blocks.rs`, `units.rs`,
   `facts.rs`, `markers.rs`, `anchors.rs`, with `mod.rs` keeping
   `parse_document` and the shared types) — follow the seams named in §3;
   the exact grouping is the executor's call as long as every file holds
   the 600-line budget.
2. No public-API change: `progress_core::parse::parse_document` (and
   whatever `lib.rs` re-exports today) keeps its signature; callers in
   `vibe-cli` compile untouched.
3. No behavior change: the split is motion, not rewrite.
4. Keep the existing `specmark::scope!` citation on every new module (same
   REQ as today's file header).

## 5. Stop-rule {#stop}

Stop and return the task if holding the budget seems to require changing
any parsing behavior, public signature, or test expectation — that is a
redesign, not a split, and needs Fable review first.

## 6. Acceptance {#acceptance}

- `cargo test -p progress-core` green (all 31 scanner tests untouched and
  passing).
- Differential oracle: `vibe progress scan` on this repository before and
  after the split produces byte-identical `run/state/corpus.json` and
  `campaign.json` counters.
- `cargo xtask conform check` reports 0 new findings (the file-length
  finding is gone, no replacement findings introduced).
- `bash tools/self-check.sh` green end to end (check the real exit code,
  not a piped tail).
