# DRIFT-014 — three deviate reasons stop denying the shipped resolver {#root}

<status stage="impl" state="plan" ref="DRIFT-014"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** resolver (vibe-resolver `#[spec(deviates)]` metadata)
**Unit-stability check:** PROP-003's solver tail and PROP-017's status line
were corrected in Phase D d2a/d2c, so both anchors these reasons cite now
state the shipped truth. The code metadata is the last side still stale.

## 1. Goal {#goal}

The three `#[spec(deviates, reason)]` sites in `vibe-resolver` describe why
they deviate **today**, instead of citing a tree state that has not existed
since 2026-06-14.

## 2. Contract {#contract}

> A rule with no checker is a WISH; **a deviation with no reason is a
> defect** (`#[spec(deviates, reason)]`).
> — `spec://org.vibevm.ai-native/core-ai-native/…#root` (the discipline's
> boot law)

> **Status.** … **the port is COMPLETE** — `ResolvoDepSolver` is the shipped
> production default (`crates/vibe-cli/src/registry.rs:117`, `--solver`
> defaults to `resolvo`).
> — `spec://vibevm/modules/vibe-resolver/PROP-017#root`

A *stale* reason is worse than a missing one: it is a defect wearing the
discipline's own badge, and every reader who trusts the badge is misled.

Finding realised: **F-047**.

## 3. Current state {#current}

From Phase C verification evidence (c4b/c4c) — do not re-discover:

- `crates/vibe-resolver/src/naive.rs:32` — reason claims *"no ResolvoSolver
  exists in tree and NaiveDepSolver is the only DepSolver impl — the known
  SAT/resolvo upgrade debt (DBT-0011)"*. Both clauses are false: resolvo
  ships and `SatDepSolver` is in tree beside it.
- `crates/vibe-resolver/src/sat.rs:174` — reason ends *"adopting resolvo
  stays an owner decision the DepSolver seam keeps open"*. That decision was
  taken on 2026-06-14 and executed.
- `crates/vibe-resolver/src/lib.rs:290-296` — the `DepSolver` trait's reason
  ends *"and SatDepSolver is not in tree (see DBT-0011)"*. It is.
- The ground truth: `crates/vibe-cli/src/registry.rs:117`
  `unwrap_or("resolvo")` and `:191` constructing it. **Do not trust these
  three deviates over the tree** — that inversion is the finding.

## 4. Required behavior {#behavior}

1. For each of the three sites, decide first **whether it still deviates at
   all**, then rewrite the reason. The two questions are separate and the
   first is the interesting one:
   - `naive.rs` — does keeping a non-primary solver still deviate from
     `PROP-002#solver`? PROP-003 `SOLVER-TWO-IMPLS` (as corrected) keeps
     naive deliberately as the fast path and the regression oracle. If the
     contract now *sanctions* it, the honest metadata may be
     `implements`-with-a-note rather than `deviates`.
   - `sat.rs` — same question; PROP-003 keeps sat as the backtracking impl.
   - `lib.rs` — the deviation is about the absent `pin_preferences` method,
     which is a real and current gap. Only the trailing "SatDepSolver is not
     in tree" clause is false.
2. Every rewritten reason names what is true **now** and cites the anchor
   that says so. No reason may name DBT-0011 as an open debt unless that
   debt is genuinely open — check the ledger before repeating it.
3. Whatever you conclude for each site, state the reasoning in §9. A
   deviate removed is as reportable as a deviate reworded.

Edge cases: if `conform` or the specmap ratchet counts these deviates,
changing a `deviates` to an `implements` moves a number — say which and by
how much in §9, and confirm the ratchet does not regress.

Error paths: none — this is metadata and doc text.

## 5. Boundaries {#boundaries}

- **Do not change resolver behaviour.** Not the default, not the selection
  seam, not an algorithm. This task edits `#[spec(...)]` attributes and the
  prose in them.
- Do not touch `crates/vibe-cli/src/registry.rs` — the R-001 sanctioned
  constructor site is a conform construction site with its own rules.
- Never edit spec text. If a reason can only be made true by changing a
  contract, that is §8.

## 6. Acceptance {#acceptance}

```bash
cargo test -p vibe-resolver -p vibe-cli
cargo xtask conform check
cargo xtask specmap                 # ratchet must not regress
bash tools/self-check.sh
```

- Each of the three reasons quoted verbatim in §9, before and after.
- `conform check` reports the same or fewer findings than before; the count
  before and after is recorded in §9.
