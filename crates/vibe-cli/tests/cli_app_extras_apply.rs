//! The user's own extras list end to end through the real `vibe` binary:
//! `[slot.<slot>.extras.<ecosystem>]` in the scratch `config.toml`, the
//! apply after `install -g` of a new version, the two decline flags, the
//! settings editors, and the failure that never touches the runtime
//! (PROP-075 `#SLOT-EXTRAS`).
//!
//! **Isolation is structural, not careful.** Every run here goes through
//! `UserScratch`, so `$VIBE_SETTINGS` is a temp directory and the
//! `config.toml` under test is the one this file wrote. Nothing reaches
//! the operator's real `~/.vibe/config.toml`, and nothing reaches a real
//! ecosystem tool: the adapter's verbs are fixture recipes that write
//! their `"$@"` into a file.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, json, ok};
use serde_json::Value;

/// The package body: a payload and one public command, so the adapter's
/// verbs can prove which version's PATH they ran under.
fn option_body() -> &'static str {
    r#"
path_dirs = ['bin']
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
"#
}

/// Two recommendations: `corepack` chosen by default, `typescript` not.
const RECOMMENDS: &str = r#"
[[application.ecosystem.recommends]]
ecosystem = "npm"
id = "corepack"
item = "corepack@latest"
default = true
reason = "Package-manager shims"

[[application.ecosystem.recommends]]
ecosystem = "npm"
id = "typescript"
item = "typescript"
name = "TypeScript"
"#;

