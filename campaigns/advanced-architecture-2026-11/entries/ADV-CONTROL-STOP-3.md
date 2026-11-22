# ADV-CONTROL-STOP-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe update -g`, `vibe uninstall -g`, `vibe app modify` and `vibe app import` accept `--stop-file` instead of refusing it, and one table in composition/cancellation.rs (`stop_route`) says, for every command, whether the project token or the application steps honour the file.

**Write.** `crates/vibe-cli/src/composition/cancellation.rs`, `crates/vibe-cli/src/composition/cancellation/tests.rs`, `crates/vibe-cli/src/cli.rs`, `vibevm/vibespecs/common/PROP-065-structured-installation.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe the_application_verbs_take_the_stop_file_through_their_steps`

**Test.** crates/vibe-cli/src/composition/cancellation/tests.rs. Replace the helper `supported_for` with `fn route_for(argv: &[&str]) -> Option<StopRoute> { let cli = Cli::try_parse_from(argv).unwrap(); stop_route(&cli.command) }`; no `supported_for` remains. New fn `the_application_verbs_take_the_stop_file_through_their_steps` asserts: route_for([vibe, update, -g, org.example/x]) == Some(StopRoute::Steps); route_for([vibe, uninstall, -g, org.example/x]) == Some(Steps); route_for([vibe, app, modify, org.example/x]) == Some(Steps); route_for([vibe, app, import, set.toml]) == Some(Steps) (the file `set.toml` need not exist: only clap parsing runs, and `file` is required unless `--resume` is given); route_for([vibe, install, -g, org.example/x]) == Some(StopRoute::Token); route_for([vibe, install, -g, mcp:org.example/x]) == None; route_for([vibe, uninstall, org.example/x]) == None. It also asserts that `for_command(&ctx, &Cli::try_parse_from([vibe, update, -g, org.example/x]).unwrap().command, Some(PathBuf::from("/tmp/stop")))` is Ok, with ctx built as in the existing tests. Rewrite `both_user_verbs_take_the_stop_source` so that `user init` and `user status` each give route_for(...) == Some(StopRoute::Token). Rewrite `the_existing_commands_keep_their_decisions` in terms of route_for: `install`, `update`, `dl`, `tool px exec -- true` give Some(Token); `init`, `list`, `check` and `version` give None; `install -g app:example` now gives Some(Token), and `update -g` gives Some(Steps). `the_gate_diagnostic_names_the_supported_commands` keeps its four substrings and also asserts the error contains "app modify/import". `a_supported_command_without_a_stop_file_still_gets_a_source` stays unchanged.

**Read first.**
- crates/vibe-cli/src/composition/cancellation.rs:1-145
- crates/vibe-cli/src/composition/cancellation/tests.rs:1-60
- crates/vibe-cli/src/cli/pkg.rs:400-412
- crates/vibe-cli/src/cli/pkg/installed.rs:50-62
- crates/vibe-cli/src/cli/app.rs:260-270
- crates/vibe-cli/src/cli/app_portable.rs:15-30

**Steps.**
1. In cancellation.rs, add `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub(crate) enum StopRoute { /// The operation's cancellation token polls the file. Token, /// The application interrupt owner (`commands::app::interrupt::Signals`) polls the file during and between steps; it reads the path from `Context::stop_file`. Steps }`.
2. Replace `pub(crate) fn supported(command: &Command) -> bool` with `pub(crate) fn stop_route(command: &Command) -> Option<StopRoute>`, using these rows. Install(args): `if args.global && !args.user_project && args.packages.iter().any(|p| p.starts_with("mcp:")) { None } else { Some(Token) }`. Update(args): `if !args.global || args.user_project { Some(Token) } else if args.packages.iter().any(|p| p.starts_with("mcp:")) { None } else { Some(Steps) }`. Uninstall(args): `(args.global && !args.user_project && !args.package.starts_with("mcp:")).then_some(Steps)`. App(args): `matches!(args.command, AppCommand::Modify { .. } | AppCommand::Import(_)).then_some(Steps)`. The rows for Registry Publish, Dl, Tool, User and Mcp(non-serve) map to Some(Token) with their current conditions and comments kept. Everything else is None.
3. Rewrite `for_command` as a match on `stop_route(command)`. Some(Token) returns `Ok(attach(ctx, stop))`. Some(Steps) and None-without-a-stop-path return `Ok((ctx.clone(), CancellationToken::new(), StopInput::none()))`. None with a stop path bails with exactly `--stop-file is supported only for project install/update, application install/update/uninstall -g, app modify/import, registry publish, dl, tool and user init/status`. Delete the `inferred` variable; the Install row now covers it.
4. In crates/vibe-cli/src/cli.rs, change the doc comment on `stop_file` to `/// Request a cooperative stop of this operation by creating this file. Vibe never deletes it.`
5. Apply the spec_edit. Update tests.rs as described in the test field. Run the check, then `cargo test -p vibe-cli --bin vibe composition::cancellation`, `cargo test -p vibe-cli --test cli_app_stop_file` (must still pass) and `cargo fmt -p vibe-cli`. Use CARGO_TARGET_DIR=/data/git/vibevm-architect-target.

**Decisions already made.**
- The table is named `stop_route` and returns `Option<StopRoute>` with the variants `Token` and `Steps`. It replaces `supported()`; no wrapper named `supported` remains. This is the single table a later `vibe capabilities` reads.
- The application verbs (update -g, uninstall -g, app modify, app import) get no project token and no watch or question. The stop file reaches them only through `Signals`, which reads `Context::stop_file`; ADV-CONTROL-STOP-2 adds that, so this entry must land after it.
- `install -g` stays on the Token route (the earlier `inferred` classification). Its application branch hands off to `Signals` through the existing `bind_legacy`.
- Global MCP install/update/uninstall (`mcp:` packages) still refuse `--stop-file`.
- The refusal text is exactly: `--stop-file is supported only for project install/update, application install/update/uninstall -g, app modify/import, registry publish, dl, tool and user init/status`.

**Not in this entry.** Adding a `vibe capabilities` command or output, honouring the stop file for non-application packages that `update -g` / `uninstall -g` touch, accepting `app resume`, or end-to-end tests of each verb.

**Depends on.** ADV-CONTROL-STOP-2

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/common/PROP-065-structured-installation.xml, unit `#cancellation`. Replace the exact text added by ADV-CONTROL-STOP-2 ``--stop-file` is the same interrupt for `install -g`, honoured` with ``--stop-file` is the same interrupt for `install -g`, `update -g`, `uninstall -g`, `vibe app modify` and `vibe app import`, honoured`.

Entries that write the same file run one after another, never in parallel.
