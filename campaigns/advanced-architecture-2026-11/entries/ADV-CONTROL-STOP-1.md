# ADV-CONTROL-STOP-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** An interrupted application run's typed error names who stopped it: `AppError::stopped_by()` returns `Some("stop-file")` or `Some("signal")` as reported by the caller's `Interrupt`, and `None` when the interrupt owner does not say.

**Write.** `crates/vibe-app/src/installer/child.rs`, `crates/vibe-app/src/error.rs`, `crates/vibe-app/src/codes.rs`, `crates/vibe-app/src/steps/run.rs`, `crates/vibe-cli/src/commands/app/display/tests.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-app --lib an_interrupted_error_names_who_stopped_it`

**Test.** crates/vibe-app/src/codes.rs, inside the existing `#[cfg(test)] mod tests`, new fn `an_interrupted_error_names_who_stopped_it`. It builds `StepError::Interrupted { key: "k".into(), during: "during step `build`".into(), step: Some("build".into()), work: "/w".into(), stopped_by: Some("stop-file".into()) }.into()` as an AppError and asserts `stopped_by() == Some("stop-file")`, `code() == "interrupted"` and `work() == Some("/w")`. It builds a second one with `stopped_by: None` and asserts `stopped_by() == None`. It builds `StepError::Failed {..}` (copy the literal already in that test module) and asserts `stopped_by() == None`.

**Read first.**
- crates/vibe-app/src/installer/child.rs:25-48
- crates/vibe-app/src/error.rs:230-245
- crates/vibe-app/src/codes.rs:290-395
- crates/vibe-app/src/steps/run.rs:255-313
- crates/vibe-app/src/steps/run.rs:365-380
- crates/vibe-cli/src/commands/app/display/tests.rs:40-70

**Steps.**
1. In crates/vibe-app/src/installer/child.rs, add a provided method to `pub trait Interrupt` after `poll`: `/// Who requested the stop, once one was requested: `"stop-file"` or `"signal"`; `None` when this owner does not say.` `fn stopped_by(&self) -> Option<&'static str> { None }`.
2. In crates/vibe-app/src/error.rs, add the field `stopped_by: Option<String>,` as the last field of `StepError::Interrupted`. Leave the #[error] message text unchanged.
3. In crates/vibe-app/src/steps/run.rs `fn interrupted`, set `stopped_by: self.ctx.interrupt.and_then(|interrupt| interrupt.stopped_by()).map(str::to_owned),`. If `self.ctx.interrupt` is not reachable under that exact name, use the same expression `check_interrupt` uses at run.rs:278-281 to reach the interrupt.
4. In crates/vibe-app/src/codes.rs, inside `impl AppError` next to `pub fn work`, add `/// Who stopped an interrupted run, when the interrupt owner said (`stop-file` or `signal`).` `pub fn stopped_by(&self) -> Option<&str> { match self { Self::Step(StepError::Interrupted { stopped_by, .. }) => stopped_by.as_deref(), _ => None } }`. Add `stopped_by: None,` to the existing Interrupted literal in the test module (around line 383), then add the new test.
5. In crates/vibe-cli/src/commands/app/display/tests.rs, add `stopped_by: None,` to the two `StepError::Interrupted` literals (lines ~53 and ~60). Do not touch anything else in vibe-cli.
6. Run the check, then `cargo check -p vibe-cli --tests` so vibe-cli still compiles. Use CARGO_TARGET_DIR=/data/git/vibevm-architect-target for both.

**Decisions already made.**
- Trait method name `stopped_by`, signature `fn stopped_by(&self) -> Option<&'static str>`, with a default body that returns None, so every existing implementor (vibe-cli Signals, the test Flag in child.rs, the doc-test Flag) compiles unchanged.
- Error field name `stopped_by: Option<String>` on `StepError::Interrupted`, placed last. The human error message is unchanged.
- Accessor `AppError::stopped_by(&self) -> Option<&str>` in codes.rs, next to `step()` and `work()`.
- The only allowed values are the strings "stop-file" and "signal". vibe-app never invents a value; it copies what the Interrupt reports.
- `InstallerError::Cancelled` (the project-token path) keeps no stopped_by. Only `StepError::Interrupted` carries it.

**Not in this entry.** Implementing `stopped_by` in vibe-cli's `Signals`, stamping it into the JSON envelope, or adding a `resume` field to the envelope. All three belong to ADV-CONTROL-STOP-2 or to later work.

Entries that write the same file run one after another, never in parallel.
