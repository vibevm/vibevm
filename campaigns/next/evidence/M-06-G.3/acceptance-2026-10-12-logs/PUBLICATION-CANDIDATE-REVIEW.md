# G3 publication candidate review

2026-10-12. Independent read-only source review of the working candidate on the Root-supplied base `1b9db35d9`. No builds/tests executed, source edits, or acceptance claimed. The separately queued static-link-precedence issue is not reviewed here.

**No concrete publication-policy defect found in the reviewed state. The implementation matches Root's decided conservative selected-member ruling in both publication paths, survives durable resolution, and keeps the existing selected Unit/root pipeline. Source tests cover the important ordinary-path preservation boundaries. Executed acceptance remains Root's responsibility.**

## Reviewed behavior

- `crates/vibe-workspace/src/install/bootgen/publication.rs::publish_node_boot` returns true for root `.` before inspecting role, returns true for non-package nodes, and otherwise suppresses only a selected `(kind, group, name)` match. It deliberately ignores version, content directory, hash and mutable-source flags. This is the ruled identity policy, not an inferred source-provenance guarantee.
- `bootgen.rs::regenerate_boot_from_traced_prepared` tests the predicate at the start of node iteration (`:172–175`), before `node_own_boot`, writer invocation and `nodes_regenerated`. The earlier `emit_package_units` remains active. The root still gets its dependencies and hoisted contributions.
- `bootgen/native_managed.rs::regenerate_boot_from_bound_native` tests the same predicate at `:155–158`, before node runtime lookup, managed writer invocation, native-continuation insertion, `NodeReplayCandidate::new` and returned node list. Selected unit emission and unit replay candidates are already produced before this loop and remain intact.
- `bootgen.rs::regenerate_boot_traced` now reads durable resolution before filtered redirect validation (`:294–300`). `validate_redirect_blocks` uses the same predicate (`:417–426`). Both install branches in `install/hook_output.rs` and direct native regeneration in `install/native.rs:143` pass the supplied resolution to validation. Thus malformed redirects in a protected source member do not block an operation that does not publish them; root/project/unselected publication retains validation.
- `materialised_read.rs::durable_row` changes `content_dir` to the installed slot and `source_mutable` to false but preserves kind/group/name. Those changes cannot bypass this predicate. Authored version drift likewise cannot enable writes.
- `replay_prepare.rs::seal_replay_set` selects from actual continuations and retained candidates; it does not require every epoch node to publish. The guarded native loop produces neither a protected Node continuation nor its candidate, so it cannot leave the unmatched direct-pending owner that sealing rejects. Retaining a protected Node in the semantic owner/build-plan inventory is intentional and distinct from publishing its boot artifacts. No late replay-publication skip was added.

The new `PROP-009#SELECTED-PACKAGE-SOURCE-MEMBER-PUBLICATION` at line 49 records the exact Root ruling, including identity-versus-provenance limits, unchanged root/project/virtual/unselected publication and complete owner inventory. It explicitly avoids claiming that a derived package unit is equivalent to every authored development-entrypoint instruction. The existing general install clause is correspondingly narrowed to published entrypoint nodes.

## Source proof inspected

`install/tests_hybrid/g3_cases.rs` now contains:

- `consumer_install_keeps_source_workspace_member_bytes`: snapshots all member paths/bytes, includes a deliberately malformed source-owned AGENTS block, performs install, asserts unchanged member and one `MEMBER_SOURCE_G3` in root STATIC, checks selected authored snippet still exists in the slot, and checks root INDEX/AGENTS. It then publishes the resolution lock, edits authored version from 1.0.0 to 1.0.1, reloads workspace, regenerates through the durable path, expects only root in returned nodes, and requires the edited source snapshot to remain unchanged with the selected root content still present.
- `standalone_package_root_still_publishes_dependency_boot`: package root keeps INDEX/AGENTS and its dependency body once.
- `ordinary_project_member_keeps_foundation_and_dependency_entrypoint`: project member retains INDEX foundation reference, redirect and dependency body once.
- `unselected_package_member_keeps_its_entrypoint`: package member outside the resolution still receives INDEX/AGENTS.
- `xml_and_markdown_nested_simple_multifragment_zones_exclude_generated_source_lanes`: selected source package snapshots remain unchanged and both consumer formats retain the expected four authored bodies once, excluding generated poison. This is inspected test intent, not an execution result from this review.

`install/tests_native_carriage.rs::unselected_member_and_package_unit_enter_the_carried_build_plan` actually constructs a **selected** package member, calls native regeneration successfully, asserts only root publishes, source member gets no AGENTS/INDEX, root retains `# member`, and both Node and Unit remain in the build owner plan. Its name is stale and misleading but its assertions fit the ruling. It demonstrates source-level native branch/sealing coverage, not an executed final replay publication or a full source-byte snapshot. The adjacent `install_entry_returns_the_supplied_epoch_and_all_owner_build_plan` checks pending-unit index and nonempty sealed replay. No additional runtime claim is inferred from either body.

No new unrelated testing demands follow from this review. Root should use the executor's actual bound nonzero results for these selected cases and any already-required native replay evidence. A passing source-protection case alone would not complete all of G3.

## Reviewed byte state

SHA256 observed after reads (live source may receive subsequent fixture edits):

| File under `crates/vibe-workspace/src/install/` | SHA256 |
|---|---|
| `bootgen/publication.rs` | `a6c78a87ebcd6559764ce26e934409dee3cdc73d9b1330420d02d70cca461c16` |
| `bootgen.rs` | `0895453c6548a524e106593ff1414d40fb9bc4969799fb0b58925473f7e6f8d2` |
| `bootgen/native_managed.rs` | `cd1079871e2d4e2184665136d2cb8c385eaf19fc1cb55abe64a330d850391c0a` |
| `tests_hybrid/g3_cases.rs` | `f351706b5925837ed214fd93949885bcd67df1f65c3e8ecfe867ca2ff953d137` |
| `tests_native_carriage.rs` | `06ab0f60c64c05bcc38003a7028d4d44e30a06494cd770e1a29246b171a63b94` |

These hashes identify the reviewed candidate; they are not test receipts or an acceptance signature.
