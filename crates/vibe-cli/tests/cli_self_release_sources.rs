//! `vibe self source` and `vibe self releases` (PROP-019
//! `#release-sources`) in a scratch settings root: choosing a source,
//! case-insensitive types, refusing an incomplete one before anything is
//! written, listing a local release drop, `$VIBEVM_RELEASE_ORIGIN` still
//! winning, and the release row visible to `vibe remap explain`. No test
//! reaches the network.

mod common;

use std::fs;
use std::process::Output;

use common::UserScratch;

fn vibe(user: &UserScratch, args: &[&str]) -> Output {
    let mut command = user.vibe();
    command.env_remove("VIBEVM_RELEASE_ORIGIN").args(args);
    command.output().unwrap()
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_str(&ok(output)).unwrap()
}

#[test]
fn a_local_release_drop_is_chosen_listed_and_explained() {
    let user = UserScratch::new();
    let drop = tempfile::tempdir().unwrap();
    for version in ["1.0.11", "1.0.13"] {
        let dir = drop.path().join(format!("v{version}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("DISTRIBUTIONS.json"), b"{}").unwrap();
    }
    fs::create_dir_all(drop.path().join("v1.0.12")).unwrap();
    let path = drop.path().to_str().unwrap();

    let shown = json(&vibe(
        &user,
        &[
            "--json", "self", "source", "set", "LocalDir", "--path", path,
        ],
    ));
    assert_eq!(shown["source"], "localdir");
    assert_eq!(shown["location"], path);
    let config = fs::read_to_string(user.settings.join("config.toml")).unwrap();
    assert!(
        config.contains("[self]") && config.contains("source = \"localdir\""),
        "{config}"
    );

    let listed = json(&vibe(&user, &["--json", "self", "releases"]));
    let versions: Vec<&str> = listed["releases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["version"].as_str().unwrap())
        .collect();
    assert_eq!(
        versions,
        ["1.0.13", "1.0.11"],
        "an empty v1.0.12 is not a release"
    );

    let explained = ok(&vibe(
        &user,
        &[
            "remap",
            "explain",
            "https://github.com/vibevm/vibevm/releases/download/v1.0.13/DISTRIBUTIONS.json",
        ],
    ));
    assert!(explained.contains("1 matching row"), "{explained}");
    assert!(explained.contains(path), "{explained}");

    // The environment variable the install scripts set still wins.
    let mut command = user.vibe();
    command
        .env("VIBEVM_RELEASE_ORIGIN", "http://127.0.0.1:9/mirror")
        .args(["--json", "self", "source", "show"]);
    let overridden: serde_json::Value =
        serde_json::from_str(&ok(&command.output().unwrap())).unwrap();
    assert_eq!(overridden["source"], "mirror");
    assert_eq!(overridden["from"], "$VIBEVM_RELEASE_ORIGIN");

    let reset = json(&vibe(&user, &["--json", "self", "source", "reset"]));
    assert_eq!(reset["source"], "github");
}

#[test]
fn an_incomplete_source_is_refused_before_anything_is_written() {
    let user = UserScratch::new();
    let refused = vibe(&user, &["self", "source", "set", "nexus"]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("--url"), "{stderr}");
    assert!(!user.settings.join("config.toml").exists());

    let unknown = vibe(&user, &["self", "source", "set", "s3"]);
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("github, gitverse, nexus, localdir"));

    // A source tree query keeps its old meaning.
    let tree = vibe(&user, &["self", "source", "--help"]);
    assert!(ok(&tree).contains("show"));
}

/// `self source set` / `reset` edit `config.toml` in place: the person's
/// comments and every unrelated table keep their exact bytes.
#[test]
fn setting_the_source_keeps_comments_and_unrelated_tables_byte_for_byte() {
    let user = UserScratch::new();
    let drop = tempfile::tempdir().unwrap();
    let path = drop.path().to_str().unwrap();
    let head = "# my vibe settings — hand-written\n\
                [init]\n\
                # who I am\n\
                last_author = 'Me <me@example.com>'\n\n";
    let tail = "[net]\n# keep the network on\noffline = false\n";
    let config = user.settings.join("config.toml");
    fs::write(&config, format!("{head}{tail}")).unwrap();

    ok(&vibe(
        &user,
        &[
            "--json", "self", "source", "set", "localdir", "--path", path,
        ],
    ));
    let written = fs::read_to_string(&config).unwrap();
    assert!(written.starts_with(head), "{written}");
    assert!(written.contains(tail), "{written}");
    assert!(
        written.contains("[self]\nsource = \"localdir\""),
        "{written}"
    );

    ok(&vibe(&user, &["--json", "self", "source", "reset"]));
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        format!("{head}{tail}"),
        "reset removes exactly the table set added"
    );
}
