# M-01-B.2 continuity review — live candidate

2026-10-12, reviewed 04:21:10–04:21:59 UTC. Read-only review; no tests, providers, builds, filesystem mutations outside this report, or task acceptance. Files remain under active implementation. Root has already queued the materialization correction. The B.1 accepted receipt is preserved; findings concern new B.2 world joins/current checks.

**Two concrete initial-join omissions and one bounded check-window gap need an exact case or source refutation. None requires a new lease, query kernel or stronger OS framework. The current implementation avoids discovering unrelated workspace-member slots during revalidation.**

## 1. Selected lock materialization does not bind the epoch root

`capture/continuity.rs::lock_rows_for_owner` compares row kind/version/content hash and ordered dependency coordinates against epoch data. It does not compare `LockedPackage.materialization` against the selected epoch package manifest or derive/check the root selected by that lock mode.

Concrete trusted-preparation sequence:

1. Prepare epoch/captures from package P at the versioned **Copy** slot.
2. The governing lock is subsequently changed/captured with the same P coordinate, version, content hash and edges, but `materialization = in-place`.
3. Pass the older epoch/captures and that newer genuine lock snapshot into `WitnessedSourceWorld::from_prepared_node`.
4. Initial join accepts the fields it compares and stores the new row. Current lock has that same new row; the old copy slot still has unchanged manifests/outputs. `revalidate_current` can therefore return success using the old selected root under a lock selecting the in-place root.

This is a constructor-time mixed selection, not a demand to prevent arbitrary later races. `extension_world/epoch.rs::resolved_slot_root` explicitly chooses different roots for in-place versus other materialization; `LockedPackage.materialization` is the governing field (`manifest/lockfile.rs:441`).

Minimum fix: in the initial join compare row materialization with `epoch.package_manifest(...).package.materialization`, and where needed verify the lock-derived slot root equals the retained provider root through the incumbent path owner. Preserve full-row equality in subsequent checks. Deciding regression: old Copy epoch/captures + current InPlace lock with otherwise equal fields must refuse before a usable current world. No executed failure claimed here.

## 2. Descriptor map source identity is not joined to the witnessed slot record

`continuity.rs::validate_record` checks record source hash, manifest output SHA and each captured output's path/SHA. It does not compare each prepared descriptor-map output row's **source path** with the corresponding slot-record row (`SlotFile.source`, or the output path for Mixed).

Concrete sequence:

1. Prepare a descriptor map for output O from authored source A, together with output bytes/digest and captured manifest.
2. A newer, structurally valid transformed slot record maps the same output O and digest to source B. A and B are distinct valid declared source paths; same output bytes do not imply the same descriptor/source identity. Capture this newer record while retaining the old map.
3. Package source witness, manifest SHA, output path/SHA all agree with the currently checked subset. Initial `validate_record` accepts; later current record matches the newer witnessed record and outputs remain unchanged, so revalidation succeeds with map A plus record B.

`slot_record.rs::SlotFile` expressly carries `source` and `disposition`; transformed record validation requires them, but it cannot compare them to the separately prepared resolver map. A valid record is not proof it is the record from which that map was prepared.

Minimum fix: for every admitted output, join the descriptor map's `output_row(path).source` to the slot record's effective source mapping, using the existing Mixed-versus-transformed rule. Keep the existing manifest-derived interpretation check. If the exact representation/recipe is part of the retained descriptor identity, bind its existing witness too; do not invent original-source byte hashes absent from the record. The deciding regression keeps bytes/hash constant but changes the record source mapping and requires a stale/invalid world. This is B.2 descriptor identity, not later M-01-C content-provenance implementation.

## 3. Final governing check covers only lock, missing in-window descriptor/member changes

Observed ordering in `revalidate_current`:

1. `check_governing_world()` reads selected lock rows.
2. Read/compare membership manifests, closure manifests and provided slot records once.
3. Read/compare captured outputs.
4. Call `check_governing_world()` again — **lock only** — then return success.

Concrete deciding race fixture: prepare a valid world, let its selected descriptor record pass step 2, then at the existing bounded-output read boundary change that descriptor record (or the selected member manifest's dependency visibility) and leave output bytes and lock rows unchanged. The output read still matches its prepared snapshot, and the last lock read agrees; current code returns success even though the governing descriptor/member world changed **inside this revalidation operation, before the final lock check**. The same scenario works with a selected source manifest descriptor edit after its first comparison.

This is a measured-window **stale-world success**, not necessarily a returned byte buffer combining old and new bytes: the method returns the old prepared binding. Its error is certifying that binding after a governing change during the checks. B.2 explicitly requires changed selected descriptor/membership to refuse and race fixtures not to accept mixed world observations. Merely describing detection-bound behavior does not justify checking the lock twice while letting other governing identities change during the enclosed output checks.

Minimum fix: make the existing governing-world check encompass the selected membership chain, closure manifests and relevant descriptor records as well as selected lock rows, and bracket output checks with it (or equivalently recheck those finite witnesses afterward). This remains ordinary bounded detection, not an atomic transaction. A change **after the final applicable recheck** remains the honest later observation/continuity limit; no endless rechecking or retained-handle redesign is demanded. M-01-C still owns the actual retained source-read/provenance layer. Deciding fixture should mutate one governing record during the output phase and expect StaleWorld/WorldUnavailable; no current test run is claimed.

## Preserved scope and limits

- `membership_chain` selects root plus the requested member's parent chain; it does not rediscover every workspace member.
- `closure_manifests` follows the selected node's retained dependency closure. It includes hidden reachable dependency manifests needed by current visibility calculation; this is broader than the final effective output set but is not an unrelated workspace-member slot scan. No concrete unrelated-broken-member read was found in this path. Future narrowing should retain the metadata needed to prove visibility, not simply drop hidden decision inputs.
- Current revalidation returns prepared captured outputs after comparison; it does not hydrate/fetch or silently replace them with current bytes.
- Missing manifests/records/outputs become named world-unavailable boundaries, and mismatches become stale. Those coarse boundary names avoid exposing filesystem paths from current read failures.
- `check_governing_world` deliberately compares selected lock rows rather than whole-file bytes, allowing unrelated lock rows to differ. That supports point-query isolation; it should not be replaced with unconditional whole-workspace revalidation to fix the three specific joins above.
- No transient missing test registration is treated as a defect. No source-query tests beyond the granted files were read. The newly imported exact test owner is `crates/vibe-workspace/src/source_query/capture/m_01_b_2_cases.rs`; grant that literal file if independent test-body review is wanted.

## Reviewed state

Initial complete continuity read at 04:21:10 UTC:

- `source_query/capture/continuity.rs`: `6cda17cc7a375379131b3c7f8821d0bac7cc2894f023a5ed1a0822c720aad997`
- `source_query/capture.rs`: `42454063160aed0637140507aa6c9a7d0589ec8065dc62d5cdbc7029e532898b`
- `source_query/visibility.rs`: `b06b5ba23c9b64b0be2c709084f10483b373cbd28a348ff4a3016556e2f8509d`

At 04:21:59 UTC, continuity had changed to `1948c71c8588034cab85f7f9f842e55787041dfbc8b8334091d7a4bf59a33298`; focused reread confirmed both initial-join functions still had the described comparisons, and a test module import had appeared. This report is tied to those observed stages, not any later executor fix. Root/executor must establish red→green or refutation for the exact cases, then decide acceptance.

## Correction review — 04:27:24 UTC

Read current continuity/test sources and supplied red/fixed logs. No tests run by this reviewer. **Initial findings 1 and 2 are closed in the reviewed source. Finding 3 remains in the last output pass; duplicating the whole check pass moved the gap instead of placing a governing check after outputs.** Root acceptance remains pending.

### Closed constructor joins

- `lock_rows_for_owner` now reads the epoch package manifest, compares `row.materialization` with its declared materialization, derives the expected root using incumbent `in_place_slot_abs_path` versus `slot_abs_path`, and compares it with `source.provider.root`. The Copy-epoch/InPlace-lock combination therefore refuses at construction. No fallback/new path scheme was added.
- `validate_record` now obtains the corresponding map output row and compares `mapped.source` with the slot record's effective source: output path for Mixed, required `row.source` otherwise. Identical output hashes cannot conceal a changed source mapping. Existing output hash/manifest/package witness checks remain.
- The exact regressions are `changed_lock_materialization_cannot_rebind_an_old_copy_root` and `unchanged_output_hash_cannot_join_different_record_source`. The supplied red log records **6 passed / 2 failed**, with these two failures. The supplied fixed log records **8 passed / 0 failed / 0 ignored**, including both. The current source-map test includes a matching-source positive before the mismatch refusal, demonstrating that the correction does not reject every transformed record.
- Authorized `crates/vibe-workspace/src/vibedeps/path.rs` is absent. No guessed alternate file was read. The repair calls the same imported incumbent slot helpers already used by the epoch; further helper-definition inspection is not essential to this comparison review. If Root needs independent helper-body review, it must supply the actual defining path.

### Still open: governing changes during second output pass

Current exact ordering:

```text
check_governing_world()          // selected lock only
check_selected_files()           // membership → manifests/records → outputs
hook after_first_pass()
check_governing_world()          // selected lock only
check_selected_files()           // membership → manifests/records → outputs
check_governing_world()          // selected lock only
return prepared
```

`mutation_between_observation_passes_cannot_publish_a_mixed_world` injects at `after_first_pass`, changing **both lock version and output bytes**. The next lock read refuses immediately. This is a valid regression for that phase, but it does not prove detection of a descriptor/member change after its second comparison and during the second output phase. Nor does it independently test a manifest-only or descriptor-only mutation with the lock unchanged inside that final output phase.

Concrete remaining scenario: second `check_selected_files` compares the old member/source manifest and old descriptor record successfully; while reading the last captured output, the selected descriptor record is changed and the output bytes/lock stay unchanged. No record comparison follows that output read. The final lock read passes and the method returns success. This is the same finite observed-window stale-world success identified earlier. It does not require arbitrary hostile Rust input, and the return value can still contain coherent old bytes; the false assurance is that their governing descriptor passed after the output-observation window when it did not.

**Minimum correction:** separate `check_selected_files` into (a) finite membership/closure-manifest/descriptor governing checks and (b) output checks, and put the governing checks **after the last output phase** as well as before it, alongside the existing selected-lock checks. Existing double output observations may be retained if justified, but adding a third complete pass is unnecessary. The final sequence should have a plainly identifiable last applicable governing recheck following all output observations.

Add or relocate a narrowly named test seam immediately after the **last output phase, before the final governing recheck**, and mutate only the descriptor record/member manifest while leaving lock/output bytes unchanged. That deterministically establishes the required bracket. It is not enough to reuse the current between-pass hook with a lock mutation.

This does not promise an atomic multi-file snapshot. A change after that file's final governing observation, or a later namespace/source change after revalidation, remains the stated detection-bound/later M-01-C limitation. No infinitely repeated checking or new retained-handle framework is requested. The finite distinction is simply that the promised governing bracket must actually enclose the final output reads.

Reviewed hashes:

- `source_query/capture/continuity.rs`: `dcdbc1d76d274e7f051f89c1aa0cd4e9038d0669757108bae4cdcdab70c4d028`
- `source_query/capture/m_01_b_2_cases.rs`: `8242eb45af6f64e298199a5e033d494ca3e42bc2720bf2e8db3e2dc847f821ca`

These describe the correction-review state; later source changes or test runs are not claimed by this section.
