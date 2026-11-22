# ADV-NATIVE-GLOBAL-ECO-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe app extras install SLOT ITEM... [--ecosystem ID] [--json]` passes exactly those items to the adapter's install verb in the slot's current version and answers `vibe-app-ecosystem/1`. It edits no settings, leaves the installation's `applied` extras record unchanged, and when the verb fails it exits 1 with a retry of `vibe app extras install SLOT --ecosystem ID ITEM...`.

**Write.** `crates/vibe-app/src/ecosystem.rs`, `crates/vibe-cli/src/cli/app_extras.rs`, `crates/vibe-cli/src/commands/app/extras.rs`, `crates/vibe-cli/tests/cli_app_extras_global.rs`, `specmap.json`

**Check.** `cargo test -p vibe-cli --test cli_app_extras_global -- --exact an_explicit_install_runs_the_install_verb_with_exactly_those_items 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_app_extras_global.rs :: an_explicit_install_runs_the_install_verb_with_exactly_those_items (new file; Linux-only like its siblings via #![cfg(all(test, target_os = "linux"))])

**Read first.**
- crates/vibe-app/src/ecosystem.rs (fn run_all, fn remember, struct Target with given_items from entry 1)
- crates/vibe-app/src/ecosystem/extras.rs (fn remember: the Install arm writes applied)
- crates/vibe-cli/src/cli/app_extras.rs (enum ExtrasCommand, struct ExtrasTargetArgs)
- crates/vibe-cli/src/commands/app/extras.rs (fn run, fn verb, fn report_run)
- crates/vibe-cli/tests/cli_app_extras_apply.rs (fns option_body, runtime, verb_script, install, extras)
- crates/vibe-cli/tests/common/app_registry.rs (AppRegistry::package, script, app, vibe, home, settings, registry)

**Steps.**
1. This entry runs after ADV-NATIVE-GLOBAL-ECO-1, which also edits crates/vibe-app/src/ecosystem.rs. In `fn run_all` there, replace the single call `remember(ctx, &mut index, &key, &document, &held)?;` with:
```
    // An install of named items is not an apply: the applied record keeps the last apply.
    if !(target.verb == EcosystemVerb::Install && target.given_items) {
        remember(ctx, &mut index, &key, &document, &held)?;
    }
```
This matches `remove`, whose rows `extras::remember` already skips.
2. In crates/vibe-cli/src/cli/app_extras.rs, add a variant to `enum ExtrasCommand` between `Import` and `Remove`:
```
    /// Install items into the slot's current version now, through the
    /// adapter's `install` verb. The items are opaque: they are handed to
    /// the verb as separate argv elements, exactly as written.
    ///
    /// This edits no settings; `vibe app extras add` is what makes every
    /// new version of the slot get an item.
    Install {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The items to install, verbatim; at least one.
        #[arg(required = true, value_name = "ITEM")]
        items: Vec<String>,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },
