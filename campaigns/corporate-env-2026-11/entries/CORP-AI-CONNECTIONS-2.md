# CORP-AI-CONNECTIONS-2

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Interactive vibe secrets offers a Show references action listing consumer IDs

**Write.** `crates/vibe-cli/src/commands/secrets.rs`, `crates/vibe-cli/tests/cli_secrets_interactive.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_secrets_interactive interactive_show_references_lists_consumer_ids`

**Test.** crates/vibe-cli/tests/cli_secrets_interactive.rs :: interactive_show_references_lists_consumer_ids. The file starts with `#![cfg(all(test, target_os = "linux"))]`. Imports: `use std::{fs, io::Write, os::unix::fs::PermissionsExt, path::Path, process::{Command, Stdio}};`. Copy fn `compiled_runner` verbatim from crates/vibe-cli/tests/cli_app_resume_modes.rs; it only needs include_str!("common/pty-runner.c") and no `mod common`. Like the existing PTY tests, the host needs `cc` and libutil (-lutil). Steps:
1. In a 0o700 tempdir, use non-TTY runs of the vibe binary (env_clear, HOME and VIBE_SETTINGS set to the tempdir, PATH=/usr/bin:/bin) to run `secrets create --kind api-key --label Synthetic --value-stdin --json` with stdin `SYNTHETIC_INTERACTIVE_SECRET_DO_NOT_PRINT`, parsing the id from items[0].id. Then run `ai connections add --label Synthetic --endpoint http://127.0.0.1:32123/v1 --api-kind openai-chat-completions --default-model synthetic --secret-ref <id> --json` and take the connection id `cid` as the first key of the `items` object.
2. Record the bytes of <tempdir>/secrets/index.toml (after the add, which bumps the store revision).
3. Run the compiled PTY runner with argv [CARGO_BIN_EXE_vibe, "secrets"] and env_clear plus HOME, VIBE_SETTINGS, PATH=/usr/bin:/bin, TERM=xterm and VIBE_TEST_PTY_INPUT="\x1b[B\r\x1b[B\x1b[B\x1b[B\x1b[B\r\x1b". The keys select the item, move to the fifth action, choose it, then Escape exits the list.
4. Assert exit status success. The runner output contains `Consumers of <id>:` and `ai:<cid>`, and does not contain the sentinel.
5. Assert the secrets/index.toml bytes are unchanged.

**Read first.**
- crates/vibe-cli/src/commands/secrets.rs
- crates/vibe-cli/src/commands/secrets/human.rs
- crates/vibe-cli/tests/cli_app_resume_modes.rs
- crates/vibe-cli/tests/common/pty-runner.c
- crates/vibe-cli/tests/cli_ai_connections.rs

**Steps.**
1. Precondition: tools/cargo-test-ran.sh must exist in the checkout you work in, and CORP-AI-CONNECTIONS-1 must be applied (human.rs exists). If either is missing, stop and report a packet defect.
2. In fn `interactive` in secrets.rs, change the action list to `output::prompts::named(&["Details", "Edit label", "Rotate value", "Remove item", "Show references"])`, appending the new action last so indices 0 to 3 keep their meaning.
3. Add match arm `4 =>` before the catch-all arm. It calls `store.references(selected)`. On Ok(consumers) it runs `println!("{}", human::references(selected.as_str(), &consumers))`. On Err(_) it runs `println!("Reference inventory unavailable or unknown.")`. Then it returns to the item list (the next loop iteration). This arm makes no mutation and shows no confirmation prompt.
4. Write the PTY test described in `test`.
5. Run `cargo fmt --all` and `cargo clippy -p vibe-cli --all-targets -- -D warnings` and fix every warning. Run `wc -l crates/vibe-cli/src/commands/secrets.rs crates/vibe-cli/tests/cli_secrets_interactive.rs` and confirm each is at or below 600 lines.

**Decisions already made.**
- The new action is labelled `Show references` and is appended at index 4.
- Rendering reuses `human::references` from CORP-AI-CONNECTIONS-1, so the human text is identical to `vibe secrets references`.
- An inventory error prints the fixed line `Reference inventory unavailable or unknown.` and returns to the list. It does not end the session.
- The test sends all keystrokes up front through VIBE_TEST_PTY_INPUT. The console crate reads keys with TCSADRAIN, so queued input is kept between prompts.
- specmap.json is not regenerated in this entry; tools/self-check.sh does not run `cargo xtask specmap --check`.

**Not in this entry.** Not in this entry: search or filter in the item list, editing kind or other metadata, changes to the Remove preview, boxed-panel widgets, and regenerating specmap.json.

**Depends on.** CORP-AI-CONNECTIONS-1

Entries that write the same file run one after another, never in parallel.
