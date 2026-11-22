# ADV-CONTROL-NOTERM-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** With stdin closed, `vibe install -g` and `vibe app modify` that lack an answer exit 2 with `error_kind: needs_input` and a `missing` list naming exactly that answer, and leave the settings home unchanged.

**Write.** `crates/vibe-cli/src/output/structured.rs`, `crates/vibe-cli/tests/cli_app_needs_input.rs`, `vibevm/vibespecs/common/PROP-065-structured-installation.xml`

**Check.** `cargo test -p vibe-cli --test cli_app_needs_input -- --exact every_missing_answer_refuses_with_its_flag_and_leaves_the_home 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** New file crates/vibe-cli/tests/cli_app_needs_input.rs (`#![cfg(target_os = "linux")]`, `mod common;`, uses common::app_registry::{AppRegistry, ok}), fn every_missing_answer_refuses_with_its_flag_and_leaves_the_home. For each refused run it asserts: exit code Some(2); the stderr line parsed as JSON with ok==false has the expected `code`, `error_kind`=="needs_input" and `missing` equal to the exact json! array; and the settings-home snapshot taken before the run equals the one after. Runs (NAME = "org.example/asker", each via apps.app(verb, NAME, extra)): (1) install -g, extra ["--json"] -> code selection_required, missing [{"kind":"option","id":"installer","flag":"--installer"}]; (2) extra ["--installer","main","--json"] -> license_required, [{"kind":"license","id":"Asker License 1.0","flag":"--accept-license"}]; (3) extra ["--installer","main","--accept-license","--json"] -> parameter_invalid, [{"kind":"parameter","id":"port","flag":"--set port=VALUE"}]; (4) extra ["--installer","main","--accept-license","--set","port=8080","--json"] -> ok(&output) succeeds; (5) verb ["app","modify"], extra ["--installer","alt","--json"] -> parameter_invalid, [{"kind":"secret","id":"token","flag":"--set token=VALUE"}]. The check passes only when this one test runs and passes.

**Read first.**
- crates/vibe-cli/src/output/structured.rs:1-50
- crates/vibe-cli/tests/common/app_registry.rs:1-160
- crates/vibe-cli/tests/cli_app_modify.rs:1-120
- crates/vibe-cli/tests/cli_app_wizard.rs:274-296
- crates/vibe-app/src/missing.rs

**Steps.**
1. In crates/vibe-cli/src/output/structured.rs, function stamp_app_error, directly after the `map.insert("code".into(), ...)` line, add a comment `// PROP-065 #needs-input-envelope: the answers a client must supply.` and: `let missing = error.missing_answers(); if !missing.is_empty() { map.insert("error_kind".into(), Value::String("needs_input".into())); map.insert("missing".into(), Value::Array(missing.iter().map(|answer| { let mut row = serde_json::json!({"kind": answer.kind.as_str(), "id": answer.id, "flag": answer.flag}); if let Some(key) = &answer.file_key { row["file_key"] = Value::String(key.clone()); } row }).collect())); }`.
2. Create crates/vibe-cli/tests/cli_app_needs_input.rs with a module doc comment naming PROP-065 #needs-input-envelope and #attended, `#![cfg(target_os = "linux")]`, `mod common;`, and a fixture function `asker(apps: &AppRegistry)` that calls apps.package("asker", "1.0.0", "asker", BODY), then apps.script(&package, "asker", "ASKER"), then fs::write(package.join("LICENSE.txt"), "Be kind.\n").
3. Add helper `refused(output: &Output) -> (Option<i32>, Value)` copied from cli_app_wizard.rs lines 286-294 (first stderr line that parses as JSON with ok==false).
4. Add helper `home(apps: &AppRegistry) -> BTreeMap<PathBuf, Vec<u8>>`: recursive walk of apps.settings() (empty map when it does not exist), key = path relative to settings, value = file bytes, or the read_link target bytes for a symlink (use symlink_metadata), skipping any entry whose file name is `.mutation.lock`.
5. Add the test fn with the five runs listed in the test field; take `let before = home(&apps);` before each refused run and assert_eq!(before, home(&apps)) after it. Insert the spec_edit line into PROP-065.
6. Run the check command; it must print `test result: ok. 1 passed`.

**Decisions already made.**
- The envelope keeps `code`; `error_kind` is exactly "needs_input"; `missing` rows carry `kind`, `id`, `flag`, and `file_key` only when present (omitted, not null).
- BODY (a Rust raw string formatted with arch = std::env::consts::ARCH; the helper already writes the first `[[installer_options]]` id="main" header): `[installer_options.license]` file="LICENSE.txt" name="Asker License 1.0"; `[[installer_options.parameters]]` id="port" name="Port" type="integer" required=true; `[[installer_options.steps]]` id="place" title="Place" kind="copy" from="payload"; `[[installer_options.launchers]]` id="main" name="Asker" executable="$app_bin/asker" kind="command"; then `[[installer_options]]` id="alt" name="Alt" platform="linux" platform_name="Linux" arch="{arch}"; `[[installer_options.parameters]]` id="token" name="Token" type="string" required=true secret=true; the same place step and the same launcher.
- Refusal order was verified against the current binary: no --installer -> selection_required; main without --accept-license -> license_required even when port is missing; main with --accept-license and no port -> parameter_invalid on port; modify to alt -> parameter_invalid on token; each refused run changes nothing in the settings root except creating applications/.mutation.lock.
- std::process::Command::output() gives the child a closed stdin, so AppRegistry::app already runs with no terminal; do not add a PTY.
- The JSON-envelope sentences live in this entry's spec anchor `needs-input-envelope`, because only this entry makes the envelope carry them.
- Commit: `feat(app): list the missing answers in the JSON refusal` with a body saying the Library, CI or a remote node can read which flag to add instead of parsing the message.

**Not in this entry.** Adding `missing` to the per-row refusals of `vibe app import` (registered wire schema application_import) or to the text output.

**Depends on.** ADV-CONTROL-NOTERM-2

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/common/PROP-065-structured-installation.xml: insert one new line directly after the line that starts with `  <p><needs-input fact="true"` (added by NOTERM-2):
  <p><needs-input-envelope fact="true" status="spec/done">When a refusal names missing answers (`#needs-input`), its JSON error envelope keeps `code` and also carries `"error_kind": "needs_input"` and `missing`: one object per missing answer with `kind`, `id`, `flag` and, only when present, `file_key`. A client supplies what is named and runs the same command again. The per-row refusals of `vibe app import` and the human text output do not carry it.</needs-input-envelope></p>

Entries that write the same file run one after another, never in parallel.
