# ADV-FIX-EXAMPLES-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A documented example never sees the VIBEVM_HOME or VIBE_LOG of the shell that runs `vibe doc check --examples`: `vibe vars` inside an example prints `VIBEVM_HOME=(none)` and `VIBE_LOG=warn` even when the caller exported both.

**Write.** `crates/vibe-doc/src/examples/sandbox.rs`, `crates/vibe-cli/tests/cli_doc_examples_env.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_doc_examples_env`

**Test.** New file crates/vibe-cli/tests/cli_doc_examples_env.rs, test fn `inherited_vibevm_home_and_vibe_log_do_not_reach_a_documented_example`. It makes TWO tempdirs: `work` and `outside`. In `work` it builds a doc package `<work>/pkg` shaped like `package()` in crates/vibe-doc/src/examples/tests.rs: `examples/bare/example.toml` = "schema = 1\n", an empty file `examples/bare/tree/work/.keep`, and page `vibevm/vibespecs/start/p.xml` holding `<example id="vars" fixture="bare"><run>vibe vars</run><expect></expect></example>` inside `<spec xmlns="https://vibevm.org/spec/1"><title id="root">P</title>…</spec>`. It runs `UserScratch::new().vibe()` with args `doc check --examples --accept --path <work>/pkg --sandbox <outside>/sand`, `.current_dir(<work>)`, `.env("VIBEVM_HOME", <outside>/leaked-home)` and `.env("VIBE_LOG", "debug")`. The sandbox and the leaked home MUST live in `outside`, never under the current directory: the tripwire (crates/vibe-doc/src/examples/tripwire.rs) records the top-level entries of the current directory before the run and fails with `appeared: repo-top:sand` if the sandbox is created there. Asserts: the process exits 0; the page file now contains `VIBEVM_HOME=(none)`; it contains `VIBE_LOG=warn`; it does not contain `leaked-home`; it does not contain `VIBE_LOG=debug`.

**Read first.**
- crates/vibe-doc/src/examples/sandbox.rs:1-60
- crates/vibe-doc/src/examples/sandbox.rs:268-300
- crates/vibe-doc/src/examples/tripwire.rs:40-65
- crates/vibe-doc/src/examples/tests.rs:1-56
- crates/vibe-cli/tests/cli_doc_captured.rs:1-10
- crates/vibe-test-support/src/lib.rs:150-165

**Steps.**
1. Run the check: it fails today because the test target does not exist.
2. Append "VIBEVM_HOME" and then "VIBE_LOG" as the last two elements of `pub const CLEARED` in crates/vibe-doc/src/examples/sandbox.rs.
3. In the module doc comment rule 2 of sandbox.rs, add `VIBEVM_HOME` and `VIBE_LOG` to the parenthesised list of removed variables.
4. Create crates/vibe-cli/tests/cli_doc_examples_env.rs with `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");` at the top and the one test described, using two `tempfile::tempdir()` values (`work`, `outside`).
5. Run the check until it passes.

**Decisions already made.**
- The fix is removal from the child environment (env_remove via CLEARED), not setting a value. VIBE_LOG is cleared deliberately: unset, the product reports its default `warn`, which is what the manual's `reference/settings-and-environment.xml#vars` golden shows.
- The test uses `--accept` on an empty golden so it asserts on the captured text and does not depend on the rest of `vibe vars` output.
- Sandbox and leaked home go in a second tempdir outside the current directory, so the tripwire's top-level guard is not tripped by the test's own layout.
- Observed today: with VIBEVM_HOME exported, reference/settings-and-environment.xml#vars in the English manual prints the caller's VIBEVM_HOME and diverges; with it unset that example matches.

**Not in this entry.** Clearing or overriding HOME/USERPROFILE or PATH for examples, or chasing `[self] install_root` from a user config.toml: those are known remaining leaks; agent-detection leaks are handled by deferral in ADV-FIX-EXAMPLES-4. Do not widen CLEARED beyond the two names above, and do not edit tripwire.rs.

Entries that write the same file run one after another, never in parallel.
