# F02 implementation evidence — workspace consumer constraints

Worktree: `/fast/git/wt/workspace-constraints`; branch `fix/workspace-constraints`; base/unchanged HEAD `1448ed5cc33a4fadbce9d8331bf5f2d96052aec3`. Implemented 2026-11-04 Moscow time. No commits or staging. Root owns review, integration and worktree cleanup. Only the eleven listed vibe-install files changed; no specs, manifests, CLI/source-factory, global configuration, or other worker files changed.

## Result

The existing install planner now retains every finalized registry/workspace-version consumer requirement. An explicit request replaces only its selected node's own registry requirement; sibling constraints remain conjunctive. Different explicit refs for one coordinate refuse before source calls, identical ones coalesce. Missing selected node refuses before legacy migration. Planner-owned consumer labels and requirements accompany solver failures; the typed solver cause remains in the error chain for existing CLI classification.

`plan/roots.rs` owns the bounded collection/validation helper. Authored requirements stay separate from held optimization pins. Explicit coordinates remain unpinned across every sibling occurrence. Unrelated satisfied pins are retained; initial and conditional held-solve conflicts retry complete authored roots. Conditional retries carry exact pins only for earlier conditional roots absent from the base requirements; visibility retries retain the complete base list. Resolved package counts are separate from declaration counts in progress reporting. All root kind assertions are checked, including later sibling assertions that the existing resolvo first-root check otherwise misses; fresh-lock checks also retain that assertion.

Selected version.var indirection is removed only for the replaced coordinate. Shared workspace versions and sibling vars stay unchanged. Existing git/path declarations are not rewritten by plain version recording. Post-edit workspace declarations produce a complete unique-coordinate lock compatibility snapshot. Equal effective PackageRefs retain their spelling; differing rows become the selected exact version with the actual kind, using the existing structural comparator helper (no synthesized range intersection or prerelease broadening). This snapshot is recomputed for both bare and explicit apply, replacing stale root identities rather than merging only the requested rows.

Legacy lock seeding now requires a genuinely undeclared workspace and raw selected manifest, including the git/path/var guards. Identical old duplicate snapshot rows can coalesce; differing duplicates refuse before mutation. One incumbent prepared-planning test was updated because it explicitly required seeding despite a supplied finalized declaration; that old expectation conflicts with the new frozen contract. A separate real-resolver legacy-seeding test retains the valid undeclared-workspace behavior.

## Verification and failures encountered

All Cargo commands below ran from this worktree with:

```text
env CARGO_TARGET_DIR=/fast/git/wt/workspace-constraints/target RUSTC_WRAPPER= VIBE_SETTINGS=/tmp/nst-constraints-test-settings
```

The target is the assigned independent reflink target. No build used the root or other worker's target. Fixture packages and projects are owned TempDirs; the test InstallSource serves those explicit candidates and does not write/read the supplied machine store. VIBE_SETTINGS is set on the launched child process, never mutated by tests; no process-global env/cwd changes. All Cargo work used `--offline --locked`.

