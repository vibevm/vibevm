//! `vibe app migrate` end to end through the real `vibe` binary and a local
//! directory registry (PROP-075 `#MIGRATION`, `#MIGRATION-DECLARATION`,
//! `#MIGRATION-FAILURE`).
//!
//! The fixture is the shape the design was written for: two installed
//! versions of one package whose `export` step writes a marker into the
//! shared work directory and whose `import` step reads it. Every failure
//! case is a real failing child, not a stub, because the whole point of
//! `#MIGRATION-FAILURE` is what the tree looks like *after* one.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, failed, json, ok};
use common::fake_agent::refused;
use serde_json::Value;

/// The package body of a version: payload, one public command and the
/// migration entries `entries` declares.
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
name='Stateful'
executable='$app_bin/st'
kind='command'
default=true
"#
}

/// One `[[application.migration]]` entry whose two sides are the named
/// package-relative recipes.
fn entry(id: &str, name: &str, default: bool) -> String {
    format!(
        "\n[[application.migration]]\nid = \"{id}\"\nname = \"{name}\"\n\
         default = {default}\n\n\
         [application.migration.export]\nid = \"export-{id}\"\n\
         title = \"Export {name}\"\nkind = \"recipe\"\nscript = \"migrate/{id}-export.sh\"\n\n\
         [application.migration.import]\nid = \"import-{id}\"\n\
         title = \"Import {name}\"\nkind = \"recipe\"\nscript = \"migrate/{id}-import.sh\"\n"
    )
}

/// Write `org.example/stateful@<version>` with the declared `entries` and
/// the two scripts of each.
fn stateful(r: &AppRegistry, version: &str, entries: &[(&str, &str, bool)]) -> PathBuf {
    let root = r.package("stateful", version, "stateful", option_body());
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mut declared = String::from("[application]\nid = \"stateful\"\n");
    for (id, name, default) in entries {
        declared.push_str(&entry(id, name, *default));
    }
    fs::write(
        &manifest,
        text.replace("[application]\nid = \"stateful\"\n", &declared),
    )
    .unwrap();
    r.script(&root, "st", &format!("st-{version}"));
    root
}

