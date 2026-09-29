# Environment A1 implementation binding (before compilation)

Governing source committed123688b24: PROP076 req1 anchors. Exact new owners environment/{mod,identity,platform,roles,compatibility,tests}.rs plus one lib export. No consumer rewrite, machine codec/hash, dependency or durable EnvironmentLock schema.

Identity: checked nonblank Reference preserves opaque supplied bytes; five distinct producer-labelled ID types DefinitionId/LockId/ExecutionId/TargetInstanceId/GenerationId prevent cross-role substitution. Equality includes producer; supplied references prove binding consistency only, not authentication, filesystem identity or effect authority.

Platform: Fact<T>={Known,Unknown,NotApplicable}, PlatformFacts has explicit OS(existing TargetOs)/architecture/ABI/libc. PlatformConstraints has Any/Exact/NotApplicable requirements; compare returns Satisfied/Missing/Unsatisfied facts. Three ToolchainPlatforms members build_executor/produced_tool_runtime/optional compiler_output are supplied independently. Native-extension facts come only from ExecutionContext; descriptive payload launcher family comes only from TargetBinding OS, not executor. No launcher executable is selected or run.

Five roles: checked EnvironmentDefinition carries named desired package roots (existing PackageRef), features/choices, target constraints, named runtime bindings and tool/service exports. EnvironmentLock carries its claimed DefinitionId, selected fully qualified exact PackageRefs, existing ContentHash and capture/configuration refs plus build/runtime dependency roles, external-backend closure refs and target constraints; constructor joins definition ID, roots, constraints and selected package semantics without resolving. ExecutionContext carries executor facts, tool/provider refs, principal and named network/mount/process capabilities. TargetBinding carries instance ID, optional GenerationId, definition/lock refs, mode, explicit target facts, storage backend/principal/root/prefix/home facts and separately labelled physical-evidence reference; pure path strings are supplied descriptive facts, not proof and are not interpreted through executor OS. EnvironmentGeneration binds exact LockId/recipe refs to payload metadata/closure/boot-adapter/service/activation references; constructor checks capture/closure joins, never authenticates realized bytes. No secrets/state fields.

Target root/prefix layout and physical verification remain external storage-owner obligations at A1. Explicit LocalWorkspaceMapping binds ExecutionId, TargetInstanceId and injected absolute local workspace root. Operation compatibility verifies those refs/root and required local platform facts, supporting only workspace mode; future user/rootfs/live modes are explicitly Unsupported before operation dispatch. LegacyApplicability uses existing TargetApplicability::decide and injected executor OS; old helpers/readers untouched. No ambient HOME/PATH/filesystem/Docker/settings lookup.

Eight bound tests in --lib environment::tests:::

- role_ids_preserve_producers_and_do_not_conflate_equal_reference_text
- windows_executor_linux_payload_and_compiler_output_keep_three_roles
- explicit_local_workspace_mapping_preserves_legacy_guard_behavior
- os_match_does_not_hide_abi_libc_mismatch_or_missing_facts
- future_target_modes_are_describable_but_operations_unsupported
- definition_and_lock_reject_duplicate_or_foreign_bindings
- generation_and_target_reject_cross_reference_tampering
- constructors_reject_blank_inputs_and_need_no_ambient_state

Run ONLY this family using CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=4 sharedroot target, cargo test -p vibe-core --lib --locked environment::tests::. Require8nonzero cases, retain raw stdout/stderr and exact exit JSON at /tmp/vibevm-next-M14-A1-tests.log/result.json. New leaf rustfmt skip_children; no fmtall/check/Clippy/fullcrate/generator/native execution. Physical/native ABI/runtime/provider support remains unclaimed. Root reviews/integrates/accepts.

Precompile owner refinement: new compatibility helper explicitly named workspace_source_projection_compatibility, with SupportedLocalWorkspaceSourceProjection. Local source materialization requires explicit map+OS/arch, applies declared constraints, and compares ABI/libc only when required; Any+Unknown remains supported without claiming payload/native ABI admission. Existing case3 now asserts that legacy positive and Exact missing/mismatch negatives. Other review edges stay in same8cases.

Observed final checkpoint: writer stopped; no further source edits. One authorized family invocation EXIT0:8passed/0failed/0ignored/533filtered. Raw combined stdout/stderr /tmp/vibevm-next-M14-A1-tests.log; exact invocation, environment, exit and source/log SHA256 /tmp/vibevm-next-M14-A1-result.json. Only existing vibe-cli unused-build-key warning; no environment diagnostic. No check, old cases, wholecrate, Clippy, codegen or runtime/provider/physical-root operation. New leaves formatted with skip_children; lib changed only the environment module export.

Covered refinements: source-only local compatibility accepts Any+UnknownABI/libc while Exact unresolved/mismatch refuses (case3); all legacy OS guard decisions/reasons equal incumbent owner (case3); wrong mapping execution/instance/root refuses (case3); Unknown versus NotApplicable under Exact and architecture field correctly labelled (case4); absent compiler output never inferred (case2); qualified selected package satisfies an unqualified desired root (case6). Native ABI descriptor uses executor only and payload family target only (case2); future mode operation remains Unsupported (case5). Cross-role producer refs, selected capture/definition/lock/generation joins and blank-input refusals retained. Pure data constructors do not authenticate content or prove physical/effect authority. Independent review and acceptance remain root-owned.

Final exact source bytes:

- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/lib.rs: 47lines, 1697bytes, SHA256 5734dcbc3bb2f20d4284e015e26269ff1496b3acc41555d7587c69abbefc30b8
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/compatibility.rs: 182lines, 6612bytes, SHA256 1dee080849d8f75fe5f183ee5352aa7c0fe954b68bb95140a20bf7867401afc4
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/identity.rs: 45lines, 1766bytes, SHA256 45032df8619a02fd9547d884a7dbd4a1dc3388b8a1dae2182db5986841dbff4f
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/mod.rs: 38lines, 1190bytes, SHA256 b8bffdc11346a5dc4fe9de2cb35b85887acf7a69f3215330041a09e873fe940c
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/platform.rs: 188lines, 5034bytes, SHA256 3bba3560b4b677112f0a1eb2b53775447e4f094b27441f4befcbb9eb4dcc9b08
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/roles.rs: 536lines, 16930bytes, SHA256 b1574c264c9844cdfd32f9ca19b85d1f7fbe0048ffe8c398e378dcb7141edac7
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/tests.rs: 598lines, 19700bytes, SHA256 3340f87e88bde25b7a17faae738f2993e8b02f67485bca96516573338210e4bf

Export-namespace correction checkpoint: independent review confirmed the combined tool/service uniqueness rule was unbound. Corrected only roles.rs to validate tool_exports and service_exports separately; the combined root/runtime referential loop is unchanged. Existing definition_and_lock_reject_duplicate_or_foreign_bindings invokes new export_assertions.rs helper (no new #[test]): cross-kind equal name accepted, duplicate tools refused, duplicate services refused. No assertions dropped; parent fixture constructor shortened only to fit registration. Final touched lengths roles536/tests599/helper32, skip_children formatting0.

Only affected exact case ran under offline/jobs4/sharedtarget: EXIT0,1passed/0failed/540filtered. Raw /tmp/vibevm-next-M14-A1-export-namespace-test.log; new exact exit/harness/log SHA and8source hashes /tmp/vibevm-next-M14-A1-export-namespace-result.json. Changed/new owners and previously bound mod hashes remeasured; other hashes explicitly carry forward from unchanged original receipt, outside correction write scope. Original tests.log/result.json bytes verified unchanged and their hashes retained in new result. Existing7unchanged case evidence retained, not rerun. No dependency, consumer, generic name grammar, old/fullfamily/check/generator/Git changes. Writer stopped; root reviews/commits.
