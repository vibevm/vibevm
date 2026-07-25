# DRIFT-007 — `progress check` verifies that a fold was lossless {#root}

<status stage="impl" state="plan" ref="DRIFT-007"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core check)
**Unit-stability check:** `CMD-CHECK` carries the owner's 2026-07-25 ruling
(«а мы можем всё-таки это сделать? сделай wire и реализуй»).

## 1. Goal {#goal}

`vibe progress check` gains the lossless-fold verification its own contract
promises, so a marker-density fold can be trusted to lose no information.

## 2. Contract {#contract}

> `check` — vocabulary hints, well-formedness, placement, shorthand
> disambiguation, foreign-grammar non-collision, `--exhaustive` unmarked
> detection, **lossless folds**, exit codes.
> — `spec://vibevm/modules/vibe-progress/PROP-043#tool`

> Marker density folds: agreeing sections collapse to unit markers
> (lossless, `check`-verified).
> — `spec://vibevm/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1#phase-f`

Anchor realised: `CMD-CHECK`.

## 3. Current state {#current}

From Phase C verification evidence — do not re-discover:

- `grep -ri "fold\|lossless" crates/progress-core crates/vibe-cli/src/commands/progress.rs`
  = **0 hits**. Every other check the contract lists was verified live and
  passes; this one has no implementation at all.
- The fold *operation* itself is honestly unimplemented too — PROP-043's
  `POST-CAMPAIGN-FOLD` is marked `spec/done` and is Phase F work.

That asymmetry decides the shape: **this task builds the verifier, not the
folder.** A fold performed by hand (or by a later Phase F tool) must be
checkable today.

## 4. Required behavior {#behavior}

1. Define the fold relation in `progress-core`: a section-level marker
   **folds** the markers of the units inside that section when every one of
   those units carries the same `(stage, state)` pair; the fold is
   **lossless** iff the section marker carries exactly that pair, and no
   folded unit carried an `action`, `actionstage`, `audience`, `ref` or
   `comment` that the section marker does not also carry.
2. Add `progress_core::rollup::fold_check(doc: &ParsedDoc) -> Vec<FoldIssue>`
   returning one issue per section whose marker claims a fold that loses
   information. `FoldIssue` names: the section anchor, the losing unit
   anchor, and which attribute was lost (`state` / `action` / `audience` / …).
3. Wire it into `check` as a new check class. It reports at the same
   severity as the existing placement checks and contributes to the same
   non-zero exit code.
4. A section whose units **disagree** is not a fold at all and is silent —
   this check never demands that anyone fold; it only catches a fold that
   lies. A section with no section-level marker is likewise silent.

Edge cases: a section containing a nested subsection folds transitively —
the nested section's own marker is the unit for the outer test. A section
whose only units are `_elements` (document markers) is silent. An empty
section is silent.

Error paths: `fold_check` never errors; it returns issues. `check`'s exit
code follows the existing convention (non-zero when any issue is found).

## 5. Boundaries {#boundaries}

- Do not implement the fold *operation* — no rewriting of markup. Phase F
  owns that; this task must not pre-empt its design.
- Do not touch the parser's segmentation rules or the cache schema.
- Never edit spec text or golden tests.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress check      # must stay 0 on this repo
bash tools/self-check.sh
```

- New test: `fold_lossless_is_silent` — a section marked `impl/done` over
  three units all `impl/done` yields no issue.
- New test: `fold_losing_state_is_caught` — the same section where one unit
  is `spec/done` yields one issue naming that unit and `state`.
- New test: `fold_losing_action_is_caught` — a unit carrying
  `action="drift"` under a section marker that does not yields an issue
  naming `action`.
- New test: `disagreeing_section_is_not_a_fold` — units with mixed stages
  under a section marker yield no issue.
- **`vibe progress check` on this repository must still report 0** — if the
  new check fires on the live corpus, that is a finding to surface (§8), not
  a reason to weaken the rule.
- Discipline: `#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#tool")]`,
  `cargo fmt --all`, clippy clean, atomic commits, no AI attribution.

## 7. Analogies {#analogies}

`crates/progress-core/src/rollup.rs` already walks the document tree and
aggregates by the fixed stage order; the fold test is a second walk over the
same structure. Imitate its shape and its test style.

## 8. Stop rule {#stop}

If the fold relation as defined in §4 contradicts anything in
`spec://vibevm/modules/vibe-progress/PROP-043#rollup`: STOP, mark
`<!-- REVIEW: … -->`, record the question here, set status `returned`. The
definition above is the reviewer's reading, not a licence to invent.

Budget signal: past ~5 files or ~350 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling.
- implemented 2026-07-25 (Opus). `fold_check` + `FoldIssue` / `FoldLoss` in
  `crates/progress-core/src/rollup.rs`; wired into the `check` path of
  `crates/vibe-cli/src/commands/progress.rs` as an `Error`-severity class
  on the existing exit code. Six tests, the four §6 ones plus transitivity
  and the section-only-attribute case. `vibe progress check` on this
  repository: **clean, exit 0** — the new class fires nowhere on the live
  corpus (verified separately that it does fire on a synthetic lossy fold).
- §8 stop rule — **not triggered**. §4's relation does not contradict
  `spec://vibevm/modules/vibe-progress/PROP-043#rollup`: it never fires on
  the mixed/divergent case that `##EXPLICIT-BEATS` calls "information, not
  noise", and its agreement precondition is `##POST-CAMPAIGN-FOLD`'s own
  wording ("a section whose units agree collapses to one unit marker …
  mixed sections stay fact-marked"). Residual tension worth the reviewer's
  eye: `##EXPLICIT-BEATS` blesses an explicit marker that diverges from its
  children, and this check now errors on one narrow such case — a section
  marker whose units **all agree** on a pair the section marker does not
  carry. That is the only configuration in which §3.9's lossless-fold duty
  can have teeth at all, so it was kept.
- **Question for the reviewer (§6 vs §4).** §6's `fold_losing_state_is_caught`
  sketch — "the same section [three `impl/done` units] where one unit is
  `spec/done`" — is a *disagreeing* section, which §4.1's precondition and
  §4.4's silence rule make unreachable; it is also indistinguishable from
  §6's `disagreeing_section_is_not_a_fold` ("units with mixed stages …
  yield no issue"). §4 was taken as governing (it is the Required-behavior
  section, and it is what PROP-043 §3.9 says), so the test was written on an
  agreeing set: one `spec/done` unit under an `impl/done` section marker →
  exactly one issue naming that unit and `state`, as §6 demands of the
  observable. A `// REVIEW:` marker sits at the gate in `rollup.rs`
  (`fold_check`). Dropping the precondition to satisfy §6's fixture
  literally was measured against the corpus first: it fires on three
  legitimate mixed sections — PROP-031 §6, PROP-032 §7, PROP-033 §7, nine
  units in total whose `spec/done` standing differs from their section's
  `spec/work` (open questions beside ratification notes) — which §3.9
  declares correct markup. Reviewer's call.
- Live-corpus survey behind that number (the whole basis of the 0): the
  observed tree carries **11** section-level markers — `PROP-006#mfbt`,
  the open-questions sections of PROP-013 / PROP-016 / PROP-028 / PROP-031 /
  PROP-032 / PROP-033, and four in `spec/design/README.md`. Three are exact
  folds (units agree and match), four are agreeing sections whose marker
  adds a `comment` the units lack (not a loss — the test is one-directional),
  and four are mixed sections that §4.4 silences.
