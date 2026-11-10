//! `keyed` version files and `--write-version-file`, end to end through the
//! real `vibe` binary and a local directory registry
//! (PROP-075 `#VERSION-FILE-KEYED`, `#VERSION-FILE-WRITE`).
//!
//! One fixture package declares one `plain` row and one `keyed` row, which
//! is the shape a Java pass needs: `.jdkrc` for the tool alone and
//! `.sdkmanrc` shared with every other tool of the project. The byte
//! assertions are deliberate — the value of the `keyed` format is that the
//! lines a write did not mean to touch are not touched.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

/// A runtime package declaring one `plain` and one `keyed` version file.
fn runtime(r: &AppRegistry, name: &str, key: &str, version: &str) {
    let root = r.package(
        name,
        version,
        name,
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
"#,
    );
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace(
            &format!("[application]\nid = \"{name}\"\n"),
            &format!(
                "[application]\nid = \"{name}\"\n\n\
                 [[application.version_file]]\nformat = \"plain\"\nnames = [\".{name}rc\"]\n\n\
                 [[application.version_file]]\nformat = \"keyed\"\n\
                 names = [\".sdkmanrc\"]\nkey = \"{key}\"\nseparator = \"equals\"\n",
            ),
        ),
    )
    .unwrap();
    r.script(&root, "rt", name);
}

fn install(r: &AppRegistry, spelling: &str, extra: &[&str]) -> Value {
    let mut args = vec!["--offline", "--unattended", "--json"];
    args.extend_from_slice(extra);
    json(&r.app(&["install", "-g"], spelling, &args))
}

/// A project directory with a `vibe.toml`, so the upward search has a root
/// to stop at and a write with no file has a place to create one.
fn project(r: &AppRegistry, name: &str) -> PathBuf {
    let dir = r.root.path().join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!("[project]\nname = \"{name}\"\nversion = \"1.0.0\"\n"),
    )
    .unwrap();
    dir
}

fn env_in(r: &AppRegistry, dir: &Path) -> Value {
    json(&r.vibe_in(dir, &["app", "env", "--project", "--json"]))
}

