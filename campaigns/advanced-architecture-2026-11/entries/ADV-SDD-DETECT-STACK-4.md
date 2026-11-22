# ADV-SDD-DETECT-STACK-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** With `--json`, a refused stack choice puts `code` (`ambiguous_stack` or `no_stack_detected`) and `candidates` (the detected coordinates) in the JSON error envelope.

**Write.** `crates/vibe-cli/src/commands/init/stack_choice.rs`, `crates/vibe-cli/src/commands/init/mod.rs`, `crates/vibe-cli/src/output/structured.rs`, `crates/vibe-cli/tests/cli_init_detect.rs`

**Check.** `cargo test -p vibe-cli --test cli_init_detect detect_refusal_json_ 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** crates/vibe-cli/tests/cli_init_detect.rs: `detect_refusal_json_carries_code_and_candidates` (does not exist yet; the filter runs 0 tests before the work, so grep fails).

**Read first.**
- crates/vibe-cli/src/output/structured.rs: fn stamp_structured_error, fn stamp_lock_hash_mismatch
- crates/vibe-cli/src/commands/init/stack_choice.rs: StackChoiceRefusal, fn select
- crates/vibe-cli/src/commands/init/mod.rs: the module list and the `pub(crate) use helpers::{current_timestamp_utc, strip_unc_public};` line
- crates/vibe-cli/tests/cli_reinstall_lock_pin.rs: the stderr envelope lookup
- crates/vibe-cli/tests/cli_init_detect.rs

**Steps.**
1. Runs after ADV-SDD-DETECT-STACK-3 (same files). In stack_choice.rs add the field `pub(crate) candidates: Vec<String>` to StackChoiceRefusal and fill it in select: the detected coordinates in detection order for `ambiguous_stack`, an empty Vec for `no_stack_detected`.
2. In mod.rs add `pub(crate) use stack_choice::StackChoiceRefusal;` directly after the existing `pub(crate) use helpers::{current_timestamp_utc, strip_unc_public};`.
3. In crates/vibe-cli/src/output/structured.rs add `fn stamp_stack_choice(payload: &mut Value, err: &anyhow::Error)` modelled on stamp_lock_hash_mismatch: `let found = err.chain().find_map(|cause| cause.downcast_ref::<crate::commands::init::StackChoiceRefusal>());` `let (Some(refusal), Value::Object(map)) = (found, payload) else { return; };` then insert `code` = `Value::String(refusal.code.into())` and `candidates` = `Value::Array(refusal.candidates.iter().cloned().map(Value::String).collect())`. Call `stamp_stack_choice(payload, err);` from stamp_structured_error directly after the `stamp_lock_hash_mismatch(payload, err);` call.
4. Add test `detect_refusal_json_carries_code_and_candidates`: fixture `Cargo.toml` + `package.json`=`{}\n`; run `vibe init --detect-stack --json --path <dir>` with `.output()` -> status not success; find the envelope exactly as tests/cli_reinstall_lock_pin.rs does: `stderr.lines().filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok()).find(|value| value["ok"] == false)` with a panic message showing stderr when none is found; assert `code == "ambiguous_stack"` and `candidates == ["org.vibevm.ai-native/rust-ai-native","org.vibevm.ai-native/typescript-ai-native"]`. Then the same run on an empty temp dir: `code == "no_stack_detected"`, `candidates == []`.

**Decisions already made.**
- The JSON error envelope (stderr, `{"ok":false,"error":...}`, written by output/lines.rs through stamp_structured_error) is the existing one; this entry only stamps two members on it, like lock_hash_mismatch.
- `candidates` carries coordinates (`org.vibevm.ai-native/<name>`); the human headline carries names.

**Not in this entry.** Not in this entry: a typed error type for the existing-manifest refusal, a registered schema for the error envelope, and changes to the human refusal rendering.

**Depends on.** ADV-SDD-DETECT-STACK-3

**Spec edit (apply as a step of this entry).**

None.

Entries that write the same file run one after another, never in parallel.