1. **RED**, before production edits: `cargo test --offline --locked -p vibe-install --test root_constraints`, exit **101**, 6 selected: **1 passed / 5 failed**. The real ResolvoDepSolver-backed fixtures showed incompatible siblings accepted, overlapping siblings selecting 2.5.0 instead of 1.5.0, explicit selected-member request erasing sibling constraint, conditional solve losing intersection, and conflicting explicit refs reaching the source instead of early refusal. First build took 52.64s due worktree-specific crate paths.
2. One generated apply_patch used unsupported numbered hunk syntax and was rejected with no planner mutation; I corrected the patch format.
3. First implementation compile: exit **101**, E0425 from my `crate::Result` reference (the alias is `crate::error::Result`) and two now-unused imports. Corrected all three. Next run: **6/6 passed**.
4. Expanded matrix first run: exit **101**, **12 passed / 3 failed**. I omitted required project.version in three fixture manifests. Corrected those fixture inputs. I also strengthened the kind test to require typed KindMismatch rather than any error, preventing a malformed manifest from falsely passing it. Corrected matrix: **15/15 passed**.
5. Incumbent `cargo test --offline --locked -p vibe-install --test prepared_planning`: exit **101**, **6 passed / 1 failed**. The sole failure was the now-incompatible legacy-seeding expectation described above; changed its name/assertions to `prepared_declarations_prevent_legacy_snapshot_seeding`, preserving supplied-tree-vs-disk coverage.
6. Added conditional pin fallback, actual visibility retry, valid legacy seeding, and source-declaration migration guards. First compile hit E0282 in a new fixture's ambiguous `.parse()` binding; added explicit `PackageRef`. The next combined run passed **19 root_constraints + 7 prepared_planning**.
7. Selected incumbent `cargo test --offline --locked -p vibe-install --test visibility_resolution invisible_constraint_stops_pinning_versions -- --exact`: exit **0**, **1 passed**, 7 filtered.
8. Selected incumbent `cargo test --offline --locked -p vibe-install --test slot_integrity_verify fresh_lock_skips_resolution_without_touching_the_source -- --exact`: exit **0**, **1 passed**, 9 filtered.
9. `cargo clippy --offline --locked -p vibe-install --test root_constraints --test prepared_planning --no-deps -- -D warnings`: exit **0**. First scoped check 38.94s; final check after fixture strengthening/formatting 0.51s. Existing Cargo manifest warning remains: `vibe-cli/Cargo.toml: unused manifest key: build`; no selected Rust lint warning.
10. Pre-review final test run after strengthening installed-conflict bytes and typed error-chain assertions: `cargo test --offline --locked -p vibe-install --test root_constraints --test prepared_planning`, exit **0**, **19 + 7 passed**, none failed/ignored. Output retained at `/tmp/nst-constraints-final-tests.log`. Final lint output: `/tmp/nst-constraints-final-clippy.log`.
11. Explicit-file `rustfmt --check --edition 2024 --config skip_children=true` over all ten edited Rust files: exit **0**. `git diff --check` independently: exit **0**.

After the root-review corrections below there are **31 distinct passing tests** across the 22 new cases, 7 prepared-planning cases, and 2 selected incumbent regressions; repeated runs are not counted as additional cases. No full suites, native Windows run, network, release, CLI binary build or global-config mutation was performed.

## Test coverage

The new integration binary uses real `ResolvoDepSolver` and `FilteringDepProvider` behind InstallSource, with versioned owned candidate packages and real content hashes. It checks:

- incompatible siblings with no fetch or project mutation; compatible intersection under both consumer orderings;
- explicit member replacement versus binding root/siblings, including refusal with already installed lock/slots preserved byte-for-byte;
- allowed movement of explicitly named old pin and unchanged unrelated pin;
- initial held-pin and later conditional held-pin fallback preserving all authored constraints;
- conditional new-root re-solve and real visibility-mask retry retaining sibling intersection;
- valid fresh lock avoiding source calls, changed second consumer leaving the fast path, and an intentionally empty root remaining empty;
- nested version.var scope, selected-only replacement, preserved shared/sibling authoring, actual plan/apply/reload/second bare Fresh;
- unique compatibility snapshots, identical-range spelling, differing-range exact version, and prerelease roundtrip;
- Latest caret versus explicit exact finalization;
- missing selected node, ambiguous historical snapshot refusal, and unambiguous legacy seeding;
- all kind assertions on fresh and solved paths, with typed cause retained for CLI exit-code consumers;
- git/path recording neutrality and migration guards without claiming source registration exists.

## Explicit remaining limits

This slice does not introduce a new resolver or choices grammar, nor fix cross-member git/path/override source registration. The existing source-factory ownership and source priority remain unchanged. The historical path-only empty-world behavior is still outside this perimeter; its migration-guard test checks only that old lock roots are not written into the host. Mixed unsupported path cases can refuse when complete snapshot construction cannot find their selected package; no registry substitution or fabricated source resolution is introduced.

Broader conditional same-coordinate expression discovery remains unchanged: `fetch.rs` still suppresses already-fetched extra coordinates and does not model every distinct conditional constraint. This change ensures every **base workspace requirement** survives the existing conditional/visibility retries. Native execution/source authentication and arbitrary concurrent writer semantics are not newly claimed. Targeted tests do not prove those broader features.

## Root-review corrections before acceptance

Root found two real defects in the first candidate: the compatibility wrapper forwarded an uncanonicalized relative/alias path into the newly strict prepared API; and snapshot construction trusted the solved version without checking the sibling declarations that apply had just reread. Both are now corrected within the original perimeter.

`plan_with_spec_format` uses the existing `Workspace::discover_selected` epoch and passes its canonical selected root to the prepared owner. It still reads the selected raw manifest at the compatibility boundary. The prepared function continues to use its supplied tree and performs no rediscovery/canonicalization.

