# Source audit: M-12–M-17 and NEXT-PREVIEW

Source: `/fast/git/v/vibevm-next` at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Comparison target: `/fast/git/vibevm` at `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`.

This is read-only archaeology, not acceptance or continuation of the old campaign. Task JSON and `REFINED-IMPLEMENTATION-PLAN.md` supply historical requirements. The user's actual development plan belongs to Zap. No old campaign instruction was executed. All source reads used the pinned Git object, including paths whose current worktree might be dirty. No build, test, network call, host mutation or publication was performed. Every JSON row has `runtime_verified:false`.

## Denominator and status meaning

92 leaf records, including the M-16-C.2 parent and its four explicitly split children. These overlap: counts are task records, never independent feature counts or delivered percentage. `implemented` means an inspectable source implementation plus relevant tests/contracts is present; it does not mean tests passed here or every platform was verified. `partial` means actual useful behavior exists but the historical task scope has a material missing part. `planned-only` means a required runtime/product outcome is not found; pre-existing prerequisite mechanisms do not count as implementation of that outcome. Two historical census tasks are `unknown`: their completeness is not a runtime property and was not independently reconstructed in this bounded audit.

| Family | Implemented | Partial | Planned only | Unknown |
|---|---:|---:|---:|---:|
| M-12 integration/publication | 0 | 2 | 6 | 0 |
| M-13 rules/migrations | 4 | 2 | 0 | 2 |
| M-14 environment seams | 7 | 0 | 0 | 0 |
| M-15 HOME/Linux/OCI delivery | 0 | 0 | 30 | 0 |
| M-16 machine/session/edits | 7 | 9 | 0 | 0 |
| M-17 universal coverage/rename | 0 | 8 | 4 | 0 |
| Preview accounting/docs/release | 3 | 4 | 4 | 0 |
| Total | 21 | 25 | 44 | 2 |

The old plan's requested outcome is recorded in each JSON row's `feature`. It must not be mistaken for observed source status. No status was inferred from receipt existence, a plan label, or a green declaration. The checked task files themselves contain instructions/goals rather than a trustworthy delivery status field.

## Features worth recovering as user requirements

1. **Preserving authored configuration during explicit migration.** The consumer is an existing project owner, not a campaign ledger. `vibe-core/src/manifest/migration.rs` plans edits; tests include token-only changes, exact fixpoints, UTF-8/CRLF/comment preservation, unknown shapes and missing user choices. `vibe-workspace/src/migration_planning.rs::plan_migration_request` exposes strict inputs/results, and CLI/MCP adapters exist. `migration_application/runtime.rs` and `runtime_tests.rs` cover retained before-images, restart, restore, third-state refusal and failure after writes. This is a concrete source-level feature. Actual cross-platform behavior still needs fresh evidence in the target design.

2. **Target-aware local deployment without ambient HOME ambiguity.** EnvironmentDefinition, ExecutionContext, TargetBinding, lock/generation and platform roles exist in `vibe-core/src/environment/roles.rs`; centralized roots are in `layout.rs`. `deploy/binding.rs` pins roots and reobserves generation; alias and stale-generation tests have concrete failure cases. `bound_effect_tests.rs::two_admitted_homes_publish_physical_builtin_and_distinct_bound_receipts` is a real local builtin path through the incumbent deploy transaction. This supports a real consumer: installing an existing tool into an explicitly selected user destination. It does not deliver an environment package manager.

3. **Lost response must not execute an effect twice.** `machine_submission.rs::submit` and `SubmissionJournal` combine reservation, journal readiness, invocation and reinspection. `tests.rs::eight_same_key_submitters_invoke_one_fixture_effect`, `invoke_error_after_counted_effect_keeps_recovery_or_uncertain_run` and `retired_namespace_replays_existing_outcome_but_refuses_new_key` encode meaningful invariants. A reconnecting local agent is a plausible concrete consumer. Domain journals preserve actual completed/partial/uncertain truth rather than making the dedup index a result database.

4. **Editor/client buffers must not silently overwrite saved state.** `document_analysis::DocumentClient` owns per-client overlays; `AnalysisCapture` freezes them with disk/world inputs. `edit/fence.rs` requires participating editor acknowledgements and retains the barrier through commit. `m_16_d_3_cases.rs` covers a late buffer change, disconnect, stale disk, cold recovery and unknown third state. This has a concrete application to MCP or future editor clients. It expressly covers cooperating registered editors, not arbitrary unregistered filesystem writers. The existing tests are source evidence only.

5. **Structured observations for long work.** The responsive session and live-owner modules have bounded queues, polling, replay and cooperative controls. The useful requirement is that querying/cancelling/reattaching must remain responsive and must not change domain result truth. That requirement should survive independently of the current fixture-specific transport wrappers.

## Machine layer: substantive gaps and unnecessary coupling to challenge

