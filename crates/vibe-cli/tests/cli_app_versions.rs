//! `vibe app versions` end to end through the real `vibe` binary
//! (PROP-075 `#VERSIONS-LIST`, `#VERSIONS-LIST-OUTPUT`) — campaign phase
//! P5b, `AV-FMT-014`.
//!
//! Two fixtures, because the listing joins two sides. The **domain** side
//! is the real `org.vibevm.node/node` template over upstream's own frozen
//! documents, which is where origins and channels come from; the one
//! `[[remap]]` row reading `https://nodejs.org/dist/` off this filesystem
//! is the mechanism a closed network uses anyway (PROP-066 `#locator`), so
//! **no run here opens a socket**. The **local-state** side is two
//! ordinary `org.example` packages that install offline, which is where
//! `installed (default)`, `installed (hidden)` and `installed (held)` come
//! from.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

/// The real template, its revision, and the frozen upstream documents the
/// other template suites drive (`crates/vibe-template/tests/fixtures/node/`).
const TEMPLATE: &str = include_str!("../../vibe-template/tests/fixtures/node/vibe.toml");
const REVISION: &str = "1.0.0";
const INDEX: &str = include_str!("../../vibe-template/tests/fixtures/node/index-excerpt.json");
const PUBLISHED_24: &str =
    include_str!("../../vibe-template/tests/fixtures/node/vibe-24.22.2.toml");
const ORIGIN: &str = "https://nodejs.org/dist/";