`workspace_root_snapshot` checks every already-read post-edit declaration's version and kind against the selected `ResolvedDep` before grouping/projection. A known incompatible sibling now produces the typed ConsumerConstraints failure naming that consumer. This happens before slot/lock effects; the existing manifest FileGuard restores a selected manifest tentatively written by apply while preserving the owner's changed sibling bytes. It is not a new freeze protocol or promise against arbitrary concurrent writers.

Inspected `apply/deferred.rs::materialise_deferred_in_place`: it updates source manifest, hashes, source URI/ref/commit and in_place_changed, but never changes `ResolvedDep.version` or `.kind`. Thus validation/snapshot correctly stays **before** deferred materialization, instead of moving a known refusal after that effectful step.

New supporting test module `tests/root_constraints/review_cases.rs` contains three regressions: an actual relative selected-member path through plan/apply/reload without changing process cwd; a native Unix symlink alias selecting the correct member; and plan→sibling version/kind edit→apply refusal preserving the new sibling plus old selected manifest, lock, slots and boot bytes.

- Review RED: `cargo test --offline --locked -p vibe-install --test root_constraints review_cases`, exit **101**, **0 passed / 3 failed**. Relative and alias cases returned SelectedConsumerMissing; edited-sibling apply incorrectly succeeded. Retained `/tmp/nst-constraints-review-red.log`. A newly unused test import warning was removed with the correction.
- Review green: same command, exit **0**, **3/3 passed**, 19 filtered. Retained `/tmp/nst-constraints-review-green.log`.
- Final affected run: `cargo test --offline --locked -p vibe-install --test root_constraints --test prepared_planning`, exit **0**, **22 + 7 passed**, zero failed/ignored. `/tmp/nst-constraints-final-tests.log` now contains this updated final result.
- Final scoped clippy command from item9, exit **0**, 0.96s. `/tmp/nst-constraints-final-clippy.log` updated.
- Explicit-file rustfmt check over all eleven files and independent `git diff --check`, both exit **0**.

Unix alias evidence was executed on this Linux host; no native Windows alias proof is claimed. No commit/staging or other worker edits occurred.

## Final source SHA-256 (after root-review corrections)

```text
754b7710b91305e6bc01ce779e92b9e9a02d299e2369ffd50506536223463552  crates/vibe-install/src/plan.rs
f112872eff30948381c4ea7e26473c77c00404674feec92e12180ed5d0963c08  crates/vibe-install/src/plan/roots.rs
c597e558967d0316831587e0d7ee1ea4ca0dbc48120a69140501137eda15fde7  crates/vibe-install/src/plan/fetch.rs
bce7ba32c7ea319dcdad40d60c8526dea4a79c7f1d0b58c7cd12054c8423c6b9  crates/vibe-install/src/record.rs
b716f482862542681fcdd78020576850f4d31dae35ad18f93fccbe873d2a69cb  crates/vibe-install/src/apply.rs
365171654d69939484284c3b928ddf27c3248f43202dded43d34b55e78b63950  crates/vibe-install/src/apply/lock.rs
9e01970a279062dee8f4bac725ac5698c9b3f87fe253f4fa15383f8df2968441  crates/vibe-install/src/error.rs
53f0ab5c0938bc2e7638198c6ca2c451d99615e6a97f97d4893eb4a03d92bde7  crates/vibe-install/tests/root_constraints.rs
ade9cc9feecb1562cc2c612f08387e42db1f2d4fa3189e98f0a2265c4c4b1c8f  crates/vibe-install/tests/root_constraints/fixture.rs
258c1e1183a557ca33716905266d3a12ba96f7673a59c72eee8039072556aed3  crates/vibe-install/tests/root_constraints/review_cases.rs
48da354292ab7d45f418c44b25016674c92b847b27cecf7307c1bac2108e3461  crates/vibe-install/tests/prepared_planning.rs
```

## Central review

Root reviewed the collector, retry, persistence and error-chain changes. Review caught relative/alias public entry compatibility and post-plan sibling constraint drift; both were reproduced as behavioral failures and corrected. The prepared API still uses supplied state, while its public wrapper establishes the canonical selected node once. Current post-edit declarations are validated before deferred/apply effects. Root scoped conform passed with zero new findings. The CLI classifier was inspected: it walks the error source chain and recognizes the retained SolveError kind mismatch. Final main integration checks and remaining debt are recorded in LOG.md.