/// One adapter with an `install` and a `list` verb. `install` appends its
/// argv — one element per line, with the version it ran in — to
/// `$HOME/install.argv`, so a test can tell one apply from the next.
fn runtime(r: &AppRegistry, version: &str) -> PathBuf {
    let root = r.package("runtime", version, "runtime", option_body());
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mut declared = format!(
        "[application]\nid = \"runtime\"\n\n[[application.ecosystem]]\n\
         id = \"npm\"\nname = \"npm packages\"\nmigration = false\n{RECOMMENDS}\n"
    );
    for verb in ["install", "list"] {
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
    r.script(&root, "rt", &format!("rt-{version}"));
    verb_script(
        &root,
        "install",
        // One line per argv element, prefixed with the version the verb
        // ran in, appended. `|` is the separator because the payload's own
        // `rt` echoes its (empty) arguments after the marker.
        "for a in \"$@\"; do printf '%s|%s\\n' \"$(rt)\" \"$a\"; done \
         >> \"$HOME/install.argv\"\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"$HOME/installed\"; done\n",
    );
    verb_script(
        &root,
        "list",
        "if [ -f \"$HOME/installed\" ]; then cp \"$HOME/installed\" \"$VIBE_ECOSYSTEM_LIST\"; fi\n",
    );
    root
}

fn verb_script(root: &Path, name: &str, body: &str) {
    let dir = root.join("eco");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.sh"));
    fs::write(&path, format!("#!/bin/sh\nset -e\n{body}")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Write the user's own `config.toml` in the scratch settings dir — the
/// only `config.toml` any run in this file can see.
fn config(r: &AppRegistry, body: &str) {
    fs::write(r.settings().join("config.toml"), body).unwrap();
}

fn config_text(r: &AppRegistry) -> String {
    fs::read_to_string(r.settings().join("config.toml")).unwrap_or_default()
}

/// `vibe install -g org.example/runtime@<version>` with `extra` flags.
fn install(r: &AppRegistry, version: &str, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["--offline", "--unattended"];
    args.extend_from_slice(extra);
    r.app(
        &["install", "-g"],
        &format!("org.example/runtime@{version}"),
        &args,
    )
}

/// `vibe app extras <verb> runtime …` against the fixture registry.
fn extras(r: &AppRegistry, verb: &str, args: &[&str]) -> std::process::Output {
    let registry = r.registry();
    let mut argv = vec!["app", "extras", verb, "runtime"];
    argv.extend_from_slice(args);
    argv.extend_from_slice(&["--registry", registry.to_str().unwrap(), "--offline"]);
    r.vibe(&argv)
}

/// What the `install` verb was handed, across every run, as
/// `<version> <item>` lines.
fn applied(r: &AppRegistry) -> Vec<String> {
    fs::read_to_string(r.home().join("install.argv"))
        .unwrap_or_default()
        .lines()
        .map(|line| match line.split_once('|') {
            Some((version, item)) => format!("{} {item}", version.trim()),
            None => line.to_owned(),
        })
        .collect()
}

fn ecosystem_of<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["ecosystems"]
        .as_array()
        .expect("ecosystems")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no ecosystem `{id}` in {document:#}"))
}

/// One installation as the inventory retains it.
fn retained(r: &AppRegistry, key: &str) -> Value {
    r.index()["installations"][key].clone()
}

/// The whole arithmetic, through the binary: the author's enabled
/// recommendation, minus the drop, plus the user's `add` — applied to the
/// version `install -g` placed, and to no other (PROP-075 `#SLOT-EXTRAS`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_users_list_is_applied_after_an_install_of_a_new_version_and_not_on_a_reinstall() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0");
    runtime(&r, "2.0.0");
    config(
        &r,
        "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\", \"corepack@latest\"]\ndrop = [\"typescript\"]\n",
    );
    ok(&install(&r, "1.0.0", &["--json"]));
    // `corepack` is the enabled recommendation; `typescript` is dropped
    // and is not chosen anyway; the user's `pnpm@9` follows, and
    // `corepack@latest` is not repeated — exact string, first wins.
    assert_eq!(
        applied(&r),
        ["rt-1.0.0 corepack@latest", "rt-1.0.0 pnpm@9"],
        "the final list, in order, in the version just installed"
    );
    // The inventory remembers what was applied, so `describe` can say so
    // without running anything.
    let record = &retained(&r, "org.example/runtime@1.0.0")["extras"][0];
    assert_eq!(record["id"], "npm");
    assert_eq!(
        record["applied"],
        serde_json::json!(["corepack@latest", "pnpm@9"])
    );
    assert_eq!(record["appliedOutcome"], "done");
    assert!(record["appliedAt"].is_string(), "{record:#}");
    // The home had to reach `/8` to hold that record, and no further.
    assert_eq!(r.index()["protocol"], "vibe-user-applications/8");

    // A reinstall of a version already installed applies nothing: the
    // list goes into a *new* version, once.
    ok(&install(&r, "1.0.0", &["--json", "--force"]));
    assert_eq!(applied(&r).len(), 2, "a reinstall applies nothing");

    // A second version is a new version, so the list goes into it too —
    // and into *it*, which is what `rt-2.0.0` proves.
    ok(&install(&r, "2.0.0", &["--json"]));
    assert_eq!(
        applied(&r),
        [
            "rt-1.0.0 corepack@latest",
            "rt-1.0.0 pnpm@9",
            "rt-2.0.0 corepack@latest",
            "rt-2.0.0 pnpm@9",
        ]
    );
}

/// The two flags are not the same flag, and a `drop` is honoured in every
/// case — including under `--no-extras`, which is the clause the owner's
/// wording left ambiguous and the design decided (PROP-075 `#SLOT-EXTRAS`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn no_extras_skips_only_the_users_add_and_no_recommends_only_the_authors() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0");
    config(
        &r,
        "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\"]\ndrop = [\"corepack\"]\n",
    );
    // `--no-extras` drops the user's `add`; their `drop` still removes the
    // author's `corepack`, so the list is empty and the adapter does not
    // run at all.
    ok(&install(&r, "1.0.0", &["--json", "--no-extras"]));
    assert!(
        applied(&r).is_empty(),
        "a drop is honoured under --no-extras: {:?}",
        applied(&r)
    );
    let listing = json(&extras(&r, "list", &["--json", "--no-extras"]));
    let row = ecosystem_of(&listing, "npm");
    assert_eq!(row["recommendations"][0]["id"], "corepack");
    assert_eq!(row["recommendations"][0]["enabled"], true);
    assert_eq!(
        row["recommendations"][0]["dropped"], true,
        "the listing says dropped, not unchosen: {row:#}"
    );
    assert_eq!(row["items"], serde_json::json!([]));
    assert_eq!(row["add"], serde_json::json!(["pnpm@9"]));
    assert_eq!(row["drop"], serde_json::json!(["corepack"]));
    // Without the flag the user's own item comes back, and the preview and
    // the apply agree.
    let listing = json(&extras(&r, "list", &["--json"]));
    assert_eq!(
        ecosystem_of(&listing, "npm")["items"],
        serde_json::json!(["pnpm@9"])
    );
    ok(&extras(&r, "apply", &["--json"]));
    assert_eq!(applied(&r), ["rt-1.0.0 pnpm@9"]);

    // `--no-recommends` is the global flag of PROP-077 and declines the
    // author's half here too; the user's own list is theirs and stays.
    let r = AppRegistry::new();
    runtime(&r, "1.0.0");
    config(&r, "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\"]\n");
    ok(&install(&r, "1.0.0", &["--json", "--no-recommends"]));
    assert_eq!(applied(&r), ["rt-1.0.0 pnpm@9"]);
    // And with both, nothing at all.
    let r = AppRegistry::new();
    runtime(&r, "1.0.0");
    config(&r, "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\"]\n");
    ok(&install(
        &r,
        "1.0.0",
        &["--json", "--no-recommends", "--no-extras"],
    ));
    assert!(applied(&r).is_empty());
}

/// A failing `install` verb leaves the version installed, the slot default
/// exactly where it was, the install's exit status at 0, one warning
/// naming the retry command, and the outcome in the record (PROP-075
/// `#SLOT-EXTRAS`). An extra is an extra.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_failing_apply_never_touches_the_runtime_and_names_its_retry() {
    let r = AppRegistry::new();
    let root = runtime(&r, "1.0.0");
    verb_script(&root, "install", "exit 3\n");
    config(&r, "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\"]\n");
    let output = install(&r, "1.0.0", &[]);
    assert!(
        output.status.success(),
        "a failing extras install must not fail the install the user asked for: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let shown = String::from_utf8_lossy(&output.stderr).into_owned()
        + &String::from_utf8_lossy(&output.stdout);
    assert!(
        shown.contains("vibe app extras apply runtime --ecosystem npm"),
        "the exact retry command is named: {shown}"
    );
    // The runtime is installed and is still the slot's default.
    assert_eq!(r.listed("org.example/runtime@1.0.0")["default"], true);
    // And the record says what was not carried, so a later `describe`
    // still does.
    let record = &retained(&r, "org.example/runtime@1.0.0")["extras"][0];
    assert_eq!(record["appliedOutcome"], "failed");
    assert_eq!(
        record["applied"],
        serde_json::json!(["corepack@latest", "pnpm@9"]),
        "what was asked for is kept, not what arrived: {record:#}"
    );
    // The retry is a command of its own, and it fails on its own terms —
    // exit status follows the run the user asked for by name.
    let retry = extras(&r, "apply", &["--json"]);
    assert_eq!(retry.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&retry.stdout).unwrap();
    assert_eq!(document["ok"], false);
    assert_eq!(document["adapters"][0]["outcome"], "failed");
    assert_eq!(
        document["adapters"][0]["retry"],
        "vibe app extras apply runtime --ecosystem npm"
    );
    assert!(Path::new(document["work"].as_str().unwrap()).exists());
}

/// `vibe app extras add|drop|import` edit the user's own file and install
/// nothing; `import` reads one item per line with `#` comments and blank
/// lines ignored — the format of `$NVM_DIR/default-packages` (PROP-075
/// `#SLOT-EXTRAS`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_settings_editors_write_the_users_own_file_and_an_import_reads_item_lines() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0");
    config(&r, "# my own notes\n[net]\noffline = false\n");
    ok(&install(&r, "1.0.0", &["--json"]));
    assert_eq!(applied(&r), ["rt-1.0.0 corepack@latest"]);

    // The ecosystem is inferred: the slot declares exactly one adapter, so
    // the nvm-shaped command line needs no `--ecosystem`.
    ok(&extras(&r, "add", &["pnpm@9", "; rm -rf $HOME"]));
    let body = config_text(&r);
    assert!(body.contains("# my own notes"), "{body}");
    assert!(body.contains("[slot.runtime.extras.npm]"), "{body}");
    assert!(
        body.contains("; rm -rf $HOME"),
        "the item is verbatim: {body}"
    );
    // Nothing was installed by the edit.
    assert_eq!(applied(&r).len(), 1, "an edit installs nothing");

    ok(&extras(&r, "drop", &["corepack"]));
    // A repeat adds nothing, and the file stays valid.
    ok(&extras(&r, "add", &["pnpm@9"]));

    let file = r.home().join("default-packages");
    fs::write(
        &file,
        "# npm globals\n\n  typescript  \n@scope/pkg@^1.2\n\n# done\n",
    )
    .unwrap();
    ok(&extras(&r, "import", &[file.to_str().unwrap()]));

    let listing = json(&extras(&r, "list", &["--json"]));
    let row = ecosystem_of(&listing, "npm");
    assert_eq!(
        row["add"],
        serde_json::json!(["pnpm@9", "; rm -rf $HOME", "typescript", "@scope/pkg@^1.2"]),
        "the user's own order, each item once: {row:#}"
    );
    assert_eq!(row["drop"], serde_json::json!(["corepack"]));
    // The final list is the user's, with the dropped recommendation gone.
    assert_eq!(
        row["items"],
        serde_json::json!(["pnpm@9", "; rm -rf $HOME", "typescript", "@scope/pkg@^1.2"])
    );
    // And applying it hands the recipe exactly those argv elements —
    // `$HOME` intact, nothing evaluated.
    ok(&extras(&r, "apply", &["--json"]));
    assert_eq!(
        applied(&r),
        [
            "rt-1.0.0 corepack@latest",
            "rt-1.0.0 pnpm@9",
            "rt-1.0.0 ; rm -rf $HOME",
            "rt-1.0.0 typescript",
            "rt-1.0.0 @scope/pkg@^1.2",
        ]
    );
    assert!(r.home().exists(), "$HOME was not evaluated away");
}

