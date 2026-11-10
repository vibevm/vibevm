//! `vibe app extras` and the ecosystem adapter end to end through the real
//! `vibe` binary and a local directory registry (PROP-075
//! `#ECOSYSTEM-ADAPTER`, `#ECOSYSTEM-RECOMMENDS`, `#ECOSYSTEM-MIGRATION`,
//! `#SLOT-EXTRAS`).
//!
//! The fixture is a runtime package declaring one adapter whose four verbs
//! are real recipes: they write their `"$@"` and their environment into
//! files the test reads. Nothing here goes near a real ecosystem tool —
//! vibe core knows no ecosystem word, and neither does this suite beyond
//! the opaque token it uses as an adapter id.
//!
//! **The injection test is this phase's own gate**: an item whose text is
//! `; rm -rf $HOME`, and one carrying a newline, a quote and a `$(…)`,
//! must reach the script as exactly those argv elements with nothing
//! evaluated.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

/// The separator the fixture recipes put after every argument, so an item
/// carrying a newline round-trips exactly.
const CUT: &str = "\n--8<--\n";

/// The package body: a payload, one public command, and the two features
/// the recommendations must **not** collide with.
fn option_body() -> &'static str {
    r#"
path_dirs = ['bin']
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='Runtime'
executable='$app_bin/rt'
kind='command'
default=true
"#
}

/// One adapter declaring all four verbs as package-relative recipes, plus
/// whatever `extra` adds (`migration = false`, recommendations …).
fn adapter(extra: &str) -> String {
    let mut text = format!(
        "\n[[application.ecosystem]]\nid = \"npm\"\nname = \"npm packages\"\n\
         description = \"The runtime's own packages\"\n{extra}\n"
    );
    for (verb, script) in [
        ("install", "install"),
        ("list", "list"),
        ("remove", "remove"),
        ("self_upgrade", "upgrade"),
    ] {
        let kind = "recipe";
        text.push_str(&format!(
            "[application.ecosystem.{verb}]\nid = \"{script}\"\n\
             title = \"Verb {verb}\"\nkind = \"{kind}\"\nscript = \"eco/{script}.sh\"\n\n"
        ));
    }
    text
}

/// The two recommendations: one chosen by default, one not.
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

/// Write `org.example/runtime@<version>` declaring `extra` on its adapter,
/// with the four verb scripts in place.
fn runtime(r: &AppRegistry, version: &str, extra: &str) -> PathBuf {
    let root = r.package("runtime", version, "runtime", option_body());
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let declared = format!("[application]\nid = \"runtime\"\n{}", adapter(extra));
    fs::write(
        &manifest,
        text.replace("[application]\nid = \"runtime\"\n", &declared),
    )
    .unwrap();
    r.script(&root, "rt", &format!("rt-{version}"));
    // Every verb records its argv, its environment and the version it ran
    // in. `$@` is written with a separator after each element, so an item
    // carrying a newline is compared exactly.
    for verb in ["install", "remove", "upgrade"] {
        verb_script(
            &root,
            verb,
            &format!(
                "for a in \"$@\"; do printf '%s--8<--\\n' \"$a\"; done > \"$HOME/{verb}.argv\"\n"
            ),
        );
    }
    // The `list` verb's answer is the file, never stdout: the script also
    // writes a notice to stdout, which must not reach the item list.
    verb_script(
        &root,
        "list",
        "echo 'npm notice: a manager writes notices'\n\
         if [ -f \"$HOME/installed\" ]; then cp \"$HOME/installed\" \"$VIBE_ECOSYSTEM_LIST\"; fi\n",
    );
    root
}

