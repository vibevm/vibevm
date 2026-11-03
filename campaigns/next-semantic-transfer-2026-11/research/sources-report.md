# Source archaeology: M-01–M-05

Source: `/fast/git/v/vibevm-next` at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`.
Target comparison: `/fast/git/vibevm` at `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`.

This report assesses source implementation, not current campaign acceptance, current Zap planning state, or readiness to transfer. Historical task records and REFINED-IMPLEMENTATION-PLAN are requirements evidence only. All source observations were read with pinned `git show`/`git grep`; the five dirty next files were not read as implementation authority. No builds, tests, external calls, product edits or commits were performed. Runtime verification is false for every row. Implemented means a reachable implementation plus concrete source tests for the task's bounded behavior, not that this session executed or accepted them.

`/tmp/nst-sources-tasks.json` contains all 61 leaf tasks: **39 implemented, 5 partial, 17 planned-only**, no superseded/unknown classifications. Parent task families can contain mixed states. In particular, M-03-A.2 is implemented as a *preparation* atom, while the user-facing full-pass journey remains absent. M-05 configured solving is partial relative to the historical feature/source algebra even though the later source specification explicitly narrows its supported cells.

## Findings that should change transfer planning

1. **The source-query proof machinery is stronger than the public query.** `vibe-install::verify_original_source_capture` replays original bytes against the independently selected package hash, and `VerifiedSourceMembership::derive_builtin_output` compares builtin conversion plus overlay against held materialized bytes. They have meaningful tamper tests. But `SourceQueryService::query`, at `crates/vibe-workspace/src/source_query/capture/continuity/service.rs:471`, serializes only observed/record-consistent and returns `unavailable("verified source proof owner")` for SourceProven. The CLI test `crates/vibe-cli/tests/m_01_e_2_cases/proof.rs:8` invokes the install helpers directly; placing that test under the CLI integration target does not establish public proof delivery. Preserve the distinction and decide whether public proof delivery is a desired feature before importing its transport scaffolding.

2. **`change apply` does not apply a change.** `crates/vibe-cli/src/commands/change/apply.rs:26` returns `status=prepared`, `production_request=null`, and `machine_operation_ids=null`; its successful preparation returns process success. `ChangeFullPassPlan::execution_carriage` returns `unavailable:m16-change-full-pass` at `crates/vibe-orchestrator/src/change/full_pass.rs:31`. The identity template is real and useful for planning, but no existing-file create→verify execution follows it. The source does not justify transferring M-03 as a completed lifecycle feature.

3. **Change clarification and durable closure remain plans.** `CapturedChangeQuery` already provides useful metadata/work/relations, but `query/result.rs:128` deliberately labels Closure inert with `derived-closure-out-of-scope`. There is no question store, answer/refinement transaction, waiver/risk policy owner, retained closure write/revalidation action or complete machine journey in the inspected change owners. `change/evidence.rs` reads ReleaseImpact records, which are not the missing closure evidence engine. The incumbent `progress-core::resolve_terminal` is real prior infrastructure, not proof that M-04 is implemented.

4. **Feature choices are substantive, but configured solving has a strict compatibility island.** `ResolvoDepSolver::solve_configuration_routes` rejects configured conditional targets/obsoletes/non-registry edges around `configuration.rs:283–305`, multiple instances at 321–323, cross-package forwarding at 401–403, and non-registry configured dependencies at 405–413. Pure B1/B2 evaluation supports more than this actual technical solver. The test `configured_forwarding_native_and_multiple_instances_are_explicit_limits` asserts refusal, not success. Naive/SAT are not full alternative backends for configured solving; the source spec's M05G1 boundary says so explicitly. Do not replace working main source routes with this restricted route globally.

5. **Saved-state and configuration recovery are not empty abstractions.** The change scaffold owns a no-overwrite journal and actual application; configuration apply/removal wrap real writers with before/checkpoint images and explicit uncertainty. Tests inject before/after manifest, slot, boot, lock and hook boundaries and assert retained recovery state. They are useful requirements and implementation references. They do not establish rollback of external hooks or arbitrary in-place writes. The implementation appropriately retains uncertainty instead of relabelling local restoration complete rollback.

## User features, actual consumers and source status

### M-01: exact source resolution

The useful feature is selecting the right source/resource in the selected workspace/package world, reading the exact held bytes safely, and explaining representation/provenance without inventing facts in ordinary data. Source consumers include compiler embeds (`FsSectionSource`), selected-provider prompts (`SelectedWorldPromptResolver`), source-query CLI/MCP, and later change-origin capture.

Implemented source includes strict `ResourceAddress` construction alongside explicit legacy conversion; shared portable component validation; no-follow bounded snapshots; selected owner visibility; membership/lock/descriptor continuity checks; classified Vibe/CommonMark/XML-data/opaque extraction; generated request/response validation; UTF-8 chunk cursor binding; and strict/default conversion reporting. Tests address duplicate stems/anchors, case/NFC aliases, symlinks/hardlinks, same-byte replacement, missing prepared captures, hidden dependencies, coordinated record+payload poisoning and mixed old/new worlds.

Useful concrete evidence:

- `crates/vibe-spec/src/address/resource.rs:ResourceAddress` and `address/m_01_a_1_cases.rs:selected_vibe_and_unique_abbreviated_stem_canonicalize`.
- `crates/vibe-safefs/src/file/bounded.rs:Project::read_file_snapshot_bounded`, with `m_01_c_1_cases.rs:same_byte_replacement_during_held_snapshot_refuses`.
- `crates/vibe-workspace/src/source_query/capture/continuity.rs:WitnessedSourceWorld::revalidate_current`, with selected-lock/descriptor/window race tests under `capture/m_01_b_2_cases.rs`.
- `crates/vibe-workspace/src/source_query/capture/continuity/extract.rs:extract_source`, with `malformed_xml_data_and_same_stem_commonmark_remain_raw_resources`.
- `crates/vibe-install/tests/m_01_c_4_cases.rs:strict_bad_second_package_preserves_manifest_lock_and_both_slots` and `default_failed_conversion_records_issue_and_applied_counts`.

Limits are intentional but material: finite observation windows are not hostile-ABA immunity; bounded original proof supports recipe zero; native/custom conversion has no proof; public query strength stops below independent source proof. `prepare_host` caps files and bytes and captures all selected files before answering a point query. The public no-write claim is plausible from the call graph and test assertions; no runtime syscall trace was collected here.

### M-02: reusable change intent, local refinements and read-only coverage

The useful feature is a project-owned local change that cites an immutable package origin, keeps local refinements independent, has stable identity despite a title/slug change, and exposes what work covers which facts. Consumers are authoring CLI/MCP, buffer-aware analysis, origin rebase and future execution. It does not need to become a personal scheduler.

The source has real grammar (`ChangeManifest`, `ChangeContext`, `WorkGraph`), separate framed intent/work hashes, two-project reuse fixtures, accepted origin captures/rebase proposals, complete-set fact observation, no-overwrite scaffold application with retained journal, and common CLI/MCP/library query contracts. Tests perturb personal plans and adoption without allowing those to become shared intent or acceptance. `CapturedFactSet::observe` avoids the ordinary query page cap and explicitly reports gaps instead of declaring completeness from a returned count.

Limits: directory selectors currently produce `CaptureGap::DirectoryCensus`; converted original bytes remain unavailable to complete fact selection. The scaffold/query are real, but current read coverage is metadata and traceability, not successful execution evidence or automatic closure. Main's requirements semantics should remain metadata-only.

### M-03: invocation roles and scoped execution

The implemented part is meaningful: `validate_invocation_prefix` checks the complete handler prefix before dispatch; unknown effects refuse; ordinary build/verify cannot dispatch ProductSource work without comparable completed input/output basis. `reusable_completion` forces actual evidence observers to run for verify rather than hiding them behind freshness. The tests under `runner/invocation/m_03_a_1_execution.rs` cover real handler paths and legacy-state non-relabeling.

The remaining scope is absent: `ProductionRequestIdentity` exists only as a pure proposed recipe; no change request is threaded across state/outbox/hosted carriers and no scoped existing-file execution/transition/current-source verification journey is wired. Existing leases and hosted continuation mechanisms are predecessors, not the requested change-specific conflict protocol. Preserve main lifecycle commands while separately deciding the desired create/build/verify semantics; the next branch already changed this compatibility law.

### M-04: analysis, clarification, closure

Planned-only except a partial metadata/work join reused from M-02. No question/answer or closure action is reachable. Existing terminality evaluation is a strong reuse candidate and should not be replaced by a second evaluator. The minimum useful new feature is explicit evidence-strength/completeness joins plus durable decision retention; neither a question count nor personal task completion establishes this. The next source is a requirements reference here, not an implementation donor.

### M-05: choices, solver intent and installation

There is substantial transferable behavior:

- One typed `FeatureEffect` algebra with dependency declaration identities, optional activation, strong/weak forwarding, config values and exported subskills; legacy lowering and comment-preserving migration.
- Four group cardinalities, positive guards, exact/additive consumer intent, omitted versus explicit empty input, dormant choices and explicit default policy.
- Immutable candidate catalog and candidate-local effects; older valid candidate selection without discarded effects; all incoming root constraints and provider provenance.
- Requested intent separate from resolved configuration in the lock; request/definition freshness checked before Fresh; exact replay and update retaining explicit/dormant choices.
- Actual subskill/boot activation without filtering captured package bytes.
- ConfigurationChoiceHost plus CLI/MCP adapters; machine paths return typed needs without hidden TTY, and assume-yes does not supply semantic answers.
- Configuration recovery and reverse-closure removal, including preservation of shared roots.
- Redbook v1.1.0 exactly-one continuity choice with independent global/branch additions and migration of manual exclusions. Independent WAL/MUP coexistence remains tested.

The historical refined plan describes Redbook v1.0.0; current task/source is v1.1.0. This is an evidence mismatch, not authority to choose either version during transfer. Choice-host preparation is caller-owned; CLI can connect to explicit `--choice-host`/`--capture`, but cannot mint admission from serialized answers. The host transport is real, while ordinary unprepared invocation can remain unavailable.

## Existing main overlap and preservation boundaries

Pinned main already has `FileResolver::with_selected_world`, selected-provider prompt resolution, source embeds, safefs, `RequirementsQuery::query`, progress-core terminal evaluation, lifecycle state/leases/hosted execution, package-source/resolver routes and feature expansion. In particular, `crates/vibe-core/src/manifest/package/features.rs:FeaturesTable` has default/string-list/exclusive groups; `crates/vibe-resolver/src/features.rs:FeatureValue`, `FeatureExpansion`, `expand_features` and `validate_features_table` already interpret dependency forwarding/subskills. The new features are validated declarations, explicit constrained choices, correct per-candidate behavior and end-to-end intent preservation—not the mere existence of features.

Main lifecycle uses `inclusive_chain` and existing runner machinery without the new invocation effect gate. Importing next's gate is a behavior migration that needs explicit compatibility decisions and tests, not an uncontroversial refactor. Main has no new change or source-query module, so these additions should attach to existing domain owners instead of replacing the rest of the command/application tree. Pinned main also contains `vibe-app/src/steps/features.rs`, a current application consumer absent from the inspected next tree; root should include that preservation seam in integration review.

## Candidates for simpler structure, with counterevidence

- **Separate query preparation from pure query, but question public plumbing depth.** `BoundSourceInputs` → `PreparedWorldWitness` → `WitnessedSourceWorld` → `SourceQueryService` spans a deep private file hierarchy. Evidence supports separate selected-world, held-byte, source-membership and transformed-output proofs; collapsing those into flags would lose real guarantees. However public source-proof construction is not connected, and `execute_json` prepares again on every call. A new public boundary can expose the useful snapshot and strength honestly without mechanically importing every private wrapper or path layout. Measure actual consumer reuse before retaining all types.

- **Keep change identities separate, avoid presenting an inert execution facade as an executable command.** Intent/work/request identities have distinct demonstrated invalidation laws. The `change apply` preparation-only facade adds an extra user concept with success exit and unavailable execution carriage. A deliberate plan/preview contract or an actually wired apply operation would be clearer. Counterevidence: the source reports its unavailability explicitly and has tests proving no effect; it is not silently running a fake producer.

- **Keep the domain choice service, reassess the required socket-host detour.** The existing host owns admissible definitions and makes CLI/MCP equivalent; that is a valid trust boundary. Yet requiring a separately prepared `--choice-host` endpoint can obstruct the ordinary local install user journey, and the transfer should evaluate an in-process trusted composition using the same service. Transport serialization itself must not become admission; simplification is justified by preserving one semantic service and reducing user-visible ceremony, not by bypassing preconditions.

- **Do not duplicate transaction authorities.** Configuration recovery already nests the incumbent boot journal and honestly cannot compensate arbitrary hooks. Its retained before/checkpoint images and operation membership are useful. Root should map these to main's actual install transaction owners and preserve phase uncertainty, instead of adding a parallel generic operation coordinator merely because next has one.

- **Do not import planned M-04 as a schema inventory.** There are no actual consumers for question/closure execution yet. Existing facts/requirements/terminality owners provide enough anchors to derive the desired small feature when prioritized. Conversely, deleting the complete fact-set seam because ordinary requirements already exists would reintroduce pagination-as-completeness errors demonstrated by M-02 tests.

## Recommended semantic units for a new plan

1. Exact classified read with containment, selected visibility, coherent snapshots and truthful strength; explicitly decide public original/derivation proof.
2. Request-preserving feature/choice configuration through main's existing package-source routes, with an explicit supported matrix and regression preservation for legacy routes.
3. Local reusable change intent, stable identities, origin/rebase and no-overwrite scaffold/query, independent of execution.
4. Separately approved lifecycle effect semantics and fully wired existing-file create→verify journey; preparation-only next code is not completion evidence.
5. Evidence/clarification/closure only from fresh user requirements, reusing existing terminal laws. Next contains intentions and partial metadata scaffolding here.

These are candidates for the root's actual Zap plan, not a proposed current frontier or inherited acceptance order.
