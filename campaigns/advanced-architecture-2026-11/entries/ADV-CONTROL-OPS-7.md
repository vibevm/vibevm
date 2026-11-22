# ADV-CONTROL-OPS-7

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Starting a new recorded operation removes finished, failed and stopped records that ended more than 30 days ago and keeps only the 200 newest of the rest; running records stay.

**Write.** `crates/vibe-cli/src/composition/operation.rs`, `crates/vibe-cli/tests/cli_operation_retention.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-cli --test cli_operation_retention 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_operation_retention.rs, fn starting_an_operation_prunes_records_past_thirty_days_and_beyond_two_hundred.

**Read first.**
- crates/vibe-cli/src/composition/operation.rs: begin, all, record_path
- vibe_core::timestamp: `pub fn now_utc() -> String`, `pub fn parse_unix_utc(s: &str) -> Option<u64>`, `pub fn format_unix_utc(secs: u64) -> String` (RFC 3339 seconds `YYYY-MM-DDTHH:MM:SSZ`)
- crates/vibe-cli/tests/cli_operations_query.rs (record planting helper)

**Steps.**
1. Add `const KEEP_DAYS: u64 = 30;`, `const KEEP_RECORDS: usize = 200;` and `fn prune(home: &Path)`. now = parse_unix_utc(&now_utc()).unwrap_or(0) and horizon = now.saturating_sub(KEEP_DAYS * 86_400). Take all(home) filtered to state Finished, Failed or Stopped as written on disk. First remove each record whose ended (or started when ended is None) parses to a time below horizon, via std::fs::remove_file(record_path(home, &record.id)) with errors ignored. Sort the remaining ones by started descending, then id descending, and remove every one at index >= KEEP_RECORDS.
2. In begin, call prune(&home) immediately before the new record is written (after the OPS-6 retry decisions, so replays and refusals never prune).
3. Test. Plant (with the cli_operations_query.rs helper shape) 205 finished records KEEP-000 to KEEP-204, where KEEP-i has started = ended = format_unix_utc(now - 3600 - i*60) (KEEP-000 newest); one finished record OLD-ONE with started = ended = format_unix_utc(now - 40*86_400); and one running record STILL-RUNNING with pid std::process::id() and machine vibe_lifecycle::LeaseHolder::current().machine. now comes from SystemTime::now() since UNIX_EPOCH in seconds. Run user.vibe() with `--json --operation-id NEW-ONE transfers`, which succeeds. Assert KEEP-000 to KEEP-199 present, KEEP-200 to KEEP-204 absent, OLD-ONE absent, STILL-RUNNING and NEW-ONE present, and exactly 202 `*.json` files in SETTINGS/operations. Flip the spec unit.
4. Run `cargo test -p vibe-cli --test cli_operation_retry --test cli_operation_record`, `cargo fmt -p vibe-cli`, `cargo clippy -p vibe-cli --all-targets -- -D warnings` and `wc -l crates/vibe-cli/src/composition/operation.rs` (600 or less).

**Decisions already made.**
- Retention runs only when a new record is about to be written. `operations show` and `list` never delete.
- Age uses ended, falling back to started. The count limit applies after the age cut, ranked by started.
- Running and lost records are never pruned by retention.
- Runs after OPS-6 and before OPS-8: same file.

**Not in this entry.** No user-config knob for the limits, no `operations clear` verb, and no pruning of orphaned temp files.

**Depends on.** ADV-CONTROL-OPS-6

**Spec edit (apply as a step of this entry).**

PROP-073: replace `<OPERATION-RETENTION fact="true" status="spec/plan">` with `<OPERATION-RETENTION fact="true" status="impl/done">`.

Entries that write the same file run one after another, never in parallel.
