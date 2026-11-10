//! `[recommends]` on the application plane (`PROP-077#APP-RECOMMENDS`,
//! `#APP-RECOMMENDS-MECHANICS`): a JDK that recommends maven installs maven
//! too, exposed, `auto`, with a **weak** requirer edge — best-effort, so
//! every reason a recommendation cannot be installed is one report line and
//! exit 0, and `--no-recommends` installs exactly what was asked for.
//!
//! The weak edge is then what removal reads: it blocks a removal in neither
//! direction, it keeps its target out of the orphan set while it is there,
//! and a package the user installed by name stays `manual` however many weak
//! edges name it.

#![cfg(target_os = "linux")]

mod common;

use std::fs;

use common::app_registry::{AppRegistry, json, ok};
use serde_json::Value;

const JDK: &str = "org.example/jdk@1.0.0";
const MAVEN: &str = "org.example/maven@1.0.0";

/// One `app` package in slot `slot` whose payload carries one command, with
/// `extra` written above `[application]` — where `[recommends]` goes.
fn app(apps: &AppRegistry, name: &str, slot: &str, extra: &str) {
    let dir = apps.registry().join(format!("org.example/{name}/v1.0.0"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
name = "{name}"
group = "org.example"
kind = "app"
version = "1.0.0"
{extra}
[application]
id = "{slot}"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"

[[installer_options.steps]]
id = "place"
title = "Place the payload"
kind = "copy"
from = "payload"

[[installer_options.launchers]]
id = "run"
name = "Run"
executable = "$app_bin/vibe-test-{name}"
kind = "command"

[[installer_options.public_commands]]
name = "vibe-test-{name}"
launcher = "run"
"#,
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    apps.script(&dir, &format!("vibe-test-{name}"), &name.to_uppercase());
}

/// A `flow` package, which the application plane cannot install at all.
fn flow(apps: &AppRegistry, name: &str) {
    let dir = apps.registry().join(format!("org.example/{name}/v1.0.0"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
name = "{name}"
group = "org.example"
kind = "flow"
version = "1.0.0"
"#
        ),
    )
    .unwrap();
}

fn recommends(packages: &[&str]) -> String {
    let list = packages
        .iter()
        .map(|package| format!("\"org.example/{package}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("\n[recommends]\npackages = [{list}]\n")
}

/// The report's recommendation rows by package spelling.
fn reported(document: &Value) -> Vec<(String, String, Option<String>)> {
    document["recommended"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|row| {
            (
                row["package"].as_str().unwrap_or_default().to_owned(),
                row["outcome"].as_str().unwrap_or_default().to_owned(),
                row["key"].as_str().map(ToOwned::to_owned),
            )
        })
        .collect()
}

fn install(apps: &AppRegistry, package: &str, extra: &[&str]) -> Value {
    let mut args = vec!["--offline", "--unattended", "--json"];
    args.extend_from_slice(extra);
    json(&apps.app(&["install", "-g"], package, &args))
}

/// The recorded requirer edges of one installation.
fn requires(apps: &AppRegistry, key: &str) -> Vec<Value> {
    apps.index()["installations"][key]["requires"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS")]
fn a_recommended_application_is_installed_exposed_auto_and_with_a_weak_edge() {
    let apps = AppRegistry::new();
    app(&apps, "maven", "maven", "");
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(report["ok"], true);
    assert_eq!(
        reported(&report),
        vec![(
            "org.example/maven".to_owned(),
            "installed".to_owned(),
            Some(MAVEN.to_owned())
        )]
    );

    // The companion is an ordinary installation with its own record: `auto`,
    // exposed (`#WEAK-OPEN` W3 — a maven the user cannot run is pointless),
    // and in `vibe app list`.
    let listed = apps.listed(MAVEN);
    assert_eq!(listed["mark"], "auto");
    assert_ne!(
        listed["hidden"], true,
        "a recommendation is installed exposed"
    );

    // And the whole new state is one weak edge on the recommender.
    let edges = requires(&apps, JDK);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["key"], MAVEN);
    assert_eq!(edges[0]["weak"], true);
    assert_eq!(
        apps.index()["protocol"],
        "vibe-user-applications/8",
        "a weak edge is the one fact the new generation carries"
    );

    // A second install of the recommender reaches the same inventory: the
    // recommendation is reported as already installed and the edge stays, so
    // the companion is never orphaned by a reinstall.
    let again = install(&apps, "org.example/jdk", &["--force"]);
    assert_eq!(
        reported(&again),
        vec![(
            "org.example/maven".to_owned(),
            "satisfied".to_owned(),
            Some(MAVEN.to_owned())
        )]
    );
    assert_eq!(requires(&apps, JDK)[0]["weak"], true);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
fn no_recommends_installs_exactly_what_was_asked_for() {
    let apps = AppRegistry::new();
    app(&apps, "maven", "maven", "");
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    let report = install(&apps, "org.example/jdk", &["--no-recommends"]);
    assert_eq!(report["ok"], true);
    assert!(reported(&report).is_empty());
    assert_eq!(apps.listed(MAVEN), Value::Null);
    assert!(requires(&apps, JDK).is_empty());
    assert_ne!(
        apps.index()["protocol"],
        "vibe-user-applications/8",
        "a home with no weak edge keeps writing the protocol it wrote before"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS")]
fn every_reason_a_recommendation_cannot_be_installed_is_a_report_line_and_exit_zero() {
    let apps = AppRegistry::new();
    flow(&apps, "discipline");
    app(
        &apps,
        "jdk",
        "jdk",
        &recommends(&["discipline", "absent", "maven"]),
    );
    // `maven` exists for a platform this host is not.
    let maven = apps.registry().join("org.example/maven/v1.0.0");
    fs::create_dir_all(&maven).unwrap();
    fs::write(
        maven.join("vibe.toml"),
        r#"[package]
name = "maven"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "maven"

[[installer_options]]
id = "main"
name = "Main"
platform = "aix"
platform_name = "AIX"
arch = "s390x"

[[installer_options.steps]]
id = "place"
title = "Place the payload"
kind = "copy"
from = "payload"
"#,
    )
    .unwrap();

    // Exit 0: a weak dependency whose failure fails the strong act is not
    // weak (`#WEAK-OPEN` W1).
    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(report["ok"], true);
    let rows = reported(&report);
    assert_eq!(rows.len(), 3, "{rows:?}");
    for (package, outcome, _) in &rows {
        assert_eq!(outcome, "skipped", "{package} should be skipped");
    }
    let reasons: Vec<String> = report["recommended"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["reason"].as_str().unwrap_or_default().to_owned())
        .collect();
    // Each reason names what it was: a package the application plane
    // cannot install at all, one no configured registry carries, and one
    // with no installer option for this host.
    assert!(reasons[0].contains("[application]"), "{reasons:?}");
    assert!(reasons[1].contains("not in the registry"), "{reasons:?}");
    assert!(reasons[2].contains("platform"), "{reasons:?}");
    // And nothing was installed beside the package the user asked for.
    assert!(requires(&apps, JDK).is_empty());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS")]
fn a_failing_recommended_install_is_reported_and_leaves_exit_zero() {
    let apps = AppRegistry::new();
    // A recommendation whose own installation fails: its step exits 1.
    let dir = apps.registry().join("org.example/maven/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
name = "maven"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "maven"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"

[[installer_options.steps]]
id = "fail"
title = "Fail on purpose"
kind = "shell"
run = "exit 1"
"#,
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    // A weak dependency whose failure fails the strong act is not weak
    // (`#WEAK-OPEN` W1): the recommender is installed and the exit status is
    // the one the user's own request earned.
    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(report["ok"], true);
    assert_ne!(apps.listed(JDK), Value::Null);
    let rows = reported(&report);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "skipped");
    assert!(
        !report["recommended"][0]["reason"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "{report}"
    );
    // And no weak edge is recorded for a recommendation that is not there.
    assert!(requires(&apps, JDK).is_empty());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS")]
fn a_recommendation_cycle_terminates_and_installs_each_package_once() {
    let apps = AppRegistry::new();
    app(&apps, "maven", "maven", &recommends(&["jdk"]));
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(report["ok"], true);
    // The visited set is seeded with the package the user named, so the
    // cycle closes after one step and there is no depth limit to reach.
    assert_eq!(
        reported(&report),
        vec![(
            "org.example/maven".to_owned(),
            "installed".to_owned(),
            Some(MAVEN.to_owned())
        )]
    );
    assert_eq!(requires(&apps, JDK).len(), 1);
    assert!(requires(&apps, MAVEN).is_empty());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS")]
fn transitive_recommendations_ride_the_same_walk() {
    let apps = AppRegistry::new();
    app(&apps, "gradle", "gradle", "");
    app(&apps, "maven", "maven", &recommends(&["gradle"]));
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    let report = install(&apps, "org.example/jdk", &[]);
    let rows = reported(&report);
    assert_eq!(rows.len(), 2, "{rows:?}");
    // Breadth-first, in declaration order: the requested package's
    // recommendations first, then the recommendations of each one installed.
    assert_eq!(rows[0].0, "org.example/maven");
    assert_eq!(rows[1].0, "org.example/gradle");
    assert_eq!(requires(&apps, MAVEN)[0]["key"], "org.example/gradle@1.0.0");
    assert_eq!(requires(&apps, MAVEN)[0]["weak"], true);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS")]
fn a_weak_edge_blocks_a_removal_in_neither_direction() {
    let apps = AppRegistry::new();
    app(&apps, "maven", "maven", "");
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));
    install(&apps, "org.example/jdk", &[]);

    // The recommended package goes although the recommender is installed: a
    // recommendation the user has removed is a decision, not a broken
    // dependency.
    ok(&apps.vibe(&[
        "uninstall",
        "-g",
        MAVEN,
        "--assume-yes",
        "--unattended",
        "--json",
    ]));
    assert_eq!(apps.listed(MAVEN), Value::Null);
    assert_ne!(apps.listed(JDK), Value::Null);

    // And the recommender goes with its recommendation still installed,
    // which it then reports as an orphan with the command that removes it.
    install(&apps, "org.example/jdk", &["--force"]);
    let removal = json(&apps.vibe(&[
        "uninstall",
        "-g",
        JDK,
        "--assume-yes",
        "--unattended",
        "--json",
    ]));
    assert_eq!(removal["ok"], true);
    assert_ne!(apps.listed(MAVEN), Value::Null);
    assert!(
        removal["message"]
            .as_str()
            .unwrap_or_default()
            .contains("orphan"),
        "{removal}"
    );

    // An orphaned recommendation is what autoremove is for.
    ok(&apps.vibe(&["app", "autoremove", "--unattended", "--json"]));
    assert_eq!(apps.listed(MAVEN), Value::Null);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS")]
fn a_package_installed_by_name_is_manual_and_autoremove_never_takes_it() {
    let apps = AppRegistry::new();
    app(&apps, "maven", "maven", "");
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    install(&apps, "org.example/maven", &[]);
    // The listing omits the mark of a manual installation, which is what
    // `manual` reads as there.
    assert_eq!(apps.listed(MAVEN)["mark"], Value::Null);
    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(
        reported(&report),
        vec![(
            "org.example/maven".to_owned(),
            "satisfied".to_owned(),
            Some(MAVEN.to_owned())
        )]
    );
    // The edge is recorded, and the mark the user earned is untouched.
    assert_eq!(requires(&apps, JDK)[0]["weak"], true);
    assert_eq!(apps.listed(MAVEN)["mark"], Value::Null);

    ok(&apps.vibe(&[
        "uninstall",
        "-g",
        JDK,
        "--assume-yes",
        "--unattended",
        "--json",
    ]));
    ok(&apps.vibe(&["app", "autoremove", "--unattended", "--json"]));
    assert_ne!(
        apps.listed(MAVEN),
        Value::Null,
        "a manual installation is never autoremoved, however many weak edges named it"
    );
}

/// The plan is the whole truth about the walk, so the walk stops wherever the
/// plan stops: at a companion that is already there
/// (`PROP-077#APP-RECOMMENDS-MECHANICS`). That is also what keeps an earlier
/// `--no-recommends` on the companion's own install from being undone by a
/// later install of something that recommends it.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS")]
fn the_plan_and_the_walk_both_stop_at_a_companion_that_is_already_there() {
    let apps = AppRegistry::new();
    app(&apps, "gradle", "gradle", "");
    app(&apps, "maven", "maven", &recommends(&["gradle"]));
    app(&apps, "jdk", "jdk", &recommends(&["maven"]));

    // The user declined maven's own companion.
    install(&apps, "org.example/maven", &["--no-recommends"]);
    assert_eq!(apps.listed("org.example/gradle@1.0.0"), Value::Null);

    // The plan of the recommender names maven as satisfied and stops.
    let plan = json(&apps.app(
        &["install", "-g"],
        "org.example/jdk",
        &["--offline", "--unattended", "--json", "--plan-only"],
    ));
    assert_eq!(
        reported(&plan),
        vec![(
            "org.example/maven".to_owned(),
            "satisfied".to_owned(),
            Some(MAVEN.to_owned())
        )]
    );

    // And the install does exactly what the plan said — nothing more.
    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(reported(&report), reported(&plan));
    assert_eq!(
        apps.listed("org.example/gradle@1.0.0"),
        Value::Null,
        "a decline on the companion's own install is not undone by its recommender"
    );
    assert_eq!(requires(&apps, JDK)[0]["weak"], true);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#SUGGESTS-UNCHANGED")]
fn the_plan_prints_the_whole_walk_and_a_suggestion_is_reported_and_never_installed() {
    let apps = AppRegistry::new();
    app(&apps, "gradle", "gradle", "");
    app(&apps, "maven", "maven", &recommends(&["gradle"]));
    app(
        &apps,
        "jdk",
        "jdk",
        &format!(
            "{}\n[suggests]\npackages = [\"org.example/ide\"]\n",
            recommends(&["maven"])
        ),
    );

    // Nothing is installed that was not printed first.
    let plan = json(&apps.app(
        &["install", "-g"],
        "org.example/jdk",
        &["--offline", "--unattended", "--json", "--plan-only"],
    ));
    let rows = reported(&plan);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0].0, "org.example/maven");
    assert_eq!(rows[0].1, "planned");
    assert_eq!(rows[1].0, "org.example/gradle");
    assert_eq!(plan["suggested"], serde_json::json!(["org.example/ide"]));
    assert_eq!(apps.listed(MAVEN), Value::Null, "a plan installs nothing");

    // The suggestion is reported after the install too, and installs
    // nothing — not even when a registry would carry it.
    let report = install(&apps, "org.example/jdk", &[]);
    assert_eq!(report["suggested"], serde_json::json!(["org.example/ide"]));
    assert_eq!(apps.listed("org.example/ide@1.0.0"), Value::Null);

    // `--no-recommends` does not silence a suggestion, because it installs
    // nothing to decline.
    let apps = AppRegistry::new();
    app(
        &apps,
        "jdk",
        "jdk",
        "\n[suggests]\npackages = [\"org.example/ide\"]\n",
    );
    let declined = install(&apps, "org.example/jdk", &["--no-recommends"]);
    assert_eq!(
        declined["suggested"],
        serde_json::json!(["org.example/ide"])
    );
}
