//! R3.4 — a direct install whose POST-DURABILITY phase row fails reports a
//! lifecycle run that keeps the slot work it already did.
//!
//! `vibe install` runs its `phase:install` ritual after the world is durable.
//! A handler that fails there is a lifecycle failure inside an install command
//! — so the document is a `cli-lifecycle-report` and explicitly not an install
//! root — and the slot rows the apply already executed belong in front of the
//! failed phase row, because they really ran.
//!
//! The existing trace suite proves the family for a project with no
//! dependencies at all. This one adds the half that only a real dependency can
//! show: the PREFIX. Delete the classifying wrapper around
//! `after_direct_install`, or the `prepend_lifecycle_rows` call inside it, and
//! the slot row vanishes from a document that still claims to describe the run.
//!
//! Everything here operates on a temporary project.

mod common;
mod trace_support;

use std::fs;
use std::path::Path;

use common::{UserScratch, git_available, run_git, write_project_with_per_package_registry};
use trace_support::index_of;

/// Publish `org.demo/tools@0.1.0` with a `slot:post-install` builtin row that
/// SUCCEEDS — the prefix this test is about.
fn publish_slot_log(root: &Path) {
    let source = root.join("src-tools");
    fs::create_dir_all(&source).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"flow\"\n\
         version = \"0.1.0\"\n\n\
         [[extension]]\nid = \"slot-log\"\npoint = \"slot:post-install\"\n\
         handler = { kind = \"builtin\", name = \"log\" }\n\
         config = { message = \"SLOT-ROW-RAN\" }\n",
    )
    .unwrap();
    fs::write(source.join("payload.txt"), "payload one\n").unwrap();
    run_git(&source, &["add", "-A"]);
    run_git(&source, &["commit", "-m", "org.demo/tools@0.1.0"]);
    run_git(&source, &["tag", "v0.1.0"]);

    let bare = root.join("org.demo.tools.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
}

/// A project wired to that registry, authoring a `phase:install` script that
/// fails with a distinctive stderr. `declared` wires the dependency in — the
/// only difference between the subject and its control.
fn project_with_failing_row(
    user: &UserScratch,
    registry: &Path,
    declared: bool,
) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(
        project.path(),
        &format!(
            "git+file://{}",
            registry.to_string_lossy().replace('\\', "/")
        ),
    );
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/fail.sh"),
        "printf HANDLER-OUT\nprintf HANDLER-ERR >&2\nexit 29\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/fail.ps1"),
        "Write-Output HANDLER-OUT\n[Console]::Error.Write('HANDLER-ERR')\nexit 29\n",
    )
    .unwrap();

    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    if declared {
        text.push_str(
            "\n[requires]\npackages = { \"flow:org.demo/tools\" = \
             { version = \"^0.1\", link = \"static\" } }\n",
        );
    }
    text.push_str(
        "\n[[extension]]\nid='post-durable'\npoint='phase:install'\n\
         handler={ kind = \"script\", base = \"scripts/fail\" }\n",
    );
    fs::write(&manifest, text).unwrap();
    project
}

fn documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// The failed phase row follows the slot row that really ran, in ONE lifecycle
/// root, and the trace says only the fixed word.
#[test]
fn a_failing_post_durability_row_reports_a_lifecycle_root_after_its_slot_prefix() {
    if !git_available() {
        eprintln!("skipping direct callback failure e2e: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    publish_slot_log(outer.path());
    let user = UserScratch::new();
    let project = project_with_failing_row(&user, outer.path(), true);

    let install = |path: &Path| {
        user.vibe()
            .args(["install", "--trace-compile", "--json", "--assume-yes"])
            .arg("--path")
            .arg(path)
            .output()
            .unwrap()
    };
    let output = install(project.path());
    assert!(
        !output.status.success(),
        "the authored handler exits 29: {}",
        String::from_utf8_lossy(&output.stdout),
    );

    let docs = documents(&output.stdout);
    let lifecycle: Vec<&serde_json::Value> = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .collect();
    assert_eq!(lifecycle.len(), 1, "exactly one Lifecycle root: {docs:#?}");
    let report = lifecycle[0];
    assert_eq!(report["ok"], false);
    assert!(
        docs.iter().all(|doc| doc["command"] != "install"),
        "and explicitly NO install root: {docs:#?}",
    );

    // ---- the prefix, in order ------------------------------------------
    let rows: Vec<(&str, &str)> = report["contributions"]
        .as_array()
        .expect("a failed run still reports what it did")
        .iter()
        .map(|row| {
            (
                row["point"].as_str().unwrap_or_default(),
                row["status"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    let slot_at = rows
        .iter()
        .position(|(point, _)| *point == "slot:post-install")
        .unwrap_or_else(|| panic!("the slot row the apply executed is present: {rows:?}"));
    let failed_at = rows
        .iter()
        .position(|(point, status)| *point == "phase:install" && *status == "fail")
        .unwrap_or_else(|| panic!("and the phase row that failed: {rows:?}"));
    assert!(
        slot_at < failed_at,
        "the slot work precedes the failure it preceded: {rows:?}",
    );
    assert_eq!(
        rows[slot_at].1, "ok",
        "the slot row is reported as it really ended: {rows:?}",
    );

    // ---- the failure stayed typed, unchanged and secret -----------------
    //
    // The control is the SAME project without the dependency: no slot rows, so
    // nothing to prepend. If gaining a prefix moved the exit code or reworded
    // the operator's error, these two would differ — the rows are additive and
    // the original error object travels untouched.
    let control = project_with_failing_row(&user, outer.path(), false);
    let bare = install(control.path());
    assert_eq!(
        output.status.code(),
        bare.status.code(),
        "the exit code does not move because rows were carried",
    );
    let tail = String::from_utf8_lossy(&output.stderr).into_owned();
    let bare_tail = String::from_utf8_lossy(&bare.stderr).into_owned();
    assert_eq!(
        tail, bare_tail,
        "nor does the terminal error:
 with: {tail}
 without: {bare_tail}",
    );
    assert!(
        !tail.contains("FailedDraft") && !tail.contains("Carried"),
        "the transport carrier never surfaces: {tail}",
    );
    let trace = trace_support::trace_member(report).expect("the requested trace rides that root");
    assert_eq!(trace["status"], "failed");
    assert_eq!(
        index_of(project.path(), trace["run_id"].as_str().unwrap())
            .failure
            .as_deref(),
        Some("command failed"),
        "a trace records the fixed word and never the command's error",
    );
    assert!(
        !trace_support::all_trace_bytes(project.path()).contains("HANDLER-ERR"),
        "the handler's captured stderr never reaches the trace",
    );
}
