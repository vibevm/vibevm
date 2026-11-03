# Independently useful first-transfer candidates (read-only evidence)

Comparison remains pinned: next `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`, main `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`. No source edits, builds or tests. These are NEED/DESIGN candidates, not approval, implementation or a new campaign order. Global archaeological debt can continue independently; none of these features requires completing every old milestone first.

Terminology correction: the earlier inventory contains **61 task records**, extracted from the nested `tasks` arrays in M-01–M-05 task JSON files. Earlier wording “61 leaf tasks” should be read as “61 task records”; it was not a claim that every current Zap decomposition leaf had been inventoried. Counts remain 39 implemented, 5 partial, 17 planned-only under the source-assessment rubric.

## Candidate 1 — Make existing embed/prompt source reads fail closed, preserving custom frontends

**NEED.** A user builds boot/compiler output or resolves an agent prompt from a `spec://` reference. Today main can silently omit a broken pattern member, select one duplicate package-name slot, or follow an unsafe filesystem path while reading the source. The useful outcome is reliable content selection and a diagnostic that explains absence versus ambiguity/unreadability/instability, on already-used source paths. This is independently valuable even if no public source-query command or new resource-address grammar is ever added.

**Concrete before/after scenarios.**

1. A pattern `spec://org.demo/plugin-*/common/PROP-042` matches a package containing both `PROP-042-one.md` and `PROP-042-two.md`. Main's `glob.rs:129` calls `resolve_doc(...).is_ok()` and drops the ambiguous member. Desired behavior: fail the expansion with AmbiguousDoc; do not produce a deceptively complete build without that member.
2. `org.one.plugin-a` and `org.two.plugin-a` both match a group-blind pattern. Main sorts then `dedup_by` at `glob.rs:118`, keeping the first physical slot. Desired behavior: explicit AmbiguousPatternMember; never choose bytes by slot ordering.
3. The selected document path contains a symlinked child to an outside source, a case/NFC alias, or an unreadable/non-directory source component. Main's plain `is_file`, `read_dir_or_empty` and subsequent path-based read can follow/suppress these conditions. Desired behavior: shared portable validation and no-follow held read, refusal before returning outside or ambiguous content.
4. A document contains two `{#same}` anchors and an embed selects `#same`. Main `embed.rs:331` directly calls `DocTree::resolve_path`; next checks `duplicate_anchors` before selection. Desired behavior: ambiguous-anchor diagnostic, while a unique anchor returns exactly the existing projection.
5. An admitted custom frontend owns `.txt`. Its exact and PROP-ID-prefix lookup must still work, its raw bytes must reach the selected frontend unchanged, and inactive `.txt` must remain unread. A safe read must not accidentally parse every custom file through Markdown/XML projection.

**Exact main gap and reusable main owners.**

- `crates/vibe-spec/src/resolver/lookup.rs:304`, `read_dir_or_empty`, flattens both directory-open and entry errors to empty.
- `lookup.rs:36` joins raw path parents; `:76` uses `is_file`, which is not a no-follow/identity boundary.
- `crates/vibe-spec/src/resolver/glob.rs:89`, `:118`, `:129` respectively use swallowed enumeration, first-slot deduplication, and suppression of all failed lookups.
- `crates/vibe-spec/src/embed.rs:234–265` uses `load_spec_text` for builtin projection and `std::fs::read` for custom bytes; `section_text` repeats the unheld builtin read.
- `crates/vibe-safefs/src/component.rs` already exposes `split_relative`, `identity_key`, `judge_selection`, `ensure_no_follow_walk`; no new component authority is necessary.
- `crates/vibe-safefs/src/file/bounded.rs:73` already exposes `Project::read_file_snapshot_bounded`: two passes over one held no-follow object, single-link validation, content digest and final-name identity check. Do not import next's later generalized capture/hardlink APIs merely for this feature.
- `vibe-spec` currently lacks a direct `vibe-safefs` dependency; that is the one clear production crate dependency needed for this integration. Its parser, DocTree, projection functions and consumer seams already exist.

**Prototype evidence.**

