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
