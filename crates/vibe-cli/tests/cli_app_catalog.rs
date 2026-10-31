//! Public Linux fixture for the launcher catalog (PROP-059
//! `#launcher-catalog`): installing a package with a `menu_group` seeds a
//! collection from it, the catalog answers `vibe-app-launchers-all/2` with a
//! stable key, collections, pins, overrides and usage facts per launcher,
//! every edit of the user layer moves the revision, `start --json` answers a
//! process id and a log and counts the start, a key that names no launcher
//! and a collection that does not exist are refusals before any effect, and
//! an uninstall takes the package's keys out of the arrangement.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

const EDITOR: &str = "org.example/editor@1.0.0";
const EDIT: &str = "org.example/editor@1.0.0#edit";
const TOOL: &str = "org.example/editor@1.0.0#tool";

fn executable(path: &Path, marker: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\necho \"{marker} $*\"\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// `org.example/editor` in the group `Editor Suite`: a `gui` launcher `edit`
/// opted into a menu and a `command` launcher `tool` that is not. Both run a
/// script that exits at once, so a detached start is observable.
fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    let editor = apps.package("editor", "1.0.0", "editor", "");
    fs::write(
        editor.join("vibe.toml"),
        format!(
            "[package]\nname = \"editor\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"editor\"\nmenu_group = \"Editor Suite\"\n\n\
             [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\n\
             arch = \"{arch}\"\n\
             [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place the payload\"\nkind = \"copy\"\nfrom = \"payload\"\n\n\
             [[installer_options.launchers]]\nid = \"edit\"\nname = \"Editor\"\nexecutable = \"$app_bin/edit\"\n\
             kind = \"gui\"\nmenu = true\ndefault = true\n\n\
             [[installer_options.launchers]]\nid = \"tool\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nkind = \"command\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    executable(&editor.join("payload/bin/edit"), "EDIT_RAN");
    executable(&editor.join("payload/bin/tool"), "TOOL_RAN");
    apps
}

fn catalog(apps: &AppRegistry) -> Value {
    json(&apps.vibe(&["app", "launchers", "-g", "--all", "--json"]))
}

fn launcher(catalog: &Value, id: &str) -> Value {
    catalog["installations"][0]["launchers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no launcher `{id}` in {catalog}"))
        .clone()
}

fn revision(apps: &AppRegistry) -> u64 {
    let answer = json(&apps.vibe(&["app", "launchers", "-g", "--revision", "--json"]));
    assert_eq!(answer["protocol"], "vibe-app-revision/1");
    answer["revision"].as_u64().unwrap()
}

#[test]
fn the_catalog_carries_keys_a_seeded_collection_and_a_revision_every_edit_moves() {
    let apps = fixture();
    ok(&apps.app(&["install", "-g"], "org.example/editor", &[]));

    let answer = catalog(&apps);
    assert_eq!(answer["protocol"], "vibe-app-launchers-all/2");
    assert_eq!(answer["home"], apps.settings().display().to_string());
    let installation = &answer["installations"][0];
    assert_eq!(installation["key"], EDITOR);
    assert_eq!(installation["state"]["hidden"], false);
    assert_eq!(installation["state"]["held"], false);
    assert_eq!(
        installation["state"]["update_available"],
        Value::Null,
        "answering it would need a registry call"
    );
    let edit = launcher(&answer, "edit");
    assert_eq!(edit["key"], EDIT);
    assert_eq!(edit["display_name"], "Editor", "the package's own name");
    assert_eq!(edit["hidden_by"], Value::Null);
    assert_eq!(edit["pinned"], serde_json::json!([]));
    assert_eq!(edit["usage"]["starts"], 0);
    assert_eq!(edit["usage"]["last_started"], Value::Null);
    assert_eq!(
        edit["collections"],
        serde_json::json!(["Editor Suite"]),
        "the menu_group seeded it: {answer}"
    );
    assert_eq!(
        launcher(&answer, "tool")["collections"],
        serde_json::json!([]),
        "a launcher outside a menu is not seeded into the group"
    );

    // The single listing bumped to the same version and gained the fields.
    let single = json(&apps.vibe(&["app", "launchers", "-g", "org.example/editor", "--json"]));
    assert_eq!(single["protocol"], "vibe-app-launchers/2");
    assert_eq!(single["launchers"][0]["key"], EDIT);
    assert_eq!(single["state"]["held"], false);

    // The seeded collection is in the user layer, flagged as seeded.
    let layer = json(&apps.vibe(&["app", "collection", "-g", "list", "--json"]));
    assert_eq!(layer["protocol"], "vibe-app-collections/1");
    assert_eq!(layer["collections"][0]["name"], "Editor Suite");
    assert_eq!(layer["collections"][0]["seeded"], true);
    assert_eq!(
        layer["collections"][0]["launchers"],
        serde_json::json!([EDIT])
    );

    // Every edit of the layer moves the revision.
    let before = revision(&apps);
    let pinned = json(&apps.vibe(&["app", "pin", "-g", EDIT, "--json"]));
    assert_eq!(pinned["pins"]["taskbar"], serde_json::json!([EDIT]));
    assert!(revision(&apps) > before, "a pin moves the revision");
    ok(&apps.vibe(&["app", "pin", "-g", TOOL, "--json"]));
    let moved = json(&apps.vibe(&["app", "pin", "-g", EDIT, "--json"]));
    assert_eq!(
        moved["pins"]["taskbar"],
        serde_json::json!([TOOL, EDIT]),
        "a repeated pin moves to the end"
    );
    let favoured = json(&apps.vibe(&["app", "pin", "-g", EDIT, "--to", "favorites", "--json"]));
    assert_eq!(favoured["pins"]["favorites"], serde_json::json!([EDIT]));
    assert_eq!(
        launcher(&catalog(&apps), "edit")["pinned"],
        serde_json::json!(["taskbar", "favorites"])
    );
    let unpinned = json(&apps.vibe(&["app", "unpin", "-g", EDIT, "--json"]));
    assert_eq!(unpinned["pins"]["taskbar"], serde_json::json!([TOOL]));
    assert_eq!(unpinned["pins"]["favorites"], serde_json::json!([EDIT]));

    // Collections are created, filled, ordered and emptied by command.
    ok(&apps.vibe(&["app", "collection", "-g", "create", "Favourites"]));
    let filled = json(&apps.vibe(&[
        "app",
        "collection",
        "-g",
        "add",
        "Favourites",
        TOOL,
        EDIT,
        "--json",
    ]));
    let favourites = |layer: &Value| layer["collections"][1].clone();
    assert_eq!(
        favourites(&filled)["launchers"],
        serde_json::json!([TOOL, EDIT])
    );
    assert_eq!(favourites(&filled)["seeded"], false);
    let ordered = json(&apps.vibe(&[
        "app",
        "collection",
        "-g",
        "order",
        "Favourites",
        EDIT,
        TOOL,
        "--json",
    ]));
    assert_eq!(
        favourites(&ordered)["launchers"],
        serde_json::json!([EDIT, TOOL])
    );
    let shortened = json(&apps.vibe(&[
        "app",
        "collection",
        "-g",
        "remove",
        "Favourites",
        TOOL,
        "--json",
    ]));
    assert_eq!(
        favourites(&shortened)["launchers"],
        serde_json::json!([EDIT])
    );
    assert_eq!(
        favourites(&shortened)["removed"],
        serde_json::json!([TOOL]),
        "the removal is remembered so seeding never undoes it"
    );
    assert_eq!(
        launcher(&catalog(&apps), "edit")["collections"],
        serde_json::json!(["Editor Suite", "Favourites"]),
        "a launcher may sit in several collections"
    );

    // A hidden launcher stays in the catalog and says who hid it.
    ok(&apps.vibe(&["app", "launcher", "-g", EDIT, "--hide"]));
    ok(&apps.vibe(&["app", "launcher", "-g", EDIT, "--name", "My Editor"]));
    let hidden = launcher(&catalog(&apps), "edit");
    assert_eq!(hidden["hidden_by"], "user");
    assert_eq!(hidden["menu"], true, "the package still opts it in");
    assert_eq!(hidden["display_name"], "My Editor");
    assert_eq!(
        hidden["name"], "Editor",
        "the package's own name stands beside it"
    );
    let icon = apps.root.path().join("mine.svg");
    fs::write(&icon, "<svg/>").unwrap();
    ok(&apps.vibe(&[
        "app",
        "launcher",
        "-g",
        EDIT,
        "--icon",
        icon.to_str().unwrap(),
    ]));
    assert_eq!(
        launcher(&catalog(&apps), "edit")["display_icon"],
        icon.display().to_string()
    );
    let reset = json(&apps.vibe(&["app", "launcher", "-g", EDIT, "--reset", "--json"]));
    assert_eq!(reset["overrides"], serde_json::json!([]));
    let plain = launcher(&catalog(&apps), "edit");
    assert_eq!(plain["hidden_by"], Value::Null);
    assert_eq!(plain["display_name"], "Editor");
    assert_eq!(plain["display_icon"], Value::Null);
    assert!(
        failed(&apps.vibe(&["app", "launcher", "-g", EDIT])).contains("--hide"),
        "one of the five overrides is required"
    );
}

#[test]
fn a_detached_start_answers_a_pid_and_a_log_and_counts_the_start() {
    let apps = fixture();
    ok(&apps.app(&["install", "-g"], "org.example/editor", &[]));
    let before = revision(&apps);

    let started = json(&apps.vibe(&["app", "start", "-g", TOOL, "--json", "--", "a.txt"]));
    assert_eq!(started["protocol"], "vibe-app-start/1");
    assert_eq!(started["key"], TOOL);
    assert!(started["pid"].as_u64().unwrap() > 0, "{started}");
    assert!(
        started["command"].as_str().unwrap().ends_with("/bin/tool"),
        "{started}"
    );
    assert_eq!(started["args"], serde_json::json!(["a.txt"]));
    let log = Path::new(started["log"].as_str().unwrap());
    assert!(log.is_absolute() && log.is_file(), "{started}");
    assert!(revision(&apps) > before, "a start moves the revision");

    let usage = launcher(&catalog(&apps), "tool")["usage"].clone();
    assert_eq!(usage["starts"], 1);
    assert!(
        usage["last_started"].as_str().unwrap().ends_with('Z'),
        "{usage}"
    );

    // The child writes into that log and exits; a slow machine only makes
    // the wait longer.
    for attempt in 0..100 {
        if fs::read_to_string(log).unwrap().contains("TOOL_RAN a.txt") {
            break;
        }
        assert!(attempt < 99, "the log never caught the child: {started}");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // `--launcher` is the other spelling of the same choice; the default
    // launcher answers a bare package.
    let by_flag = json(&apps.vibe(&[
        "app",
        "start",
        "-g",
        "org.example/editor",
        "--launcher",
        "tool",
        "--json",
    ]));
    assert_eq!(by_flag["key"], TOOL);
    let by_default = json(&apps.vibe(&["app", "start", "-g", "org.example/editor", "--json"]));
    assert_eq!(by_default["key"], EDIT, "the declared default");
    assert!(
        failed(&apps.vibe(&["app", "start", "-g", TOOL, "--launcher", "edit", "--json"]))
            .contains("already names launcher")
    );
    assert_eq!(
        launcher(&catalog(&apps), "tool")["usage"]["starts"],
        2,
        "the flagged start counted too"
    );

    // Without `--json` the start stays attached and passes the exit status.
    let attached = apps.vibe(&["app", "start", "-g", TOOL, "--", "b.txt"]);
    assert_eq!(
        String::from_utf8_lossy(&attached.stdout).trim(),
        "TOOL_RAN b.txt"
    );
}

#[test]
fn a_missing_key_or_collection_is_refused_and_an_uninstall_drops_the_keys() {
    let apps = fixture();
    ok(&apps.app(&["install", "-g"], "org.example/editor", &[]));

    let refused = |args: &[&str], code: &str| {
        let output = apps.vibe(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let text = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(text.contains(code), "{args:?}: {text}");
    };
    refused(
        &["app", "pin", "-g", "org.example/editor@1.0.0#ghost"],
        "launcher_missing",
    );
    refused(
        &[
            "app",
            "launcher",
            "-g",
            "org.example/absent@1.0.0#main",
            "--hide",
        ],
        "launcher_missing",
    );
    refused(
        &["app", "collection", "-g", "add", "Absent", EDIT],
        "collection_missing",
    );
    refused(
        &["app", "collection", "-g", "delete", "Absent"],
        "collection_missing",
    );
    refused(
        &["app", "collection", "-g", "create", "Editor Suite"],
        "collection_exists",
    );

    // An arrangement of the user's own survives its package, the seeded
    // collection does not, and no key of the package stays behind.
    ok(&apps.vibe(&["app", "collection", "-g", "create", "Mine"]));
    ok(&apps.vibe(&["app", "collection", "-g", "add", "Mine", EDIT]));
    ok(&apps.vibe(&["app", "pin", "-g", EDIT]));
    ok(&apps.vibe(&["app", "launcher", "-g", TOOL, "--hide"]));
    ok(&apps.vibe(&["uninstall", "-g", "org.example/editor"]));

    let layer = json(&apps.vibe(&["app", "collection", "-g", "list", "--json"]));
    let names: Vec<&str> = layer["collections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Mine"], "the untouched seeded group went too");
    assert_eq!(layer["collections"][0]["launchers"], serde_json::json!([]));
    assert_eq!(layer["pins"]["taskbar"], serde_json::json!([]));
    assert_eq!(layer["overrides"], serde_json::json!([]));
    assert!(
        catalog(&apps)["installations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