```
3. In crates/vibe-cli/src/commands/app/extras.rs `fn run`, add the arm `ExtrasCommand::Install { slot, items, target } => verb(ctx, settings, &slot, EcosystemVerb::Install, &items, &target, embedded_root, offline),` before the `Remove` arm. Change the doc line of `fn verb` to `/// \`vibe app extras install|remove|self-upgrade SLOT\`: one verb of one or every`. Nothing else in this file changes. `--no-recommends` with `install` is already refused by crates/vibe-cli/src/composition.rs (it admits it only for Apply and List); leave that as is.
4. Create crates/vibe-cli/tests/cli_app_extras_global.rs. Start with `//! Global packages of a runtime's own ecosystem through the real \`vibe\` binary (PROP-075 \`#SLOT-EXTRAS\`): an install of named items, and the home-wide listing.`, then `#![cfg(all(test, target_os = "linux"))]`, `mod common;`, `use std::fs;`, `use std::path::{Path, PathBuf};`, `use common::app_registry::{AppRegistry, json, ok};`. Copy `option_body()` and `verb_script(root, name, body)` verbatim from cli_app_extras_apply.rs, and `extras(r, verb, args)` verbatim too (it appends slot `runtime`, `--registry <registry>` and `--offline`). Add this fixture exactly; the `r.script` call must stay because option_body()'s copy step reads payload/, which only `r.script` creates:
```
fn runtime(r: &AppRegistry, version: &str) -> PathBuf {
    let root = r.package("runtime", version, "runtime", option_body());
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mut declared = String::from(
        "[application]\nid = \"runtime\"\n\n[[application.ecosystem]]\n\
         id = \"npm\"\nname = \"npm packages\"\nmigration = false\n\n",
    );
    for verb in ["install", "list", "remove"] {
        declared.push_str(&format!(
            "[application.ecosystem.{verb}]\nid = \"{verb}\"\ntitle = \"Verb {verb}\"\n\
             kind = \"recipe\"\nscript = \"eco/{verb}.sh\"\n\n"
        ));
    }
    fs::write(
        &manifest,
        text.replace("[application]\nid = \"runtime\"\n", &declared),
    )
    .unwrap();
    // option_body()'s copy step needs payload/, which this creates.
    r.script(&root, "rt", &format!("rt-{version}"));
    verb_script(
        &root,
        "install",
        "for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"$HOME/install.argv\"; \
         printf '%s\\n' \"$a\" >> \"$HOME/installed\"; done\n",
    );
    verb_script(
        &root,
        "list",
        "if [ -f \"$HOME/installed\" ]; then cp \"$HOME/installed\" \"$VIBE_ECOSYSTEM_LIST\"; fi\n",
    );
    verb_script(&root, "remove", "exit 0\n");
    root
}

fn install(r: &AppRegistry, version: &str) {
    ok(&r.app(
        &["install", "-g"],
        &format!("org.example/runtime@{version}"),
        &["--offline", "--unattended", "--json"],
    ));
}
```
The adapter has no recommends, so `install -g` hands the install verb no items, the adapter row is skipped, and no `applied` record exists afterwards.
5. Add the test `an_explicit_install_runs_the_install_verb_with_exactly_those_items` with `#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]`. In order: (a) `let r = AppRegistry::new(); let root = runtime(&r, "1.0.0"); install(&r, "1.0.0");`. (b) `let document = json(&extras(&r, "install", &["typescript", "pnpm@9", "--json"]));`; assert `document["protocol"] == "vibe-app-ecosystem/1"`, `document["verb"] == "install"`, `document["ok"] == true`; find the row with id `npm` in `document["adapters"]` and assert `row["outcome"] == "done"` and `row["items"] == serde_json::json!(["typescript", "pnpm@9"])`. (c) Assert `fs::read_to_string(r.home().join("install.argv")).unwrap() == "typescript\npnpm@9\n"`. (d) `let listing = json(&extras(&r, "list", &["--json"]));`; find the row with id `npm` in `listing["ecosystems"]` and assert `row.get("applied").is_none()` (this proves the `run_all` guard of step 1) and `row["observed"] == serde_json::json!(["typescript", "pnpm@9"])`. (e) Assert `fs::read_to_string(r.settings().join("config.toml")).unwrap_or_default()` does not contain `typescript`. (f) `verb_script(&root, "install", "exit 3\n");` then `let failed = extras(&r, "install", &["eslint", "--json"]);`; assert `failed.status.code() == Some(1)`, parse `failed.stdout` with `serde_json::from_slice::<serde_json::Value>`, and assert the npm adapter row has `outcome == "failed"` and `retry == "vibe app extras install runtime --ecosystem npm eslint"`.
6. Run `cargo fmt --all`, then `cargo xtask specmap`, then the check. Then run `cargo clippy -p vibe-app -p vibe-cli --all-targets -- -D warnings`. A fixture fn or import the test file does not use is a dead-code/unused error: delete it rather than adding `#[allow]`.

**Decisions already made.**
- Verb name `install`, variant `ExtrasCommand::Install { slot, items, target }`, items required (at least one), same `--ecosystem`/`--registry` flags as `remove` via `ExtrasTargetArgs`.
- An Install run with given items does not call `remember`, so `ExtrasRecord.applied`, `applied_at` and `applied_outcome` keep describing the last apply and no inventory publish happens, as for `remove`. Remove, SelfUpgrade, List and apply behaviour is unchanged.
- It reuses `fn verb` and `report_run` unchanged, so the human and JSON output are those of `remove`: JSON `vibe-app-ecosystem/1`, exit `Some(i32::from(!document.ok))`.
- No consent prompt: installing a named item is what the user typed, exactly as `remove` today. The consent contract belongs to ADV-CONTROL.
- New test file crates/vibe-cli/tests/cli_app_extras_global.rs; cli_app_extras.rs and cli_app_extras_apply.rs are not grown.

**Not in this entry.** Not in this entry: the home-wide listing (entry 3), changing `apply`/`add`, a `--save` flag that also writes the user's add list, an `upgrade ITEM` verb, or refactoring the duplicated test fixtures into tests/common.

**Depends on.** ADV-NATIVE-GLOBAL-ECO-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
