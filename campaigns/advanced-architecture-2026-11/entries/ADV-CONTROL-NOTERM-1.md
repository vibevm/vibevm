# ADV-CONTROL-NOTERM-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The application operations that decide through `global::attended` (install/update/uninstall -g, app modify, app configuration, app portable, app migrate, node remediation and agent consent) ask a question only when stdin and stdout are both terminals and none of --json, --progress-format json or --unattended is in effect.

**Write.** `crates/vibe-cli/src/commands/app/global.rs`, `crates/vibe-cli/src/commands/app/global/terminal.rs`, `vibevm/vibespecs/common/PROP-065-structured-installation.xml`

**Check.** `cargo test -p vibe-cli --bin vibe -- --exact commands::app::global::terminal::tests::prompts_need_two_terminals_and_no_machine_output 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** crates/vibe-cli/src/commands/app/global/terminal.rs, #[cfg(test)] mod tests, fn prompts_need_two_terminals_and_no_machine_output. Six assert! lines: prompts_allowed(false,false,false,true,true) is true; then each single change gives false: json=true (true,false,false,true,true); unattended=true (false,true,false,true,true); machine_progress=true (false,false,true,true,true); stdin_tty=false (false,false,false,false,true); stdout_tty=false (false,false,false,true,false). The check passes only when exactly this one test runs and passes, so a missing `mod terminal;` line (0 tests run) fails the grep. The change to `attended()` (step 4) is shown only by compilation; no test asserts the wiring.

**Read first.**
- crates/vibe-cli/src/commands/app/global.rs:1-70
- crates/vibe-cli/src/commands/app/global.rs:95-125
- crates/vibe-cli/src/output.rs:530-545
- vibevm/vibespecs/common/PROP-065-structured-installation.xml:36-36

**Steps.**
1. Create crates/vibe-cli/src/commands/app/global/terminal.rs with the module doc line `//! When an application operation may ask a question (PROP-065 #attended).` and `pub(super) fn prompts_allowed(json: bool, unattended: bool, machine_progress: bool, stdin_tty: bool, stdout_tty: bool) -> bool { !json && !unattended && !machine_progress && stdin_tty && stdout_tty }` annotated `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#attended")]`.
2. In the same file add `#[cfg(test)] mod tests { use super::prompts_allowed; #[test] fn prompts_need_two_terminals_and_no_machine_output() { ... } }` with the six assert! lines named in the test field.
3. In crates/vibe-cli/src/commands/app/global.rs add the line `mod terminal;` directly after the existing line `mod node_remediation;` (plain `mod`, no #[path], exactly like node_remediation).
4. Replace the body of `pub(super) fn attended(ctx: &output::Context) -> bool` in global.rs with: `use std::io::IsTerminal as _; terminal::prompts_allowed(ctx.is_json(), ctx.is_unattended(), ctx.is_machine_progress(), std::io::stdin().is_terminal(), console::user_attended())`.
5. Insert the spec_edit line into vibevm/vibespecs/common/PROP-065-structured-installation.xml.
6. Run the check command; it must print `test result: ok. 1 passed`.

**Decisions already made.**
- Function name `prompts_allowed` in new module `terminal` (file global/terminal.rs); do not reuse the name `attended_terminal` (a field in vibe-app launch.rs).
- Argument order is exactly (json, unattended, machine_progress, stdin_tty, stdout_tty).
- stdout terminal detection stays `console::user_attended()`; stdin detection is `std::io::stdin().is_terminal()`.
- No environment variable other than the existing VIBE_UNATTENDED / VIBE_PROGRESS_FORMAT handling (already inside ctx) affects the result.
- global.rs is 579 lines today and must stay under 600: only the `mod terminal;` line and the changed body go there.
- The spec text covers only the operations that call `global::attended` (callers verified: global.rs operation context and line ~357, global/node_remediation.rs, app/configuration.rs, app/agents.rs, app/portable.rs, app/portable/full.rs, app/migrate.rs). It makes no claim about app launch, install/confirm.rs, trust.rs, ordinary_answers.rs or secrets.rs.
- Commit: `fix(app): ask questions only with a terminal on stdin and stdout` with a body saying a machine progress stream or a non-terminal stdin now means these application operations never show a prompt.

**Not in this entry.** Aligning the other terminal checks (commands/app/launch.rs, commands/install/confirm.rs, trust.rs, ordinary_answers.rs, secrets.rs, prune.rs, clean.rs) to the same function, or widening the spec text to cover them.

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/common/PROP-065-structured-installation.xml: insert one new line directly after the line that ends with `</error-codes></p>`:
  <p><attended fact="true" status="spec/done">The application operations `install -g`, `update -g`, `uninstall -g`, `app modify`, `app configuration`, `app portable`, `app migrate`, the node runtime remediation and an agent step's consent are attended, and may ask a question, only when standard input and standard output are both terminals and none of `--json`, `--progress-format json` (or `VIBE_PROGRESS_FORMAT=json`) and `--unattended` (or `VIBE_UNATTENDED`) is in effect. No other environment signal makes these runs attended. When such a run is not attended it asks nothing: each question takes its flag, its recorded value or its default, or the run refuses. Other commands keep their own terminal checks for now.</attended></p>

Entries that write the same file run one after another, never in parallel.
