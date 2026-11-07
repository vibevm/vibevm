# A4 facts report/adopt adapter decision

Read-only source design, 2026-11-07. No implementation, tests, Cargo or Git mutation. Root authorizes the slice after boot-check acceptance. This is a command adapter and bounded rederive preparation, not a new migration/transaction framework.

## Missing behavior and shared read seam

`facts report` currently resolves a slot then calls scan_authored_facts on physical contents. `Adoption::prepare` separately requires Mixed record and exact_declared_files; directory sources refuse, transformed output mapping is not consumed. Both can reuse public workspace `prepare_declared_query_read`, lower PreparedInstalledSource/FileResolver and `vibe_facts::observe_installed_source`. No fake PackageMeta or private requirements query access is needed.

Factor a CLI-private selected package observation used by report and adopt: capture one lock-selected full LockedPackage, held workspace/slot/manifest/record, independently selected effective SpecFormat and complete selected source read. Preserve original logical addresses and direct Spec/Doc fact traversal; record-derived output/media mapping is evidence to validate against independent expectation, not a source for expected hash/format. The existing installed binding admits finite record-backed directory selections, so do not retain exact-file-only rejection or invent a global source census. Preserve full witness capture before native syntax interpretation.

Report is read-only and can use the observation immediately. Available yields existing adoption counts; invalid/unavailable authored totals must be reported truthfully with bounded reason rather than claiming no materialized slot. Retain command's current package enumeration/selection and counts, not a second requirements report schema. No registry mutation or rederive is part of report.

## Hash meanings — keep them separate

| Value / owner | Actual meaning |
|---|---|
| LockedPackage.content_hash; SlotRecord.source_hash | Independently selected original package content identity. Compare them; never replace with transformed file/registry digest. Editable record equality is consistency, not publisher authentication. |
| SlotRecord file row.sha256 | Installed output bytes at the recorded physical path. Transformed row.source preserves original logical path; it is not an original-file digest. |
| SourceFileWitness(path, hash) from installed observer | Original logical label plus hash of captured materialized bytes. Existing requirements source digest frames these witnesses; do not call this original package hash. |
| SlotRecord.derived_hash | Existing prepared/recorded derived payload hash, owned by materializer. |
| overlay_file_hash | SHA-256 of exact persisted per-package adoption registry TOML bytes, not semantic statuses-only hash. Absence/unreadable has existing optional handling; do not silently redefine it in this adapter. |
| FactEntry / AuthoredFact | Registry entry stores address/origin/package/status/comment; AuthoredFact stores address/requirements/status. Neither stores a source hash. Adoption does not need a new evidence field/schema. |

The installed query reads the current derived materialized statuses. That is already the command's historical source surface; original-source status recovery is not implied. Adopt only adds marked missing registry entries and keeps existing entries, including indeterminate ones; prefix filtering remains before candidate creation. Do not silently overwrite existing adoption or discard void statuses.

## Transformed adoption must still rederive

Current adopt writes registry entries, then calls rederive_or_defer unless declared Mixed admission retained the no-op. `rederive_package` obtains exact locked source from the local immutable store, requires locked bytes, checks configuration, and calls materialise_with_spec_format. Transformed materialization loads PackageOverlay and applies statuses before output/record generation; Mixed deliberately omits overlay transformation. Therefore allowing transformed reads does NOT justify marking transformed adopt as no-op or forcing Mixed rederive. Preserve post-adopt transformed derivation.

Split existing rederive_package into preparation and execution at the CLI composition seam, retaining its legacy wrapper. Before the first upsert, prepare the exact selected source/cache fetch (offline), pin hash/identity, effective format, workspace/package configuration and supported conversion/materialization inputs. Determine the exact candidate additions against the loaded registry, validate entries/destination and preflight the prospective PackageOverlay through the incumbent converter/preparation owner. Only then publish registry additions and execute the retained rederive operation with the same format/selection. Known missing store/config/unsupported conversion/collision must fail before registry changes.