/// A package-relative adapter script with the shared preamble: it records
/// the ecosystem environment and the version it ran in, then does `body`.
fn verb_script(root: &Path, name: &str, body: &str) {
    let dir = root.join("eco");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.sh"));
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nset -e\n\
             printf 'id=%s count=%s out=%s sources=%s root=%s who=%s pwd=%s\\n' \
               \"$VIBE_ECOSYSTEM_ID\" \"$VIBE_ECOSYSTEM_ITEM_COUNT\" \
               \"${{VIBE_OUT-unset}}\" \"${{VIBE_SOURCES-unset}}\" \"$VIBE_APP_ROOT\" \
               \"$(rt)\" \"$(pwd)\" > \"$HOME/{name}.facts\"\n\
             printf '%s' \"$VIBE_ECOSYSTEM_ITEMS\" > \"$HOME/{name}.items\"\n\
             {body}"
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn install(r: &AppRegistry, version: &str) {
    ok(&r.app(
        &["install", "-g"],
        &format!("org.example/runtime@{version}"),
        &["--offline", "--unattended", "--json"],
    ));
}

/// `vibe app extras <verb> runtime …` with the fixture registry.
fn extras(r: &AppRegistry, verb: &str, args: &[&str]) -> std::process::Output {
    let registry = r.registry();
    let mut argv = vec!["app", "extras", verb, "runtime"];
    argv.extend_from_slice(args);
    argv.extend_from_slice(&["--registry", registry.to_str().unwrap(), "--offline"]);
    r.vibe(&argv)
}

/// The argv one verb recorded, split back into its exact elements.
fn argv(r: &AppRegistry, verb: &str) -> Vec<String> {
    let text = fs::read_to_string(r.home().join(format!("{verb}.argv"))).unwrap();
    text.split(CUT.trim_start_matches('\n'))
        .map(|part| part.trim_end_matches('\n').to_owned())
        .filter(|part| !part.is_empty())
        .collect()
}

/// One installation as the **inventory** retains it: the chosen feature
/// set and the declared migration entries live there, not in the listing.
fn retained(r: &AppRegistry, key: &str) -> Value {
    r.index()["installations"][key].clone()
}

fn ecosystem_of<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["ecosystems"]
        .as_array()
        .expect("ecosystems")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no ecosystem `{id}` in {document:#}"))
}

fn adapters_of<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["adapters"]
        .as_array()
        .expect("adapters")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no adapter `{id}` in {document:#}"))
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn items_reach_the_recipe_as_exactly_those_argv_elements_and_nothing_is_evaluated() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "");
    install(&r, "1.0.0");
    // Three items a shell would mangle if anything ever spliced them into
    // a command string: a command separator, a substitution inside
    // quotes, and one carrying a real newline.
    let items = [
        "; rm -rf $HOME",
        "a\"b$(touch $HOME/pwned)c",
        "two\nlines",
        "@scope/pkg@^1.2",
    ];
    let mut args: Vec<&str> = items.to_vec();
    args.push("--json");
    let output = extras(&r, "remove", &args);
    let document = json(&output);
    assert_eq!(document["protocol"], "vibe-app-ecosystem/1");
    assert_eq!(document["ok"], true);
    // The argv the script saw is the input list, element for element.
    assert_eq!(argv(&r, "remove"), items);
    // And the record carries them verbatim too.
    assert_eq!(
        adapters_of(&document, "npm")["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect::<Vec<_>>(),
        items
    );
    // Nothing was evaluated: `$HOME` is intact and `$(…)` ran nowhere.
    assert!(r.home().join("remove.argv").exists());
    assert!(
        !r.home().join("pwned").exists(),
        "a command substitution inside an item was evaluated"
    );
    // `VIBE_ECOSYSTEM_ITEMS` carries the same list, one per line, and the
    // count is the number of elements and not of lines.
    let env_items = fs::read_to_string(r.home().join("remove.items")).unwrap();
    assert_eq!(
        env_items,
        format!("{}\n", items.join("\n")),
        "VIBE_ECOSYSTEM_ITEMS"
    );
    let facts = fs::read_to_string(r.home().join("remove.facts")).unwrap();
    assert!(facts.contains("id=npm count=4"), "{facts}");
    // An adapter produces no payload, so there is no `$VIBE_OUT` and no
    // `$VIBE_SOURCES`; the PATH leads with the configured installation, so
    // `rt` is that version's; and the working directory is the run's own.
    assert!(facts.contains("out=unset sources=unset"), "{facts}");
    assert!(facts.contains("who=rt-1.0.0"), "{facts}");
    assert!(
        facts.contains(&format!("pwd={}", document["work"].as_str().unwrap())),
        "{facts}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn the_list_verbs_answer_is_the_file_and_a_missing_one_is_an_empty_list() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "");
    install(&r, "1.0.0");
    // No answer file at all: an empty list, reported as such, never an
    // error — and the manager's stdout notice is not an item.
    let document = json(&extras(&r, "list", &["--json"]));
    assert_eq!(document["protocol"], "vibe-app-extras/1");
    let row = ecosystem_of(&document, "npm");
    assert!(row["observed"].is_null() || row["observed"].as_array().unwrap().is_empty());
    assert!(row["observed_failed"].is_null(), "{row:#}");
    assert_eq!(
        row["verbs"].as_array().unwrap().len(),
        4,
        "all four verbs are declared"
    );
    // Now the verb has something to say: lines are trimmed, the empty ones
    // dropped, and nothing else is parsed.
    fs::write(
        r.home().join("installed"),
        "  typescript  \n\nnpm notice: ignored by nobody\n@scope/pkg@^1.2\n\n",
    )
    .unwrap();
    let document = json(&extras(&r, "list", &["--json"]));
    assert_eq!(
        ecosystem_of(&document, "npm")["observed"],
        serde_json::json!([
            "typescript",
            "npm notice: ignored by nobody",
            "@scope/pkg@^1.2"
        ])
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
fn a_recommendation_is_one_installer_feature_and_the_listing_reads_the_chosen_set() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", RECOMMENDS);
    // The synthesised rows are ordinary features: an unattended install
    // takes the defaults, and `--feature` names one of them.
    install(&r, "1.0.0");
    assert_eq!(
        retained(&r, "org.example/runtime@1.0.0")["features"],
        serde_json::json!(["corepack"])
    );
    let document = json(&extras(&r, "list", &["--json"]));
    let row = ecosystem_of(&document, "npm");
    let names: Vec<&str> = row["recommendations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["corepack", "typescript"]);
    assert_eq!(row["recommendations"][0]["enabled"], true);
    assert_eq!(row["recommendations"][0]["item"], "corepack@latest");
    assert_eq!(row["recommendations"][1]["enabled"], false);
    assert_eq!(row["recommendations"][1]["name"], "TypeScript");
    assert_eq!(row["items"], serde_json::json!(["corepack@latest"]));
    // `--no-feature` / `--feature` turn one recommendation off and the
    // other on, with no new vocabulary at all.
    ok(&r.app(
        &["app", "modify"],
        "org.example/runtime@1.0.0",
        &[
            "--no-feature",
            "corepack",
            "--feature",
            "typescript",
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    let document = json(&extras(&r, "list", &["--json"]));
    assert_eq!(
        ecosystem_of(&document, "npm")["items"],
        serde_json::json!(["typescript"])
    );
    // And the whole choice is visible without running anything.
    let described = json(&r.app(
        &["app", "describe"],
        "org.example/runtime@1.0.0",
        &["--json", "--offline"],
    ));
    let declared = &described["application"]["ecosystems"][0];
    assert_eq!(declared["id"], "npm");
    assert_eq!(declared["migration"], true);
    assert_eq!(declared["recommends"][0]["item"], "corepack@latest");
    let features: Vec<&str> = described["options"][0]["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert_eq!(features, ["corepack", "typescript"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn self_upgrade_takes_no_items_and_an_undeclared_verb_is_refused_by_name() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "");
    install(&r, "1.0.0");
    let document = json(&extras(
        &r,
        "self-upgrade",
        &["--ecosystem", "npm", "--json"],
    ));
    assert_eq!(document["verb"], "self_upgrade");
    assert_eq!(adapters_of(&document, "npm")["outcome"], "done");
    assert!(
        adapters_of(&document, "npm")["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let facts = fs::read_to_string(r.home().join("upgrade.facts")).unwrap();
    assert!(facts.contains("id=npm count=0"), "{facts}");
    // The work directory is kept on **success** too, and listed by
    // `vibe app work` beside the installation and migration ones.
    let work = Path::new(document["work"].as_str().unwrap());
    assert!(work.join("ecosystem.json").exists());
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let keys: Vec<&str> = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["key"].as_str().unwrap())
        .collect();
    assert!(
        keys.contains(&"ecosystem:runtime:1.0.0:self_upgrade"),
        "{keys:?}"
    );
    // An adapter id the package does not declare names what it does.
    let message = failed(&extras(&r, "self-upgrade", &["--ecosystem", "pip"]));
    assert!(
        message.contains("unknown_ecosystem") || message.contains("names no adapter"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn a_failing_verb_keeps_the_work_directory_names_the_retry_and_changes_nothing() {
    let r = AppRegistry::new();
    let root = runtime(&r, "1.0.0", "");
    install(&r, "1.0.0");
    // A real failing child, because the whole point is what the tree looks
    // like *after* one.
    verb_script(&root, "remove", "exit 3\n");
    let output = extras(&r, "remove", &["typescript", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["ok"], false);
    let row = adapters_of(&document, "npm");
    assert_eq!(row["outcome"], "failed");
    assert_eq!(
        row["retry"],
        "vibe app extras remove runtime --ecosystem npm typescript"
    );
    assert!(Path::new(row["log"].as_str().unwrap()).exists());
    // Nothing was rolled back: the version is installed and still the
    // slot's default.
    assert_eq!(
        r.listed("org.example/runtime@1.0.0")["default"],
        true,
        "the version stays installed and the slot default stays put"
    );
    // The kept work directory is listed and cleaned by `vibe app work`.
    let work = Path::new(document["work"].as_str().unwrap());
    assert!(work.join("ecosystem.json").exists());
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let keys: Vec<&str> = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"ecosystem:runtime:1.0.0:remove"), "{keys:?}");
    // The retry command is not prose: it parses and it runs. (The
    // registry and `--offline` are this fixture's, not the command's.)
    verb_script(&root, "remove", "exit 0\n");
    let retry = row["retry"].as_str().unwrap();
    let registry = r.registry();
    let mut words: Vec<&str> = retry.split_whitespace().skip(1).collect();
    words.extend_from_slice(&[
        "--registry",
        registry.to_str().unwrap(),
        "--offline",
        "--json",
    ]);
    let again = json(&r.vibe(&words));
    assert_eq!(again["ok"], true);
    assert_eq!(adapters_of(&again, "npm")["outcome"], "done");
    ok(&r.vibe(&["app", "work", "-g", "--clean", "--all", "--json"]));
    assert!(!work.exists(), "a clean of the home removes it");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-MIGRATION")]
fn the_adapter_synthesises_one_migration_entry_that_observes_and_reinstalls() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "");
    runtime(&r, "2.0.0", "");
    install(&r, "1.0.0");
    // What the old version carries, which its `list` verb will answer.
    fs::write(r.home().join("installed"), "typescript\n@scope/pkg@^1.2\n").unwrap();
    install(&r, "2.0.0");
    // The synthesised entry is retained with the installation, so the
    // offer and `--only` need no package tree.
    let retained = retained(&r, "org.example/runtime@2.0.0");
    assert_eq!(retained["migrations"][0]["id"], "ecosystem:npm");
    assert_eq!(retained["migrations"][0]["default"], true);
    let registry = r.registry();
    let document = json(&r.vibe(&[
        "app",
        "migrate",
        "runtime",
        "1.0.0",
        "2.0.0",
        "--only",
        "ecosystem:npm",
        "--yes",
        "--registry",
        registry.to_str().unwrap(),
        "--offline",
        "--json",
    ]));
    assert_eq!(document["ok"], true);
    assert_eq!(document["entries"][0]["id"], "ecosystem:npm");
    assert_eq!(document["entries"][0]["outcome"], "done");
    // The export ran the `list` verb in the OLD version and the import the
    // `install` verb in the NEW one, with exactly the observed items as
    // its argv.
    assert_eq!(argv(&r, "install"), ["typescript", "@scope/pkg@^1.2"]);
    let facts = fs::read_to_string(r.home().join("install.facts")).unwrap();
    assert!(facts.contains("who=rt-2.0.0"), "{facts}");
    assert!(facts.contains("count=2"), "{facts}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-MIGRATION")]
fn migration_false_suppresses_the_synthesis() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "migration = false");
    runtime(&r, "2.0.0", "migration = false");
    install(&r, "1.0.0");
    install(&r, "2.0.0");
    assert_eq!(
        retained(&r, "org.example/runtime@2.0.0")["migrations"],
        Value::Null,
        "nothing is synthesised, so the inventory retains no entry"
    );
    let described = json(&r.app(
        &["app", "describe"],
        "org.example/runtime@2.0.0",
        &["--json", "--offline"],
    ));
    assert_eq!(
        described["application"]["ecosystems"][0]["migration"],
        false
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn a_runtime_with_no_adapter_says_so_by_name() {
    let r = AppRegistry::new();
    let root = r.package("plain", "1.0.0", "plain", option_body());
    r.script(&root, "pl", "pl-1.0.0");
    ok(&r.app(
        &["install", "-g"],
        "org.example/plain@1.0.0",
        &["--offline", "--unattended", "--json"],
    ));
    let registry = r.registry();
    let message = failed(&r.vibe(&[
        "app",
        "extras",
        "list",
        "plain",
        "--registry",
        registry.to_str().unwrap(),
        "--offline",
    ]));
    assert!(
        message.contains("no_ecosystem") || message.contains("declares no ecosystem adapter"),
        "{message}"
    );
}
