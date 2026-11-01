//! Public Linux fixture for the runtime half of the launcher catalog
//! (PROP-059 `#launcher-runtime`): a detached start is listed by `running`
//! until `stop` ends it, `watch` reports the start and the exit as lines,
//! `--export-system` writes `.desktop` files and icon copies into the XDG
//! data home and `--remove-system` takes them back, an update carries the
//! user's pin and collection membership to the new version's keys, and
//! `modify --keep-work` keeps the work directory (PROP-065 `#work`).

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

const EDITOR: &str = "org.example/editor";
const SERVE: &str = "org.example/editor@1.0.0#serve";
const EDIT: &str = "org.example/editor@1.0.0#edit";
const EDIT_NEXT: &str = "org.example/editor@1.1.0#edit";

/// A PNG signature and `IHDR` chunk declaring a square of `size` pixels.
fn png(size: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend(size.to_be_bytes());
    bytes.extend(size.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]);
    bytes
}

fn script(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// `org.example/editor@<version>` in the group `Editor Suite`: a `gui`
/// launcher `edit` in the menu with an SVG and a 256-pixel PNG, and a
/// `command` launcher `serve` that runs until it is ended.
fn package(apps: &AppRegistry, version: &str) {
    let dir = apps.package("editor", version, "editor", "");
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"editor\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"{version}\"\n\n\
             [application]\nid = \"editor\"\nmenu_group = \"Editor Suite\"\n\n\
             [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\n\
             arch = \"{arch}\"\n\
             [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place the payload\"\nkind = \"copy\"\nfrom = \"payload\"\n\n\
             [[installer_options.launchers]]\nid = \"edit\"\nname = \"Editor\"\nexecutable = \"$app_bin/edit\"\n\
             kind = \"gui\"\nmenu = true\ndefault = true\ncategories = [\"Development\"]\n\
             icon = {{ svg = \"icons/edit.svg\", png = {{ \"256\" = \"icons/edit-256.png\" }} }}\n\n\
             [[installer_options.launchers]]\nid = \"serve\"\nname = \"Server\"\nexecutable = \"$app_bin/serve\"\n\
             kind = \"command\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("icons")).unwrap();
    fs::write(dir.join("icons/edit.svg"), "<svg/>").unwrap();
    fs::write(dir.join("icons/edit-256.png"), png(256)).unwrap();
    script(&dir.join("payload/bin/edit"), "echo EDIT_RAN");
    // `exec` keeps one process, so the recorded pid is the sleeper itself.
    script(&dir.join("payload/bin/serve"), "exec sleep 60");
}

fn installed() -> AppRegistry {
    let apps = AppRegistry::new();
    package(&apps, "1.0.0");
    ok(&apps.app(&["install", "-g"], EDITOR, &[]));
    apps
}

/// `vibe app watch -g --json` as a child whose stdout lines arrive on a
/// channel; dropping its stdin ends it.
struct Watch {
    child: Child,
    lines: mpsc::Receiver<String>,
}

impl Watch {
    fn start(apps: &AppRegistry) -> Self {
        // A plain process rather than the harness's `Command`, which only
        // runs a child to completion.
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .current_dir(apps.root.path())
            .env("HOME", apps.home())
            .env("VIBE_SETTINGS", apps.settings())
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .args(["app", "watch", "-g", "--json"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        // The watch reads the journal's end when it starts; give it a tick
        // to get there before anything changes.
        std::thread::sleep(Duration::from_millis(500));
        Self { child, lines }
    }

    /// Every line until one satisfies `done`, within `limit`.
    fn until(&self, limit: Duration, done: impl Fn(&Value) -> bool) -> Vec<Value> {
        let deadline = Instant::now() + limit;
        let mut seen = Vec::new();
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            let Ok(line) = self.lines.recv_timeout(left) else {
                break;
            };
            let event: Value = serde_json::from_str(&line).unwrap();
            let finished = done(&event);
            seen.push(event);
            if finished {
                break;
            }
        }
        seen
    }

    fn close(mut self) {
        drop(self.child.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.child.try_wait().unwrap().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        panic!("watch did not end when its stdin closed");
    }
}

#[test]
fn a_started_launcher_is_running_until_stopped_and_watch_reports_both() {
    let apps = installed();
    let watch = Watch::start(&apps);

    let started = json(&apps.vibe(&["app", "start", "-g", SERVE, "--json"]));
    let pid = started["pid"].as_u64().unwrap();
    let listed = json(&apps.vibe(&["app", "running", "-g", "--json"]));
    assert_eq!(listed["protocol"], "vibe-app-running/1");
    let rows = listed["running"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{listed}");
    assert_eq!(rows[0]["key"], SERVE);
    assert_eq!(rows[0]["pid"], pid);
    assert_eq!(rows[0]["kind"], "command");
    assert!(rows[0]["started"].as_str().unwrap().ends_with('Z'));

    let stopped = json(&apps.vibe(&["app", "stop", "-g", SERVE, "--json"]));
    assert_eq!(stopped["stopped"][0]["key"], SERVE);
    assert_eq!(stopped["stopped"][0]["pid"], pid);
    assert_eq!(stopped["stopped"][0]["ended"], "terminated", "{stopped}");
    assert_eq!(stopped["running"], serde_json::json!([]));
    let listed = json(&apps.vibe(&["app", "running", "-g", "--json"]));
    assert_eq!(listed["running"], serde_json::json!([]));
    assert!(
        failed(&apps.vibe(&["app", "stop", "-g", SERVE])).contains("not_running"),
        "nothing left to stop"
    );

    let events = watch.until(Duration::from_secs(10), |event| event["event"] == "exited");
    let capture: Vec<String> = events.iter().map(Value::to_string).collect();
    // The capture the report quotes.
    println!("watch capture:\n{}", capture.join("\n"));
    let started_event = events
        .iter()
        .find(|event| event["event"] == "started")
        .unwrap_or_else(|| panic!("no started line: {capture:?}"));
    assert_eq!(started_event["protocol"], "vibe-app-events/1");
    assert_eq!(started_event["key"], SERVE);
    let exited = events.last().unwrap();
    assert_eq!(exited["event"], "exited", "{capture:?}");
    assert_eq!(exited["key"], SERVE);
    assert_eq!(exited["pid"], pid);
    assert!(exited["revision"].as_u64().unwrap() >= started_event["revision"].as_u64().unwrap());
    watch.close();
}

#[test]
fn the_system_projection_writes_desktop_files_and_icons_and_removes_them() {
    let apps = installed();
    let data = apps.root.path().join("xdg-data");
    let project = |args: &[&str]| {
        let mut command = apps.user.vibe();
        command
            .current_dir(apps.root.path())
            .env("HOME", apps.home())
            .env("XDG_DATA_HOME", &data)
            .args(["app", "launchers", "-g"])
            .args(args);
        command.output().unwrap()
    };
    let desktop = data.join("applications/vibe-org.example-editor-edit.desktop");
    let png = data.join("icons/hicolor/256x256/apps/vibe-org.example-editor-edit.png");
    let svg = data.join("icons/hicolor/scalable/apps/vibe-org.example-editor-edit.svg");

    let planned = json(&project(&["--export-system", "--dry-run", "--json"]));
    assert_eq!(planned["protocol"], "vibe-app-exported/1");
    assert_eq!(planned["platform"], "linux");
    assert_eq!(planned["dry_run"], true);
    let entries = planned["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "only the menu launcher: {planned}");
    assert_eq!(entries[0]["key"], EDIT);
    assert_eq!(entries[0]["paths"][0], desktop.display().to_string());
    assert!(!desktop.exists(), "a dry run writes nothing");

    let written = json(&project(&["--export-system", "--json"]));
    assert_eq!(written["dry_run"], false);
    let text = fs::read_to_string(&desktop).unwrap();
    assert!(text.contains(&format!("\nX-Vibe-Key={EDIT}\n")), "{text}");
    assert!(text.contains("\nName=Editor\n"), "{text}");
    assert!(
        text.contains("\nIcon=vibe-org.example-editor-edit\n"),
        "{text}"
    );
    assert!(text.contains("\nCategories=Development;\n"), "{text}");
    assert!(
        text.contains("app start -g --json \"org.example/editor#edit\"\n"),
        "the entry starts the current version detached: {text}"
    );
    assert!(
        ok(&apps.vibe(&["app", "start", "-g", "org.example/editor#edit"])).contains("EDIT_RAN"),
        "the spelling the entry runs names a launcher"
    );
    assert!(png.is_file() && svg.is_file());
    assert!(apps.settings().join("applications/exported.json").is_file());

    // A launcher the user hid leaves the projection on the next export.
    ok(&apps.vibe(&["app", "launcher", "-g", EDIT, "--hide"]));
    let pruned = json(&project(&["--export-system", "--json"]));
    assert_eq!(pruned["entries"], serde_json::json!([]));
    assert!(!desktop.exists() && !png.exists() && !svg.exists());
    ok(&apps.vibe(&["app", "launcher", "-g", EDIT, "--show"]));
    ok(&project(&["--export-system"]));
    assert!(desktop.is_file());

    let removed = json(&project(&["--remove-system", "--json"]));
    let gone: Vec<&str> = removed["removed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect();
    assert_eq!(gone.len(), 3, "{removed}");
    assert!(!desktop.exists() && !png.exists() && !svg.exists());
    assert!(!apps.settings().join("applications/exported.json").exists());

    // An uninstall removes what an export wrote for the package.
    ok(&project(&["--export-system"]));
    assert!(desktop.is_file());
    ok(&apps.vibe(&["uninstall", "-g", "org.example/editor@1.0.0"]));
    assert!(!desktop.exists() && !png.exists() && !svg.exists());
}

#[test]
fn an_update_carries_the_pin_and_the_collection_and_modify_keeps_work() {
    let apps = installed();
    ok(&apps.vibe(&["app", "pin", "-g", EDIT, "--json"]));
    ok(&apps.vibe(&["app", "collection", "-g", "create", "Daily"]));
    ok(&apps.vibe(&["app", "collection", "-g", "add", "Daily", EDIT]));

    package(&apps, "1.1.0");
    ok(&apps.app(&["update", "-g"], EDITOR, &[]));
    let layer = json(&apps.vibe(&["app", "collection", "-g", "list", "--json"]));
    assert_eq!(layer["pins"]["taskbar"], serde_json::json!([EDIT_NEXT]));
    let collection = |name: &str| {
        layer["collections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == name)
            .unwrap_or_else(|| panic!("no collection {name}: {layer}"))["launchers"]
            .clone()
    };
    assert_eq!(collection("Daily"), serde_json::json!([EDIT_NEXT]));
    assert_eq!(
        collection("Editor Suite"),
        serde_json::json!([EDIT_NEXT]),
        "the seeded key and the carried one are one entry"
    );

    // The rollback carries the arrangement back.
    ok(&apps.vibe(&["app", "rollback", "-g", "org.example/editor@1.1.0"]));
    let layer = json(&apps.vibe(&["app", "collection", "-g", "list", "--json"]));
    assert_eq!(layer["pins"]["taskbar"], serde_json::json!([EDIT]));

    let report = json(&apps.app(
        &["app", "modify", "-g"],
        "org.example/editor@1.0.0",
        &["--keep-work", "--json"],
    ));
    let work = Path::new(
        report["work"]
            .as_str()
            .unwrap_or_else(|| panic!("{report}")),
    );
    assert!(work.join("steps.json").is_file(), "{report}");
}
