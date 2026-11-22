# ADV-SDD-DETECT-STACK-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe init --detect-stack` in a repository with one detected stack initializes the project without an `[active]` table, keeps the CLAUDE.md user text outside the <vibevm> block, prints `next: vibe install stack:<coordinate>`, and refuses an existing vibe.toml or an unattended ambiguous detection.

**Write.** `crates/vibe-cli/src/cli/init.rs`, `crates/vibe-cli/src/commands/init/mod.rs`, `crates/vibe-cli/src/commands/init/proposal.rs`, `crates/vibe-cli/src/commands/init/stack_choice.rs`, `crates/vibe-cli/tests/cli_init_detect.rs`

**Check.** `cargo test -p vibe-cli --test cli_init_detect detect_apply_ 2>&1 | grep -F 'test result: ok. 3 passed'`

**Test.** crates/vibe-cli/tests/cli_init_detect.rs: `detect_apply_initializes_without_active_and_keeps_claude_text`, `detect_apply_refuses_existing_manifest`, `detect_apply_refuses_ambiguous_stack_unattended` (none exist yet; the filter runs 0 tests before the work, so grep fails).

**Read first.**
- crates/vibe-cli/src/commands/init/mod.rs: fn run, fn create_project
- crates/vibe-cli/tests/cli_init.rs: fn init_stack_flag_sets_active_stack (why the detected stack is not passed as --stack: init refuses an [active].stack that names no installed stack)
- crates/vibe-cli/src/commands/init/detect.rs: STACKS, Detection
- crates/vibe-cli/src/output/prompts/ask.rs: fn choose, fn named
- crates/vibe-cli/src/commands/init/helpers.rs: struct ProjectCreation
- crates/vibe-cli/tests/cli_init_detect.rs

**Steps.**
1. Runs after ADV-SDD-DETECT-STACK-2 (same files). In crates/vibe-cli/src/cli/init.rs change the `detect_stack` attribute to `#[arg(long = "detect-stack", conflicts_with = "stack")]` (drop `requires = "plan_only"`). Keep `plan_only` requiring `detect_stack`.
2. Create crates/vibe-cli/src/commands/init/stack_choice.rs (scope line `specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization");`). Define exactly:
```
#[derive(Debug, thiserror::Error)]
#[error("{code}: {headline} (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization; fix: `vibe init --stack <name>` after installing the stack, or `vibe init` without --detect-stack)")]
pub(crate) struct StackChoiceRefusal {
    pub(crate) code: &'static str,
    headline: String,
}
```
Codes and headlines: `ambiguous_stack` with `format!("several stacks match this project ({names}) and an unattended run does not choose one")` where names are the detected stack names joined by ", "; `no_stack_detected` with the headline text `no Cargo.toml, package.json, tsconfig.json or go.mod was found at the root or one level below, and an unattended run does not choose a stack`.
3. In stack_choice.rs write `fn select(ctx: &crate::output::Context, detection: &super::detect::Detection, interactive: bool) -> anyhow::Result<Option<String>>` returning the chosen coordinate. The prompt API today is `pub(crate) fn named(names: &[&str]) -> Vec<Choice>` and `pub(crate) fn choose(prompt: &str, choices: &[Choice], default: usize) -> dialoguer::Result<usize>`, both re-exported from `crate::output::prompts`. Exactly one stack -> `Ok(Some(coordinate))`. Several stacks and interactive -> `let names: Vec<&str> = detection.stacks.iter().map(|s| s.name.as_str()).collect();` then `let picked = ctx.suspend_progress(|| crate::output::prompts::choose("Stack", &crate::output::prompts::named(&names), 0))?;` and return that stack's coordinate. No stack and interactive -> `let names: Vec<&str> = super::detect::STACKS.iter().map(|row| row.name).chain(["no stack"]).collect();`, choose with default 3; index 3 yields `Ok(None)`, any other index that STACKS row's coordinate. Not interactive -> `Err(StackChoiceRefusal { code, headline }.into())` with `ambiguous_stack` for several stacks and `no_stack_detected` for none.
4. In stack_choice.rs write `pub(super) fn run(ctx: &crate::output::Context, args: &crate::cli::InitArgs, project_path: &std::path::Path, user_config: &mut vibe_core::user_config::UserConfig, interactive: bool) -> anyhow::Result<()>`: `let root = super::helpers::canonical_no_unc(project_path)?;` when `root.join(vibe_core::manifest::Manifest::FILENAME).exists()` bail with exactly this text:
```
detect_stack_existing_manifest: `vibe.toml` already exists in this directory; --detect-stack starts a new project and never rewrites an authored manifest (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization; fix: `vibe init --detect-stack --plan-only`)
```
then `let detection = super::detect::detect(&root)?;`, `let chosen = select(ctx, &detection, interactive)?;`, `let created = super::create_project(ctx, args, project_path, user_config, interactive)?;` (args.stack is None here because clap rejects --stack with --detect-stack, so no InitArgs literal is built and no [active] table is written), then when `chosen` is `Some(coordinate)` `ctx.hint(&format!("next: vibe install stack:{coordinate}"));`, then `created.report(ctx)`. Do not call std::env::current_dir in this file.
5. In mod.rs add `mod stack_choice;` and change the `InitMode::Project` arm of the match in `run` to call `stack_choice::run(ctx, &args, &project_path, &mut user_config, interactive)?;` when `args.detect_stack`, and the existing `create_project(ctx, &args, &project_path, &mut user_config, interactive)?.report(ctx)?;` otherwise. In proposal.rs add field `next_commands: Vec<String>` filled in `new`: empty when `root.join("vibe.toml")` exists; else with one stack `vec!["vibe init --detect-stack".to_string(), format!("vibe install stack:{coordinate}")]`; with several stacks one `format!("vibe install stack:{coordinate}")` per stack in order; with none empty. Human mode adds `ctx.hint(&format!("next: {command}"))` per command before the trailer.
6. Add tests. `detect_apply_initializes_without_active_and_keeps_claude_text`: fixture `Cargo.toml` (as in the scan test) + `CLAUDE.md`=`# Notes\nmine\n`; plan JSON `next_commands` == `["vibe init --detect-stack","vibe install stack:org.vibevm.ai-native/rust-ai-native"]`; run `vibe init --detect-stack --path <dir>` -> success, stdout contains `next: vibe install stack:org.vibevm.ai-native/rust-ai-native`, vibe.toml parsed as `toml::Table` has no key `active`, and for CLAUDE.md `text.split_once("</vibevm>")` is Some and its rest, after `trim_start_matches('\n')`, equals `# Notes\nmine\n`. `detect_apply_refuses_existing_manifest`: fixture `Cargo.toml`, `user.init_project(dir)`, read vibe.toml bytes, run `vibe init --detect-stack --path <dir>` -> failure, stderr contains `already exists`, vibe.toml bytes unchanged. `detect_apply_refuses_ambiguous_stack_unattended`: fixture `Cargo.toml` + `package.json`=`{}\n`; run `vibe init --detect-stack --unattended --path <dir>` -> failure, stderr contains `ambiguous_stack`, `rust-ai-native` and `typescript-ai-native`, and `vibe.toml` does not exist.