### Protected build/package routes are bounded proof slices

`crates/vibe-lifecycle/src/mechanism/build/journal.rs` is production source, not merely a test fixture. It hard-codes `TARGET="fixture-tool"`, package `vibe-r8-fixture`, binary `r8probe`, exact `Cargo.toml` bytes and `src/main.rs` containing `println!("r8")`. `subject()` rejects variable targets, outputs, source bytes and profiles. It requires a specific clean offline project shape and reports its toolchain scope as `ambient-rustup-unproven`.

The package journal likewise accepts a single static-file target/source/output. `machine_operation/fixed_cargo/contract.rs` checks a literal artifact-record path, and the package contract expects `.vibe/state/artifacts/launcher.bin.json`. These routes demonstrate real pre-effect retention and replay using incumbent providers; they are not a protected general Cargo/package API. The proper transfer is the invariant and reusable domain journal mechanism. Shipping fixture-specific routes as broad user features would be a false claim. No claim here that a bounded proof slice is intrinsically wrong: it is a useful test instrument, but its generality must remain explicit.

`publication_operation.rs::submit_fake_tag_create` is crate-private and explicitly a fake remote-tag path; tests prove uncertain remote responses do not repeat an effect. General publication work also exists elsewhere in the source, but this slice alone cannot close M-16-C.2.4's all-domain remote submission/lifetime requirements.

### One universal facade has not actually emerged

`MachineOperationService::validate_route_metadata` rejects validated/available/admitted foreign operations; only the recognized migration-planner identity is allowed. It even binds a fixed historical `PLANNER_EVIDENCE` identity. Fixed lifecycle/build/package/deploy and later domain routes live alongside it. CLI `commands/machine_session.rs::validate_mode` selects one fixed mode per session, and the dispatcher constructs the corresponding runner.

This is a measurable architectural discontinuity, not a LOC criticism: the capability registry does not enumerate/dispatch the effectful routes as one shared domain service. Before transplanting it, decide whether one common local protocol is a real product requirement and define its first consumers. Preserve domain ownership, typed errors and no-duplicate-effect behavior; do not automatically preserve one runner/schema/host mode for each historical proof increment.

Counterevidence: the code intentionally keeps domain journals authoritative, makes unknown routes unavailable, and does not introduce a second resolver or generic scheduler. Separate routes are safer than falsely admitting an unfinished universal route. Their existence is evidence of a partial implementation, not evidence of malicious or broken behavior.

### Authority layers need a declared threat model

Current layers include connection-local context IDs, an owner-issued SubmissionNamespace/SubmissionScope, durable reservation, `PolicyOwner`, per-domain grant pointers (Issuing/Issued), domain readiness journals and physical-root leases. They do different things in code:

- session IDs prevent accidental cross-session request association;
- namespace/key/plan identity prevents duplicate effects and key reuse;
- policy generation permits current-policy revalidation;
- grant pointers retain a host-selected namespace across process exit;
- pinned roots and cooperating locks constrain which destination a local operation uses;
- domain journals retain actual effect truth.

`CliGrantService` explicitly says it does not authenticate an OS user. CLI principals are local labels such as `cooperating-project-cli` and `vibe-cli-project-local/e1`; these are not multi-user identity proof. `PolicyOwner` explicitly excludes root ABA and deleting/rolling back all host records, and reports weaker filesystem durability honestly. Rust capabilities do not sandbox arbitrary native code.

Therefore the layers cannot simply be sold as a new security boundary. Their strongest demonstrated purpose is cooperative local correctness/reconnect/recovery. Some layers may still be necessary for a long-lived embedded service or multiple independent clients; decide that from a consumer and threat model. The requirement to avoid stale authority and duplicate effects is clear. The need to copy all persistent policy/grant/namespace objects into the new architecture is not yet established. Counterevidence is important: omitting current-policy revalidation or durable issuing fences would reintroduce concrete replay/expiry races, so flattening requires an equivalent proof, not merely deleting types.

### Documentation and inventory have fallen behind source

`docs/machine-api/framed-session.md` says there is no public command, only pure migration planning, and no streaming/replay/control. `commands/machine_session.rs::run/dispatch` exists and binds effectful fixed runners; responsive controls also exist. `docs/machine-api/README.md` still describes the pure planner as the sole shared route. That statement is accurate about the narrow generic service but misleading as an overview of the whole shipped source surface.

The retained `coverage-e1.json` has 247 surfaces and 35 models, plus 15 planned families. Its surface statuses are 145 source-observed/unknown external access, 72 source-observed/structured, 23 source-observed/legacy, 6 gap/legacy, 1 gap/unimplemented. This is a finite historical observation, not complete external conformance. Separate source-delta files exist, but do not make the baseline an executable whole-product closure. M-17 cannot be marked complete from these counts.

