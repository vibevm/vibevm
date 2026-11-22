# ADV-CONTROL-CODES-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Under --json, a command line clap cannot parse prints the envelope {ok:false,error,code:"usage",refusal:true} on stderr with clap's exit code 2. --help and --version stay plain text.

**Write.** `crates/vibe-cli/src/cli.rs`, `crates/vibe-cli/tests/cli_usage_envelope.rs`, `VIBEVM-SPEC.md`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_usage_envelope usage_envelope`

**Test.** New file crates/vibe-cli/tests/cli_usage_envelope.rs (uses `mod common; use common::UserScratch;` and `UserScratch::new().vibe()`). (1) a_mistyped_flag_under_json_is_a_usage_envelope: `vibe install --no-such-flag --json` exits 2, stdout is empty, and stderr trimmed is exactly one line that parses as JSON with ok==false, code=="usage", refusal==true and error containing "--no-such-flag". (2) help_under_json_is_not_a_usage_envelope: `vibe --json --help` exits 0, and stdout is non-empty and does not start with '{'. (3) a_mistyped_flag_without_json_prints_no_usage_envelope: `vibe install --no-such-flag` exits 2, stderr does not start with '{', and it contains "--no-such-flag".

**Read first.**
- crates/vibe-cli/src/cli.rs:100-130,286-316
- crates/vibe-cli/tests/cli_lifecycle_lease.rs:1-60

**Steps.**
1. In cli.rs add a private fn `fn exit_on_parse_error(error: clap::Error, json: bool, protected: bool) -> !`. If error.kind() is DisplayHelp or DisplayVersion it calls error.exit(). Otherwise the message is the existing protected sentence when `protected`, else `error.render().to_string().trim_end().to_owned()`. With `json` it calls `eprintln!("{}", serde_json::json!({"ok": false, "error": message, "code": "usage", "refusal": true}))` and exits with `std::process::exit(if protected { 2 } else { error.exit_code() })`. Without `json`, a protected error keeps today's eprintln + exit(2) and any other error calls error.exit().
2. In parse_invocation, compute `let json = args.iter().skip(1).take_while(|a| *a != "--").any(|a| a == "--json");` from the raw OsString args before they are moved. Route both `try_get_matches_from(args).unwrap_or_else(...)` and `Self::from_arg_matches(&matches).unwrap_or_else(...)` through exit_on_parse_error(error, json, protected). The second call passes protected = false.
3. Write the three tests in the new file.
4. Apply the spec_edit to VIBEVM-SPEC.md.
5. Run `cargo fmt --all`, the check command, and `cargo clippy -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- --json is detected by exact string equality with "--json" among the raw args after argv[0] and before a literal "--". The bool flag takes no value, so the `--json=...` spelling is not considered.
- The envelope has exactly four members: ok, error, code, refusal. invoked_by and unattended are not stamped, because no output::Context exists yet.
- Every clap error kind other than DisplayHelp and DisplayVersion is a usage envelope under --json. That includes MissingSubcommand (for example `vibe --json` alone, which today prints "'vibe' requires a subcommand" with exit 2) and DisplayHelpOnMissingArgumentOrSubcommand.
- The protected `vibe secrets` sentence is kept word for word as `error` under --json, so a rejected secret value is never echoed.
- The exit code is clap's own error.exit_code(), which is 2 for usage errors and is unchanged.

**Not in this entry.** Not in this entry: building an output::Context or routing through exit_code/classify, adding a `kind` field from clap's ErrorKind, and changing human parse-error text.

**Spec edit (apply as a step of this entry).**

VIBEVM-SPEC.md §9.3 {#output-format}: append this paragraph at the end of the section (after the paragraph that begins "Installation and self-update display live overall"):

Under `--json`, a command line that does not parse is written on stderr as the JSON envelope `{"ok":false,"error":…,"code":"usage","refusal":true}` with exit `2`; `--help` and `--version` stay plain text.

Entries that write the same file run one after another, never in parallel.
