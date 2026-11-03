# F02 design input: preserve workspace consumer constraints

Read-only inspection of `/fast/git/wt/next-semantic-transfer`, 2026-11-03. No source changes, builds, tests, shared-target writes or campaign writes. Root owns acceptance and logging. Conclusions below distinguish observed code from the proposed fix contract.

## Decisive observations

1. `vibe-install/src/plan.rs:219–240` deduplicates bare-install declarations by `(group,name)` while traversing root then rel-path-sorted members. Thus two consumer constraints become the first consumer's constraint.
2. The explicit branch seeds `request.roots` and its identity set, then ignores **every** workspace declaration of a requested coordinate, including other consumers. This permits a selected member's install to override sibling requirements.
3. `vibe-resolver/src/resolvo_engine/mod.rs:131–146` already supports the needed distinction: every root contributes a version requirement; only the output root-order identity list is deduplicated. A new solver or choices grammar is unnecessary.
4. `vibe-workspace::freshness::check` checks every consumer's version constraint before comparing root identity sets. It correctly rejects a locked version outside a second member's requirement. The subsequent stale solve can nevertheless lose that requirement in plan.rs.
5. `plan/fetch.rs:277–291` performs a second identity dedup before conditional re-solving. Correct initial roots will be lost there unless this join changes too. `resolve_effective` forwards its roots unchanged to `solve_masked`, so visibility retries can preserve constraints if fed the correct root list.
6. Explicit-branch held pins are currently incorporated into `roots` itself. The `solve_roots != roots` fallback therefore cannot free those explicit-mode held pins. This is a separate existing minimum-churn limitation to consider while separating authored constraints from optimization pins.

## Proposed semantic contract (root to accept)

One solved package coordinate can have many authored consumer constraints. Preserve all applicable constraints; deduplicate solved packages, not declarations. Byte-identical duplicate requirements may be coalesced as an optimization, but equality must include kind/version intent and must not erase differing requirements. Keeping duplicates is simpler semantically and already supported by resolvo.

For bare install: collect every finalized node's registry requirement, including expanded workspace-version requirements, in deterministic node/declaration order. All coexist conjunctively.

For explicit install: determine the exact selected node using the supplied workspace's node identity (`Workspace::node_rel_of` or its existing selected-node helper semantics). Start with explicit requested requirements. Replace the old requirement for each requested coordinate **only in the selected node**, matching what apply will record there. Keep every other node's requirement for that coordinate. Keep selected-node requirements for unrelated coordinates. A request is an intended edit to one consumer, not global permission to remove all other consumers' intent.

A missing selected node must be a refusal, not a fallback to root or omission of every old declaration. Prepared inputs have an explicit canonical-path precondition; do not reread manifests or rediscover/canonicalize the tree in this helper. Existing `prepared_planning.rs` tests intentionally make supplied tree and disk disagree.

Kind remains metadata under PROP-008, but preserving constraints must not silently erase a conflicting kind assertion. Use existing post-resolution kind checks rather than changing identity to `(kind,group,name)`.

Repeated conflicting explicit requests need deterministic refusal or conjunction. Do not introduce last-wins behavior accidentally through the same identity set that caused this bug. The user-visible persistence policy for repeated explicit refs should match solve semantics.

## Correct held-pin treatment

Separate `authored_roots` (the complete constraint set after selected-node replacement) from `solve_roots` (temporary lock preferences encoded as pins).

- No explicit arguments: existing `hold_pins(authored_roots, lock)` may tighten each satisfied registry requirement. If a changed sibling requirement conflicts with that temporary pin, retry the **complete unpinned authored roots**. This allows moving a package when all current consumers allow it.
- Explicit coordinate X: never hold X to its old locked version through **any** retained sibling constraint. Retain the sibling's authored range verbatim, and apply the request conjunctively. Otherwise a sibling `>=1,<3` becomes old `=1.0` and wrongly blocks selected request `=2.0`.
- Unrelated coordinate Y: preserve all consumers' ranges, and hold its satisfied registry pin where valid. A pin conflict can trigger a complete free retry, as PROP-011 specifies; the retry must retain every declaration.
- `hold_pins` checks lock `source_kind == Registry`; git/path/override rows are intentionally not converted to registry version pins. Do not replace source routing with pins.
- Do not store pin-tightened optimization roots as if they were authored intent.