/// A package-relative migration script, made executable for good measure
/// (the engine runs it through `/bin/sh`, so the bit is not required).
fn migration_script(root: &Path, name: &str, body: &str) {
    let dir = root.join("migrate");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\nset -e\n{body}")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The export every happy case uses: it records what it saw and leaves the
/// carried marker in the shared work directory.
const EXPORT: &str = r#"
printf '%s\n' "$VIBE_FROM_VERSION" > "$VIBE_MIGRATION_WORK/carried"
printf 'id=%s from=%s to=%s out=%s sources=%s root=%s who=%s\n' \
  "$VIBE_MIGRATION_ID" "$VIBE_FROM_VERSION" "$VIBE_TO_VERSION" \
  "${VIBE_OUT-unset}" "${VIBE_SOURCES-unset}" "$VIBE_FROM_APP_ROOT" "$(st)" \
  > "$VIBE_MIGRATION_WORK/export.facts"
printf 'ran\n' >> "$VIBE_MIGRATION_WORK/export.count"
"#;

/// The import every happy case uses: it refuses to invent state it was not
/// handed, and records the new version's own view.
const IMPORT: &str = r#"
test -f "$VIBE_MIGRATION_WORK/carried"
printf 'carried=%s to=%s out=%s root=%s who=%s\n' \
  "$(cat "$VIBE_MIGRATION_WORK/carried")" "$VIBE_TO_VERSION" \
  "${VIBE_OUT-unset}" "$VIBE_TO_APP_ROOT" "$(st)" \
  > "$HOME/imported"
"#;

fn install(r: &AppRegistry, version: &str) {
    ok(&r.app(
        &["install", "-g"],
        &format!("org.example/stateful@{version}"),
        &["--offline", "--unattended", "--json"],
    ));
}

/// `vibe app migrate stateful FROM TO …` with the fixture registry.
fn migrate(r: &AppRegistry, from: &str, to: &str, extra: &[&str]) -> std::process::Output {
    let registry = r.registry();
    let mut args = vec!["app", "migrate", "stateful", from, to];
    args.extend_from_slice(&["--registry", registry.to_str().unwrap(), "--offline"]);
    args.extend_from_slice(extra);
    r.vibe(&args)
}

fn entry_of<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no entry `{id}` in {document:#}"))
}

/// The two versions of the happy fixture, both installed.
fn pair(r: &AppRegistry, export: &str, import: &str) {
    let entries = [("state", "Carried state", true)];
    let old = stateful(r, "1.0.0", &entries);
    migration_script(&old, "state-export.sh", export);
    migration_script(&old, "state-import.sh", import);
    let new = stateful(r, "2.0.0", &entries);
    migration_script(&new, "state-export.sh", export);
    migration_script(&new, "state-import.sh", import);
    install(r, "1.0.0");
    install(r, "2.0.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
fn the_happy_path_carries_the_state_with_each_side_on_its_own_version() {
    let r = AppRegistry::new();
    pair(&r, EXPORT, IMPORT);
    let before = r.index()["slots"]["stateful"]["default"].clone();

    let document = json(&migrate(&r, "1.0.0", "2.0.0", &["--all", "--json"]));
    assert_eq!(document["protocol"], "vibe-app-migration/1");
    assert_eq!(document["ok"], true);
    assert_eq!(document["from"], "org.example/stateful@1.0.0");
    assert_eq!(document["to"], "org.example/stateful@2.0.0");
    assert_eq!(entry_of(&document, "state")["outcome"], "done");

    // The export ran on the OLD version's PATH and package, the import on
    // the NEW one's, and neither saw a payload directory
    // (PROP-075 `#MIGRATION-DECLARATION`).
    let work = PathBuf::from(document["work"].as_str().unwrap());
    let facts = fs::read_to_string(work.join("export.facts")).unwrap();
    assert!(facts.contains("id=state from=1.0.0 to=2.0.0"), "{facts}");
    assert!(facts.contains("out=unset sources=unset"), "{facts}");
    assert!(facts.contains("who=st-1.0.0"), "{facts}");
    let imported = fs::read_to_string(r.home().join("imported")).unwrap();
    assert!(imported.contains("carried=1.0.0 to=2.0.0"), "{imported}");
    assert!(imported.contains("out=unset"), "{imported}");
    assert!(imported.contains("who=st-2.0.0"), "{imported}");

    // The work directory is kept and `vibe app work` lists it.
    assert!(work.is_dir(), "{} is gone", work.display());
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let rows = listing["entries"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|row| row["key"] == "migrate:stateful:1.0.0-to-2.0.0")
        .unwrap_or_else(|| panic!("no migration row in {listing:#}"));
    assert_eq!(row["state"], "done");
    assert_eq!(row["path"], document["work"]);
    // A per-package clean never takes it, because the row is keyed by a
    // version **pair** and not by a package: with no installation work
    // directory kept, that clean has nothing at all to select and refuses.
    let per_package = failed(&r.vibe(&[
        "app",
        "work",
        "-g",
        "--clean",
        "org.example/stateful",
        "--json",
    ]));
    assert!(per_package.contains("work_missing"), "{per_package}");
    assert!(work.is_dir(), "a package clean must not take a migration");
    // The home-wide clean does take it, which is AV-FMT-103.
    let cleaned = json(&r.vibe(&["app", "work", "-g", "--clean", "--all", "--json"]));
    assert!(
        cleaned["removed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path == &document["work"]),
        "{cleaned:#}"
    );
    assert!(!work.exists());

    // The outcome is recorded with the NEW installation, and that is the
    // first thing in this home that needs `/7` (AV-FMT-017).
    let index = r.index();
    assert_eq!(index["protocol"], "vibe-user-applications/7");
    let record = &index["installations"]["org.example/stateful@2.0.0"];
    assert_eq!(record["migrated"][0]["id"], "state");
    assert_eq!(record["migrated"][0]["result"], "done");
    assert_eq!(record["migrated"][0]["from"], "org.example/stateful@1.0.0");
    assert_eq!(record["migrations"][0]["id"], "state");
    // Nothing moved the slot's default.
    assert_eq!(r.index()["slots"]["stateful"]["default"], before);
    // A `/7` index is still routable: every command of the slot answers.
    let listed = r.listed("org.example/stateful@2.0.0");
    assert_eq!(listed["key"], "org.example/stateful@2.0.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn a_failing_export_changes_nothing_and_never_runs_the_import() {
    let r = AppRegistry::new();
    pair(
        &r,
        "printf 'ran\\n' >> \"$VIBE_MIGRATION_WORK/export.count\"\nexit 3\n",
        IMPORT,
    );
    let output = migrate(&r, "1.0.0", "2.0.0", &["--all", "--json"]);
    let document: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document["ok"], false);
    let state = entry_of(&document, "state");
    assert_eq!(state["outcome"], "export_failed");
    assert_eq!(state["side"], "export");
    assert_eq!(state["step"], "export-state");
    assert!(state["retry"].as_str().unwrap().contains("--only state"));
    // The import never ran: it is the only writer of `$HOME/imported`.
    assert!(!r.home().join("imported").exists());
    let work = PathBuf::from(document["work"].as_str().unwrap());
    assert!(work.is_dir());
    assert!(!work.join("carried").exists());
    assert!(fs::read_to_string(state["log"].as_str().unwrap()).is_ok());
    let record = &r.index()["installations"]["org.example/stateful@2.0.0"];
    assert_eq!(record["migrated"][0]["result"], "export_failed");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn failed_import_keeps_the_new_version() {
    let r = AppRegistry::new();
    pair(
        &r,
        EXPORT,
        "test -f \"$VIBE_MIGRATION_WORK/carried\"\nexit 4\n",
    );
    let before = r.index()["slots"]["stateful"]["default"].clone();

    let output = migrate(&r, "1.0.0", "2.0.0", &["--all", "--json"]);
    let document: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document["ok"], false);
    let state = entry_of(&document, "state");
    assert_eq!(state["outcome"], "import_failed");
    assert_eq!(state["side"], "import");
    assert_eq!(state["step"], "import-state");
    // What was not carried, where its log is, and the exact retry.
    assert_eq!(state["name"], "Carried state");
    assert!(
        state["reason"].as_str().unwrap().contains("import"),
        "{state:#}"
    );
    let log = state["log"].as_str().unwrap();
    assert!(Path::new(log).is_file(), "{log}");
    assert_eq!(
        state["retry"],
        "vibe app migrate stateful 1.0.0 2.0.0 --only state --resume --yes"
    );

    // Nothing was rolled back: the new version is still installed, exposed
    // and the slot's default is exactly where it was.
    let index = r.index();
    assert!(
        index["installations"]
            .get("org.example/stateful@2.0.0")
            .is_some()
    );
    assert_eq!(index["slots"]["stateful"]["default"], before);
    assert_eq!(
        r.listed("org.example/stateful@2.0.0")["key"],
        "org.example/stateful@2.0.0"
    );
    // The work directory is kept, with the export's own evidence in it.
    let work = PathBuf::from(document["work"].as_str().unwrap());
    assert!(work.join("carried").is_file());
    assert_eq!(
        index["installations"]["org.example/stateful@2.0.0"]["migrated"][0]["result"],
        "import_failed"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn one_entrys_failure_does_not_stop_anothers() {
    let r = AppRegistry::new();
    let entries = [("first", "First", true), ("second", "Second", true)];
    for version in ["1.0.0", "2.0.0"] {
        let root = stateful(&r, version, &entries);
        migration_script(&root, "first-export.sh", EXPORT);
        migration_script(&root, "first-import.sh", "exit 5\n");
        // The marker is named after the entry on purpose: the top of the
        // shared work directory belongs to the scripts, never to the logs.
        migration_script(
            &root,
            "second-export.sh",
            "printf 'two\\n' > \"$VIBE_MIGRATION_WORK/second\"\n",
        );
        migration_script(
            &root,
            "second-import.sh",
            "cp \"$VIBE_MIGRATION_WORK/second\" \"$HOME/second\"\n",
        );
        install(&r, version);
    }
    let output = migrate(&r, "1.0.0", "2.0.0", &["--all", "--json"]);
    let document: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(entry_of(&document, "first")["outcome"], "import_failed");
    assert_eq!(entry_of(&document, "second")["outcome"], "done");
    assert!(r.home().join("second").is_file());
    // The entry's logs are a level down, so the marker above could keep the
    // entry's own name.
    let work = PathBuf::from(document["work"].as_str().unwrap());
    assert!(work.join("second").is_file() && work.join("logs/second/export.log").is_file());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn resume_re_runs_only_the_import() {
    let r = AppRegistry::new();
    pair(
        &r,
        EXPORT,
        "test ! -f \"$HOME/let-it-pass\" && exit 6\ncp \"$VIBE_MIGRATION_WORK/carried\" \"$HOME/imported\"\n",
    );
    let first = migrate(&r, "1.0.0", "2.0.0", &["--all", "--json"]);
    let document: Value = serde_json::from_str(&String::from_utf8_lossy(&first.stdout)).unwrap();
    assert_eq!(entry_of(&document, "state")["outcome"], "import_failed");
    let work = PathBuf::from(document["work"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(work.join("export.count")).unwrap(),
        "ran\n"
    );

    fs::write(r.home().join("let-it-pass"), "yes").unwrap();
    let again = json(&migrate(
        &r,
        "1.0.0",
        "2.0.0",
        &["--only", "state", "--resume", "--json"],
    ));
    assert_eq!(again["resumed"], true);
    assert_eq!(again["work"], document["work"]);
    assert_eq!(entry_of(&again, "state")["outcome"], "done");
    // The export did not run a second time: its own counter is unchanged.
    assert_eq!(
        fs::read_to_string(work.join("export.count")).unwrap(),
        "ran\n"
    );
    assert_eq!(
        fs::read_to_string(r.home().join("imported")).unwrap(),
        "1.0.0\n"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn an_unattended_run_without_yes_refuses_and_prints_the_exact_command() {
    let r = AppRegistry::new();
    pair(&r, EXPORT, IMPORT);
    let output = migrate(&r, "1.0.0", "2.0.0", &["--json"]);
    let (code, envelope) = refused(&output);
    assert_eq!(code, Some(2));
    assert_eq!(envelope["code"], "migration_consent_required");
    let message = envelope["error"].as_str().unwrap();
    assert!(
        message.contains("vibe app migrate stateful 1.0.0 2.0.0 --only state --yes"),
        "{message}"
    );
    // Nothing ran and nothing was recorded.
    assert!(!r.home().join("imported").exists());
    let record = &r.index()["installations"]["org.example/stateful@2.0.0"];
    assert!(
        record["migrated"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn only_refuses_an_id_the_new_version_does_not_declare() {
    let r = AppRegistry::new();
    pair(&r, EXPORT, IMPORT);
    let output = migrate(&r, "1.0.0", "2.0.0", &["--only", "absent", "--json"]);
    let (code, envelope) = refused(&output);
    assert_eq!(code, Some(2));
    assert_eq!(envelope["code"], "unknown_migration");
    assert!(
        envelope["error"]
            .as_str()
            .unwrap()
            .contains("declared: state"),
        "{envelope:#}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn install_of_a_new_version_offers_and_does_not_migrate() {
    let r = AppRegistry::new();
    let entries = [("state", "Carried state", true)];
    for version in ["1.0.0", "2.0.0"] {
        let root = stateful(&r, version, &entries);
        migration_script(&root, "state-export.sh", EXPORT);
        migration_script(&root, "state-import.sh", IMPORT);
    }
    install(&r, "1.0.0");
    // The second install is the one that can offer; it is human output, so
    // the offer is read off stdout rather than a JSON envelope.
    let second = r.app(
        &["install", "-g"],
        "org.example/stateful@2.0.0",
        &["--offline", "--unattended"],
    );
    let text = ok(&second);
    assert!(
        text.contains("state from 1.0.0 can be carried over")
            && text.contains("vibe app migrate stateful 1.0.0 2.0.0 --only state --yes"),
        "{text}"
    );
    // It offered and nothing else: no migration ran.
    assert!(!r.home().join("imported").exists());
    assert!(
        crate::common::app_registry::ok(&r.vibe(&["app", "work", "-g", "--json"]))
            .contains("\"entries\": []")
            || !r.index().to_string().contains("\"migrated\"")
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
fn an_unmet_need_refuses_the_entry_before_its_child_starts() {
    let r = AppRegistry::new();
    let entries = [("state", "Carried state", true)];
    for version in ["1.0.0", "2.0.0"] {
        let root = stateful(&r, version, &entries);
        // The export declares a command this host does not have.
        let manifest = root.join("vibe.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            text.replace(
                "kind = \"recipe\"\nscript = \"migrate/state-export.sh\"",
                "kind = \"recipe\"\nneeds = [\"command:definitely-not-on-this-host\"]\n\
                 script = \"migrate/state-export.sh\"",
            ),
        )
        .unwrap();
        migration_script(&root, "state-export.sh", EXPORT);
        migration_script(&root, "state-import.sh", IMPORT);
        install(&r, version);
    }
    let output = migrate(&r, "1.0.0", "2.0.0", &["--all", "--json"]);
    let document: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    let state = entry_of(&document, "state");
    assert_eq!(state["outcome"], "export_failed");
    assert!(
        state["reason"]
            .as_str()
            .unwrap()
            .contains("definitely-not-on-this-host"),
        "{state:#}"
    );
    // The child never started, so the export wrote nothing at all.
    let work = PathBuf::from(document["work"].as_str().unwrap());
    assert!(!work.join("carried").exists());
    assert!(!r.home().join("imported").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn a_maintenance_entry_carries_one_version_into_itself() {
    let r = AppRegistry::new();
    let entries = [("latest-dep", "Bundled dependency", true)];
    let root = stateful(&r, "1.0.0", &entries);
    migration_script(&root, "latest-dep-export.sh", EXPORT);
    migration_script(
        &root,
        "latest-dep-import.sh",
        "printf '%s\\n' \"$VIBE_TO_VERSION\" > \"$HOME/refreshed\"\n",
    );
    install(&r, "1.0.0");
    let document = json(&migrate(&r, "1.0.0", "1.0.0", &["--all", "--json"]));
    assert_eq!(document["ok"], true);
    assert_eq!(document["from"], document["to"]);
    assert_eq!(entry_of(&document, "latest-dep")["outcome"], "done");
    assert_eq!(
        fs::read_to_string(r.home().join("refreshed")).unwrap(),
        "1.0.0\n"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn an_unknown_slot_and_an_uninstalled_version_refuse_before_any_effect() {
    let r = AppRegistry::new();
    pair(&r, EXPORT, IMPORT);
    let unknown = failed(&r.vibe(&[
        "app",
        "migrate",
        "absent",
        "1.0.0",
        "2.0.0",
        "--all",
        "--offline",
    ]));
    assert!(unknown.contains("absent"), "{unknown}");
    let missing = failed(&migrate(&r, "1.0.0", "9.9.9", &["--all"]));
    assert!(missing.contains("9.9.9"), "{missing}");
    assert!(
        r.index()["installations"]["org.example/stateful@2.0.0"]["migrated"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
}