/// A stale line in a hand-written settings file is reported once and
/// otherwise ignored, and `vibe app describe` says what the installed
/// version carries without running anything (PROP-075 `#SLOT-EXTRAS`,
/// `AV-FMT-032`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_stale_settings_line_is_reported_and_describe_answers_off_the_record() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0");
    config(
        &r,
        "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\"]\ndrop = [\"gone\"]\n\
         [slot.runtime.extras.cargo]\nadd = [\"ripgrep\"]\n\
         [slot.python.extras.pip]\nadd = [\"wheel\"]\n",
    );
    // The install still happens, and the extras it can make sense of are
    // applied: a stale line never refuses anything.
    ok(&install(&r, "1.0.0", &["--json"]));
    assert_eq!(applied(&r), ["rt-1.0.0 corepack@latest", "rt-1.0.0 pnpm@9"]);
    let listing = json(&extras(&r, "list", &["--json"]));
    let notices: Vec<&str> = listing["notices"]
        .as_array()
        .expect("notices")
        .iter()
        .map(|line| line.as_str().unwrap())
        .collect();
    assert_eq!(notices.len(), 3, "{notices:#?}");
    for named in ["`python`", "`cargo`", "`gone`"] {
        assert!(
            notices.iter().any(|line| line.contains(named)),
            "{named} is not named in {notices:#?}"
        );
    }

    // `describe` carries what the record holds, having run nothing.
    let described = json(&r.app(
        &["app", "describe"],
        "org.example/runtime@1.0.0",
        &["--json", "--offline"],
    ));
    let adapter = &described["application"]["ecosystems"][0];
    assert_eq!(adapter["id"], "npm");
    assert_eq!(
        adapter["applied"],
        serde_json::json!(["corepack@latest", "pnpm@9"])
    );
    assert!(adapter["appliedAt"].is_string(), "{adapter:#}");
    // The observation is the one `extras list` took above.
    assert_eq!(
        adapter["observed"],
        serde_json::json!(["corepack@latest", "pnpm@9"])
    );
}