- `crates/vibe-spec/src/resolver/portable.rs:split/directory/entries/exact_entry` and `lookup.rs:resolve_doc_with_formats` route physical lookup through the shared authority.
- `resolver/read.rs:FileResolver::read_observed_source` calls the bounded SafeFS snapshot, and `read_observed_projected_source` projects the same bytes. Custom readers use `read_preflighted_source` rather than builtin projection.
- `resolver/glob.rs:expand_pattern` propagates all errors except genuine DocNotFound and refuses duplicate matched package names.
- `embed/selection.rs:resolve_section` refuses duplicate referenced anchors.
- `resolver/m_01_a_3_cases.rs`: `ambiguous_glob_document_refuses_instead_of_disappearing`, `pattern_member_collision_refuses_instead_of_choosing_a_slot`, `unreadable_pattern_slot_does_not_disappear_from_set`, `duplicate_anchor_does_not_return_the_first_section`, `absent_document_remains_not_found`.
- `resolver/m_01_a_2_cases.rs`: `portable_refusals_precede_lookup_of_hostile_components`, `directory_file_case_and_nfc_aliases_refuse_instead_of_opening`, `symlinked_child_refuses_before_observing_outside_target`, `nested_portable_document_keeps_its_exact_logical_name`.
- `resolver/m_01_c_1_cases.rs`: `same_byte_replacement_during_actual_embed_read_refuses` exercises the actual consumer and an existing race hook.
- `crates/vibe-lifecycle/src/agent/resolver.rs`: `provider_prompt_unreadable_is_not_reported_missing`, `provider_prompt_duplicate_anchor_never_returns_first_body`.

**DESIGN seam and minimum necessary dependencies.** Keep `SpecAddress`, `FileResolver`, `SectionSource` and `FsSectionSource` as the callers' interfaces. Add typed lookup/read refusals and a small internal physical-read helper, use existing SafeFS, project builtin bytes in memory, and preserve custom frontend admission before opening custom bytes. Keep overlays as explicit caller-supplied bytes; a disk-read policy should not manufacture a disk read for an overlay. Adapt next's behavior and tests, not whole next files: its same files also contain SourceDescriptor/ResourceAddress/query changes that are unnecessary here.

Plausible target changes: `vibe-spec/Cargo.toml`, `resolver.rs`, `resolver/lookup.rs`, `resolver/glob.rs`, internal resolver physical/read helpers, `embed.rs`, their tests; touch the lifecycle prompt caller only if shared refusal propagation needs it. Add lifecycle regression tests at its existing resolver test seam. No machine operation host, generated source-query schemas, workspace world captures, proof API, new CLI or source classification migration is required.

**Compatibility decisions that must be explicit.**

- Rejecting duplicate pattern members is intentionally different from main's documented lexicographically first survivor. Keep valid group-blind matching and deterministic order unless separately redesigning that language.
- Preserve legal empty pattern sets and omit only genuinely absent documents; unreadable/colliding members must not become absence.
- Preserve active custom formats, scoped canonical extension stripping, physical stems, builtin/custom collision diagnostics, and frontend admission-before-read. Both main and next pattern APIs currently take no active-format list and call builtin `resolve_doc`; this candidate must not claim to add custom-format pattern support accidentally.
- Rejecting symlink/hardlink aliases changes previously readable layouts. Identify supported in-place package roots versus unsafe links below an admitted root before defining rejection breadth. Do not derive the compatibility rule from next's implementation alone.
- Next's SOURCE_CAP is 64 MiB. A bounded read needs a deliberate limit/reporting contract; copying this number without reviewing main's supported source sizes would be an arbitrary behavior change.
- A finite held-read window does not prevent later mutation or hostile ABA. Do not advertise a stronger guarantee.

**Selected test targets (not run).** The primary target is `cargo test -p vibe-spec --lib`; bind narrow cases rather than all crates. Existing main regressions to preserve:

- `resolver::tests::active_custom_format_resolves_exact_and_id_prefix_without_global_registration`
- `resolver::tests::active_custom_form_joins_same_stem_collision_with_builtin_forms`
- `resolver::tests::custom_canonical_stripping_is_scoped_and_legacy_spellings_are_exact`
- `resolver::glob::tests::g1_two_matches_in_name_order`, `g4_match_without_doc_is_not_a_member`, `g5_empty_set_is_legal`, `g6_freshest_version_selected`, `g_non_pattern_expands_to_self`
- the `frontend_discovery_tests::worklist_reaches_a_raw_sentinel_and_inactive_formats_read_nothing` and `frontend_execution_tests::unselected_custom_format_refuses_before_alternate_source_read` families in the same lib target (resolve full module names from source or a bounded test inventory before exact selection).

