# DRIFT-015 — the progress adapter goes back under its file budget {#root}

<status stage="impl" state="plan" ref="DRIFT-015"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress adapter)
**Unit-stability check:** no spec anchor moves — this is a structural fix to
code the Phase E batch grew.

## 1. Goal {#goal}

`bash tools/self-check.sh` is green again: `commands/progress.rs` is back
under the 600-line budget, split the way `commands/progress/rescan.rs`
already started.

## 2. Contract {#contract}

> `file-length … violates REQ discipline://rust-ai-native-lang/guide#surface-form:
> 897 lines exceeds the 600-line file budget`
> — `cargo xtask conform check`, 2026-07-25

> Cells: one cell = one file-set, single registration point; cells import
> seams + core only, never sibling cells. Ambient coupling is forbidden.
> — the AI-Native Rust stack's boot snippet

Finding realised: **F-058**.

## 3. Current state {#current}

- `crates/vibe-cli/src/commands/progress.rs` is **897 lines**. It was ~430
  this morning; four Phase E tasks landed in it in one day — the `gate`
  subcommand (DRIFT-008), the evidence-provider selection (DRIFT-006), the
  fold-check wiring (DRIFT-007) and the incremental `ground()` (DRIFT-010).
- The split has already begun and its shape is settled: `mod rescan;` at the
  top of the file, with `commands/progress/rescan.rs` beside it (DRIFT-009)
  and `commands/progress/tests.rs` (DRIFT-010). A `foo.rs` + `foo/`
  module directory is the established pattern here.
- `conform check` reports exactly **1 finding**, this one. Everything else in
  the floor is green.
- Precedent: DRIFT-002 split `progress-core/src/parse.rs` for the same
  reason. This is that move, one crate over.

## 4. Required behavior {#behavior}

1. Move subcommand bodies out of `progress.rs` into siblings under
   `commands/progress/`, one file per coherent group, until the parent is
   comfortably under 600 lines — not at 599. A file that re-crosses the
   budget on the next feature has not been fixed.
2. What stays in `progress.rs`: the `run()` dispatcher, `Ground`,
   `ground()`, `resolve_campaign`, `campaign_id`, `refresh_state` — the
   shared spine every subcommand calls. What moves: the subcommand bodies
   and their tests, grouped by what they do, not by what fits.
3. **This is a move, not a rewrite.** No behaviour changes, no signature
   changes beyond the `pub(super)` / `use` adjustments a move forces. If you
   find a bug while moving, record it in §9 and leave it — a refactor that
   also fixes things is a diff nobody can verify.
4. Keep every `specmark::scope!` and `#[spec(...)]` attribute with the code
   it describes. A moved item that loses its tag becomes a specmap orphan,
   and the ratchet will say so.

Edge cases: `commands/progress/tests.rs` already exists and may need to
follow its subject. `mod rescan;` sits at line 25 with a doc comment — keep
that shape for the new modules.

Error paths: none — this is a move.

## 5. Boundaries {#boundaries}

- Do not change what any subcommand does. Not the output, not the flags, not
  the order of operations.
- Do not touch `progress-core`. The budget problem is in the adapter.
- Never edit spec text or golden tests.

## 6. Acceptance {#acceptance}

```bash
cargo xtask conform check                                  # 0 findings
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress scan       # 58 files, 4979 markers, 0 errors
cargo run -q -p vibe-cli --bin vibe -- progress check      # 0
cargo xtask specmap                                        # ratchet must not regress (37)
bash tools/self-check.sh                                   # green, no VIBE_SETTINGS override
```

- `conform check` reports **0 findings** — that is the whole point.
- The scan line is byte-identical to the one above. A refactor that changes
  a count changed behaviour.
- Report the line count of every file you created and of `progress.rs`
  after, in §9.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-cli/src/commands/progress/rescan.rs` — the split that already
happened, including its doc comment on the `mod` line. And
`crates/progress-core/src/parse/` — DRIFT-002's version of this same move.

## 8. Stop rule {#stop}

If a subcommand cannot be moved without changing a signature in
`progress-core`: STOP, record it in §9, return. The core is another owner's
surface and three tasks just landed in it.

Budget signal: past ~8 files, stop and return — a split that needs eight new
files is a design question, not a move.

## 9. Log {#log}

- queued 2026-07-25 (Fable). The floor's only red after F-055 closed.
