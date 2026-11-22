# ADV-CONTROL-CAPS-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe capabilities` lists the closed set of named features. Three unit tests prove that each feature's backing command line parses, and that the stop-file features appear exactly when the stop-file admission (`composition::cancellation::supported`) accepts their command.

**Write.** `crates/vibe-cli/src/commands/capabilities.rs`, `crates/vibe-cli/src/commands/capabilities/features.rs`

**Check.** `cargo test -p vibe-cli --bin vibe capabilities::features::tests 2>&1 | grep -q 'test result: ok. 3 passed'`

**Test.** Inline `#[cfg(test)] mod tests` at the bottom of crates/vibe-cli/src/commands/capabilities/features.rs, using the `probe` helper from the decisions: (1) `every_feature_probe_parses`: for every row of FEATURES and every argv in it, `probe(prefix, argv).is_ok()`, where prefix is `&[]` for Parses rows and `STOP_PREFIX` for StopFile rows. (2) `stop_file_features_follow_cancellation_support`: for every StopFile row, `advertised().contains(&name.to_string())` equals `argv.iter().all(|a| probe(STOP_PREFIX, a).map(|cli| supported(&cli.command)).unwrap_or(false))`. A parse error counts as not supported and does not panic. (3) `todays_features_are_advertised`: `advertised()` equals exactly ["project-plan-only", "source-policy", "activation", "self-import-no-profile", "stop-file:project"]. Before the work the module does not exist, so cargo prints `0 passed` and the grep in the check fails. The check passes only when exactly these three tests run and pass.

**Read first.**
- crates/vibe-cli/src/composition/cancellation.rs:1-36
- crates/vibe-cli/src/cli.rs:105-120
- crates/vibe-cli/src/cli/pkg.rs:85-120
- crates/vibe-cli/src/cli/mcp.rs:240-264
- crates/vibe-cli/src/cli/vvm.rs:139-175
- crates/vibe-cli/src/commands/capabilities.rs

**Steps.**
1. Create crates/vibe-cli/src/commands/capabilities/features.rs with `pub(crate) enum Probe { Parses, StopFile }`, `pub(crate) struct Feature { pub name: &'static str, pub probe: Probe, pub argv: &'static [&'static [&'static str]] }`, `const STOP_PREFIX: &[&str] = &["--stop-file", "stop"];`, the const FEATURES from the decisions, and the `probe` helper from the decisions.
2. Add `pub(crate) fn advertised() -> Vec<String>`. It keeps FEATURES order. A Parses row is included when `probe(&[], argv).is_ok()` for every argv. A StopFile row is included when, for every argv, `probe(STOP_PREFIX, argv)` is Ok and `crate::composition::cancellation::supported(&cli.command)` is true.
3. In crates/vibe-cli/src/commands/capabilities.rs add `mod features;` and set `features: features::advertised()` in place of `Vec::new()`.
4. Add the inline tests module with the three tests at the bottom of features.rs.
5. Run the check. Then run `cargo test -p vibe-cli --test cli_capabilities` once to confirm ADV-CONTROL-CAPS-3's tests still pass.

**Decisions already made.**
- The probe helper, used by `advertised()` and by all three tests: `fn probe(prefix: &[&str], argv: &[&str]) -> Result<crate::cli::Cli, clap::Error> { <crate::cli::Cli as clap::Parser>::try_parse_from(std::iter::once("vibe").chain(prefix.iter().copied()).chain(argv.iter().copied())) }`. `--stop-file` is a global flag on Cli (crates/vibe-cli/src/cli.rs), so it is valid before the subcommand.
- FEATURES, in this order: ("project-plan-only", Parses, [["install","--plan-only"]]); ("source-policy", Parses, [["install","--prefer-hosting","github"],["install","--exclude-hosting","github"]]); ("activation", Parses, [["mcp","enable","vibevm","--agent","claude-code"],["skill","enable","example","--package","org.example/skill","--agent","claude-code"]]); ("self-import-no-profile", Parses, [["self","import","./vibe","--tag","1.0.0","--no-profile"]]); ("stop-file:project", StopFile, [["install"],["update"]]); ("stop-file:app", StopFile, [["install","-g","example"]]).
- `stop-file:app` stays in the table and is computed. Today `supported()` returns false for `install -g example` (global without user_project), so it is not advertised. When the app-verb stop lands, it appears without a code change here.
- The table is the single home of feature names. Later capabilities (operation records, stable codes) add rows here.
- The check pipes through `grep -q 'test result: ok. 3 passed'` because cargo exits 0 when a filter matches no test. Keep exactly three tests in this module so the count matches.
- Tests stay inline in features.rs, so no `#[path]` attribute is needed. The module path must be `commands::capabilities::features::tests`.

**Not in this entry.** Not in this entry: changing `cancellation::supported()` or making `install -g` accept --stop-file (that is the app-verb stop capability), adding features for things that do not exist yet (operation-records), adding a fourth test to the module, or human-output changes.

**Depends on.** ADV-CONTROL-CAPS-3

Entries that write the same file run one after another, never in parallel.