fn selection<'a>(document: &'a Value, slot: &str) -> &'a Value {
    document["selections"]
        .as_array()
        .expect("selections")
        .iter()
        .find(|row| row["slot"] == slot)
        .unwrap_or_else(|| panic!("no selection for `{slot}` in {document:#}"))
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn one_keyed_file_resolves_two_slots_each_from_its_own_key() {
    let r = AppRegistry::new();
    runtime(&r, "alpha", "alpha", "1.0.0");
    runtime(&r, "alpha", "alpha", "2.0.0");
    runtime(&r, "beta", "beta", "1.0.0");
    install(&r, "org.example/alpha@1.0.0", &[]);
    install(&r, "org.example/alpha@2.0.0", &[]);
    install(&r, "org.example/beta@1.0.0", &[]);

    // The declaration reaches the sidecar as declared, the keyed row with
    // its whole line syntax.
    let sidecar: Value = serde_json::from_slice(
        &fs::read(r.settings().join("applications/version-files.json")).unwrap(),
    )
    .unwrap();
    let rows = &sidecar["installations"]["org.example/alpha@2.0.0"]["rows"];
    assert_eq!(rows[0]["format"], "plain");
    assert_eq!(rows[1]["format"], "keyed");
    assert_eq!(rows[1]["key"], "alpha");
    assert_eq!(rows[1]["separator"], "equals");
    assert_eq!(rows[1]["comment"], "hash", "the default is written as read");
    assert_eq!(rows[1]["extra"], "refuse");

    let dir = project(&r, "shared");
    fs::write(
        dir.join(".sdkmanrc"),
        "# the team's toolchain\nalpha=1.0.0\nbeta = 1.0.0  # pinned\ngamma=9.9.9\n",
    )
    .unwrap();
    let document = env_in(&r, &dir);
    // One file, two slots, each from its own key — and `gamma`, which this
    // home has no package for, is ignored without a notice.
    assert_eq!(
        selection(&document, "alpha")["key"],
        "org.example/alpha@1.0.0"
    );
    assert_eq!(
        selection(&document, "beta")["key"],
        "org.example/beta@1.0.0"
    );
    let notices = document["notices"].as_array().cloned().unwrap_or_default();
    assert!(
        notices
            .iter()
            .all(|notice| !notice.as_str().unwrap_or_default().contains("gamma")),
        "another slot's key in a shared file is not a defect: {document:#}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn install_g_writes_the_resolved_version_into_the_named_keyed_file() {
    let r = AppRegistry::new();
    runtime(&r, "alpha", "alpha", "1.0.0");
    runtime(&r, "alpha", "alpha", "2.0.0");
    let dir = project(&r, "write");
    let shared = dir.join(".sdkmanrc");
    let before = "# the team's toolchain\nbeta = 3.9.9  # pinned\n\n";
    fs::write(&shared, before).unwrap();

    // `install -g` of a channel-free requirement, writing the file the
    // flag names. The version written is the resolved one.
    let registry = r.registry();
    let report = json(&r.vibe_in(
        &dir,
        &[
            "install",
            "-g",
            "org.example/alpha@^1",
            "--registry",
            registry.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
            "--write-version-file=.sdkmanrc",
        ],
    ));
    assert_eq!(report["installation"], "org.example/alpha@1.0.0");
    assert_eq!(report["versionFile"]["version"], "1.0.0");
    assert!(
        report["versionFile"]["path"]
            .as_str()
            .unwrap()
            .ends_with(".sdkmanrc"),
        "{report:#}"
    );
    // The other key's line, its comment, the heading comment and the blank
    // line are byte-identical; only `alpha` was added.
    assert_eq!(
        fs::read_to_string(&shared).unwrap(),
        format!("{before}alpha=1.0.0\n")
    );

    // A second install rewrites that one line in place.
    let report = json(&r.vibe_in(
        &dir,
        &[
            "install",
            "-g",
            "org.example/alpha@2.0.0",
            "--registry",
            registry.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
            "--write-version-file=.sdkmanrc",
        ],
    ));
    assert_eq!(report["versionFile"]["version"], "2.0.0");
    assert_eq!(
        fs::read_to_string(&shared).unwrap(),
        format!("{before}alpha=2.0.0\n")
    );
    // And the file now selects that version through the ordinary search.
    assert_eq!(
        selection(&env_in(&r, &dir), "alpha")["key"],
        "org.example/alpha@2.0.0"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn app_use_writes_the_nearest_file_and_a_bare_flag_creates_the_first_name() {
    let r = AppRegistry::new();
    runtime(&r, "alpha", "alpha", "1.0.0");
    runtime(&r, "alpha", "alpha", "2.0.0");
    install(&r, "org.example/alpha@1.0.0", &[]);
    install(&r, "org.example/alpha@2.0.0", &[]);
    let dir = project(&r, "use");

    // No declared file exists yet: the first name of the first row is
    // created in the project root.
    let listing = json(&r.vibe_in(
        &dir,
        &[
            "app",
            "use",
            "-g",
            "alpha",
            "org.example/alpha@1.0.0",
            "--json",
            "--write-version-file",
        ],
    ));
    assert_eq!(listing["version_file"]["version"], "1.0.0");
    assert_eq!(fs::read_to_string(dir.join(".alpharc")).unwrap(), "1.0.0\n");

    // Now that file is the nearest one the search finds, so a bare flag
    // run from a sub-directory edits it rather than creating a second.
    let deep = dir.join("sub");
    fs::create_dir_all(&deep).unwrap();
    let text = ok(&r.vibe_in(
        &deep,
        &[
            "app",
            "use",
            "-g",
            "alpha",
            "org.example/alpha@2.0.0",
            "--write-version-file",
        ],
    ));
    let _ = text;
    assert_eq!(fs::read_to_string(dir.join(".alpharc")).unwrap(), "2.0.0\n");
    assert!(!deep.join(".alpharc").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-VALUE")]
fn a_written_version_file_pins_what_it_recorded_and_decides_a_bare_install() {
    let r = AppRegistry::new();
    runtime(&r, "alpha", "alpha", "1.0.0");
    runtime(&r, "alpha", "alpha", "1.1.0");
    install(&r, "org.example/alpha@1.1.0", &[]);
    let dir = project(&r, "pin");
    let registry = r.registry();

    // `vibe install -g` with no version: the nearest declared file decides,
    // and the install says which one did (`B-262`).
    fs::write(dir.join(".alpharc"), "1.0.0\n").unwrap();
    let output = r.vibe_in(
        &dir,
        &[
            "install",
            "-g",
            "org.example/alpha",
            "--registry",
            registry.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    );
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    let report = json(&output);
    assert_eq!(
        report["installation"], "org.example/alpha@1.0.0",
        "the file asks for 1.0.0, not the newest: {report:#}"
    );
    assert!(
        said.contains(".alpharc") && said.contains("=1.0.0"),
        "the install must name the file that decided: {said}"
    );

    // And the file keeps pinning what `--write-version-file` recorded, with
    // a newer version installed beside it (`B-265`).
    assert_eq!(
        selection(&env_in(&r, &dir), "alpha")["key"],
        "org.example/alpha@1.0.0"
    );
    // A partial value is a prefix: `1.0` is still 1.0.0, `1` is the newest
    // of the major.
    fs::write(dir.join(".alpharc"), "1.0\n").unwrap();
    assert_eq!(
        selection(&env_in(&r, &dir), "alpha")["key"],
        "org.example/alpha@1.0.0"
    );
    fs::write(dir.join(".alpharc"), "1\n").unwrap();
    assert_eq!(
        selection(&env_in(&r, &dir), "alpha")["key"],
        "org.example/alpha@1.1.0"
    );

    // The command line is untouched: `SLOT=VER` stays a requirement, so a
    // bare `1.0.0` there still resolves to the newest that satisfies it.
    let document = json(&r.vibe_in(&dir, &["app", "env", "--json", "--use", "alpha=1.0.0"]));
    assert_eq!(
        selection(&document, "alpha")["key"],
        "org.example/alpha@1.1.0",
        "a command line's `1.0.0` is a requirement, never a pin: {document:#}"
    );

    // A spelling that already carries a version is never second-guessed.
    fs::write(dir.join(".alpharc"), "1.0.0\n").unwrap();
    let report = json(&r.vibe_in(
        &dir,
        &[
            "install",
            "-g",
            "org.example/alpha@=1.1.0",
            "--registry",
            registry.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(report["installation"], "org.example/alpha@1.1.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn every_write_refusal_lands_before_a_byte_is_written() {
    let r = AppRegistry::new();
    runtime(&r, "alpha", "alpha", "1.0.0");
    install(&r, "org.example/alpha@1.0.0", &[]);
    let dir = project(&r, "refuse");

    // A path matching no declared name names the path and the names.
    let message = failed(&r.vibe_in(
        &dir,
        &[
            "app",
            "use",
            "-g",
            "alpha",
            "org.example/alpha@1.0.0",
            "--write-version-file=.jvmrc",
        ],
    ));
    assert!(message.contains(".jvmrc"), "{message}");
    assert!(message.contains(".sdkmanrc"), "{message}");
    assert!(!dir.join(".jvmrc").exists());

    // A keyed file carrying the key twice names both lines and writes
    // nothing.
    let twice = "alpha=1.0.0\nbeta=2\nalpha=2.0.0\n";
    fs::write(dir.join(".sdkmanrc"), twice).unwrap();
    let message = failed(&r.vibe_in(
        &dir,
        &[
            "app",
            "use",
            "-g",
            "alpha",
            "org.example/alpha@1.0.0",
            "--write-version-file=.sdkmanrc",
        ],
    ));
    assert!(message.contains("lines 1 and 3"), "{message}");
    assert_eq!(fs::read_to_string(dir.join(".sdkmanrc")).unwrap(), twice);

    // The flag belongs to an application install and to `app use`; a
    // project install refuses it by name.
    let message = failed(&r.vibe_in(&dir, &["install", "--write-version-file"]));
    assert!(message.contains("--write-version-file"), "{message}");
    assert!(message.contains("application install"), "{message}");
}