New/adapted main regressions are the prototype refusal/held-read cases above, plus an actual FsSectionSource custom-format read with hostile path and an admitted valid `.txt` raw-byte control. Secondary target: `cargo test -p vibe-lifecycle --lib` with the two prompt resolver cases. If SafeFS implementation is unchanged, its entire test suite need not become a prerequisite; exercise its actual boundary through the changed consumer. Unix symlink tests establish Unix behavior only; portable cases and native Windows cases remain distinct.

## Candidate 2 — Preserve every workspace consumer constraint during install planning

**NEED.** A workspace with multiple consumers must install one dependency version satisfying every consumer, or refuse before mutation. The current planner can discard a member's constraint before an already-capable solver sees it. This is a correctness property of existing installation, independent of new choice syntax.

**Concrete before/after.** Root project requires `org.demo/shared@^1`, member requires `org.demo/shared@=2.0.0`. Main deduplicates by `(group,name)` and passes only the first requirement, so the solve can succeed for a version that violates the member. Desired behavior: both constraints reach solve and conflict, leaving manifest/lock/slots/boot unchanged. Compatible requests such as `>=1,<3` and `=2.0.0` must retain both and select 2.0.0. For an explicit install from one selected node, replacing that node's request must not replace another member's independent requirement.

**Exact main gap.** `crates/vibe-install/src/plan.rs:220–241` builds a coordinate-deduplicated workspace root vector; `:253–270` similarly drops requirements for named coordinates during explicit install. This is not a solver limitation: main `resolvo_engine/mod.rs:136–147` pushes every version requirement and deduplicates output ordering only; main `naive.rs:69–81` explicitly supports duplicate root keys and enqueues/checks each; `sat.rs:203` delegates each attempt to that path.

**Prototype evidence and caveat.** `vibe-install/src/plan.rs:effective_root_constraints` keeps roots separate from pin preferences and records consumer/declaration provenance; `resolvo_engine/request.rs:NormalizedRequest::all_roots` retains route constraints. Concrete positive tests include `vibe-install/tests/m_05_g_1_cases.rs:workspace_root_union_and_selected_providers`, `conflicting_workspace_roots_keep_both_origins`, and `vibe-resolver/src/resolvo_engine/m_05_c_3_cases.rs:duplicate_root_constraints_are_not_first_wins`.

Important: next `vibe-install/tests/m_05_a_1_cases.rs:root_union` is a historical characterization that STILL asserts loss of the second constraint on its legacy RecordingSource route. It is evidence of a defect, not a correctness oracle to copy. This strengthens the case for fixing main's actual owner directly and testing all supported source routes, rather than importing normalized configured machinery and declaring universal parity.

**DESIGN seam.** Main's effective-root collection should preserve a multiset/list of incoming requirements with owner provenance; deduplication is valid for display/output identities, not input constraints. Distinguish selected-node explicit replacement from other-node retention and preserve unrelated pin preferences. Fit this into existing `InstallSource::solve` and planner data; it does not inherently need consumer-configuration schemas, candidate choices, an ETO host or a new transaction catalog. Provenance can be a small local structure if existing diagnostics cannot identify both consumers.

Plausible target changes/tests: `vibe-install/src/plan.rs` and planner integration tests; only change `vibe-resolver` or source adapters if tests reveal route-specific deduplication. Test ordinary manifest install, explicit selected-node install, compatible duplicate roots, incompatible duplicate roots, order independence, and provider/lock immutability on conflict. Use a recording source test to prove both requirements reach the boundary, then a real local registry/source fixture with actual solver to prove the selected graph/refusal. Target `cargo test -p vibe-install --test <new focused integration target>` and exact relevant `vibe-resolver --lib` backend cases. Include path/git/workspace placeholder routes so a registry-only fix is not mistaken for all-root correctness.

**Compatibility risk.** Workspaces that previously succeeded by ignoring constraints will now fail. This is the intended correction, but explicit-install replacement semantics and pin holding must be preserved. There is no need to change feature algebra/default policy in the same feature. A related Fresh-request-loss defect exists, but it is a separate user behavior and should not silently enlarge this candidate.

## Candidate 3 — Report failed install-time source conversion and offer strict preflight

**NEED.** A user chooses XML spec materialization and installs a package whose selected Markdown spec cannot convert. Today main can copy the Markdown unchanged and report it as an ordinary copied file. The useful outcome is a durable explanation of what failed while preserving the current default availability, with an explicit strict mode that refuses before publishing any package's install state.

