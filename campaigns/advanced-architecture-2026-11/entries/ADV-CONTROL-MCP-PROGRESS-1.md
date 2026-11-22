# ADV-CONTROL-MCP-PROGRESS-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** vibe-wire turns a vibe_core progress event into the vibe-progress-event/1 wire object, redacting its text the way the CLI does, and the CLI machine stream uses that encoder.

**Write.** `crates/vibe-wire/src/behaviour/progress_event.rs`, `crates/vibe-wire/src/behaviour/mod.rs`, `crates/vibe-cli/src/output/machine.rs`, `crates/vibe-cli/src/output/progress/sanitize.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-wire --lib behaviour::progress_event::tests`

**Test.** crates/vibe-wire/src/behaviour/progress_event.rs, #[cfg(test)] mod tests, fn encoder_numbers_events_keeps_labels_and_redacts_text: builds `vibe_core::progress::Progress::new(recorder.clone())` where `recorder: Arc<Recording>` and Recording implements ProgressObserver by pushing every ProgressEvent into a Mutex<Vec>. Starts a task with label "fetch https://user:pw@example.org/x", calls detail("token=abc123") on it, then finish(). Encodes the three recorded events in order with one ProgressEventEncoder and at="2026-01-01T00:00:00.000Z". Asserts: seq values are 1,2,3; protocol == "vibe-progress-event/1" on all three; kinds serialize to "start","detail","finish"; the start label contains "[redacted]@example.org" and not "pw"; the detail message equals "token=[redacted]"; the finish line carries the same label as the start.

**Read first.**
- crates/vibe-cli/src/output/machine.rs:1-190
- crates/vibe-cli/src/output/progress/sanitize.rs:1-126
- crates/vibe-cli/src/output/progress.rs:15-30
- crates/vibe-wire/src/behaviour/mod.rs:1-45
- crates/vibe-wire/src/generated/progress_event/mod.rs:1-120
- crates/vibe-core/src/progress.rs:60-140

**Steps.**
1. Create crates/vibe-wire/src/behaviour/progress_event.rs containing: `pub const PROGRESS_EVENT_PROTOCOL: &str = "vibe-progress-event/1";`, `pub const MAX_MESSAGE_CHARS: usize = 480;`, `pub fn sanitize_progress_text(input: &str) -> String` (move the bodies of sanitize, redact_url and redact_assignments from crates/vibe-cli/src/output/progress/sanitize.rs, keeping the helpers private), and `#[derive(Debug, Default)] pub struct ProgressEventEncoder { seq: u64, labels: BTreeMap<TaskId, String> }`.
2. In the same file implement `impl ProgressEventEncoder { pub fn new() -> Self; pub fn encode(&mut self, event: vibe_core::progress::ProgressEvent, at: String) -> crate::generated::progress_event::ProgressEvent }` by moving the body of MachineRenderer::observe (from building `line` through setting `line.label`) plus the free functions end_figures and wire_record from crates/vibe-cli/src/output/machine.rs. Use the `at` argument instead of calling chrono, and sanitize_progress_text wherever the CLI called it.
3. Add `pub mod progress_event;` to crates/vibe-wire/src/behaviour/mod.rs in alphabetical order (after `pub mod ordinary_install_answers;`).
4. Rewrite crates/vibe-cli/src/output/machine.rs: State keeps `writer` and replaces `seq`/`labels` with `encoder: vibe_wire::behaviour::progress_event::ProgressEventEncoder`. observe() locks the state, calls `encoder.encode(event, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true))`, serializes the result and writes it as one line exactly as today. Delete the moved functions and every import no longer used. Keep the test module unchanged.
5. Reduce crates/vibe-cli/src/output/progress/sanitize.rs to its specmark scope line plus exactly: `#[cfg(test)] pub(super) use vibe_wire::behaviour::progress_event::MAX_MESSAGE_CHARS;`, `pub(super) fn sanitize(input: &str) -> String { vibe_wire::behaviour::progress_event::sanitize_progress_text(input) }` and `pub(crate) fn sanitize_progress_text(input: &str) -> String { sanitize(input) }`. The #[cfg(test)] is required because progress.rs imports MAX_MESSAGE_CHARS only under #[cfg(test)] and clippy runs with -D warnings.
6. Add the test, run the check, then run as regressions `cargo test -p vibe-cli --bin vibe output::` (output::machine::tests::every_observation_is_one_json_line_with_identity_and_parent and the progress tests must still pass) and `cargo clippy -p vibe-wire -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- The shared encoder lives in vibe-wire because vibe-cli is a bin-only crate (no lib.rs) and vibe-wire already depends on vibe-core, serde_json and chrono.
- Module path: vibe_wire::behaviour::progress_event. Public names: PROGRESS_EVENT_PROTOCOL, MAX_MESSAGE_CHARS, sanitize_progress_text, ProgressEventEncoder::{new, encode}.
- encode takes the timestamp as a String argument, so the encoder reads no clock.
- Mapping and labels stay byte-identical to the CLI's today, including the text "stopped without an outcome" for ProgressEventKind::Stopped and dropping the label after a terminal event.
- Add no new dependency to any Cargo.toml.

**Not in this entry.** Any change to the vibe-progress-event/1 schema or the generated types, any vibe-mcp code, and any refactor of the human/plain progress renderers in output/progress.rs.

Entries that write the same file run one after another, never in parallel.
