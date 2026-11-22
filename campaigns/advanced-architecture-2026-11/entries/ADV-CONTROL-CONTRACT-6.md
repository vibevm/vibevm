# ADV-CONTROL-CONTRACT-6

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A Linux vibe-cli test runs the Library's application verbs with the Library's exact flags. Each stdout document conforms to its registered format, and every stderr line of an install, update or uninstall run conforms to vibe-progress-event/1.

**Write.** `crates/vibe-cli/tests/client_contract_app.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test client_contract_app library_application_verbs_conform_to_their_registered_formats`

**Test.** crates/vibe-cli/tests/client_contract_app.rs (#![cfg(target_os = "linux")]): library_application_verbs_conform_to_their_registered_formats. With the fixture from the decisions it runs in order and asserts common::wire::conforms on the single stdout document: `app describe org.example/editor --registry R --json` → application_describe::ApplicationDescribe; `app plan org.example/editor --registry R --json` → application_plan::ApplicationPlan; `install -g org.example/editor --registry R` + LIBRARY_RUN → application_report::ApplicationReport; `app list --json` → application_list::ApplicationList; `app launchers --all --json` → application_launchers_all::ApplicationLaunchersAll; `app launchers --revision --json` → application_revision::ApplicationRevision; `app running --json` → application_running::ApplicationRunning; `app outdated --json` → application_outdated::ApplicationOutdated; `app runs org.example/editor --json` → application_runs::ApplicationRuns; `app work --json` → application_work::ApplicationWork; `app errors --json` → application_errors::ApplicationErrors; `app export --json` → apps_manifest::AppsManifest; `app alternatives editor --json` → application_alternatives::ApplicationAlternatives; `uninstall -g org.example/editor --plan-only --json` → application_removal::ApplicationRemoval; `update -g org.example/editor --registry R` + LIBRARY_RUN → ApplicationReport; `transfers --json` → transfers_listing::TransfersListing; `uninstall -g org.example/editor` + LIBRARY_RUN → ApplicationReport. For the three LIBRARY_RUN runs it also asserts that stderr has at least one non-empty line and that every non-empty stderr line, parsed as JSON, conforms to progress_event::ProgressEvent.

**Read first.**
- crates/vibe-cli/tests/common/app_registry.rs:1-165
- crates/vibe-cli/tests/cli_app_catalog.rs:1-60
- crates/vibe-cli/tests/common/library.rs
- crates/vibe-cli/tests/common/wire.rs

**Steps.**
1. Create crates/vibe-cli/tests/client_contract_app.rs. Start it with #![cfg(target_os = "linux")] and `mod common;`, then import common::app_registry::{AppRegistry, ok}, common::library::LIBRARY_RUN and common::wire::{documents, conforms}.
2. Write fn fixture() -> AppRegistry: `let apps = AppRegistry::new(); let package = apps.package("editor", "1.0.0", "editor", OPTION); apps.script(&package, "edit", "EDIT_RAN"); apps`, with OPTION as given in the decisions.
3. Write fn one(apps: &AppRegistry, args: &[&str]) -> (serde_json::Value, String). It runs apps.vibe(args) and takes stdout through ok(). It asserts that documents(stdout) has exactly one element and returns that element with the stderr text.
4. Write the test described in `test`, building the registry argument from apps.registry().
5. Run the check and cargo fmt -p vibe-cli.

**Decisions already made.**
- OPTION = "[[installer_options.steps]]\nid = \"place\"\ntitle = \"Place the payload\"\nkind = \"copy\"\nfrom = \"payload\"\n\n[[installer_options.launchers]]\nid = \"edit\"\nname = \"Editor\"\nexecutable = \"$app_bin/edit\"\nkind = \"command\"\ndefault = true\n". It is a copy step plus one launcher, with no shell and no network.
- Each verb's argument vector is the Library's own (from its invocation tables): reading verbs take `--json` alone, and runs take LIBRARY_RUN. `--registry R` is passed where the Library passes a registry (describe, plan, install, update).
- The test is gated to Linux like the 53 existing cli_app_* test files: AppRegistry writes platform = "linux" options.
- With the built vibe of 2026-11-22, all 17 of these documents and all progress lines already conform. This entry only adds the test that keeps them conforming; its check passes as soon as the file exists.
- `app installers` is deliberately left out here; ADV-CONTROL-CONTRACT-7 adds it together with its fix.

**Not in this entry.** Not in this entry: `app installers` (entry 7), the other Library verbs (start, stop, pin, unpin, collection, launcher, modify, import), and any change under crates/*/src.

**Depends on.** ADV-CONTROL-CONTRACT-1

Entries that write the same file run one after another, never in parallel.
