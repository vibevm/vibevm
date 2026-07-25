# DRIFT-009 — baseline invalidation gets its other two rules {#root}

<status stage="impl" state="plan" ref="DRIFT-009"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core baseline / rescan)
**Unit-stability check:** `BASELINE-INVALIDATION` carries the owner's
2026-07-25 ruling («сделай wire и реализуй»).

## 1. Goal {#goal}

The monthly re-run stops trusting a verdict whose *code* moved, and samples
the ones nothing moved under — so `rescan` costs O(delta) without quietly
becoming O(nothing).

## 2. Contract {#contract}

> Invalidation: unit hash changed ⇒ suspect; **named crate has commits after
> the verdict date ⇒ suspect**; **a random control sample of
> carried-forward units is re-verified** regardless.
> — `spec://vibevm/modules/vibe-progress/PROP-043#baseline`

> `rescan --baseline <prev>/baseline.json` → new / suspect /
> carried-forward lists.
> — `spec://vibevm/modules/vibe-progress/PROP-043#tool`

Anchor realised: `BASELINE-INVALIDATION`.

## 3. Current state {#current}

From Phase C verification evidence — do not re-discover:

- `crates/progress-core/src/baseline.rs` — `BaselineUnit` carries
  `crates: Vec<String>` at line 25, and it is written as the **constant
  empty vec** at line 155. Nothing ever populates it.
- The hash-changed ⇒ suspect rule and the marker-diverged-outside-campaign
  flag are implemented; `RescanClass` is `New | Changed | CarriedForward`.
- So two of the four rules ship. The named-crates rule cannot fire because
  the field is always empty, and there is no control sample at all.

## 4. Required behavior {#behavior}

1. **Populate `crates`.** When a baseline is written, a unit's `crates` is
   the set of crate names its evidence refs point into — derive it from the
   evidence refs already stored on the unit (`crates/<name>/…` → `<name>`),
   de-duplicated and sorted. A unit with no refs keeps an empty vec, and the
   named-crate rule simply does not apply to it.
2. **The named-crate rule.** `rescan` marks a carried-forward unit `suspect`
   when any crate in its `crates` has a commit newer than the unit's
   `verified_at`. Ask git once per crate, not once per unit:
   `git log -1 --format=%cI -- crates/<name>` and compare timestamps.
3. **The control sample.** After classification, promote a deterministic
   pseudo-random sample of the still-`CarriedForward` units to a new class
   `ControlSample`. Default rate 5 %, minimum 1 unit when any exist,
   overridable with `--control-rate <0.0..=1.0>`; `--control-rate 0`
   disables it. Determinism matters: seed the choice from the baseline's own
   content (e.g. hash of `campaign_id` + unit address), never from a clock or
   an RNG, so a rescan is reproducible and reviewable.
4. `rescan`'s output and its state projection list the four classes
   separately. A `ControlSample` unit is re-verified like a suspect one; the
   distinction exists so the report can say *why* it is being re-checked.

Edge cases: git unavailable, or the repo not a git checkout ⇒ the named-crate
rule is skipped with one warning line, never a hard failure — a consuming
project may vendor its baseline without history. A crate named in `crates`
that no longer exists on disk ⇒ suspect (its code moved in the strongest
possible sense). A baseline with zero carried-forward units ⇒ no sample, no
warning.

Error paths: an unreadable baseline is an error naming the file. A malformed
`--control-rate` is a clap error.

## 5. Boundaries {#boundaries}

- Shelling out to git lives in the **adapter**, never in progress-core: the
  core takes the crate→last-commit map as an argument. progress-core has no
  business knowing this project uses git.
- Do not change `BaselineUnit`'s existing fields or the file's schema
  version unless the added data forces it — and if it does, say so in §9
  before writing.
- Never edit spec text or golden tests.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
bash tools/self-check.sh
```

- New test: `crates_derived_from_refs` — a unit whose refs are
  `crates/vibe-core/src/x.rs:1` and `crates/vibe-cli/src/y.rs:2` gets
  `["vibe-cli", "vibe-core"]`.
- New test: `named_crate_commit_makes_suspect` — a unit verified at T with a
  crate whose last commit is T+1 classifies suspect; T−1 stays carried-forward.
- New test: `control_sample_is_deterministic` — two runs over the same
  baseline pick the same units; `--control-rate 0` picks none; a baseline
  with one carried-forward unit and rate 0.05 still picks that one.
- New test: `missing_git_skips_rule_without_failing` — an empty crate→commit
  map leaves everything carried-forward and does not error.
- CLI scenario: `vibe progress rescan --baseline <fixture>` prints four
  labelled counts.
- Discipline: `#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#baseline")]`,
  `cargo fmt --all`, clippy clean, atomic commits, no AI attribution.

## 7. Analogies {#analogies}

`crates/progress-core/src/baseline.rs`'s existing `RescanClass` logic is the
shape to extend. For the adapter-side git call, imitate however the
workspace already shells out to git (`crates/vibe-registry/src/git_backend/shell.rs`
is the sanctioned idiom — reuse it rather than adding a new process helper).

## 8. Stop rule {#stop}

If the 5 % default or the "minimum 1" rule conflicts with anything in
PROP-043 §7.3: STOP, mark `<!-- REVIEW: … -->`, record it here, set status
`returned`. The contract says "a random control sample" and fixes no rate;
the numbers in §4 are the reviewer's, and a conflict is the owner's call.

Budget signal: past ~6 files or ~450 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling.