Concrete positive: selected A previously `X=1.0`, B `X>=1,<3`, lock X1.0, request in A `X=2.0`. Solve must permit X2.0. Negative twin: B says `X^1`, same request must refuse. This pair rejects both global replacement and over-pinning “fixes.”

## Conditional/visibility retries

Keep all base authored/accepted constraints in every conditional solve. Current code appends exact pins for every fetched root then identity-dedups; the dedup happens to discard appended pins where a base root already exists. Merely deleting `retain` will add unwanted exact constraints and freeze existing roots. Preserve its intended distinction explicitly: add a carried exact pin only for a previously added conditional-root coordinate not already represented in the base root constraints; never remove base sibling constraints. Then append genuinely new extra dependencies. Visibility retries receive that full list.

There is a neighboring, broader conditional issue: the `already` check at `fetch.rs:230–244` suppresses a conditional dependency solely because its coordinate is fetched or already in `extra`, even if its constraint differs. That is outside “workspace root preservation” unless the accepted F02 contract includes complete conditional-declaration constraint semantics. At minimum record this limitation and do not claim full conditional correctness from the narrower regression. A fresh unrelated conditional root is sufficient to exercise the root-loss defect in scope.

Useful regression: A `X>=1,<3`, B `X<2`, candidate X1.5 and X2.5. A trigger root activates new Y, forcing the conditional re-solve. Initial correct solve chooses X1.5; the identity-dedup retry would choose X2.5 if B's condition is dropped. Assert final graph still X1.5 and Y included. This catches the second loss after an apparently correct first solve.

## Source routes: observed boundaries, not assumed support

### Registry and workspace versions

`Workspace::expand::finalize_one` drains `var_packages` into concrete `packages` for each node using nearest-enclosing workspace versions. The planner must consume this supplied finalized tree, never the raw selected manifest as the whole union. Different nested workspace definitions must remain separate constraints on a shared package.

Raw selected-node persistence matters: `record::merge_manifest_requires` inserts an explicit root into `requires.packages` but does not remove a matching raw `var_packages` entry. `RequiresWire::from` serializes packages first, vars last into the same map. A same-kind var key overwrites the explicit replacement on disk. Therefore explicit replacement of a selected `version.var` cannot be claimed complete by fixing the solver alone. Preferred semantic contract: replace only the selected dependency's var indirection with the explicitly requested constraint, preserving the shared `[workspace.versions]` table and all sibling var declarations. If that behavior is not yet accepted, refuse that shape explicitly rather than report success while discarding the request. A kind-prefixed key mismatch can also create duplicate coordinate spellings; normalize identity consistently.

### Git

Planner currently converts each git declaration to Latest and relies on the provider for source identity/version. `builder::open_multi_from` registers only `manifest.requires.git_packages` from the **raw selected manifest**. The CLI factory receives that manifest via `PackageSourceBuild`, not the complete workspace. A sibling git declaration may therefore contribute a root but its source binding is absent from the constructed resolver. Optional git-version requirements are enforced within the registered declaration/source route, not by the planner's Latest placeholder.

`--git` has its own prior mutation path: `CliGitSourceMutation::apply` persists the selected raw declaration and replays the same delta into the selected finalized node. `record::merge_manifest_requires` deliberately leaves existing selected git declarations unchanged. An ordinary explicit pkgref is not permission to rewrite their git URL/ref. Do not filter away a selected git source because you are replacing a registry version requirement.

Multiple distinct git sources for one coordinate cannot be resolved by blindly concatenating them: `with_git_packages` collects into a HashMap, last entry wins. Full workspace source composition needs a conflict rule and provenance, not just duplicate PackageRefs.