Do not call materialise merely to test whether it would work before registry writes: it is itself publication and would apply the old overlay. If a public pure seam is missing, expose a narrow workspace preflight over existing SourcePolicy/declared preparation and a pure projected overlay, not another parser/writer. PackageOverlay currently derives only from Registry and has private entries; a narrow facts-owned `package_overlay_with_additions` (validated known entries) or equivalent prepared projection can supply that prospective input without writing a temporary registry. Exact overlay_hash should still be computed from actual persisted deterministic bytes by the existing owner after writes; no guessed hash of the prospective map.

Keep separate pre-write checks for registry destination identity, lock/record/policy and selected materialized source buffers. They must not be dropped when switching to the reusable reader. Retain rechecks before each upsert, and named post-write failure behavior for late I/O/concurrent drift. This is predictable-error prevalidation, not rollback/whole-command atomicity. Existing rederive returns false for genuinely absent package and leaves Mixed/in-place alone; do not mislabel a failed preparation as successful deferred work.

## Legacy and in-place

Unrecorded undeclared legacy keeps the incumbent scanner/read/rederive compatibility route. Recorded legacy should use the same approved manifest/hash coherence check as accepted package query before fallback; document the stricter corrupt-metadata result. Declared in-place currently has no installed record binding and remains a precise unsupported provenance case until a separately justified actual source owner exists. Do not fabricate a record or grant installed authority from its path. A4 completion accounting must retain this explicit boundary instead of claiming every materialization mode was migrated.

## Suggested perimeter / compatibility

CLI commands/facts.rs, facts/selected.rs (or bounded selected-observation child), commands/reinstall.rs narrow prepare/execute extraction, focused facts CLI tests. Only if necessary: facts overlay pure projection and workspace materializer preflight export using existing owners. No edits to accepted requirements report/digest/schema are needed. Public Rust helper additions and command behavior deserve FMT entry; no vibe.toml key, lock/slot registry schema, source-hash recipe or global install dialect changes are implied. State that transformed/directory facts report/adopt is newly supported and declared Mixed-only guard superseded; install -g still shares existing package/format mechanics.

## Real CLI discriminators

1. Actual local-registry install in Mixed/Markdown/Xml with native arbitrary original suffix, directory-selected native facts and raw same-stem/nonUTF8 resources. `facts report <package>` counts only native original addresses; adopt adds marked statuses/void, skips unmarked, respects prefix and keeps existing entries. Repeated adopt reports kept and does not duplicate.
2. Transformed adopt completes actual rederive: slot record keeps locked source_hash, correct format/recipe, overlay_hash equals actual registry-file hash and computed derived hash matches produced payload. Compare original registry fields/schema; no invented source hash in FactEntry.
3. Missing/mismatched selected hash/record/output, unsupported native syntax, changed lock/policy/destination and unavailable exact store/configuration refuse before any registry upsert. A later invalid source must not follow an earlier addition. Use real command and source/registry byte snapshots; runtime I/O failure after publication is separately qualified.
4. Report observes invalid/unavailable source without writes, not a fabricated zero total. Clean recorded legacy/unrecorded legacy retain useful behavior; declared in-place refuses explicitly. Existing Mixed adoption no-op and legacy rederive tests remain.

Root must approve the prospective-overlay preflight seam before coding; merely swapping the reader and deleting Mixed checks would leave known post-adopt failure and incorrect rederive behavior.

## Root verdict

Approved next bounded adapter for versioned declared package report/adopt across supported formats and record-backed directory sources. Preserve actual post-adopt rederive and each existing hash meaning. Before any upsert, validate selected source/configuration/destination and prospective overlay through existing pure preparation; no temporary registry writes or materialization-as-preflight. A narrow facts-owned prospective overlay and workspace preparation export are permitted if existing APIs cannot express this. Late I/O/concurrent failure remains explicitly qualified, not a new global transaction. Declared in-place has no invented provenance and stays a named limitation pending its real source owner; legacy remains compatible. No registry/schema change or requirements report redesign.
