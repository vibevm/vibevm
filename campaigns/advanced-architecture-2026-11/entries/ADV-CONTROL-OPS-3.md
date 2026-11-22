# ADV-CONTROL-OPS-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A `vibe --json` project install whose client has closed both stdout and stderr runs to its end and exits 0 instead of panicking with exit 101.

**Write.** `crates/vibe-cli/src/output/sink.rs`, `crates/vibe-cli/src/output.rs`, `crates/vibe-cli/src/output/lines.rs`, `crates/vibe-cli/src/composition/outcome.rs`, `crates/vibe-cli/tests/cli_operation_client_exit.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-cli --test cli_operation_client_exit 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_operation_client_exit.rs, fn a_json_install_finishes_when_its_reader_closes_both_pipes. Verified in round 1: the same invocation exits 101 with 'failed printing to stdout: Broken pipe' from Context::emit_json, after the install effects are done.

**Read first.**
- crates/vibe-cli/src/output.rs: Context::emit_json (`println!("{rendered}");`) and Context::flush_json_plans (`println!("{}", serde_json::to_string_pretty(&value)?);`). The file is 598 lines and the conform limit is 600.
- crates/vibe-cli/src/output/lines.rs: every println!/eprintln!, including Context::summary (`println!("{text}")`) and the Mode::Json branch of Context::error_with_suffix (`self.suspend_progress(|| eprintln!("{payload}"));`)
- crates/vibe-cli/src/composition/outcome.rs fn finish (`println!("{report}")` and `eprintln!("{report}")` in the cancellation branch)
- crates/vibe-cli/src/output/machine.rs (the progress writer already ignores write errors: the model)
- crates/vibe-cli/tests/cli_transfers.rs fn vibe (spawning the binary with std::process::Command and scratch env)

**Steps.**
1. Create crates/vibe-cli/src/output/sink.rs:
```
//! The standard-stream writes of a run that must outlive its reader
//! (PROP-073 `#OPERATION-CLIENT-EXIT`): the client that started vibe may
//! exit or crash and close the pipes, and a failed write must never end
//! the run. `println!` and `eprintln!` panic on a broken pipe; these do not.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-CLIENT-EXIT");

use std::io::Write;

/// One line on standard output; a closed or broken stream is ignored.
pub(crate) fn out_line(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{text}").and_then(|()| out.flush());
}

/// One line on standard error; a closed or broken stream is ignored.
pub(crate) fn err_line(text: &str) {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "{text}").and_then(|()| err.flush());
}

/// One JSON document on standard output.
pub(crate) fn document(text: &str) {
    out_line(text);
}

/// The JSON error envelope on standard error.
pub(crate) fn envelope(text: &str) {
    err_line(text);
}
```
2. crates/vibe-cli/src/output.rs: add `pub(crate) mod sink;` directly after `mod progress;` and add no other lines (the file must stay at or under 600 lines). In flush_json_plans replace `println!("{}", serde_json::to_string_pretty(&value)?);` with `sink::document(&serde_json::to_string_pretty(&value)?);`. In emit_json replace `println!("{rendered}");` with `sink::document(&rendered);`.
3. crates/vibe-cli/src/output/lines.rs: change `use super::{Context, Mode, structured};` to `use super::{Context, Mode, sink, structured};`. In error_with_suffix's Mode::Json branch replace `eprintln!("{payload}")` with `sink::envelope(&payload.to_string())`. Replace every other `println!(<fmt>)` with `sink::out_line(&format!(<fmt>))` and every other `eprintln!(<fmt>)` with `sink::err_line(&format!(<fmt>))`, except: when the format string is a single placeholder and nothing else, pass the value directly instead of wrapping it in format! (clippy::useless_format fires otherwise; verified on clippy 1.93). Concretely Context::summary's `println!("{text}")`, where text is &str, becomes `sink::out_line(text)`. Inside match arms like write_trailer's, the arm becomes `Stream::Out => sink::out_line(&format!("{gap}{text}")),`. The multi-line `eprintln!("{} {}{err:#}{}", ...)` in the HumanQuiet branch keeps its arguments inside format!.
4. crates/vibe-cli/src/composition/outcome.rs: replace `println!("{report}");` with `crate::output::sink::document(&report.to_string());` and `eprintln!("{report}");` with `crate::output::sink::err_line(&report.to_string());`.
5. Create the test. Use vibe_test_support::UserScratch::new(), a tempfile::tempdir() project and user.init_project(project.path()). Spawn std::process::Command::new(env!("CARGO_BIN_EXE_vibe")) with env VIBE_SETTINGS=user.settings, vibe_test_support::REGISTRY_CACHE_ENV=user.cache, vibe_test_support::SEARCH_CACHE_ENV=user.search_cache and VIBE_NO_DEFAULT_REGISTRY=1, and env_remove VIBEVM_HOME, VIBEVM_INSTALL_ROOT, VIBE_INVOKED_BY and VIBE_PROGRESS_FORMAT. Args are `--json --progress-format json install --path <project> --assume-yes`, with stdin Stdio::null() and stdout and stderr Stdio::piped(). Right after spawn, `drop(child.stdout.take()); drop(child.stderr.take());`, then wait. Assert status.code() == Some(0) and that project/.vibe/lifecycle.toml is a file (vibe init does not create it; the install does). Flip the spec unit.
6. Run `cargo fmt -p vibe-cli`, `cargo clippy -p vibe-cli --all-targets -- -D warnings`, `cargo test -p vibe-cli --bin vibe output::` (the output unit tests still pass) and `wc -l crates/vibe-cli/src/output.rs` (must print 600 or less).

**Decisions already made.**
- Only the shared output layer is converted: Context's JSON documents (emit_json, flush_json_plans), the error envelope, lines, diagnostics, decisions, blocks, trailers, and the cancellation report. The verified panic site is Context::emit_json.
- OPERATION-CLIENT-EXIT is worded to exactly that layer and says that commands printing their JSON report directly (18 files under commands/, for example requirements.rs, explain.rs, doc.rs, query.rs) still end on a broken pipe, so impl/done is true for what this entry delivers.
- document and envelope are separate functions now so OPS-4 can capture them for the record without touching output.rs again.
- Interactive-only writers (output/prompts.rs and the eprint! calls in output/progress.rs) are left alone: they run only on a terminal.
- No signal handling changes: Rust already ignores SIGPIPE, so a broken pipe reaches these writes as an io error.

**Not in this entry.** Do not convert the println! calls inside individual commands (including the direct JSON report prints), the startup warnings in composition.rs, composition/network.rs, composition/transfers.rs, composition/remap.rs and main.rs, or tracing output. No --operation-id flag yet.

**Depends on.** ADV-CONTROL-OPS-1

**Spec edit (apply as a step of this entry).**

PROP-073: replace `<OPERATION-CLIENT-EXIT fact="true" status="spec/plan">` with `<OPERATION-CLIENT-EXIT fact="true" status="impl/done">`.

Entries that write the same file run one after another, never in parallel.