### Path

`path_packages` are absent from both plan root branches. `orchestrator/install/mod.rs`'s empty-world test checks only packages/git_packages; plan's `roots.is_empty()` also returns Fresh before freshness. A path-only workspace can therefore take an empty-world path even though `freshness::check` would conservatively say Stale if reached.

The lower provider supports `ResolvedPathDep`/`with_path_packages`, verifies the path manifest coordinate and optional dual-form version, and defines override > path > git > registry priority (`multi_registry_resolver/walk.rs::resolve_inner`). But a source search finds production install composition does not call `with_path_packages`; current callsites are provider tests. `ResolvedPathDep` itself says caller/workspace lowering is a later milestone. Do not claim preservation of path constraints or safe registry fallback from the F02 root-list fix alone.

### Overrides

Overrides are selected by effective selected-manifest + global registry configuration, then provider priority. `hold_pins` excludes lock rows marked Override. Root-list correction should leave the existing source-selection boundary unchanged. It cannot establish cross-member override policy or per-consumer source resolution; introducing that would be a larger contract.

Recommended scope language: fix **registry-version and expanded workspace-version consumer constraints through existing install planning/retries/apply**, preserve existing source dispatch behavior, and explicitly log remaining cross-member git/path composition debt. If root instead requires all source kinds within F02, widen the packet to orchestrator source construction, registry source conflict handling, empty-world paths and selected authored recording before implementation; do not disguise it as a root Vec change.

## Freshness and lock snapshot

Test valid unchanged all-consumer world: one locked X version satisfies all constraints, all slots exist, representation is current → Plan::Fresh and zero source solves. `slot_integrity_verify.rs::fresh_lock_skips_resolution_without_touching_the_source` is the incumbent behavior.

Test invalid later consumer: lock satisfies first A but violates B → no Fresh; run real resolver with all constraints and refuse or pick a jointly valid version. A recording mock alone cannot establish satisfiability.

Lock metadata is not a per-consumer authority. Bare `record_lock` calls `replace_root_dependencies(roots)`; explicit calls `merge_root_dependencies(finalized_roots)` into prior metadata. Package rows remain unique; freshness checks root identity sets then current consumer versions. The new multi-constraint solver list must not accidentally make root_dependencies a misleading held-pin history. Duplicate root constraints can currently serialize as a Vec, but case-c migration copies that list into **one** selected manifest, whose wire table requires one coordinate key. Thus blindly persisting every workspace constraint as duplicate roots is a separate compatibility hazard. Keep solver constraints and lock's compatibility root snapshot conceptually separate, and define/test the snapshot projection rather than using one Vec for both without analysis. Authoritative consumer requirements remain their manifests.

Explicit install on a workspace with previously unrecorded unrelated roots currently merges only finalized requested roots into metadata, even though its solved graph is complete. Check second bare install behavior; do not claim a fresh stable lock until this projection matches the authored root identity set. An identity-only display/metadata set is legitimate; it must never feed back into constraint solving as the complete authored requirement list.

## Concrete integration fixture strategy

Use `vibe-install/tests/root_constraints.rs` (proposed safe test binary name; this crate has `[lib] test=false` for Windows UAC). Reuse the pattern, not wholesale copied fixtures, from `tests/visibility_resolution.rs`: in-memory candidate manifests + fixture package directories, `TestProvider: DepProvider + VersionEnumerator`, real `ResolvoDepSolver` in InstallSource::solve/solve_masked, and actual CachedPackage returns. No network, provider model or external source is needed. `prepared_planning.rs` recording-only source is useful for input-shape assertions but cannot be the final semantic oracle.

Suggested small meaningful matrix:

