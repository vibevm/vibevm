# ADV-CONTROL-OPS-6

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Starting an operation whose ID is already recorded replays a finished, failed or stopped outcome without running again; different arguments are refused (operation_id_conflict), a live holder is refused (operation_running), and a lost record is replaced.

**Write.** `crates/vibe-cli/src/composition/operation.rs`, `crates/vibe-cli/tests/cli_operation_retry.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-cli --test cli_operation_retry 2>&1 | grep -F "test result: ok. 3 passed"`

**Test.** crates/vibe-cli/tests/cli_operation_retry.rs, fns a_retry_of_a_finished_operation_replays_its_outcome_without_running, a_retry_with_different_arguments_is_refused_as_a_conflict and an_operation_id_held_by_a_live_process_is_refused_as_running.

**Read first.**
- crates/vibe-cli/src/composition/operation.rs: begin, stored_argv, refuse (from OPS-4) and read, observed (from OPS-5)
- crates/vibe-cli/src/output/sink.rs: out_line, err_line
- crates/vibe-cli/tests/cli_operation_record.rs (spawn and env pattern)
- crates/vibe-cli/tests/cli_operations_query.rs (record planting helper)

**Steps.**
1. In begin, after the home is known and before the new record is built, compute holder (LeaseHolder::current()), argv (stored_argv()) and path once, and use them for the new record too. Then match read(&path). Err(e) refuses with ("operation_record_unreadable", format!("cannot read the operation record `{}`: {e}", path.display()), 2). Ok(None) continues as today. Ok(Some(earlier)): let earlier = observed(earlier, &holder.machine), then match earlier.state. Running refuses with ("operation_running", format!("operation `{id}` is running as process {} on `{}` since {}", earlier.pid, earlier.machine, earlier.started), 2). Lost continues, and the new record replaces it. Finished, Failed or Stopped: if earlier.argv != argv, refuse with ("operation_id_conflict", format!("operation `{id}` was recorded for a different command: vibe {}", earlier.argv.join(" ")), 2); otherwise return Some(replay(&earlier)).
2. Add `fn replay(record: &OperationRecord) -> ExitCode`. For each document (type &Option<serde_json::Value>) call `sink::out_line(&serde_json::to_string_pretty(document).unwrap_or_else(|_| "null".into()))` (None prints null). If error is Some(value) and value is not Value::Null, call `sink::err_line(&value.to_string())`. Return `ExitCode::from(u8::try_from(record.exit_code.unwrap_or(1)).unwrap_or(1))`. Replay does not call start_capture and does not set ACTIVE, so finish leaves the record untouched.
3. Tests, with one ID constant per test and the spawn env of cli_operation_record.rs, using `.output()` (pipes open). (1) Run `--json --operation-id <ID> install --path <project> --assume-yes`, which succeeds. Remove <project>/.vibe/lifecycle.toml and run the same args again: it succeeds, its stdout parsed with serde_json::Deserializer::from_slice(..).into_iter::<serde_json::Value>() equals the first run's parsed stdout, and <project>/.vibe/lifecycle.toml still does not exist. (2) First run as in (1), then the same args plus `--offline`: exit code 2 and the last non-empty stderr line parses to JSON with code "operation_id_conflict". (3) Plant SETTINGS/operations/<ID>.json in state running with pid std::process::id(), machine vibe_lifecycle::LeaseHolder::current().machine and the other fields as in cli_operations_query.rs, then run `--json --operation-id <ID> transfers`: exit code 2, the stderr envelope has code "operation_running" and an error containing format!("process {}", std::process::id()). Flip the spec unit.
4. Run `cargo test -p vibe-cli --test cli_operation_record` (OPS-4 tests stay green), `cargo fmt -p vibe-cli` and `cargo clippy -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- The order of decisions is unreadable, then running, then lost, then terminal. A running record from another machine counts as running, because its liveness cannot be known here.
- Same arguments means equal stored argv (redacted, --set values hidden) after the program name, so a client that moved the vibe binary still replays.
- Replay writes the documents the record holds pretty-printed (as emit_json does), the envelope compact, and returns the recorded exit code; a missing exit code counts as 1. A command whose JSON report bypasses the output layer replays without that report, as OPERATION-RECORD-WRITE states.
- Refusals exit 2 (a refusal before any effect). A lost record is re-run.
- Runs after OPS-5 and before OPS-7: all three edit composition/operation.rs.

**Not in this entry.** No locking against two clients starting the same ID in the same instant, no --resume semantics, and no change to how a lost record's earlier output is kept.

**Depends on.** ADV-CONTROL-OPS-5

**Spec edit (apply as a step of this entry).**

PROP-073: replace `<OPERATION-RETRY fact="true" status="spec/plan">` with `<OPERATION-RETRY fact="true" status="impl/done">`.

Entries that write the same file run one after another, never in parallel.
