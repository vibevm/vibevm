# A4 facts command prospective-overlay review

Read-only live-source review, 2026-11-07. No code edits, Cargo/tests or Git mutation. FC-OVERLAY-ROOT-01 is resolved on the frozen ten-path candidate; no remaining concrete blocking defect found in this bounded review. Full A4 is not claimed.

## FC-OVERLAY-ROOT-01 — closed, shared-slot ownership preserved

Root chose the existing workspace-root overlay owner, not a new member refusal or per-member slot layout. `PreparedPointRederive::preflight_adoption` now branches on actual registry_root versus workspace root. At the workspace root it projects prospective additions into the root registry overlay. For member-local adoption it captures the workspace package overlay file's optional exact bytes through held Project reads, loads the current workspace Registry overlay, immediately rechecks those bytes, and uses THAT overlay for original-source derivation preflight. Member additions remain local registry state. Execute still calls materialise with the same workspace root, so the preflight and actual shared-slot output use the same owner.

AdoptionRederivePreflight retains both original source observation and optional WorkspaceOverlayInput. Recheck runs before every member upsert and before execute, detecting absent→present and byte drift in the workspace overlay. Selected admission separately rechecks workspace root policy/lock/slot and member destination identity. No new overlay schema, ownership redirect or merged member view is claimed.

The actual CLI test `member_adoption_preserves_workspace_overlay_owner` seeds root REAL=spec/work and member REAL=impl/work, runs member report without writes, adopts one missing marked fact while keeping the member REAL status, and asserts: root registry bytes unchanged; member registry has its local additions/status; slot.overlay_hash equals actual root registry hash and differs from member hash; materialized output contains root spec/work and not member impl/work; derived payload hash is consistent. `/tmp/nst-a4-facts-final-selected.log` passes all5 selected CLI cases including this discriminator.

## Supported mechanisms in the inspected candidate

Adoption selected read now obtains independent kind/group/name/version/content_hash from captured Lockfile and effective input format from the actual configuration owner. prepare_declared_query_read and observe_installed_source reuse accepted recorded output mapping/direct parser; directory rows and transformed original logical names no longer require suffix scanning or Mixed-only reads. Materialized bytes never become the expected original package hash. Declared in-place still refuses; undeclared unrecorded legacy retains scanner/rederive compatibility. Recorded legacy passes common manifest/hash coherence before fallback.

All candidate additions are collected and validated before the first upsert; existing registry entries, including indeterminate ones, are kept. Prefix and unmarked filtering remain. Prospective PackageOverlay validates origin/package/address and preserves existing precedence. It is a pure projection; no temporary registry is written.

For transformed declared adoption, prepare_rederive_package performs offline exact locked source acquisition and locked-byte/configuration checks before registry publication. The workspace preflight calls SourcePolicy::observe and existing declared::prepare on the prospective overlay and retains the original policy for recheck. That helper publishes neither slot nor registry. Source acquisition can use the existing local cache/store machinery; this is not a claim that every preparation call is globally effect-free.

Selected admission rechecks root/slot policy, exact lock/record/document bytes, independently selected current format and registry destination directory/no-follow path before writes. Preflight rechecks original source before each upsert and again before execution. Actual transformed rederive is retained after additions; Mixed is the no-op. Late failures explicitly say entries may be persisted. No whole-command rollback/CAS claim follows.

Distinct hash meanings remain: locked content_hash/record.source_hash identify selected original package; per-output row SHA and SourceFileWitness describe materialized bytes; derived_hash describes produced payload; overlay_hash is exact persisted registry TOML bytes through existing materializer owner. FactEntry gains no hash field. No new wire/schema or source-digest recipe was observed.

Report now catches selected observation refusal and prints unknown authored total with “selected source invalid or unavailable”, rather than calling every failure an absent slot. It remains text-only and read-only. It does not perform transformed rederive merely to count facts.

## Evidence actually observed / limits

Final source manifest `/tmp/nst-a4-facts-commands-sha256.json`: 10 entries, all10 current hashes matched during this review. No zero-file or inferred seal claim.

- `nst-a4-facts-final-selected.log`:5 actual selected CLI tests PASS, including all3formats/directory original IDs, prefix/indeterminate/repeat,9 failure modes, actual transformed output and divergent member/root overlay ownership.
- `nst-a4-facts-final-runtime.log`: actual CLI families7+13+5 PASS; counts overlap with the selected rerun.
- `nst-a4-facts-retained-final.log`:5 filtered unit cases PASS, including the new workspace overlay missing→present/byte-drift guard; other selected-admission/legacy cases are not extra independent coverage.
- `nst-a4-facts-unit-final.log`:5 retained admission cases PASS; `nst-a4-facts-overlay.log` and `nst-a4-facts-preflight.log` each contain one meaningful positive/negative family. Preflight test asserts no output/record or registry publication, then converter collision refusal and retained source drift.

The earlier failing native fixture log was intermediate and superseded by final receipts. This review did not rerun tests. The ten-path feature seal matches; root still owns canonical specifications/format ledger/map and commit acceptance.

Compatibility limits are explicit: member adoption changes member registry only; shared materialized slots retain workspace overlay meaning. Declared in-place lacks the independent installed binding and remains unsupported. Recorded legacy coherence is stricter as already approved; unrecorded legacy remains. Offline cache/source preparation is distinct from pure materializer preflight and is not claimed globally effect-free. Predictable input/config/conversion failures precede upserts, but late source drift/I/O can follow persisted earlier entries and is reported honestly. No global transaction/CAS, new evidence hash recipe or full A4 completion is claimed.