/// A home with no `[slot]` table behaves exactly as it did before this
/// phase, and a package with no adapter at all says nothing (PROP-075
/// `#SLOT-EXTRAS`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_home_with_no_slot_table_is_unchanged_and_a_package_with_no_adapter_is_silent() {
    let r = AppRegistry::new();
    // No `config.toml` at all: the author's enabled recommendation is the
    // whole list, exactly as P7b left it.
    runtime(&r, "1.0.0");
    ok(&install(&r, "1.0.0", &["--json"]));
    assert_eq!(applied(&r), ["rt-1.0.0 corepack@latest"]);

    // A package declaring no adapter: the install is silent about extras
    // and the home stays on the protocol it was writing.
    let r = AppRegistry::new();
    r.package("plain", "1.0.0", "plain", option_body());
    let root = r.registry().join("org.example/plain/v1.0.0");
    r.script(&root, "plainrt", "plain-1.0.0");
    let output = r.app(
        &["install", "-g"],
        "org.example/plain@1.0.0",
        &["--offline", "--unattended"],
    );
    ok(&output);
    let shown = String::from_utf8_lossy(&output.stderr);
    assert!(
        !shown.contains("extras"),
        "a package with no adapter says nothing about extras: {shown}"
    );
    assert_eq!(
        r.index()["protocol"],
        "vibe-user-applications/6",
        "no adapter record and no weak edge, so the home stays at the protocol its \
         exposed payload directory already needed — not `/8`"
    );
    assert!(
        r.index()["installations"]["org.example/plain@1.0.0"]["extras"].is_null(),
        "{:#}",
        r.index()
    );
}
