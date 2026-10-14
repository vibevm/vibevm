M-10-B.1 unreviewed candidate report

Implemented a separate epoch-one authored DerivationBinding in the manifest owner; no change to existing vibe.toml authored grammar. Exact pinned owners, canonical resource IDs, exact observed base identity, explicit SourceDescriptor, retained base/local byte inventories and the closed replace-selected-inventory/1 recipe are required. Unknown fields/epochs, absent/stale base, copied upstream identity, empty/cache-only source, duplicate/alias footprints, incompatible descriptors and ambiguous Vibe document representations refuse. Batch identity/footprint validation is explicit and has no winner selection.

SHA-256 framing is fully specified in PROP-074#AUTHORED-BINDING. Every label/value has an unsigned 64-bit big-endian length. Inventory ordering is canonical. Base/local provenance and result identities remain distinct even with identical output bytes. Source identities are observed data only, not ContentHash production proofs. Existing CapturedSourceFile, SourceInput, SourceDescriptor, SourceStrength and vibe-spec canonical_doc_path owners are reused. No VerifiedSourceMembership or source-query proof envelope is fabricated.

The generated strict binding carrier delegates semantic checks to core. The generated strict result carrier embeds the exact authored TOML including comments and both inventories; it carries all navigable base/local/provenance/result hashes. Workspace validates its supplied current observation before preparing an immutable NoExternalEffects result. The retained document has no cache dependency. Identity retention reuses M13 replay_authored_tokens; unsupported/ambiguous legacy input refuses. Since this is a new separate document, no existing authored epoch is silently upgraded.

Boundary and limitations: passive validation/serialization only. The APIs do not persist, publish, install, hydrate, execute, grant, or mutate installed source. Persistence/activation and independent production source proof remain with their existing owners. Current-base comparison is against the supplied M10A2 observed inventory, not a live or authenticated capture assertion. Authored path grammar is deliberately portable ASCII; Unicode is refused rather than implementing another alias normalizer. The recipe replaces a complete selected inventory; named-export editing and semantic compilation are not added. Zap gate admission, acceptance and task dispatch remain outside this candidate. Historical accepted M10A contracts were used only as source basis, never replayed events. No child agents, Git writes/staging/commits, dependency/lock/toolchain changes or unrelated source edits.

Read scope: named PROJECT-BOUNDARIES, task packet/READ-ROSTER, accepted PROP-074, PROP-044; manifest/package/module and source descriptor/migration owners; workspace derivatives/M10A2 harness, CapturedSourceFile membership, retained capture and descriptor map owners; install slot verification/derivation proof owner; resolver lookup canonical owner; xtask main/codegen layout/domain/strictness/format routing and wire-diff behavior; selected generated source-query and new derivative definitions. No root AGENTS or boot/stewardship traversal.

Target ownership: inspected process list before first use and after quota failures; no cargo/rustc worker was active. Used only transferred /tmp/vibevm-managed-m06b1/cargo-target sequentially. Removed failed vibe_wire-1wd5r28psw8yj incremental directory, then the five older idle vibe_wire incremental directories after a second quota failure. No target copying or active target sharing. Compilation subsequently passed.

Selection before compilation: m_10_b_1_cases bounded library families selected by exact source names; first core selection 5, final core 6, workspace initially 4 and final 5 before its first compile, wire 4. Test-name inventory and actual output follow. No full suite, Lean or cross-platform run.

Actual commands / exits / counts:

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo xtask codegen --help
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/codegen-help.log
  Result: Entrypoint help; this initial help invocation did not use offline/locked flags. No manifest, lockfile or toolchain changes.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo run --offline --locked --quiet -p xtask -- codegen
  Exit: 101
  Log: /tmp/vibevm-managed-m10b1/codegen.log
  Result: Bootstrap compile failed: workspace registration referenced the not-yet-generated result. Registration deferred until generation.

- Command: same existing codegen command
  Exit: 1
  Log: /tmp/vibevm-managed-m10b1/codegen-2.log
  Result: Generator refused missing x-default on optional descriptor fields. Corrected schema metadata; removed only failed generated scratch tree.

- Command: same existing codegen command
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/codegen-3.log
  Result: Successful generation; new schema directory caused unrelated generated header changes.

- Command: same existing codegen command
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/codegen-4.log
  Result: Relocated new schemas to existing root schema home and regenerated. Unrelated header churn removed by owner tooling.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo test --offline --locked -p vibe-core --lib m_10_b_1_cases
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/core-test.log
  Result: 5 passed, 0 failed before final framing and collision refinements.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo test --offline --locked -p vibe-wire --lib m_10_b_1_cases
  Exit: 101
  Log: /tmp/vibevm-managed-m10b1/wire-test.log
  Result: 0 tests executed; compilation failed with disk quota exceeded writing wire incremental query cache.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo test --offline --locked -p vibe-workspace --lib m_10_b_1_cases
  Exit: 101
  Log: /tmp/vibevm-managed-m10b1/workspace-test.log
  Result: 0 tests executed; wire dependency compilation failed with disk quota exceeded writing rmeta.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo test --offline --locked -p vibe-workspace --lib m_10_b_1_cases
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/workspace-test-2.log
  Result: 5 passed, 0 failed, 699 filtered out.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo test --offline --locked -p vibe-wire --lib m_10_b_1_cases
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/wire-test-2.log
  Result: 4 passed, 0 failed, 250 filtered out.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo test --offline --locked -p vibe-core --lib m_10_b_1_cases
  Exit: 0
  Log: /tmp/vibevm-managed-m10b1/core-test-2.log
  Result: 6 passed, 0 failed, 619 filtered out; final core source.