**Decisions already made.**
- `vibe init --detect-stack` never writes `[active].stack`: init refuses an `[active].stack` that names no installed stack (tests/cli_init.rs init_stack_flag_sets_active_stack), and init never installs. The detected stack only chooses the printed next command.
- next_commands and the `next:` hint describe installing the stack, not activating it; no command today writes [active].stack into an existing vibe.toml.
- Refusal codes are snake_case (`ambiguous_stack`, `no_stack_detected`, `detect_stack_existing_manifest`), matching the codebase's existing error codes (`lock_hash_mismatch`).
- Attended means the existing `interactive` value in run (`console::user_attended() && !ctx.is_json() && !ctx.is_unattended()`); JSON and unattended runs never prompt. Detection and the choice run before create_project, so a refusal writes nothing.
- With several stacks the plan lists one install command per stack; the apply run refuses unattended rather than guessing which to name.
- The next command is a human hint only; the success JSON stays today's cli-init-report unchanged, because adding a member there needs JTD codegen with the gitignored tools/jtd-codegen binary.
- An existing vibe.toml is refused before detection; `--plan-only` still works there and reports empty next_commands.
- `code` and `headline` are read by the thiserror Display, so there is no dead-field warning; the candidates list is added in ADV-SDD-DETECT-STACK-4.

**Not in this entry.** Not in this entry: running `vibe install`, recording or activating `[active].stack` (after install or otherwise, a separate capability), stamping the JSON error envelope, adding next_commands to the InitReport wire schema, and detection for `init package` or `init group`.

**Depends on.** ADV-SDD-DETECT-STACK-2

**Spec edit (apply as a step of this entry).**

None. stack_choice.rs uses the existing anchor spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization through specmark::scope!.

Entries that write the same file run one after another, never in parallel.