/// The template registry with upstream's frozen version list behind one
/// remap row, and the newest concrete tag published beside the template —
/// the shape `/fast/git/vibe-pkg-node` really has.
fn node_fixture() -> AppRegistry {
    let fixture = AppRegistry::new();
    let root = fixture.root.path();
    let dir = fixture
        .registry()
        .join("org.vibevm.node/node")
        .join(format!("v{REVISION}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("vibe.toml"), TEMPLATE).unwrap();
    let concrete = fixture.registry().join("org.vibevm.node/node/v24.22.2");
    fs::create_dir_all(&concrete).unwrap();
    fs::write(concrete.join("vibe.toml"), PUBLISHED_24).unwrap();

    let mirror = root.join("dist");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("index.json"), INDEX).unwrap();
    fs::write(
        root.join("remap.toml"),
        format!(
            "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
             reason = \"the suite's frozen copy of the Node.js release host\"\n",
            mirror.display()
        ),
    )
    .unwrap();
    fixture
}

/// `vibe app versions <target> --registry … --remap … --json <extra…>`.
fn listing(fixture: &AppRegistry, target: &str, extra: &[&str]) -> Value {
    let remap = fixture.root.path().join("remap.toml");
    let mut args: Vec<&str> = vec!["--remap", remap.to_str().unwrap(), "--json"];
    args.extend_from_slice(extra);
    json(&fixture.app(&["app", "versions"], target, &args))
}

fn rows(listed: &Value) -> Vec<(String, String, String)> {
    listed["packages"][0]["versions"]
        .as_array()
        .expect("the rows")
        .iter()
        .map(|row| {
            (
                row["version"].as_str().unwrap_or_default().to_owned(),
                row["origin"].as_str().unwrap_or_default().to_owned(),
                row["state"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_template_package_lists_source_and_published_versions_with_their_origins() {
    let fixture = node_fixture();
    let listed = listing(&fixture, "org.vibevm.node/node", &["--all"]);
    assert_eq!(listed["protocol"], "vibe-app-versions/1");
    let rows = rows(&listed);
    assert!(
        rows.len() > 100,
        "the frozen window is the whole domain: {}",
        rows.len()
    );
    // Newest first, and the newest of the frozen window is upstream's own.
    assert_eq!(rows[0].0, "26.13.0");
    assert_eq!(rows[0].1, "template");
    // The one version published concretely beside the template wins and
    // says so (PROP-075 `#TEMPLATE-CONCRETE-WINS`).
    let concrete = rows
        .iter()
        .find(|(version, ..)| version == "24.22.2")
        .expect("the published tag");
    assert_eq!(concrete.1, "overrides");
    // The template revision is not a produced version.
    assert!(
        !rows.iter().any(|(version, ..)| version == REVISION),
        "a revision is never a row: {rows:?}"
    );
    // Nothing is installed, so every row is `available` — and nothing was
    // rendered, because a listing is the skeleton path.
    assert!(rows.iter().all(|(.., state)| state == "available"));
    assert!(
        !fixture
            .settings()
            .join("cache/templates/org.vibevm.node/node/rendered")
            .exists(),
        "a listing renders nothing (PROP-075 `#VERSIONS-LIST-OUTPUT`)"
    );
    // The template's provenance, as `--json` carries it.
    let template = &listed["packages"][0]["template"];
    assert_eq!(template["coordinate"], "org.vibevm.node/node");
    assert_eq!(template["revision"], REVISION);
    assert!(template["fetched_at"].is_string(), "{template}");
    assert_eq!(template["version_list_url"], format!("{ORIGIN}index.json"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn the_channel_memberships_are_the_template_s_own_rules() {
    let fixture = node_fixture();
    let listed = listing(&fixture, "org.vibevm.node/node", &["--all"]);
    let table = &listed["packages"][0]["channels"];
    // `lts` holds every release whose `lts` field is truthy, newest first;
    // `lts/jod` holds that codename's.
    assert_eq!(table["lts"][0], "26.13.0");
    assert_eq!(table["lts/jod"][0], "22.23.4");
    // And the row of a member names the channels it belongs to.
    let newest = &listed["packages"][0]["versions"][0];
    assert_eq!(newest["channels"][0], "lts");

    // `--channel` narrows the listing to that channel's members.
    let jod = listing(
        &fixture,
        "org.vibevm.node/node",
        &["--channel", "lts/jod", "--all"],
    );
    let rows = rows(&jod);
    assert!(!rows.is_empty());
    assert_eq!(rows[0].0, "22.23.4");
    assert!(
        rows.iter().all(|(version, ..)| version.starts_with("22.")),
        "`lts/jod` is one codename's releases: {rows:?}"
    );

    // A codename upstream really used, but below the template's window, is
    // no channel of this package — refused by name, never widened.
    let remap = fixture.root.path().join("remap.toml");
    let refusal = failed(&fixture.app(
        &["app", "versions"],
        "org.vibevm.node/node",
        &[
            "--remap",
            remap.to_str().unwrap(),
            "--json",
            "--channel",
            "lts/argon",
        ],
    ));
    assert!(
        refusal.contains("unknown_channel") && refusal.contains("lts/argon"),
        "{refusal}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn without_all_the_newest_twenty_are_shown_and_the_rest_counted() {
    let fixture = node_fixture();
    let listed = listing(&fixture, "org.vibevm.node/node", &[]);
    let package = &listed["packages"][0];
    assert_eq!(package["shown"], 20);
    let total = package["total"].as_u64().unwrap();
    assert_eq!(package["more"].as_u64().unwrap(), total - 20);
    assert_eq!(rows(&listed).len(), 20);
    // `--limit` takes the newest N instead, and the human trailer says how
    // many were left out.
    let listed = listing(&fixture, "org.vibevm.node/node", &["--limit", "3"]);
    assert_eq!(listed["packages"][0]["shown"], 3);
    assert_eq!(rows(&listed)[0].0, "26.13.0");
    let remap = fixture.root.path().join("remap.toml");
    let human = ok(&fixture.app(
        &["app", "versions"],
        "org.vibevm.node/node",
        &["--remap", remap.to_str().unwrap(), "--limit", "3"],
    ));
    assert!(human.contains("… and 403 more"), "{human}");
    assert!(
        human.contains("26.13.0 (template) [lts, lts/lithium]"),
        "{human}"
    );
    // The published row's human spelling is the long one, and it is shown
    // where the limit reaches it.
    let whole = ok(&fixture.app(
        &["app", "versions"],
        "org.vibevm.node/node",
        &["--remap", remap.to_str().unwrap(), "--all"],
    ));
    assert!(
        whole.contains("24.22.2 (published (overrides template))"),
        "the published tag's human origin is the long spelling"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_pattern_narrows_the_listing_by_the_prefix_rule_a_version_file_uses() {
    let fixture = node_fixture();
    let versions = |extra: &[&str]| {
        listing(&fixture, "org.vibevm.node/node", extra)
            .pointer("/packages/0/versions")
            .cloned()
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["version"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    // A bare major is every version of it, newest first, and nothing else.
    let major = versions(&["--all", "22"]);
    assert!(major.len() > 1, "{major:?}");
    assert!(
        major.iter().all(|version| version.starts_with("22.")),
        "{major:?}"
    );
    assert_eq!(major[0], "22.23.4");
    // A minor prefix is that minor alone.
    assert_eq!(versions(&["--all", "22.9"]), ["22.9.0"]);
    // A complete version is itself, and never the newer one a caret would
    // have reached — the same rule a version file's value follows.
    assert_eq!(versions(&["--all", "22.9.0"]), ["22.9.0"]);
    assert_eq!(versions(&["--all", "v22.9.0"]), ["22.9.0"]);
    // A requirement works as written and a channel narrows as `--channel`
    // does.
    assert_eq!(versions(&["--all", "=22.9.0"]), ["22.9.0"]);
    let lts = versions(&["--all", "lts/jod"]);
    assert_eq!(lts, versions(&["--all", "--channel", "lts/jod"]));
    assert!(!lts.is_empty() && lts.iter().all(|v| v.starts_with("22.")));
    // An `@VER` suffix on the target is the same filter, which is the
    // spelling that used to be accepted and then ignored.
    assert_eq!(
        listing(&fixture, "org.vibevm.node/node@22.9.0", &["--all"])["packages"][0]["total"],
        1
    );
    // The human listing is narrowed too, with no "and N more" left over.
    let remap = fixture.root.path().join("remap.toml");
    let human = ok(&fixture.app(
        &["app", "versions"],
        "org.vibevm.node/node",
        &["--remap", remap.to_str().unwrap(), "22.9"],
    ));
    assert!(human.contains("1 version(s)"), "{human}");
    assert!(human.contains("22.9.0 (template)"), "{human}");
    assert!(!human.contains("more"), "{human}");
    // A pattern that is neither a version pattern nor a channel refuses by
    // name, before any version is listed.
    let message = failed(&fixture.app(
        &["app", "versions"],
        "org.vibevm.node/node",
        &["--remap", remap.to_str().unwrap(), "22.."],
    ));
    assert!(message.contains("22.."), "{message}");
}

// ---------------------------------------------------------------------------
// the local-state side: two ordinary packages that install offline
// ---------------------------------------------------------------------------

/// An `org.example` application in slot `slot` that installs from its own
/// payload, so every run here is offline.
fn app(fixture: &AppRegistry, name: &str, version: &str, slot: &str) {
    let root = fixture.package(
        name,
        version,
        slot,
        r#"
path_dirs = ['bin']
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='App'
executable='$app_bin/rt'
kind='command'
default=true
"#,
    );
    fixture.script(&root, "rt", &format!("{name}-{version}"));
}

fn install(fixture: &AppRegistry, spelling: &str) {
    ok(&fixture.app(
        &["install", "-g"],
        spelling,
        &["--offline", "--unattended", "--json"],
    ));
}

/// `vibe app versions <target> --registry … --json <extra…>`, no remap.
fn plain_listing(fixture: &AppRegistry, target: &str, extra: &[&str]) -> Value {
    let mut args: Vec<&str> = vec!["--json"];
    args.extend_from_slice(extra);
    json(&fixture.app(&["app", "versions"], target, &args))
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn the_local_state_and_the_trailer_are_this_home_s_own() {
    let fixture = AppRegistry::new();
    for version in ["1.0.0", "1.1.0", "1.2.0"] {
        app(&fixture, "runtime", version, "runtime");
    }
    install(&fixture, "org.example/runtime@=1.0.0");
    install(&fixture, "org.example/runtime@=1.1.0");
    install(&fixture, "org.example/runtime@=1.2.0");
    // The first install owns the slot; name the default, then hold one and
    // hide another, so all four states are present at once.
    ok(&fixture.vibe(&[
        "app",
        "use",
        "runtime",
        "org.example/runtime@1.2.0",
        "-g",
        "--json",
    ]));
    ok(&fixture.vibe(&["app", "hold", "org.example/runtime@1.1.0", "-g", "--json"]));
    ok(&fixture.vibe(&["app", "hide", "org.example/runtime@1.0.0", "-g", "--json"]));

    let listed = plain_listing(&fixture, "runtime", &[]);
    assert_eq!(listed["slot"], "runtime");
    assert_eq!(listed["default"], "org.example/runtime@1.2.0");
    assert_eq!(listed["off"], false);
    assert_eq!(
        rows(&listed),
        vec![
            (
                "1.2.0".into(),
                "published".into(),
                "installed_default".into()
            ),
            ("1.1.0".into(), "published".into(), "installed_held".into()),
            (
                "1.0.0".into(),
                "published".into(),
                "installed_hidden".into()
            ),
        ]
    );
    // The human trailer names the slot and its default.
    let human = ok(&fixture.vibe(&["app", "versions", "runtime", "-g"]));
    assert!(human.contains("slot `runtime`"), "{human}");
    assert!(
        human.contains("default org.example/runtime@1.2.0"),
        "{human}"
    );
    assert!(
        human.contains("1.1.0 (published) — installed (held)"),
        "{human}"
    );

    // A slot switched off says so, and still names the remembered default.
    ok(&fixture.vibe(&["app", "use", "runtime", "--none", "-g", "--json"]));
    let listed = plain_listing(&fixture, "runtime", &[]);
    assert_eq!(listed["off"], true);
    assert_eq!(listed["default"], "org.example/runtime@1.2.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn installed_answers_with_the_registry_taken_away_entirely() {
    let fixture = AppRegistry::new();
    for version in ["1.0.0", "1.1.0"] {
        app(&fixture, "runtime", version, "runtime");
    }
    install(&fixture, "org.example/runtime@=1.0.0");
    install(&fixture, "org.example/runtime@=1.1.0");
    // With the registry gone there is nothing to read a domain from, so a
    // listing that consulted one could not answer at all.
    fs::remove_dir_all(fixture.registry()).unwrap();
    let listed =
        json(&fixture.vibe(&["app", "versions", "runtime", "--installed", "-g", "--json"]));
    assert_eq!(
        rows(&listed),
        vec![
            ("1.1.0".into(), "published".into(), "installed".into()),
            (
                "1.0.0".into(),
                "published".into(),
                "installed_default".into()
            ),
        ]
    );
    assert_eq!(listed["packages"][0]["total"], 2);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_slot_with_two_packages_lists_each_under_its_own_heading() {
    let fixture = AppRegistry::new();
    app(&fixture, "runtime", "1.0.0", "runtime");
    app(&fixture, "other", "2.0.0", "runtime");
    install(&fixture, "org.example/runtime@=1.0.0");
    install(&fixture, "org.example/other@=2.0.0");
    let listed = plain_listing(&fixture, "runtime", &["--installed"]);
    let headings: Vec<&str> = listed["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| package["package"].as_str().unwrap())
        .collect();
    assert_eq!(headings, ["org.example/other", "org.example/runtime"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_word_that_names_nothing_is_refused_and_names_the_slots_that_exist() {
    let fixture = AppRegistry::new();
    app(&fixture, "runtime", "1.0.0", "runtime");
    install(&fixture, "org.example/runtime@=1.0.0");
    let refusal = failed(&fixture.vibe(&["app", "versions", "nodejs", "-g", "--json"]));
    assert!(
        refusal.contains("unknown_target") && refusal.contains("`runtime`"),
        "{refusal}"
    );
}
