# ADV-FIX-PUBDRY-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Commands run by `vibe doc check --examples` never inherit a `VIBEVM_PUBLISH_TOKEN*` variable from the operator's environment.

**Write.** `crates/vibe-doc/src/examples/sandbox.rs`, `crates/vibe-doc/src/examples/tests.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-doc --lib publish_token_variables_are_cleared_from_example_commands`

**Test.** crates/vibe-doc/src/examples/tests.rs, new `#[test] fn publish_token_variables_are_cleared_from_example_commands()`: asserts `crate::examples::sandbox::is_publish_token_var` returns true for `VIBEVM_PUBLISH_TOKEN`, `VIBEVM_PUBLISH_TOKEN_GITHUB`, `VIBEVM_PUBLISH_TOKEN_GITVERSE` and false for `VIBEVM_INSTALL_ROOT`, `VIBE_SETTINGS`, `PATH`.

**Read first.**
- crates/vibe-doc/src/examples/sandbox.rs:40-60
- crates/vibe-doc/src/examples/sandbox.rs:268-300
- crates/vibe-doc/src/examples/tests.rs:1-30

**Steps.**
1. In sandbox.rs, directly after the `CLEARED` constant, add `pub fn is_publish_token_var(name: &str) -> bool { name.starts_with("VIBEVM_PUBLISH_TOKEN") }` with the doc comment `/// A publish token the operator holds: never handed to a documented command, so a rehearsal never reaches a host with it and its golden does not depend on who runs the check.`
2. In `pub fn run`, directly after the `for key in CLEARED { child.env_remove(key); }` loop, add `for (key, _) in std::env::vars_os() { if key.to_str().is_some_and(is_publish_token_var) { child.env_remove(&key); } }`.
3. Append the test `publish_token_variables_are_cleared_from_example_commands` to crates/vibe-doc/src/examples/tests.rs as specified.
4. Run the check.

**Decisions already made.**
- Match by prefix `VIBEVM_PUBLISH_TOKEN` so the legacy variable and every per-host `VIBEVM_PUBLISH_TOKEN_<HOST>` are removed.
- Token files need no change: the runner already points VIBE_SETTINGS at the sandbox home, so `~/.vibe/*.publish.token` is not visible.
- The `CLEARED` list itself is not changed.
- Commit message: `fix(doc): keep publish tokens out of documented example commands` with a body saying a rehearsal example must not reach a host with the operator's token or vary by machine.

**Not in this entry.** Clearing other credential variables (git credential helpers, index tokens) or refactoring CLEARED into a pattern list.

Entries that write the same file run one after another, never in parallel.