- The specmap ratchet count is unchanged or lower (37 gated orphans
  host-side is today's number).
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

Look at a `#[spec(deviates, reason)]` elsewhere in the workspace that was
written *after* the resolvo port — it is the register to imitate: what is
true, why it is acceptable, and under what condition it stops being
acceptable.

## 8. Stop rule {#stop}

If making a reason true would require changing what a contract says: STOP,
mark `<!-- REVIEW: … -->` at the site, record the question in §9, set status
`returned`. Spec text is not yours, and this task exists precisely because
code metadata drifted from it once already.

Budget signal: past ~4 files or ~150 lines, stop and return — this is a
metadata fix, not a resolver change.

## 9. Log {#log}

- queued 2026-07-25 (Fable). One of these verdicts was itself corrected
  mid-campaign (c4b confirm → c4c drift), which is why the evidence above is
  written as "do not trust the deviates over the tree".
- impl 2026-07-25 (Opus). §4.1's first question answered per site before any
  wording was touched. Net: **two deviates removed, one reworded.** The tree
  was taken as ground truth throughout — `vibe-cli/src/registry.rs:117`
  `unwrap_or("resolvo")` and `:191` constructing `ResolvoDepSolver` — and no
  spec file was opened for editing; §8 never fired, because none of the three
  reasons needed a contract to change in order to become true.

  **Site 1 — `crates/vibe-resolver/src/naive.rs:32` (`NaiveDepSolver`).
  Verdict: no longer deviates. `deviates` → `implements`.** Before, verbatim:

  ```
  #[spec(
      deviates = "spec://vibevm/modules/vibe-registry/PROP-002#solver",
      reason = "PROP-002 §2.8 decides resolvo is the PRIMARY depsolver; no ResolvoSolver \
                exists in tree and NaiveDepSolver is the only DepSolver impl — the known \
                SAT/resolvo upgrade debt (DBT-0011), recorded honestly until the second \
                impl lands"
  )]
  ```

  After, verbatim:

  ```
  #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#solver")]
  ```

  Reasoning. The old reason is false four ways over:
  `ResolvoDepSolver` exists (`resolvo_engine/mod.rs`), naive is not the
  only impl (`Sat` and `ResolvoDepSolver` sit beside it), the "second impl"
  landed twice, and DBT-0011 is **fixed** in the ledger
  (`discipline/registry/DEBT.md:48` / `discipline/DEBT.md:33`) — so §4.2's
  "no reason may name DBT-0011 as an open debt" bars repeating it. That
  disposes of the wording. The predicate underneath it also fails: PROP-002
  §2.8's normative content is `RESOLVO-PRIMARY` (resolvo is the primary
  depsolver) plus `LIBSOLV-FALLBACK-SLOT` (the `DepSolver` trait is the
  `GitBackend`-style slot alternatives drop into). The tree satisfies both.
  Naive breaks neither clause; it is the slot being used. And the contract
  family now *sanctions* it by name — PROP-003 §2.1 `SOLVER-TWO-IMPLS`
  ("`NaiveDepSolver` stays in tree as the … fast path"), `two-impls-why`
  ("the cheapest oracle we'll ever have"), PROP-017 §6 `CELLS-STAY` ("naive
  and sat stay in tree … both still selectable via `--solver`"). A deviation
  the contract sanctions is not a deviation.

  Why `implements` and not simply deleting the edge: the same crate already
  treats a cell behind this seam as implementing §2.8 —
  `local_registry_provider.rs:17` and `multi_registry_provider.rs:19` both
  carry `implements = …PROP-002#solver`, as do `lib.rs`'s `DepProvider` and
  `DepSolver` traits. §2.8 is satisfied by the conjunction of the cells
  behind it, not by one item; naive is one of them. Deleting instead would
  have silently dropped `naive.rs`'s only link to §2.8. The "note" half of
  §4.1's "`implements`-with-a-note" had to go into the doc comment, not the
  attribute: specmark's grammar **rejects `reason` on every verb except
  `deviates`** (`core-ai-native-specmark-grammar`), so there is no in-attribute
  note to write. The struct's rustdoc now carries it — non-primary by design,
  the two jobs the spec keeps it for, and the first-pick-wins limit as the
  price of selecting it.

  **Site 2 — `crates/vibe-resolver/src/sat.rs:174` (`impl DepSolver for
  Sat<P>`). Verdict: no longer deviates. `deviates` → `implements`.** Before,
  verbatim:

  ```
  #[spec(
      deviates = "spec://vibevm/modules/vibe-registry/PROP-002#solver",
      reason = "PROP-002 §2.8 names resolvo as the primary industrial solver; this cell \
                implements chronological backtracking natively over the unmodified \
                DepProvider trait instead, reusing the naive cell as its branch checker. \
                The backtracking half of DBT-0011 retires here; adopting resolvo stays \
                an owner decision the DepSolver seam keeps open"
  )]
  ```

  After, verbatim:

  ```
  #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#solver")]
  ```

  Reasoning. The clause that made this a deviation was the last one —
  "adopting resolvo stays an owner decision the DepSolver seam keeps open".
  That decision was taken 2026-06-14 and executed; with resolvo adopted, this
  cell no longer stands *instead of* the primary solver, it stands beside it,
  exactly where PROP-003 §2.1 `SOLVER-TWO-IMPLS` (which names `SatDepSolver`)
  and PROP-017 §6 `CELLS-STAY` ("sat as a recorded pure-Rust backtracker")
  put it. Same verdict as site 1, same reasoning. The edge was left on the
  impl block rather than moved to the `Sat` struct: `LIBSOLV-FALLBACK-SLOT`'s
  own phrasing is "swap cost: one impl block, one factory line", so the impl
  block is the precise carrier, and PROP-014 §2.3 lists impl blocks as taggable
  units. The descriptive half of the old reason (backtracking natively over
  the unmodified `DepProvider`, naive as branch checker) was true and is kept,
  moved into a rustdoc comment on the impl.

  **Site 3 — `crates/vibe-resolver/src/lib.rs:290` (`pub trait DepSolver`).
  Verdict: still deviates, genuinely. Reason rewritten, verb kept.** Before,
  verbatim:

  ```
  reason = "PROP-003 §2.1 adds `pin_preferences(&mut self, pins)` to this trait for \
            minimum-churn re-resolution; the method is absent — PROP-011 Phase 3 \
            holds pins via constraint-tightening at the install layer instead, and \
            SatDepSolver is not in tree (see DBT-0011)"
  ```

  After, verbatim:

  ```
  reason = "PROP-003 §2.1 adds `pin_preferences(&mut self, pins)` to this trait for \
            minimum-churn re-resolution; the method is still absent. PROP-011 Phase 3 \
            holds pins at the install layer instead — `freshness::hold_pins` rewrites \
            every unchanged root to `=<locked>` before the solve — so the property \
            ships without the trait method, at the cost of a registry walk the solver \
            could have skipped. It stops being acceptable when a re-resolve needs \
            preference *inside* the solve; resolvo's `sort_candidates` (PROP-017 §3) \
            is where that would land"
  ```

  Reasoning. Only the trailing clause was false. The gap itself is real and
  current: the shipped trait declares `solve` and nothing else, while §2.1
  `TRAIT-PIN-PREFERENCES` specifies `pin_preferences`. The spec tree already
  agrees — that unit is marked `@spec/done`, not `@impl/done`, so making the
  reason true needed no spec edit and §8 did not fire. The replacement was
  verified rather than copied: `vibe-workspace/src/freshness.rs:219`
  `hold_pins` rewrites each unchanged root to `=<locked>`,
  `vibe-install/src/plan.rs:219` calls it and `:228` falls back to a free
  re-resolve when the held set over-constrains. "PROP-011 Phase 3" is kept
  because PROP-011's own `HISTORY-SHIPPED` line uses that name for this work.
  Per §7 the reason now also names the condition under which it stops being
  acceptable, which the old one never did.

  **Numbers (§4 edge case, §6 acceptance).** `cargo xtask conform check`:
  **0 findings before, 0 after** (0 frozen in baseline, 0 new, both runs).
  Conform's rules (R-001 flag-sites, unsafe, ambient-env, Class-F/G doctests,
  REQ-citing errors, file budget, unwrap ban) do not score `implements`
  vs `deviates`; the one place a `deviates` *does* suppress a conform finding
  is testimony pointed at `ENGINE-CONFORM-v0.1#rules` (as `activation.rs:261`
  does for an ambient-env read), and none of these three pointed there.
  `cargo xtask specmap` ratchet: **37 gated orphans before, 37 after** — no
  regression. It cannot regress from this change by construction:
  `ratchet::orphans` gates only `pub` top-level `fn`/`struct`/`enum`/`trait`/
  `type` carrying *no edge of any verb*, and impl blocks are explicitly out of
  scope; `NaiveDepSolver` and `Sat` both keep edges either way. What did move
  is the edge census in the committed `specmap.json`: workspace-wide
  `deviates` **14 → 12**, `implements` **662 → 664**. `specmap.json` was
  regenerated (it carries the reason strings verbatim and is gated by
  `specmap --check`); note the regeneration also absorbed line-number shifts
  from other agents' concurrent edits, since the index is derived from the
  whole tree.

  One caveat on the conform number, recorded so the reviewer is not surprised:
  a *later* run in the same session reported **1 new finding**, `file-length
  crates/vibe-cli/src/commands/progress.rs:1 — 892 lines exceeds the 600-line
  file budget`. That file belongs to another agent's concurrent work and is
  untouched here; it was deliberately left alone. Excluding it, this task's
  contribution to conform remains 0. Worth flagging separately:
  `crates/vibe-resolver/src/lib.rs` is now 571 lines, 29 from the same budget
  — four of them added by the rewritten reason.

  **Two decisions the task did not spell out.**

  1. *Scope of the prose fix.* §5 confines the edit to `#[spec(...)]`
     attributes and their prose. `sat.rs`'s module header, though, restated
     the deviation and pointed at it — "see the `deviates` edge on the impl …
     the 'industrial solver' half stays an owner option" — which would have
     become a dangling pointer to a removed attribute, and its first line
     mis-cited the anchor as "PROP-003 §2.8" when `#solver-upgrade` is §2.1.
     Both were corrected as part of making this edit coherent, not as
     independent work.
  2. *Adjacent drift found, deliberately left standing.* Three more instances
     of exactly this finding's shape are in the tree and are **not** in
     `findings.json`, so they were recorded here rather than silently
     absorbed: `crates/vibe-resolver/src/lib.rs:3` ("Two traits and one
     implementation in this crate") and `:27–33` (naming "adding a
     `ResolvoSolver` (PROP-002 §2.8 primary)" as a *future* trigger) — the
     crate module docs still describe the pre-port world; and
     `crates/vibe-workspace/src/freshness.rs:218` ("deferred with the SAT
     solver", which shipped). Each denies the shipped resolver the way F-047
     does. They want their own finding and task; folding them in here would
     have understated F-047's blast radius and blown §5's budget signal.

  **Gates run.** `cargo fmt --all` (clean on re-check), `cargo clippy -p
  vibe-resolver --all-targets -- -D warnings` clean, `cargo xtask specmap`
  37 with the committed index in sync (`specmap --check` reaches the ratchet,
  so the regenerated `specmap.json` matches the tree; the ratchet itself is
  the pre-existing host-side 37, unchanged by this task and not a step of
  `self-check.sh`). `cargo test -p vibe-resolver -p vibe-cli` all green with
  `VIBE_SETTINGS` pointed at an empty temp directory — 395 + 87 unit, every
  integration target including `cli_pkg_cycle` 20/20 (F-055 no longer red on
  this run) and `cli_registry_mgmt` 67/67, the three differential oracles, and
  11 doctests. That suite had to be retried: on the first attempt `vibe-cli`
  did not compile against another agent's half-landed `progress` work
  (`missing field cache in … commands::progress::Ground`, `missing field
  no_cache in … cli::progress::ProgressCommonArgs`, `cannot find value user`
  in `cli_registry_mgmt`); per the concurrency rule it was waited out, not
  "fixed", and it went green once that agent's edit completed.

  `bash tools/self-check.sh --keep-going` (same isolated `VIBE_SETTINGS`)
  exits 1 on exactly two steps, **neither of them this task's**:

  - step 4, `cargo run -p vibe-cli -- check --path . --quiet`, exit 101 —
    `failed to remove file target\debug\vibe.exe`, a Windows file lock from
    another agent running the binary at the same moment. Re-run in isolation
    immediately afterwards: **`vibe check: 0 errors, 1 warning, 0 info`**,
    green on the first attempt. Environmental, not a floor regression.
  - step 5, `cargo xtask conform check` — the single `file-length` finding on
    `crates/vibe-cli/src/commands/progress.rs` described above.

  Everything else is green, including the two steps that would actually catch
  a mistake here: **step 2 `cargo test --workspace` and step 3 `cargo clippy
  --workspace --all-targets -- -D warnings` both pass**, as do fmt,
  `sync-engines --check`, all four package workspaces, and all four package
  self-traces (0 orphans each). F-055 did not reproduce on any run this
  session.