**Concrete before/after.** A two-package install has a valid first spec and malformed second spec. Default after-state: install remains available, preserves failed source bytes, emits a path/class/line diagnostic and records `copied-unconverted` distinctly from intentional copies. Strict after-state: failure is discovered for the complete selected set before either slot/manifest/lock/boot is committed. A normal README that was never a conversion subject remains a plain copy without a warning.

**Exact main gap.** `vibe-workspace/src/vibedeps/derived.rs:transform_file` returns `Ok(None)` on text read error at 437 or parse failure at 452 unless an overlay applies. `materialise_with_spec_format_report` maps None to `SlotFileDisposition::Copied` at 186–190, erasing the distinction. Main already has the converter/pivot, slot record, PreparedSlotFile, materialization report and install application owners; these are the right owners to extend.

**Prototype evidence.**

- `vibe-workspace/src/vibedeps/derived/transform.rs:DeclaredTransform::Failed` and typed conversion issue mapping.
- `vibe-workspace/src/vibedeps/slot_record.rs:SlotFileDisposition::CopiedUnconverted`, `ConversionFailure`, `ConversionIssue`.
- `vibe-install/src/apply/strict_conversion.rs:prepare_strict_conversion` retains all prepared converted bytes before publication and explicitly refuses strict transformed in-place sources it cannot prepare.
- `vibe-install/tests/m_01_c_4_cases.rs:strict_bad_second_package_preserves_manifest_lock_and_both_slots`, `default_failed_conversion_records_issue_and_applied_counts`.
- `vibe-workspace/src/vibedeps/derived/m_01_c_4_cases.rs:legacy_requested_markdown_failure_is_recorded_ordinary_other_path_is_copied`, `applied_slot_counts_each_conversion_once_and_skipped_repeat_counts_zero`.
- `vibe-cli/src/commands/install/closure_diff.rs` carries applied conversion counts/failures; `vibe-wire/src/m_01_c_4_cases.rs:schema_two_decodes_copied_unconverted_issues_and_refuses_unknown_class` checks record schema handling.

**DESIGN seam.** Separate conversion outcomes in the existing converter/materializer, persist them in the existing slot record format with its compatibility reader, expose diagnostics via existing install reports, and preflight the full selected set for explicit strict mode. Required dependencies are converter diagnostics, materialization, record/schema compatibility, and install/CLI plumbing. No source-query service, original membership proof, change domain or machine operation host is necessary.

Classification needs a deliberate boundary: next's broader descriptor model is not automatically required. Main's existing legacy `is_spec_document` selection is evidence of the current contract, and the next test explicitly exercises legacy requested Markdown conversion separately. The first design must define exactly which existing selected source files were requested for conversion and not label arbitrary package Markdown as broken specs. If that distinction cannot be represented under main's current selection law, admit a focused selection change explicitly rather than dragging in all M-06 source classification.

Plausible targets: `vibe-workspace` derived conversion, slot record and report owners; `vibe-install` prepare/apply boundary; CLI install arguments/output; registered slot schema and compatibility tests. Test `vibe-workspace --lib` conversion outcomes and old/new record read/write; `vibe-install --test <focused conversion integration target>` for default success versus strict no-publication; a focused CLI parsing/report test. Bind exact names once new tests are authored. Keep ordinary copies, mixed format, overlays, skipped-existing slots, and in-place handling in the compatibility matrix.

**Risks and uncertainty.** Schema changes need explicit legacy-read compatibility; default warnings must not become an implicit install failure. Strict preflight must own the exact prepared bytes later published, not rerun a mutable converter after checking. Supported in-place behavior must be chosen openly. No runtime proof has been collected in this investigation.

## Decision support

Candidate 1 fits the root's proposed first feature and has a concrete consumer already shipping in main. Its necessary SafeFS implementation is already present; the work is integrating it without changing valid frontend semantics. Candidate 2 has equally strong correctness value and a narrow dependency boundary despite spanning all source routes. Candidate 3 is a useful diagnostic/strictness feature but requires a record compatibility decision and conversion-subject definition before coding.

A rolling NEED → DESIGN → LAND flow can choose one now while keeping the complete inventory/audit debt visible. These recommendations depend on user-facing correctness and existing consumers, not on code volume or the historical milestone order.
