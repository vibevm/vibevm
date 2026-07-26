# DRIFT-025 — the progress adapter splits before it is forced to {#root}

<status stage="impl" state="plan" ref="DRIFT-025"/>

**Status:** ready — owner ruled "split now", 2026-07-26
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (the progress adapter)
**Unit-stability check:** no spec anchor moves. This is motion, not redesign.

## 1. Goal {#goal}

`crates/vibe-cli/src/commands/progress.rs` is back under its budget with real
headroom, and the next change to it fits.

## 2. Contract {#contract}

> The per-file line budget (GUIDE-AI-NATIVE-RUST §2).
> — `conform.toml`, `max_file_lines = 600`

Finding realised: **F-072**.

## 3. Current state {#current}

- `progress.rs` is at **599 of 600 lines**. DRIFT-024 hit 614 on its first
  version, watched `conform check` fail on `file-length`, and rewrote its edit
  to the true minimum to land at 599. It then declined to take the split,
  correctly — that is a structural decision.
- **The next change does not fit**, and the next change is already queued:
  F-067 adds a `verify --seal` verb to this same adapter.
- The file already demonstrates the shape the split should take: `mod baseline`
  and `mod rescan` are submodules under `commands/progress/`, each owning one
  verb.

## 4. Required behavior {#behavior}

**This is a motion, not a rewrite.** Move code; do not reshape it, do not
rename its items, do not "improve" it on the way past. A reviewer must be able
to read the diff as *this block moved there* and nothing else.

1. Extract the campaign-grounding cell into its own submodule under
   `commands/progress/`: `Ground`, `ground()`, `refresh_state()`, and the
   campaign-path helpers (`resolve_campaign`, `campaign_id`, `payload_dir`).
   They are one cell — every verb calls `ground()` first, and nothing else
   needs the path helpers.
2. Keep every item's **visibility and name** exactly as they are, re-exported
   from `progress.rs` if that is what keeps callers untouched. **No call site
   outside this file may change.**
3. The submodule carries the doc-comments that travel with the moved code, and
   the `specmark::scope!` the file already declares must still cover the moved
   items — check the specmap ratchet does not gain an orphan (§6).

Edge cases: if a helper turns out to be used by exactly one verb rather than by
`ground`, it belongs with that verb, not in the new cell — say so in §9 rather
than dragging it along.

Error paths: unchanged everywhere. This task changes no behaviour at all.

## 5. Boundaries {#boundaries}

- **No behaviour change.** If any test needed editing to keep passing, the move
  was not a move — STOP and explain (§8).
- Do not touch `spec/**`.
- Do not split any file other than `progress.rs`, and do not raise
  `max_file_lines`. Raising the budget to fit is the failure mode this task
  exists to avoid.
- Do not renumber or reorder the verbs.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
cargo xtask conform check
cargo xtask specmap
bash tools/self-check.sh
```

- `progress.rs` is **meaningfully** under 600 — report the number. Landing at
  598 would satisfy the letter and defeat the point; if the natural cell does
  not buy real headroom, say so rather than shaving comments.
- The new submodule is also under budget. Report both numbers.
- `conform check` 0 findings; **specmap ratchet unchanged at 37** — a moved
  `pub` item that loses its scope coverage shows up here, and that is the
  likeliest way this task breaks something invisibly.
- **No test file is modified.** That is the proof it was a motion. If one had
  to change, §8 applies.
- `git diff --stat` shows lines leaving one file and arriving in another in
  roughly equal number. Report it.
- Discipline: `cargo fmt --all`, clippy clean, no AI attribution.

## 7. Analogies {#analogies}

`commands/progress/baseline.rs` and `commands/progress/rescan.rs` are the
shape — a submodule owning one concern, called from the hub. DRIFT-002 split
`progress-core`'s `parse.rs` the same way ("six modules, max 261 lines;
motion, not rewrite") and is the precedent for how this is reviewed.

## 8. Stop rule {#stop}

If the split cannot be done without editing a test or changing a call site
outside `progress.rs`: **STOP and report what coupling forced it.** That
coupling is a finding about the adapter's shape and is worth more than a
completed task.

Budget signal: past ~3 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable), on the owner's "разделить сейчас". Filed as
  F-072 an hour earlier, when DRIFT-024's executor reported landing at 599/600
  and deliberately left the structural call to the reviewer.