- Command: cargo fmt --all -- --check
  Exit: 1
  Log: /tmp/vibevm-managed-m10b1/fmt-all.log
  Result: Pre-existing unrelated formatting differences; no cleanup performed.

- Command: CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target cargo run --offline --locked --quiet -p xtask -- check-codegen
  Exit: 1
  Log: /tmp/vibevm-managed-m10b1/check-codegen.log
  Result: Generation succeeded; git diff --exit-code against committed HEAD reports the intentional uncommitted generated registration changes. Five affected generated outputs are byte-identical before/after this run.

Additional actual checks:
- rustfmt --edition 2024 --config skip_children=true on only the eight affected handwritten Rust files: exit 0 (format operation).
- rustfmt --check --edition 2024 --config skip_children=true on those same files: exit 0.
- Python ElementTree.parse on PROP-074-local-derivatives.xml: exit 0, "PROP-074 XML parses".
- git diff --check: exit 0, repeated after final generation.
- SHA-256 comparison of generated root module, format_id/mod.rs, format_id/properties.rs, derivative_binding/mod.rs and derivative_result/mod.rs before/after check-codegen: exit 0; 5/5 byte-identical. This distinguishes repeatability from the committed-baseline check's nonzero exit.
- Patch perimeter assertion and whitespace check for all new files: exit 0.

Conditional checks: check-codegen was run because the actual schema and generated registration changed and there is no selective built-in check-codegen command. wire-diff was not run: its projection rebuild/epoch global oracle is unnecessary for this new authored-golden family; the narrower generated-reader/semantic corpus tests cover the actual changed formats, and existing EPOCHS.toml is pre-publication. Whole-project check and facts exhaustive were not run: no dependency, boot, fact/adoption/extraction or composition declarations changed. PROP-074 added descriptive text without fact or closure assertions, checked as XML.

Global formatting differences (all pre-existing, including the generated MachineProfiledDeployActionRequest arm verified against HEAD; left untouched):
/fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs
/fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs
/fast/git/v/vibevm-next/crates/vibe-core/src/manifest/change/m_02_b_1_cases.rs
/fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs
/fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs
/fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs
/fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission.rs
/fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs
/fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs

Final actual output core-test-2.log:

running 6 tests
test manifest::derivation::m_10_b_1_cases::interpretation_shape_alias_and_cache_only_refuse ... ok
test manifest::derivation::m_10_b_1_cases::hash_framing_and_inventory_order_are_explicit ... ok
test manifest::derivation::m_10_b_1_cases::duplicate_upstream_missing_and_stale_refuse ... ok
test manifest::derivation::m_10_b_1_cases::explicit_tree_xml_vibe_and_raw_interpretations ... ok
test manifest::derivation::m_10_b_1_cases::equal_bytes_keep_owner_and_base_provenance ... ok
test manifest::derivation::m_10_b_1_cases::retained_round_trip_and_comments ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 619 filtered out; finished in 0.00s



Final actual output workspace-test-2.log:

running 5 tests
test derivatives::binding::m_10_b_1_cases::cache_only_and_ambiguous_interpretation_refuse ... ok
test derivatives::binding::m_10_b_1_cases::canonical_vibe_representation_collision_refuses ... ok
test derivatives::binding::m_10_b_1_cases::identical_output_separate_owners_and_bases ... ok
test derivatives::binding::m_10_b_1_cases::retained_chain_survives_deleted_cache_and_round_trips ... ok
test derivatives::binding::m_10_b_1_cases::missing_stale_base_and_forged_result_refuse ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 699 filtered out; finished in 0.00s



Final actual output wire-test-2.log:

running 4 tests
test behaviour::derivative::m_10_b_1_cases::strict_carriers_refuse_unknown_fields_and_proof_claims ... ok
test behaviour::derivative::m_10_b_1_cases::result_hash_cannot_be_replaced_with_equal_byte_identity ... ok
test behaviour::derivative::m_10_b_1_cases::ambiguous_descriptor_missing_source_and_stale_capture_refuse ... ok
test behaviour::derivative::m_10_b_1_cases::authored_and_generated_corpus_round_trip ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 0.00s



Patch files (tracked and new; no staging):
crates/vibe-core/src/manifest/mod.rs
crates/vibe-wire/src/behaviour/mod.rs
crates/vibe-wire/src/generated/format_id/mod.rs
crates/vibe-wire/src/generated/format_id/properties.rs
crates/vibe-wire/src/generated/mod.rs
crates/vibe-workspace/src/derivatives.rs
formats/REGISTRY.toml
vibevm/vibespecs/common/PROP-074-local-derivatives.xml
crates/vibe-core/src/manifest/derivation.rs
crates/vibe-core/src/manifest/derivation/m_10_b_1_cases.rs
crates/vibe-wire/src/behaviour/derivative.rs
crates/vibe-wire/src/generated/derivative_binding/mod.rs
crates/vibe-wire/src/generated/derivative_result/mod.rs
crates/vibe-workspace/src/derivatives/binding.rs
crates/vibe-workspace/src/derivatives/binding/m_10_b_1_cases.rs
formats/corpora/derivatives/e1/binding.json
formats/corpora/derivatives/e1/binding.toml
formats/corpora/derivatives/e1/result.json
schemas/derivative_binding.jtd.json
schemas/derivative_result.jtd.json

Patch bytes: 54920
Patch SHA-256: 85d7b366a1ac206c0862e4af49472499691e29fe31e1777d380ca6cd07ccd36f

Criterion dispositions in capture order: strict authored/generated schema and corpus coverage satisfied by the scoped validation/corpus tests; navigable stored base→local→result chain satisfied by round-trip/cache-deletion and provenance tests. These are candidate dispositions, not acceptance. The unrelated global formatting failures and baseline-relative check-codegen exit are explicitly retained for Root review.
