M-06-B.1 correction candidate, unreviewed; no acceptance or next-task dispatch.

Continues the original retained implementation. Read original final findings/report/candidate and all test log tails/exits before editing. Read PROJECT-BOUNDARIES.xml, supplied literal task, explicit READ-ROSTER.json, compiler domain/wire owners, trace generated index/validator and workspace compile_trace owner. VibeVM owns this implementation; Zap retains planning/acceptance; PodBay retains physical lifecycle. No child agents, Git mutations, staging, commits, dependency/lock/toolchain edits, production extension semantics changes, or unrelated formatter cleanup.

Source history: original admission baseline 68; current Root-confirmed 69 includes M05A2 receipt 31bc69fb4. These are supplied source facts, not new acceptance actions.

Implemented correction:
- All compiler snapshot arms now expose source/contribution views; lane nodes retain separate ordered occurrences with capture-local closure node identity, including repeated nodes. Emitted witnesses expose all contribution variants without fabricating occurrence counts from digests.
- The complete existing generated compiler IR is retained in BootGraph.carrier after existing strict decoding/verifying. Its nested absorption/link plans, pending source/embed snapshots, source text/subjects, trees, lane chunks/fences/ordinals, renames, context and emitted provenance remain typed and traversable. Original bytes remain in RetainedGraph. This additive field reuses the owner's vocabulary, not a new IR/context engine.
- read_run freezes one validated trace index, traverses every event/snapshot, and retains all scopes (including scopes without events), event statuses, exact identities and individual availability results. Missing children do not hide siblings. It uses pinned reads only.
- Shared INDEX.md parser/projection preserves static pointer, every ordered entry, repeats, path, exact kind spelling and unevaluated when. The CLI delegates parsing to the shared workspace owner, preserving its existing output mapping.
- Added targeted tests for all lane contribution variants, repeated node identities, typed carrier preservation, whole-run traversal with the writer held and provider counters unchanged, missing-child sibling preservation, dynamic-index conditions/repeats/unknown kinds, and CLI index order.
- Corrected exactly nine ExtensionDecl fixtures in the eight authorized files by inserting effects: None, preserving the previous meaning. No owning type was edited.
- PROP072 CAPTURED-GRAPH accurately describes typed carrier preservation and ordered index/run traversal. Existing IDENTITY/KNOWLEDGE/READ-ONLY anchors remain.

Required criteria in capture order:
1. UNSATISFIED for full exact why reconstruction. The previously absent lane/emitted/dynamic-index projections and full retained run traversal are implemented. Full available typed evidence is accessible, but exact occurrence ancestry is not reconstructible from the retained fields. ClosureOccurrence needs a parent-occurrence/edge reference; ClosureEdge needs directive span. LaneNode carries neither. EmissionContributionWitness has chunk_digest instead of chunks/occurrence ancestry. The narrow gap is capture-time provenance fields propagated through existing carriers, not a solver or canonical IR rewrite. No guessed edge chain or cross-snapshot equality is declared successful.
2. SATISFIED as candidate evidence: previously textual static marker nesting is structured, alongside ordered dynamic-index declarations, lane occurrences and emitted witnesses. Existing Unknown vs checked negative and Missing vs Unavailable distinctions are maintained. Exact static nesting remains tested; compiler ancestry gaps remain explicit.

Selected checks: only m_06_b_1_cases; vibe-workspace --lib, vibe-spec --lib, vibe-cli --bin vibe. Original bindings retained, new cases bound by source names before dispatch. Every cargo test used --offline --locked and CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target. No copied/shared other-task target, broad test suite or discovery compilation.

Actual retry commands/results (logs and matching .exit under this report directory):

`cargo test --offline --locked -p vibe-spec --lib m_06_b_1_cases` (spec-test.log), exit 0:
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1006 filtered out; finished in 0.00s

`cargo test --offline --locked -p vibe-spec --lib m_06_b_1_cases` (spec-test-final.log), exit 0:
test compiler::boot_graph::m_06_b_1_cases::repeated_occurrences_share_one_source_without_becoming_unresolved ... ok
test compiler::boot_graph::m_06_b_1_cases::lane_traversal_preserves_all_contribution_kinds_and_repeated_node_identity ... ok
test compiler::boot_graph::m_06_b_1_cases::snapshot_reader_preserves_source_identity_and_positions ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1006 filtered out; finished in 0.00s

`cargo test --offline --locked -p vibe-workspace --lib m_06_b_1_cases` (workspace-test.log), exit 0:
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 689 filtered out; finished in 0.03s

`cargo test --offline --locked -p vibe-workspace --lib m_06_b_1_cases` (workspace-test-final.log), exit 0:
test boot_debugger::m_06_b_1_cases::repeated_sources_nested_why_positions_and_capture_identity ... ok
test boot_debugger::m_06_b_1_cases::dynamic_index_preserves_repeats_conditions_and_unknown_kinds ... ok
test boot_debugger::m_06_b_1_cases::retained_absence_and_corruption_never_create_writer_state ... ok
test boot_debugger::m_06_b_1_cases::retained_exact_snapshot_reads_while_writer_is_held_without_provider_calls ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 689 filtered out; finished in 0.03s

`cargo test --offline --locked -p vibe-cli --bin vibe m_06_b_1_cases` (cli-test.log), exit 0:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 933 filtered out; finished in 0.00s

`cargo test --offline --locked -p vibe-cli --bin vibe m_06_b_1_cases` (cli-test-final.log), exit 0:
test commands::tree::artifacts::m_06_b_1_cases::shared_dynamic_index_keeps_cli_order_and_conditions ... ok
test commands::tree::artifacts::m_06_b_1_cases::repeated_shared_sources_and_exact_nested_why_chain ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 933 filtered out; finished in 0.00s

Additional actual checks:
- cargo fmt --all -- --check: exit 1 (fmt-check.log); only out-of-scope existing formatting differences, unchanged.
- rustfmt --check --edition 2024 --config skip_children=true on all candidate Rust paths in patch-paths.json: exit 0 (scoped-fmt.log).
- git diff --check: exit 0 (diff-check.log), read-only.
- patch --dry-run --reverse -p1 < candidate.patch: exit 0 (patch-check.log), all 22 paths checked without changing them.
- Python ElementTree parse of PROP072: success.

Retained original evidence (not rerun claims):
- /tmp/vibevm-managed-m06b1/workspace-test-final.log: 3 passed, exit 0.
- /tmp/vibevm-managed-m06b1/cli-test-2.log: 1 passed, exit 0.
- /tmp/vibevm-managed-m06b1/spec-test.log: exit 101, zero cases, nine missing effects fields; cleared by this retry's 3-case final run.
- Earlier original CLI import failure was already cleared in the original final CLI log; no redundant export repair was made.

Patch isolation: candidate.patch is the combined M06B1 delta, not merely the retry delta. For the original five existing edited files it uses the original worker's saved before copies; original new M06B1 files use /dev/null. The eight fixture files use retry-start bytes so prior M05 work is excluded. One new dynamic_index.rs is included. No staging or Git baseline mutation. Exact included paths are in patch-paths.json. All source/test/PROP072 edits are within M06B1 write paths. No M05 WIP is included.

Global format differences (outside candidate):
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs:571:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:10:
Diff in /fast/git/v/vibevm-next/crates/vibe-core/src/manifest/change/m_02_b_1_cases.rs:188:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:83:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:92:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:234:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs:100:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs:2:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission.rs:48:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:34:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:184:
Diff in /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs:289:
