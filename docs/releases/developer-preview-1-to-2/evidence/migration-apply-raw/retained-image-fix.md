# Retained before-image prerequisite correction

Writer STOPPED. Runtime storage/recovery now require the actual existing-file synchronization owner before a retained before-image authorizes target mutation. Only storage.rs/recovery.rs/runtime_tests.rs and the approved new retained_sync_tests.rs were changed; no main/lib/foundation/primitive/schema/Git/Cargo or production-env hook edit occurred. Public Result report-evidence retention is unchanged.

Matching orphan path now verifies exact expected snapshot, rechecks physical barrier and configured cache/store policy, then calls ExternalStore::sync_stable_file_bounded before prepared intent. Loaded completion/restore rereads the retained image, verifies its bytes/rawhash/length against the validated journal and invokes the same owner immediately before target mutation. Inspect and exact terminal no-new-target-write replay perform no sync. The owner confirms held file.sync_all with identity/snapshot rechecks and returns actual parent tier; no file bytes are rewritten. Weak namespace tier remains usable, while failed confirmation prevents target publication/restoration.

The aggregate stays conservative: successful image confirmation alone does not authenticate older journal/directory boundaries. Known-unconfirmed evidence stays unconfirmed until every needed boundary could be established; byte reads never manufacture file/directory sync. The matching-orphan baseline still checks this invariant.

## Focused proof and process boundaries

Root reported the three exact selected runtime cases each **1passed/0failed/569filtered**: existing prior_checkpoint_and_unknown_durability_are_not_erased_by_later_attempts plus matching_orphan_requires_actual_image_sync_and_failure_blocks_target and loaded_restore_requires_image_sync_but_inspect_and_replay_do_not. These tests use the actual owner hook after native preproof/immediately before file.sync_all: false counts and continues, true refuses. They compare target snapshot/identity and retained image, prove no target effect on failure, and confirm inspect/replay do not reach the sync boundary. I did not run Cargo. The owner's two selected primitive tests/build were reported green separately; Linux evidence does not become other-platform proof.

Root then required process-prefix refresh for the added synchronization. storage.confirm_image now invokes before_retained_image_sync after lock/policy rechecks immediately before the native call; after_retained_image_sync follows its successful returned file-sync acknowledgement **before** evaluating the returned parent tier. This timing exposes a truthful successful file-sync boundary without pretending parent durability is already accepted.

Existing abrupt restart cases now list16apply and8restore points (24scenario-specific observations), each exact ignored child must exit73. The two new labels appear in both scenarios; all other logical write boundaries remain. **Their two exact parent reruns are pending root execution**, not claimed passed. The three retained-image cases were not repeated after the hook-only patch, and no all-nine/other-seven/native matrix rerun occurred. Historical first runtime-family failure, later correction receipts and earlier nine-case evidence remain historical rather than erased or relabeled acceptance.

## Final hashes / source perimeter

- storage.rs571lines SHA46aceb9e118b9a9e9fe80c2f9b936e982d55f630990344f912443bfef72be45d
- recovery.rs497lines SHA8d03723029139bf03ae9669595a06b2e55bc7cdb8f0a7465d5f0d8cf1699014f
- runtime_tests.rs529lines SHA9d3e48b1478865c5d813ba708e62f049b5c482598a0075d36b51c3743a090dcc
- retained_sync_tests.rs151lines SHA6daff116407fd6521f08ca3c405e9c27520dad8ddb89a56099563d9ff31c127a

Added cases are nested under migration_application::runtime::runtime_tests::retained_sync_tests; source citations use APPLICATION-WIRE req3. Restart parent cases retain APPLY req1. All cells stay≤600. No source/application/native-platform or task acceptance is manufactured; root owns pending prefix proof and final admission.
