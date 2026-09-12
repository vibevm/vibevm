//! What the reader is before it answers anything: where it binds, what
//! it is pointed at, and what it refuses to start on.

use chrono::{TimeZone, Utc};
use vibe_doc::citations::SpecSources;

use super::*;

fn when() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap()
}

fn package(dir: &Path, extra: &str) {
    std::fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"thing-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
             version = \"0.2.0\"\ntitle = \"Thing Manual\"\n\
             abstract = \"\"\"\nWhat it covers.\n\"\"\"\n\
             [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n{extra}"
        ),
    )
    .unwrap();
}

/// The address a reader answers under is the package's own coordinate
/// and version — the same one the public site mounts it at, so a link
/// written on a page works in both worlds.
#[test]
fn the_mount_is_the_packages_own_address() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path(), "");
    let reader = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when()).unwrap();
    assert_eq!(reader.mount(), "/doc/com.example/thing-docs/0.2.0/");
}

/// The host is not a setting. `##LOCAL-SERVE` binds the loopback and
/// records «binding to 0.0.0.0» as considered and rejected, so the
/// constant is the whole of the decision and there is no flag beside it.
#[test]
fn the_bind_host_is_the_loopback_and_is_not_configurable() {
    assert!(BIND_HOST.is_loopback());
    // The config a caller may set carries a port and no host — the type
    // itself is what makes the rule unbreakable from the CLI.
    let config = Config::new("x");
    assert_eq!(config.port, DEFAULT_PORT);
}

/// The policy names no external source, and `frame-ancestors` is the
/// one part a launch parameter moves.
#[test]
fn the_policy_names_no_outside_source_and_only_frames_are_a_parameter() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path(), "");
    let plain = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when()).unwrap();
    assert!(
        plain.csp.ends_with("frame-ancestors 'none'"),
        "{}",
        plain.csp
    );
    for source in ["http:", "https:", "*", "unsafe-inline", "unsafe-eval"] {
        assert!(!plain.csp.contains(source), "{source} in {}", plain.csp);
    }

    let embedded = Reader::open(
        &Config {
            frame_ancestor: Some("vscode-webview://abc".to_string()),
            ..Config::new(tmp.path())
        },
        SpecSources::new(),
        when(),
    )
    .unwrap();
    assert!(
        embedded
            .csp
            .ends_with("frame-ancestors vscode-webview://abc"),
        "{}",
        embedded.csp
    );
}

/// One package is one language, so `--lang` is the caller stating which
/// one it wants; a package in another is refused at START-UP rather than
/// served under a label that is not true.
#[test]
fn a_package_in_another_language_is_refused_before_it_is_served() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path(), "[i18n]\ncanonical = \"en\"\n");
    let refused = Reader::open(
        &Config {
            lang: Some("ru".to_string()),
            ..Config::new(tmp.path())
        },
        SpecSources::new(),
        when(),
    )
    .expect_err("the package is English");
    assert!(refused.to_string().contains("thing-docs-ru"), "{refused}");
}

/// A directory that is not a package cannot be served at an address,
/// and the refusal says which rule it broke and what to do.
#[test]
fn a_directory_that_is_not_a_package_is_a_named_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let e = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when())
        .expect_err("there is no manifest");
    assert!(e.to_string().contains("LOCAL-SERVE"), "{e}");
    assert!(e.to_string().contains("vibe cache add"), "{e}");
}
