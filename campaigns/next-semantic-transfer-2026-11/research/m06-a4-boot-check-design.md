# A4 bound boot-graph check — bounded decision

Read-only, 2026-11-07. No implementation/tests/Cargo/Git mutation. Root authorizes coding after own-boot acceptance.

## Problem and existing owners

`verify_boot_graph` reads the durable resolution, calls prepare_with_nodes with no materialized format, discards that prepared context, builds the legacy unit table, and frames owner plans with the Mixed-default plan_digest_frames. Declared installed input consequently refuses, and transformed output paths/format-sensitive plan frames cannot be checked correctly by this route.

CLI append_boot_graph_findings currently ignores BOTH Workspace::load and verifier errors. Thus a declared installed graph can contribute no finding despite never being verified. This is a source-established gap, not a runtime failure reproduced in this audit.

Reuse existing owners: read_durable_resolution provides lock-selected identity/hash and rejects malformed/nonregular locks/missing named slots; prepare_with_nodes provides installed bindings and retained authored-node context; build_unit_table_bound maps original snippets to recorded outputs; plan_digest_frames_for includes the selected target; verify_fingerprints owns INDEX fingerprint/native-pending semantics and stale propagation. Do not create another slot parser, freshness algorithm or artifact authentication scheme.

## Proposed implementation

Add public `verify_boot_graph_with_spec_format(&Workspace, SpecFormat) -> Result<Vec<UnitId>, WorkspaceError>` and export it next to verify_boot_graph. Use a private shared implementation if that avoids duplicated world/versions/runtimes setup. Keep the old unbound wrapper's absent-format declared refusal and existing Mixed legacy frame contract; do not silently guess declared format in it.

Bound branch retains the prepared sources, builds build_unit_table_bound with those sources, lowers the existing owner runtime set and calls verify_fingerprints with plan_digest_frames_for(runtimes, supplied format). Preserve deterministic stale UnitId ordering and existing native-pending checks. No mutation, provider invocation, lifecycle lease or compilation is required. Do not call generated_inputs::admit: this operation diagnoses stale/missing INDEX, rather than requiring generated input freshness to consume a STATIC stream; admission would turn the intended stale-list result into an unrelated read refusal.

Materialized format is selected at the actual CLI boundary by existing reinstall::effective_rederive_format(&ws.root_manifest), which loads the operator default and reuses install::resolve_spec_format. It remains current selected intent, not historical installation proof. Mismatching retained slot representation refuses truthfully; no new lock field, CLI option or suffix inference. Output-target plan frames must use the same selected format as the bound graph generation path.

## Applicability and findings

Treat absent durable lock as no installed graph: skip this adapter without manufacturing warnings or loading irrelevant user configuration. A valid empty durable package epoch has no units and produces no stale findings. Existing project-file checker remains responsible for independent authored/project issues.

Use symlink_metadata/typed durable reader semantics to distinguish genuinely absent lock from present nonregular/unreadable/malformed input; never use exists() to silently erase read failures. Within an applicable installed workspace, workspace load, selected-format configuration, slot binding or verifier errors must produce a visible finding rather than `return`. Proposed mapping: existing CheckId::BootGraphIntegrity + Severity::Error with a bounded actionable message that verification could not complete. Stale unit ids retain existing Severity::Warning and reinstall advice. No new CheckId/schema is necessary.

This preserves existing run/emit laws: errors make JSON ok=false and exit1 after report emission; warnings alone remain exit0 with visible warnings. Avoid early anyhow propagation that bypasses the report. Root should approve the new error severity as the truthful unavailable-verification outcome. No claim that every missing materialization is merely stale: a missing named source slot is an error, while a missing/invalid unit INDEX returned by verify_fingerprints is an existing stale-unit warning.

`check --path` presently requires a directory containing vibe.toml and loads that root; do not introduce new member-discovery semantics in this slice. If member selection needs redesign, record separately. Duplicate malformed-manifest diagnostics may be avoided only through explicit applicability, not by swallowing an installed graph failure.

## Concrete tests / perimeter

1. Actual local-registry parent+static-child install using a declared arbitrary-suffix native parent; run `check --json` in Mixed and Xml. Require no BootGraphIntegrity finding on fresh state. Use otherwise clean fixture or inspect that check id separately so unrelated project warnings do not disguise results.
2. Corrupt parent unit INDEX fingerprint, then separately remove its INDEX. Same actual check must report parent BootGraphIntegrity Warning and warning-only exit0. Restore valid INDEX control.
3. Install Xml, change selected config to Mixed, then check: visible BootGraphIntegrity Error and exit1/JSON ok=false, not silent clean. Independently malformed lock and missing selected slot should similarly refuse; no record-derived format fallback.
4. No-lock project remains non-applicable with no boot-graph finding or introduced state. Existing activated-transform fresh-tree, native pending/stale and legacy verifier tests stay green.
5. Snapshot project bytes/directories around check, including refusal cases; no generated rewrite, lifecycle lock or lease is created by verification.

Write perimeter: workspace bootgen verifier implementation/reexport; focused existing bootgen/hybrid tests or bounded new sibling; CLI commands/check.rs actual format/finding branch plus new focused CLI test. Add permanent PROP-072 traceability and format-log behavior note under root control. Existing CheckReport Finding construction is public; helper err/warn methods are crate-private, so use the current direct push shape rather than widening checker API solely for this adapter.

## Limits

Current verify_fingerprints checks recorded INDEX fingerprints/pending data and dependency staleness; it is not a cryptographic STATIC-body integrity proof. The requested missing-INDEX test matches that owner. Preserve this scope. This adapter is an A4 consumer completion, not docs/skills or whole-campaign completion.

## Root verdict

Approved as the next bounded adapter. Missing lock remains non-applicable; stale unit INDEX remains Warning/exit0; an applicable graph that cannot be verified yields existing BootGraphIntegrity Error/exit1 after normal report emission. This makes failure visible without a new schema or bypassing the report. Use selected format and existing binding/table/fingerprint owners, not generated-input consumption admission. Own-boot integration is now accepted; keep broader host check/doc/skill support separate and explicit.
