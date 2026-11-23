//! Install trailers close the merged human stream with actual run evidence.
#![cfg(unix)]

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::UserScratch;

/// Merge streams at the process boundary, including during isolated init.
fn command(user: &UserScratch, home: &Path, args: &[&str]) -> Output {
    Command::new("sh")
        .args([
            "-c",
            "exec \"$@\" 2>&1",
            "vibe-test",
            env!("CARGO_BIN_EXE_vibe"),
        ])
        .args(args)
        .env("HOME", home)
        .env("VIBE_SETTINGS", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS")
        .output()
        .unwrap()
}

/// Successful human installs end after their completed root progress row.
fn trailer(output: &Output) -> String {
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{text}");
    let lines: Vec<_> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let root = lines
        .iter()
        .position(|line| line.starts_with("✓ Installing packages"))
        .unwrap_or_else(|| panic!("no root row: {text}"));
    let last = lines.len() - 1;
    assert!(
        lines[last].starts_with("✻ "),
        "trailer must be last: {text}"
    );
    assert!(root < last, "root row must precede trailer: {text}");
    lines[last].to_string()
}

/// Publish local packages, optionally with one real logging contribution.
fn registry(root: &Path, logging: bool) {
    for name in ["root", "leaf"] {
        let package = root.join("org.fixture").join(name).join("v0.1.0");
        std::fs::create_dir_all(package.join("hooks")).unwrap();
        let extra = if name == "root" {
            "\n[requires.packages]\n\"org.fixture/leaf\" = \"^0.1\"\n"
        } else {
            ""
        };
        let hook = if name == "root" && logging {
            "\n[[extension]]\nid = \"fixture-log\"\npoint = \"slot:post-install\"\nhandler = { kind = \"script\", base = \"hooks/log\" }\napplies_to = { packages = [\"org.fixture/root\"] }\n"
        } else {
            ""
        };
        std::fs::write(package.join("vibe.toml"), format!(
            "[package]\ngroup = \"org.fixture\"\nname = \"{name}\"\nkind = \"tool\"\nversion = \"0.1.0\"\n{extra}{hook}"
        )).unwrap();
        std::fs::write(
            package.join("hooks/log.sh"),
            "printf 'ACTUAL-FIXTURE-LOG\\n'\n",
        )
        .unwrap();
    }
}

/// Init is isolated just like the install, not delegated to an inherited HOME.
fn init(user: &UserScratch, home: &Path, project: &Path) {
    let output = command(
        user,
        home,
        &[
            "init",
            "--path",
            project.to_str().unwrap(),
            "--group",
            "org.test",
            "--name",
            "trailer",
            "--kind",
            "flow",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn install_trailer_is_last_line() {
    let user = UserScratch::new();
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    init(&user, home.path(), project.path());
    let output = command(
        &user,
        home.path(),
        &[
            "install",
            "--offline",
            "--path",
            project.path().to_str().unwrap(),
        ],
    );
    assert!(!trailer(&output).contains("log"));
}

#[test]
fn logged_install_ends_with_evidence_directory() {
    let user = UserScratch::new();
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    registry(local.path(), true);
    init(&user, home.path(), project.path());
    let output = command(
        &user,
        home.path(),
        &[
            "install",
            "org.fixture/root@^0.1",
            "--offline",
            "--assume-yes",
            "--registry",
            local.path().to_str().unwrap(),
            "--path",
            project.path().to_str().unwrap(),
        ],
    );
    let last = trailer(&output);
    assert!(last.contains("org.fixture/root@0.1.0"), "{last}");
    assert!(!last.contains("org.fixture/leaf@"), "{last}");
    assert!(last.contains("1 contribution ran"), "{last}");
    let (_, directory) = last
        .rsplit_once("log ")
        .unwrap_or_else(|| panic!("no log reference: {last}"));
    let directory = Path::new(directory);
    assert!(directory.is_dir(), "{last}");
    assert!(
        std::fs::read_dir(directory.join("logs"))
            .unwrap()
            .any(|entry| {
                std::fs::read_to_string(entry.unwrap().path())
                    .is_ok_and(|text| text.contains("ACTUAL-FIXTURE-LOG"))
            }),
        "the trailer must name the run containing the actual fixture log: {last}"
    );
}

#[test]
fn explicit_root_without_evidence_has_no_log_suffix() {
    let user = UserScratch::new();
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    registry(local.path(), false);
    init(&user, home.path(), project.path());
    let output = command(
        &user,
        home.path(),
        &[
            "install",
            "org.fixture/root@^0.1",
            "--offline",
            "--assume-yes",
            "--registry",
            local.path().to_str().unwrap(),
            "--path",
            project.path().to_str().unwrap(),
        ],
    );
    let last = trailer(&output);
    assert!(last.contains("org.fixture/root@0.1.0"), "{last}");
    assert!(!last.contains("log"), "{last}");
}

#[test]
fn multiple_explicit_roots_have_no_single_root_attribution() {
    let user = UserScratch::new();
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    registry(local.path(), false);
    init(&user, home.path(), project.path());
    let output = command(
        &user,
        home.path(),
        &[
            "install",
            "org.fixture/root@^0.1",
            "org.fixture/leaf@^0.1",
            "--offline",
            "--assume-yes",
            "--registry",
            local.path().to_str().unwrap(),
            "--path",
            project.path().to_str().unwrap(),
        ],
    );
    let last = trailer(&output);
    assert!(!last.contains("org.fixture/"), "{last}");
    assert!(!last.contains("log"), "{last}");
}
