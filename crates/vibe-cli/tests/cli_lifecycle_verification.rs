//! End-to-end: what `vibe verify --json` really returns (R7.5 P2/A5b).
//!
//! These run the real binary over a real tree, so they prove the whole funnel
//! at once — boundary, reconciliation, carrier, family projection, emission
//! policy and wire law. The two decisive ones are the stale STOP, which is the
//! only path where a silent failure site would drop the member the operator is
//! told to read, and the byte-shape check that a non-verify run still omits
//! the key entirely.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use vibe_wire::behaviour::verification_evidence::validate;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::shared::EvidenceStatus;

/// A project whose build row declares an input scope.
fn project(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Verify Test"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    fs::create_dir_all(project.path().join("data")).unwrap();
    fs::write(project.path().join("data/a.txt"), "one").unwrap();
    append(
        project.path(),
        r#"
[[extension]]
id = "declared-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "DECLARED-BUILD" }
inputs = ["data/**"]
"#,
    );
    project
}

fn append(root: &Path, body: &str) {
    let manifest = root.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(body);
    fs::write(manifest, text).unwrap();
}

/// A create contribution that rewrites a MEASURED build input inside the same
/// invocation — the architecture's own uninterrupted stale example (§4.3).
fn create_row_that_mutates_the_build_input(root: &Path) {
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("scripts/touch.sh"),
        "printf 'two' > data/a.txt\n\
         printf '%s' '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        root.join("scripts/touch.ps1"),
        "'two' | Set-Content -NoNewline data/a.txt\n\
         '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' | Set-Content -NoNewline $env:VIBE_REPLY\n",
    )
    .unwrap();
    append(
        root,
        "\n[[extension]]\nid = \"mutating-create\"\npoint = \"phase:create\"\n\
         handler = { kind = \"script\", base = \"scripts/touch\" }\n",
    );
}

fn run(user: &UserScratch, project: &Path, phase: &str) -> (bool, LifecycleReport, String) {
    let output = user
        .vibe()
        .arg(phase)
        .arg("--json")
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let values = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("stdout is not a JSON stream: {error}\n{stdout}"));
    let last = values
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("no document at all\n{stdout}"));
    let report: LifecycleReport = serde_json::from_value(last.clone()).unwrap_or_else(|error| {
        panic!("the generated reader rejected the root: {error}\n{last:#}")
    });
    (output.status.success(), report, stdout)
}

/// A verify over an untouched tree returns the member, `matched`, and the
/// member obeys its own wire law on the way out.
#[test]
fn verify_json_returns_a_valid_matched_member() {
    let user = UserScratch::new();
    let project = project(&user);

    let (ok, report, stdout) = run(&user, project.path(), "verify");
    assert!(ok, "an untouched tree verifies: {stdout}");
    assert!(report.ok);
    let member = report
        .verification
        .unwrap_or_else(|| panic!("`vibe verify --json` owes the member: {stdout}"));
    validate(&member).expect("what the surface published is a valid member");
    assert_eq!(member.status, EvidenceStatus::Matched);
    assert_eq!(member.run.requested, "verify");
    assert!(
        member.inputs.iter().any(|row| row.phase == "build"),
        "the declared build row is compared: {member:?}",
    );
}

/// A create contribution rewrites a measured build input inside ONE
/// invocation: verify reports `stale`, the command fails, and — the point of
/// this test — the JSON root is still emitted, carrying the exact comparison
/// the operator is told to read.
#[test]
fn a_stale_stop_still_returns_its_member_on_a_failed_verify() {
    let user = UserScratch::new();
    let project = project(&user);
    create_row_that_mutates_the_build_input(project.path());

    let (ok, report, stdout) = run(&user, project.path(), "verify");
    assert!(!ok, "a stale identity stops the chain: {stdout}");
    assert!(!report.ok, "the command's own axis is false");
    let member = report
        .verification
        .unwrap_or_else(|| panic!("a silent stop would emit no root at all: {stdout}"));
    validate(&member).expect("a stopping member is still a valid member");
    assert_eq!(member.status, EvidenceStatus::Stale);
    assert!(
        report
            .steps
            .iter()
            .all(|step| step.phase != "package" && step.phase != "deploy"),
        "verify stopped before any later phase: {:?}",
        report.steps,
    );
}

/// A run that never asked for verify omits the KEY, not merely its value —
/// every pre-R7.5 document stays byte-shape compatible.
#[test]
fn a_build_run_omits_the_member_entirely() {
    let user = UserScratch::new();
    let project = project(&user);

    let (ok, report, stdout) = run(&user, project.path(), "build");
    assert!(ok, "{stdout}");
    assert!(report.verification.is_none());
    assert!(
        !stdout.contains("verification"),
        "an absent member is an absent key: {stdout}",
    );
}
