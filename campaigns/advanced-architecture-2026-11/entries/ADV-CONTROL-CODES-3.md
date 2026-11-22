# ADV-CONTROL-CODES-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Under --json, an unanswered remap offer's envelope carries remedy.args ["--accept-remap=<key>", …] and an unaccepted license's envelope carries remedy.args ["--accept-license"].

**Write.** `crates/vibe-cli/src/exit_code/classify.rs`, `crates/vibe-cli/src/exit_code/classify/tests.rs`, `crates/vibe-cli/src/commands/remap/consent.rs`, `crates/vibe-cli/src/commands/remap/uninstall.rs`, `crates/vibe-cli/tests/cli_remap_packages.rs`, `crates/vibe-cli/tests/cli_app_dependencies.rs`, `VIBEVM-SPEC.md`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe --test cli_remap_packages --test cli_app_dependencies remedy_args`

**Test.** Unit tests in crates/vibe-cli/src/exit_code/classify/tests.rs: (1) remedy_args_of_unanswered_remap_offers: classify(&anyhow::Error::from(ConsentRequired{count:1, what:"remap package offer(s)", packages:"org.corp/intranet".into(), accept:..., reject:..., remedy_args: vec!["--accept-remap=org.corp/intranet".into()]}).context("wrapped")) gives code "consent_required", exit 2, refusal true, remedy == Some(json!({"args":["--accept-remap=org.corp/intranet"]})). (2) remedy_args_of_a_license_refusal: classify(&anyhow::Error::from(vibe_app::AppError::from(vibe_app::StepError::LicenseRequired{key:"k".into(), license:"L".into(), file:"LICENSE".into()}))) gives code "license_required", exit 2, refusal true, remedy == Some(json!({"args":["--accept-license"]})). CLI tests: (3) tests/cli_remap_packages.rs remedy_args_of_an_unanswered_offer_under_json: uses the same World setup as an_unattended_run_refuses_an_unanswered_offer_and_names_the_verb, then runs world.install(&user, "org.example/remapped", &["--unattended", "--json"]). It asserts exit 2, and that the last non-empty stderr line parses with code=="consent_required", refusal==true and remedy.args==["--accept-remap=org.corp/intranet"]. (4) tests/cli_app_dependencies.rs remedy_args_of_an_unaccepted_license_under_json: uses the setup of a_licensed_requirement_is_accepted_like_the_package_that_requires_it, then runs apps.app(&["install","-g"], "org.example/consumer", &["--assume-yes","--json"]). It asserts exit 2, and that the last non-empty stderr line parses with code=="license_required", refusal==true and remedy.args==["--accept-license"].

**Read first.**
- crates/vibe-cli/src/exit_code/classify.rs
- crates/vibe-cli/src/commands/remap/consent.rs:76-200
- crates/vibe-cli/src/commands/remap/uninstall.rs:80-100
- crates/vibe-app/src/error.rs:95-160
- crates/vibe-cli/tests/cli_remap_packages.rs:200-320
- crates/vibe-cli/tests/cli_app_dependencies.rs:320-360

**Steps.**
1. Add the field `pub(crate) remedy_args: Vec<String>` to `ConsentRequired` in commands/remap/consent.rs, with a doc line saying it holds the flags that answer yes on the same command line. The Display text stays unchanged.
2. Fill the field at the three construction sites. Both offer sites in consent.rs (resolve and preview) use `keys.iter().map(|k| format!("--accept-remap={k}")).collect()`. The orphaned-row site in commands/remap/uninstall.rs uses `vec!["--keep-remap".to_owned()]`.
3. In classify.rs, the ConsentRequired arm finds the cause with downcast_ref and sets `remedy: Some(serde_json::json!({"args": consent.remedy_args}))`. In the app arm, when `app_err.code() == "license_required"`, it sets `remedy: Some(json!({"args": ["--accept-license"]}))`.
4. Write the two unit tests and the two CLI tests named in `test`. Each CLI test file gets a private helper that parses the last non-empty stderr line as JSON.
5. Apply the spec_edit to VIBEVM-SPEC.md.
6. Run `cargo fmt --all`, the check command, and `cargo clippy -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- remedy is an object, `{"args": [...]}`, where args are flags to add to the same command line, in order.
- A remap acceptance uses the `--accept-remap=<key>` form (one element per key) rather than two separate tokens, because the flag takes 0..=1 values and a separated key could be read as a positional.
- An orphaned-row consent's remedy is `["--keep-remap"]`.
- License remedy is matched on the typed app error's code `license_required`. Prose is never matched.
- The rule of ADV-CONTROL-CODES-1 holds: classify.rs and its tests never glob-import `super::*`.
- The human refusal text and the stdout offer document are not changed.

**Not in this entry.** Not in this entry: remedies for any other code (offline, migration_consent_required, agent consent_required, prune_consent_required), and the ambiguous-package candidates (ADV-CONTROL-CODES-4).

**Depends on.** ADV-CONTROL-CODES-1

**Spec edit (apply as a step of this entry).**

VIBEVM-SPEC.md §9.3 {#output-format}: append to the paragraph that begins "Under `--json`, a failed command's last stderr line is one JSON error envelope" this sentence:

When the fix is a flag the envelope also carries `remedy.args`, the flags to add to the same command line (for example `["--accept-license"]`).

Entries that write the same file run one after another, never in parallel.
