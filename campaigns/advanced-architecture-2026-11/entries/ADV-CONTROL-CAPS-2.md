# ADV-CONTROL-CAPS-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** vibe-doc can walk a command tree whose help screens a caller supplies, through `surface::commands::tree_from`, and `tree(env)` becomes that same walk fed by `--help` runs of a binary, with every error message byte-identical to today.

**Write.** `crates/vibe-doc/src/surface/commands.rs`, `crates/vibe-doc/tests/surface_tree_from.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-doc --test surface_tree_from`

**Test.** crates/vibe-doc/tests/surface_tree_from.rs: (1) `tree_from_walks_supplied_help_screens`: a closure answers path [] with the root help screen and path ["build"] with the build help screen from the decisions. The test asserts that the result paths are exactly ["vibe", "vibe build"] (the `help` subcommand is skipped), that `result[1].summary == "Render a package"`, that `result[1].flags[0].name == "--format"` and `result[1].flags[0].value == "FORMAT"`, and that `result[0].flags[0].name == "--json"`. (2) `tree_from_names_the_command_whose_help_failed`: the closure returns `Err("boom".to_string())` for path ["build"]. The test asserts that the returned error's `to_string()` contains "`vibe build --help`: boom". Before the work the test target does not exist.

**Read first.**
- crates/vibe-doc/src/surface/commands.rs:1-130
- crates/vibe-doc/src/surface.rs:61-103
- crates/vibe-doc/src/error.rs:200-215

**Steps.**
1. In crates/vibe-doc/src/surface/commands.rs add `pub fn tree_from(root: &str, help: &mut dyn FnMut(&[String]) -> std::result::Result<String, String>) -> Result<Vec<SurfaceCommand>>`. It creates `out` and `seen` exactly as `tree` does today and calls `walk(help, root, &[], String::new(), &mut out, &mut seen)`.
2. Change `walk`'s first parameter from `env: &SurfaceEnv` to `help: &mut dyn FnMut(&[String]) -> std::result::Result<String, String>`. Replace the line `let help = help_of(env, path, &spelling)?;` with a call `help(path)` whose `Err(message)` becomes `DocError::Surface { message: format!("`{spelling} --help`: {message}") }` (bind the screen to a new local name such as `screen`, since `help` is now the parameter). The recursive call passes `help` on. MAX_DEPTH, the `seen` check and the HELP_SUBCOMMAND skip are unchanged.
3. Today `help_of` is `fn help_of(env: &SurfaceEnv, path: &[String], spelling: &str) -> Result<String>`, and it builds the prefix inside a local `fail` closure. Change it to `fn help_of(env: &SurfaceEnv, path: &[String]) -> std::result::Result<String, String>`. Delete the `spelling` parameter and the `fail` closure. The spawn error becomes `Err(format!("cannot run `{}`: {e}", env.binary.display()))`, and the non-zero exit becomes `Err(format!("exited {}: {}", output.status.code().unwrap_or(-1), String::from_utf8_lossy(&output.stderr).trim()))`. Both texts are exactly today's texts without the prefix. The Command setup and the Ok value stay unchanged.
4. Make `tree(env)` call `tree_from(&program_name(env), &mut |path| help_of(env, path))`.
5. Create crates/vibe-doc/tests/surface_tree_from.rs with the two tests. The help screens are `&str` constants in the file.
6. Run the check, then run `cargo test -p vibe-doc` once to confirm the existing surface tests still pass.

**Decisions already made.**
- Function name and signature exactly: `pub fn tree_from(root: &str, help: &mut dyn FnMut(&[String]) -> std::result::Result<String, String>) -> Result<Vec<SurfaceCommand>>` in module `vibe_doc::surface::commands`.
- Root help fixture: "Do things.\n\nUsage: vibe [OPTIONS] <COMMAND>\n\nCommands:\n  build  Render a package\n  help   Print this message\n\nOptions:\n      --json  Machine output\n  -h, --help  Print help\n"
- Build help fixture: "Render a package.\n\nUsage: vibe build [OPTIONS]\n\nOptions:\n      --format <FORMAT>  Which projection\n  -h, --help             Print help\n"
- Any other path given to the fixture closure returns Err(format!("unexpected path {path:?}")).
- The final error text of `tree(env)` stays byte-identical: `` `{spelling} --help`: cannot run `...`: ... `` and `` `{spelling} --help`: exited N: ... ``. The prefix is now added in `walk`, not in `help_of`.
- `tree`, `parse`, `Help` keep their current behaviour and visibility; `help_of` stays private.

**Not in this entry.** Not in this entry: changing the help parser, the snapshot format, the env handling of the `--help` child, or adding a clap dependency to vibe-doc.

Entries that write the same file run one after another, never in parallel.
