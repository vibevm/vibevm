# D1 native macOS/Windows proof selection — unrun

Read-only preparation against the named current owners and closure audit. No builds/tests/Git/source/user-state operation ran; no native results are claimed. Mutable owned1.1/public hold remains. API availability/selected Linux receipts are not macOS or Windows completion.

Execute from the same reviewed source checkout on EACH actual native OS; cross-compiling on Linux is insufficient. Set CARGO_BUILD_JOBS=4 and CARGO_TARGET_DIR to that native checkout's existing shared target. Use pinned --locked --offline initially; a dependency-cache miss before tests is compilation evidence only, not runtime/platform refusal. Any dependency acquisition needs root disposition, not a provider/install invocation. Retain exact command, raw stdout/stderr, exit, actual native OS/filesystem and nonzero test count.

## Minimum common selection: seven ordinary cases on each native OS

Each command below expects exactly1ordinary test; --nocapture retains failures/child observations. No package-wide family/list run is required.

```text
cargo test --locked --offline -p vibe-safefs --lib transaction::tests::existing_project_lock_reacquires_rechecks_and_store_token_is_stable -- --exact --nocapture
cargo test --locked --offline -p vibe-workspace --lib migration_application::runtime::runtime_tests::apply_reader_mode_restore_and_terminal_replay_preserve_exact_bytes -- --exact --nocapture
cargo test --locked --offline -p vibe-workspace --lib migration_application::runtime::runtime_tests::retained_sync_tests::matching_orphan_requires_actual_image_sync_and_failure_blocks_target -- --exact --nocapture
cargo test --locked --offline -p vibe-workspace --lib migration_application::runtime::runtime_tests::retained_sync_tests::loaded_restore_requires_image_sync_but_inspect_and_replay_do_not -- --exact --nocapture
cargo test --locked --offline -p vibe-workspace --lib migration_application::runtime::runtime_tests::process_restart_apply_boundaries_preserve_supported_prefixes -- --exact --nocapture
cargo test --locked --offline -p vibe-workspace --lib migration_application::runtime::runtime_tests::process_restart_restore_boundaries_are_idempotent -- --exact --nocapture
cargo test --locked --offline -p vibe-workspace --lib migration_application::runtime::runtime_tests::prior_checkpoint_and_unknown_durability_are_not_erased_by_later_attempts -- --exact --nocapture
```

Obligations: safefs case acquires/reacquires the actual held OS lock, rechecks named identity after a forced retry, verifies stable/distinct store/project tokens and refuses hardlinks/available symlinks. It does not alone prove simultaneous cross-process serialization. Basic runtime case proves real reader-visible snapshot→copy, exact bytes/CRLF/Unicode, restore and no-new-write terminal replay. Unix mode preservation assertion is cfgUnix; Windows has byte preservation, not fabricated Unix-mode proof.

The two retained-sync cases reach the actual safefs pre-file-sync boundary, prove injected failure prevents target effects/changes, and prove successful orphan/loaded restore confirmation is invoked. Inspect and terminal replay do not sync/write retained state. The intentional failure is not a native flush success; the nonfailing half must also pass. Workspace dev-dependency enables safefs inject-failures automatically; no production environment hook is introduced.

Restart parents spawn current_testexe's EXACT ignored restart_probe with only four fixture-bound MIGRATION_TEST_* variables and require exit73 at each boundary. Apply has16points; restore8. Parent then uses fresh grants to inspect/recover/idempotently replay and compare temporary trees. The ignored child is not an extra ordinary green case and must not be directly selected without the parent-owned parameters. Process exit/recovery proves only surviving-evidence behavior, not power-loss directory persistence.

Sticky-history case checks completed/uncertain effects are retained and later Synced observations cannot upgrade known-Unconfirmed history; its actual orphan replay remains Unconfirmed. No cfgOS excludes these seven ordinary cases.

## Windows-only supplemental native checks: two cases

```text
cargo test --locked --offline -p vibe-safefs --lib transaction::tests::windows_external_bootstrap_admits_only_the_liveness_safe_journal_recoverable_class -- --exact --nocapture
cargo test --locked --offline -p vibe-safefs --lib transaction::tests::held_external_lock_denies_namespace_replacement_for_its_lifetime -- --exact --nocapture
```

Each expects1 on Windows; both are cfgWindows and would execute zero on macOS. First accepts actual Synced OR JournalRecoverable bootstrap tiers and invokes the existing bootstrap contract. Second proves a held barrier prevents its named-file rename until drop. Symlink subcases in the common barrier test are privilege-dependent and may skip; do not report them performed without observing setup success.

## Strong-tier assertion defect and evidence gap

transaction/tests/external.rs:24–26, directory_flush_reopens_readable_capability_without_namespace_creation is cfgUnix, therefore includes macOS. It unconditionally readable.sync_all().unwrap() and asserts root.sync_directory()==Synced. That is a strict positive-flush diagnostic, unsuitable as a REQUIRED portable-usability gate where the owner permits weaker namespace durability. A legitimate unsupported directory tier would fail this test even if ordinary migration can honestly proceed at a reduced tier. Existing runtime observe_durability(runtime.rs:93–132) accepts JournalRecoverable/Unsupported with DurabilityReduced, preserves knownUnconfirmed and refuses actual Failed. No inspected ordinary runtime case unconditionally requires FileAndDirectorySynced.

Do not weaken genuine file-data sync or turn actual failures into success. Root should bind a narrow diagnostic assertion correction if it intends this Unix case as portable proof; otherwise keep it a separately labelled strong-tier diagnostic. No macOS error or OS-wide runtime defect has been observed here.

The selected current cases do NOT explicitly assert the complete Unsupported→JournalRecoverable runtime report/warning branch: Windows bootstrap accepts either tier without printing which, while sticky-history case deliberately checks Unconfirmed. To close truthful weaker-tier evidence, root needs a targeted existing-owner assertion/receipt extension for actual reported tier and DurabilityReduced (or actual observed source of Unconfirmed), not an inferred Windows outcome. No new test/code was authored. safefs primitive retained-image cases live in registered tests/external_sync.rs, absent from this packet's content reads; a direct primitive selector/cfg audit needs that exact additional binding.

## Fixture/admission limits

All selected work occurs in tempfile-owned project/store siblings, exact known vibe.toml/before.bin/journal names and test-owned lock/link entries. No fixture upstream, live user root, settings/HOME lookup, provider or Docker operation is selected. Capabilities/grants bind those paths explicitly. storage.rs was read only for module registration (none); runtime.rs registers runtime_tests.rs, which registers retained_sync_tests.rs. Native preparation remains evidence, not task/platform acceptance; root retains closure judgment.
