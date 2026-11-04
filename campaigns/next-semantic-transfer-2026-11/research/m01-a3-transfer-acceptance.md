# M-01-A.3 — exact internal-caller transfer review

Main `a344c6389fdb60a8db075f60b8a7c89712572916`; original task at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`, original accepted change `f6b2fb27a`. READ ONLY; no tests/builds/product edits. This is a recommendation for root, not root acceptance.

**Verdict:** semantic equivalence is supported at the original internal-caller boundary. Recommend **full**, subject to root judging the explicitly stated compositional evidence below. The prior crosswalk’s generic “whole source service open” objection was incorrect for this task and must be removed.

The original acceptance is (1) “Every inventoried caller has a result/refusal fixture”; (2) “No alternate lookup path bypasses containment.” Its safe stop explicitly says “Internal callers complete; public exposure waits M-01-D.” The original accepted report/commit identifies point/pattern resolver, FsSectionSource/recursive embed, compiler frontend AdmittedSource and selected provider prompt. No public query service, generated wire contract or independent source authentication is required here.

## Caller-to-evidence matrix

### FileResolver point/pattern membership (shared lower lookup boundary)

FileResolver.resolve_file → resolve_source_file/locate_source; expand_pattern → held InstalledSlots/selected-world point lookup → member.

Result fixtures: `source_selection_probe::control_absence_unmatched_names_and_empty_world_remain_empty`; `source_selection_probe::invariant_selected_world_drives_membership_instead_of_unselected_disk_slots`; `source_selection_probe::explicit_pattern_pin_is_honored_and_missing_pin_does_not_fall_back`.

Refusal fixtures: `source_selection_probe::invariant_matched_document_failures_do_not_disappear_from_pattern`; `source_selection_probe::matching_non_directory_slot_refuses_as_a_source_observation`; `source_selection_probe::matching_symlink_slots_refuse_while_other_names_and_groups_are_not_opened`; `source_selection::edges::matching_version_and_document_parent_symlinks_are_errors_not_absence`.

Covered. Actual member only excludes DocNotFound/PackageSlotNotFound; SourceObservation/AmbiguousDoc propagate. Current fixed-group semantics intentionally supersede old cross-group collision rule.

### FsSectionSource.section_text and recursive expand_embeds

section_text → locate_source → read_location → held bytes → project_bytes/resolve_unique_path; expand_rec preserves reason in EmbedError::Unresolved.

Result fixtures: `source_selection_probe::control_unique_xml_projection_remains_readable`; `source_selection_probe::control_markdown_paths_and_characterize_xml_global_id_uniqueness`; `embed::source_reads::explicit_root_alias_and_shared_regular_file_remain_readable`.

Refusal fixtures: `source_selection_probe::invariant_requested_duplicate_anchor_refuses_in_section_and_embed`; `embed::source_reads::redirected_document_and_parent_refuse_actual_embed_content`; `embed::source_reads::redirected_layout_and_derived_package_slot_are_not_root_aliases`.

Covered for inventoried incumbent source/embed boundary. Descriptor protocol is not required; every actual current source read converges on held locator/reader.

### Compiler frontend source_metadata/read_resolved_source (including active custom formats)

AdmittedSource.load preserves metadata Err as Ok(Err(reason)); custom admission precedes read; FsSectionSource.read_resolved_source → read_resolved → locator consistency → same read_location.

Result fixtures: `frontend_discovery_tests::only_effective_activated_frontend_formats_resolve_or_read_txt`; `embed::source_reads::custom_metadata_is_content_free_and_the_admitted_read_keeps_raw_bytes`.

Refusal fixtures: `frontend_discovery_tests::production_selects_txt_then_returns_typed_frontend_deferred`; `frontend_execution_tests::unselected_custom_format_refuses_before_alternate_source_read`; `embed::source_reads::supplied_metadata_cannot_substitute_an_outside_document`; `embed::source_reads::metadata_drift_refuses_without_reading_a_substituted_id_match`; `embed::source_reads::same_byte_replacement_is_refused_by_builtin_and_custom_consumers`; `compiler::artifact_tests::repair::embed_failure_precedence_follows_interleaved_plan_roots`.

Covered by actual compiler boundary fixtures plus shared source reader cases. No direct new complete compiler×every filesystem-error matrix claimed; exact failure-forwarding body reviewed.

### SelectedWorldPromptResolver::resolve (provider self and selected cross-package embed)

construct FsSectionSource with exact provider root and selected world; section_text? then expand_embeds with exact reason conversion; no fallback read.

Result fixtures: `agent::tests::resolver::a_self_prompt_resolves_from_the_executing_provider_instance`; `agent::tests::resolver::a_cross_package_embed_resolves_only_the_lock_selected_version`; `agent::tests::resolver::actual_prompt_refuses_ambiguous_requested_anchor_but_keeps_qualified_leaf_paths`.

Refusal fixtures: `agent::tests::resolver::blank_and_missing_prompts_refuse`; `agent::tests::resolver::actual_prompt_embed_refuses_an_ambiguous_selected_dependency_anchor`; `agent::tests::resolver::actual_prompt_refuses_same_byte_replacement_during_source_read`; `agent::tests::resolver::an_embed_into_an_unselected_coordinate_refuses`.

Covered by real result/refusal/race fixtures and direct same-reader propagation. No dedicated prompt filesystem-unreadable-not-missing fixture found in current resolver test module; this specific cell is compositionally proven from shared SourceObservation path and direct ? propagation, not freshly executed at prompt level.

## Exact error and containment reasoning

`SourceLocation::read` maps read errors and post-selection absence to SourceObservation, with disappearance described as “selected source disappeared before reading”; it does not turn that into DocNotFound. `glob::member` skips only true DocNotFound/PackageSlotNotFound and propagates other errors. Held directory opening/metadata faults become SourceObservation. `FsSectionSource` obtains content through `read_location`; supplied metadata is revalidated and cannot substitute an outside path. `read_location` projects/parses the held bytes or exact overlay. Requested anchors use resolve_unique_path. Recursive expand preserves the reason inside EmbedError::Unresolved. Compiler load returns source errors without fallback; prompt uses section_text? and expand_embeds with precise text. Thus outer failures are not guessed absence, and no alternate current read route was found among the inventoried callers.

The old and new public source/prompt APIs both return textual causes; retaining different Rust variant names is not required. Current SourceObservation retains precise cause, while true absence remains distinct. Existing direct compiler and prompt valid/missing/selected cases predate this campaign: JSON verifies their symbol presence at baseline6d2f6e59. New accepted F03/F05 fixtures add held reads, ambiguity and races.

## Intentional supersession

- fixed group plus actual selected world; foreign groups neither select nor sabotage membership. Reference: F05 root design f6856d522 and implementation df040becb; pattern_group_is_fixed_and_other_groups_cannot_control_membership.
- requested full path uniqueness; distinct qualified leaves/whole documents retain legitimate reads. Reference: F05 resolve_unique_path + qualified-leaf tests.
- held no-follow observer, typed SourceObservation with precise reason, unchanged textual caller interface. Reference: F03 accepted6dafbac72; SourceLocation.read and failure; old root acceptance explicitly preserves textual public reasons.
- current incumbent filesystem and overlay paths share held reader; future descriptor service not A3 acceptance dependency. Reference: task safe_stop Internal callers complete; public exposure waits M-01-D.

Portable sibling aliases belong to A2 and remain outside this A3 verdict; no borrowed gap from a neighboring task is used to deny A3. Conversely this verdict does not close A2.

## Evidence limits requiring explicit root judgment

No dedicated structural unreadable-not-missing fixture was found in the current prompt resolver test module. That cell is supported by the actual shared refusal fixture plus the prompt’s direct `?` propagation; it is not claimed as a freshly run prompt-specific IO test. Similarly, compiler-facing read refusal/race fixtures and actual compiler admission/replay tests are composed with the thin frontend body; every filesystem failure was not injected through the complete compiler pipeline.

These are test-specificity limits, not demonstrated missing semantics. The two literal acceptance bullets have coverage, including positive and negative fixtures per caller. If root requires all four outcome categories as separate outer-call tests, retain only those specific evidence cells pending; do not reintroduce the unrelated source-query service or old type names. The original acceptance itself disclosed no end-to-end compiler/prompt race; current main adds an actual prompt replacement fixture.

Existing accepted evidence: F05 main17source+14prompt tests and scoped gates; F03 accepted held-byte tests with explicit platform limits; F01 root compiler frontend/no-alternate-read controls. This review read the actual tests/callees and records their hashes but ran none. Final main hashes, exact original task, original accepted report, per-caller mapping and baseline symbol-presence checks are in the sibling JSON.

Root owns whether this supports advancing the old-task complete lower bound from0to1. This report does not edit the113crosswalk or acceptance ledger.


## Root acceptance

Accepted as full semantic transfer of M-01-A.3 at its original internal-caller boundary. Root verified all25input hashes, confirmed reviewed product files unchanged at current main, and re-read SourceLocation.read, pattern membership, FsSectionSource held/overlay reads, compiler AdmittedSource and selected-provider prompt forwarding. Both literal acceptance clauses are met by the actual per-caller positive/refusal fixtures and shared lower-boundary evidence. The explicitly stated compositional test limits are accepted for these direct forwarding paths; no new test execution or complete public M-01-D service is inferred. Approved fixed-group/unique-anchor/shared-inode alternatives remain valid. This advances the proven old113-task completion lower bound to1; it does not certify the unreviewed remainder.