## Absent or unfinished outcomes

**M-15 is not a delivered HOME/Linux/OCI manager.** No `crates/vibe-orchestrator/src/environment/` runtime owner exists at the pin; no rootfs or Docker implementation files appear in the audited product paths. Environment roles are supplied data and pure validation. `environment/tests.rs::future_target_modes_are_describable_but_operations_unsupported` expresses the intended boundary. Real deploy-to-user-root functionality is counterevidence to any claim that the entire area is empty, but it does not provide standalone resolution, immutable shared payload store/profile generations, dependency-safe remove, generation GC, BuildKit, service catalog/probes, Arch transactions, offline supply-chain proof or the complete HOME/Docker journeys. All 30 M-15 records stay planned-only for their requested outcome.

A particularly strong missing obligation is `vibe-workspace/src/bins.rs::DeclaredBinary::artifact`: it still chooses a debug artifact if `debug.exists()`, otherwise release. This file is byte-identical to current main. M-15-A.3 explicitly requires replacing that existence-based dispatch with recorded artifact/closure validation; it has not happened.

**Semantic package-coordinate rename is absent.** Generic saved-file edits exist, but no owned-reference universe/semantic rename planner/apply/recovery service was found. The only rename-named crate file is pkgskill receipt rename, which is a different feature. M-17-B's three records remain planned-only. The two-client editor tests are not rename journey proof.

**Retained universal debugger access is partial.** Boot/document observations exist. `compile_trace/open.rs::prepare` still opens a project and takes a cooperative trace lock before existing-only access. That file is byte-identical to main. It does not establish the requested detached no-writer-ownership complete read/export of every meaningful internal stage. Do not conflate readonly domain projections with complete universal stage capture.

**Initial integration and publication are not closed.** No `fixtures/next/integration` subtree exists at the source pin. Per-domain install/adapters and publication mechanisms do not prove the combined full graph/offline replay or a ready exact publication batch. Optional tracker tasks have no runtime implementation. There is no basis here to infer publication authorization or execution.

## Preview accounting: tool exists, final outcome does not

The Python wrappers are thin transports into Rust xtask readers/checkers. The substantive source checks capture membership, contribution/affected-source review and native release-record binding. The assembler tests cover deterministic drafts, malformed inputs, no-impact distinctions and source/symlink preservation. These are useful release-engineering capabilities, though not runtime requirements for ordinary package operations.

`xtask/src/bin/preview-transition-check/readiness.rs` unconditionally emits pending/unsupported prerequisites for late documentation/release/retired closure; `documentation.rs` implements selected byte/page subclaims and explicitly leaves full intake pending. `domain_bindings.rs` implements selected inventory and native-planner joins, not a completed universal linkage. Retained `transition.json` declares DP1 capture `30d217bfce47bcaccadabd9468f00e315dbcfead`, but DP2 remains `pending`, documentation is `pending-owner-binding`, migration guide is `pending`. These scalars align with source limitations; they are not used as proof of implementation.

Thus do not transfer the old campaign's contribution/attestation bureaucracy as an automatic requirement of the product. Preserve the user outcomes: truthful migration guidance, exact before/after provenance, documentation consistent with behavior, and explicit unsupported platforms. Decide separately whether the new release process needs the whole permanent validator chain. Counterevidence: it catches genuine stale-evidence, undeclared-delta and destructive draft-output errors, so its useful predicates may be worth retaining as focused release tooling.

## Current-main overlap and transfer recommendation

Verified by Git blob identity at the two pins:

- `crates/vibe-workspace/src/bins.rs` and `compile_trace/open.rs` are identical. Their limitations are inherited main behavior, not new implementation benefits in next.
- `vibe-lifecycle/src/mechanism/build.rs`, `package.rs`, `deploy/ownership.rs`, `deploy/protocol.rs` exist in both and differ. Treat next as modifications of existing domain owners, not an invitation to duplicate deploy/build engines.
- environment role module, manifest migration planner/application, machine-operation/session modules, durable submission state, document-analysis overlays and preview tools are absent at those paths in main. They are genuinely new source areas requiring semantic selection, integration and tests.
- `tree/model.rs` exists in both and differs; its CLI-local position persists.

Recommended semantic atoms, subject to Zap priorities: (1) authored migration with exact preservation/recovery; (2) explicit local target binding and root/generation fencing integrated with main deploy; (3) domain-owned durable execution/replay for a real chosen operation rather than hard-coded fixture routes; (4) client overlay/saved-state barrier only if a concrete external client consumes it; (5) focused release/migration documentation checks. Retain HOME/OCI/service manager and semantic rename as requirements requiring fresh design/implementation, not delivered code to cherry-pick. Rebuild capability discovery from the actually selected product surface. Do not copy old accepted-task labels, all-domain claims or fixture-specific machine modes into the target product contract.