1. Bare incompatible A X^1 / B X^2 → error before fetch/apply, all authored/lock/slot bytes unchanged.
2. Bare overlapping A X>=1,<3 / B X<2 with candidates1.5/2.5 → exactly X1.5, regardless of member ordering.
3. Explicit from member A old X^1 → X=2 with sibling B X>=1,<3 and old lock1 → X2 success; selected manifest changed only, B unchanged, unrelated Y retained.
4. Same request with B X^1 → refusal; selected old manifest/lock unchanged (exclude the intentionally separate --git pre-write case).
5. Request from member B proves exact selected-node replacement; root A's old requirement remains binding.
6. Satisfied unrelated Y pin remains stable while X moves; conflicting held optimization retries full original constraints, not a dedup projection.
7. Conditional new Y forces a second solve while A/B X intersection remains binding; optional visibility masking retry verifies same root multiplicity survives.
8. Fresh all-consumer lock skips source; conflicting second consumer cannot fast-path.
9. Different nested version.var definitions participate after expansion. Explicit selected var replacement persists and reloads to the intended constraint; sibling var and shared version table survive.
10. One full plan→apply→reload→bare-plan case proves unique materialized package, correct chosen version, preserved sibling manifests and stable root metadata. Add CLI fixture only if source-factory behavior is changed; a crate-only fake cannot prove actual git/path composition.

For refusal fixtures initialize manifests explicitly to avoid legacy case-c migration's intentional pre-solve write. Use actual content hashes for materialization tests and keep fixture stores isolated; no global environment change across concurrently running tests. F01 owns shared target currently, so these are test designs only.

## Ownership and existing anchors

- Root-set assembly/selected replacement/pin separation: `vibe-install/src/plan.rs`, bounded helper under `plan/selection.rs` or a new plan-owned module.
- Conditional preservation: `vibe-install/src/plan/fetch.rs`; same accepted roots must reach `visibility_projection::resolve_effective` retries.
- Existing pin/freshness policy: `vibe-workspace/src/freshness.rs`; do not change its correct all-consumer satisfiability loop merely to fix planner dedup.
- Selected authored persistence and lock compatibility projection: `vibe-install/src/record.rs`, `apply.rs`, `apply/lock.rs`; retain rollback/FileGuard behavior.
- Larger source-route follow-on: `vibe-orchestrator::ports::PackageSourceBuild`, install composition, `vibe-package-source::builder`, existing `vibe-registry` route owner. No second source resolver.

Precise contracts:

- `PROP-009-loading-model.xml#INSTALL-UNIFIED`: one unified resolution across every member, one package slot and one root lock.
- `PROP-009#SCOPE-FLAG`: member selection scopes reporting, never a partial dependency world.
- `PROP-011-incremental-install.xml#EXPLICIT-PKGREF-FULL-SOLVE`: named packages re-resolve fresh, full union retained.
- `PROP-011#ORACLE-SAT-TEST`, `#ORACLE-ROOT-SET`, `#ORACLE-SCOPE`: every consumer constraint checked; conservative source-kind fast paths.
- `PROP-011#RES-HOLD-LOCK`, `#HOLD-CONFLICT-FALLBACK`: minimum churn with a complete free retry.
- `PROP-007-workspace.xml#MATRYOSHKA-RESOLUTION`, `#DUAL-FORM`, `#PATH-PRIORITY`: expanded versions and source boundaries.
- `PROP-003-dep-evolution.xml#req-conditional-fixpoint`: conditional re-solve must preserve the world.
- `PROP-009#INSTALL-ROLLBACK`: failed apply retains original authored/lock/slot state.

Incumbent tests: `prepared_planning::the_root_union_reads_the_supplied_tree_not_the_file`, `the_explicit_pkgref_union_reads_the_supplied_tree`; `slot_integrity_verify::fresh_lock_skips_resolution_without_touching_the_source`; `visibility_resolution::invisible_constraint_stops_pinning_versions`; workspace freshness hold_pins tests; CLI `cli_clean_and_world::{an_explicit_pkgref_install_keeps_the_rest_of_the_world,an_offline_pkgref_install_keeps_the_rest_of_the_world}`. These protect neighboring contracts but presently do not assert different constraints for the same coordinate across consumers.
