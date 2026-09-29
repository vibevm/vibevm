# CLI migration-operation candidate — writer stopped

Implemented only the bounded early CLI surface. Main borrows the existing parsed command into machine_entry before configuration promotion, tracing, Context, VVM, registry or progress initialization. That helper routes only plan-migration (unchanged handler) and migration-operation. The new command uses stdin or a request-json literal and has the single SilentMetadata progress arm; reaching normal refactor dispatch refuses.

Generated outer request decoding and schema admission precede grant/project opening. For apply, the existing pure planner also detects embedded structural/schema/null preflight before Project open; questions, domain refusals and three-ID replay remain owned by shared operation validation/runtime. The explicit operator invocation grants exactly the authored-file selection through OwnedManifestMigrationFile. No new path/parser/hash/recipe or lifecycle engine was introduced.

Normal runtime reports are owner-validated before one JSON document plus LF. Completed/no-op exit0, pending and invalid request/schema exit2, other refusals/uncertain exit1; invalid UTF8 stdin gets invalid_request/request.input and exit1. Internal ReportValidationFailure emits no normal stdout: stderr labels the fault and complete original UNVALIDATED JSON evidence, exit1. Output failure has fixed diagnostic that preserves the possibility of earlier effects.

## Actual verification

Only `CARGO_BUILD_JOBS=4 cargo test --locked --offline -p vibe-cli --test migration_operation` was initially run. Exit101: 5 passed, 2 failed, 0 filtered. I incorrectly expected Pending/exit2 for both absent-store and published inspection. Actual owner behavior is Refused/journal_invalid exit1 for absent store, and Completed exit0 for verified published state. These were fixture assumptions; production stayed unchanged. I corrected those exact expectations, retaining all no-write and byte/mode checks.

Only the two affected exact selectors were rerun (same flags, `-- --exact`): absent_store_inspection_stdin_literal_parity_creates_nothing and apply_inspect_restore_terminal_replay_preserve_reader_bytes_and_mode. Each exit0: 1 passed, 0 failed, 6 filtered. The original five unchanged passing cases plus those two exact retries establish seven distinct selected passing cases, not a fresh seven-case family run. Logs:

- `/tmp/vibevm-next-M13-D1-cli-operation-checks.log` retains initial 5/2 failure.
- `/tmp/vibevm-next-M13-D1-cli-operation-inspect-absent-retry.log`
- `/tmp/vibevm-next-M13-D1-cli-operation-roundtrip-retry.log`

Cases cover isolated poisoned HOME/config/stale VVM startup; malformed/schema/null input including embedded preflight precedence over missing root; stdin/literal full-report parity; no-creation inspection; exact comments/Unicode bytes and Unix mode through apply/current-reader/inspect/restore/terminal no-op; mismatched planner/intent IDs; changed/third states with retained completed effects; unsupported epoch/escape/Unix link refusals; and deliberate inconsistent-report diagnostic preservation without new filesystem mutation. All seven cases cite SURFACES req2. The native executable is CARGO_BIN_EXE_vibe; child authority and state live only in temporary directories.

A pre-existing vibe-scrape unused-variable warning appears in retained logs. No broad tests, old planning target, codegen, Git mutations, public/provider operations, or native macOS/Windows proof were run. Pending exit routing now has the actual retained-prefix proof below. This is candidate evidence, not acceptance. Writer stopped; root retains independent review, denominator/account changes and integration.

## Final authored file identities

- `crates/vibe-cli/src/main.rs`: 597 lines; SHA256 `2e7d84ff34f9e89b757944f5ad8999357ecb847f284360cbcf038bcba201a0cb`.
- `crates/vibe-cli/src/command_progress.rs`: 396 lines; SHA256 `19f1784e0e662945ac0a2b973a21466dc1067e4e70e7857f517c68136c958f2c`.
- `crates/vibe-cli/src/commands/refactor/mod.rs`: 313 lines; SHA256 `0473cb57809c4723c6290e26250833b2b5fc2420a359bb96dd2e51790eab93e2`.
- `crates/vibe-cli/src/commands/refactor/machine_entry.rs`: 24 lines; SHA256 `a15e1e09df879e6524c45b1e5c976a29b94bdd1b65d6a9895f144b832e8110bb`.
- `crates/vibe-cli/src/commands/refactor/migration_operation.rs`: 202 lines; SHA256 `96a5a46ee6499022d2de7aa7e9a6936a708c942b55308c3dc659433530aac337`.
- `crates/vibe-cli/tests/migration_operation.rs`: 448 lines; SHA256 `51d9b988f275ee2924a9bacf211946a4dc5605127057e03799280847d2bd392d`.

## Actual Pending prefix follow-up

Root requested one missing external branch. Only the existing roundtrip case changed. After successful apply, the fixture writes known test-owned foreign bytes at the exact Restore stage returned by the public identity owner. A real CLI restore returns Refused/exit1 and leaves the authored target and foreign occupant untouched. Reading the generated journal proves actual phase Restoring; no checkpoint/effects were hand-authored. CLI inspection then returns Pending/exit2 and leaves the entire fixture tree unchanged. The test explicitly removes only its own stage and completes normal restore/terminal replay.

The exact selector `apply_inspect_restore_terminal_replay_preserve_reader_bytes_and_mode` was rerun with the same locked/offline/jobs4 target, `-- --exact`: exit0, 1 passed, 0 failed, 6 filtered. Log `/tmp/vibevm-next-M13-D1-cli-operation-pending-proof.log`. No other cases or production files were rerun/edited. This supersedes the roundtrip case's earlier narrower passing receipt; all historical logs remain. Writer stopped after this bounded follow-up.
