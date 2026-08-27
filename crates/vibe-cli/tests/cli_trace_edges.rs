//! R3.4 — the edges: activation across a workspace, the clean epoch's strict
//! rediscovery, a tracked handler failure, and quiet's one-line law under a
//! real failing script.
//!
//! The activation table is the sharp part. `[compile] trace` is read from the
//! SELECTED node and nowhere else, and the trace is stored at the canonical
//! workspace ROOT — so a member that asks for tracing gets a trace, and gets
//! it at the root, and a member that does not ask gets none even when its root
//! does. Those four rows are the difference between "one workspace, one
//! observer" and two members racing for the same lock.

mod common;
mod trace_support;

use std::fs;
use std::path::Path;

use common::UserScratch;
use trace_support::{documents, index_of, run_directories, trace_dir, trace_member};

/// A root with one member. `root`/`member` say which of them declares
/// `[compile] trace = true`.
fn workspace(user: &UserScratch, root_traced: bool, member_traced: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    user.init_project(dir.path());
    let root_manifest = dir.path().join("vibe.toml");
    let mut text = fs::read_to_string(&root_manifest).unwrap();
    text.push_str("\n[workspace]\nmembers = [\"member\"]\n");
    if root_traced {
        text.push_str("\n[compile]\ntrace = true\n");
    }
    fs::write(&root_manifest, text).unwrap();

    let member = dir.path().join("member");
    fs::create_dir_all(&member).unwrap();
    let mut body = String::from(
        "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\n\
         version = \"0.1.0\"\n",
    );
    if member_traced {
        body.push_str("\n[compile]\ntrace = true\n");
    }
    fs::write(member.join("vibe.toml"), body).unwrap();
    dir
}

/// Run `vibe install --json` at `path` and return its one install root.
fn install_root(user: &UserScratch, path: &Path, extra: &[&str]) -> serde_json::Value {
    let output = user
        .vibe()
        .args(["install", "--json", "--offline", "--assume-yes"])
        .args(extra)
        .arg("--path")
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    documents(&output.stdout)
        .into_iter()
        .find(|doc| doc["command"] == "install")
        .expect("one install root")
}

/// The whole activation table, row by row, each on its own workspace.
///
/// The storage half is asserted on every traced row: the run directory belongs
/// to the workspace ROOT, and the member never grows a `.vibe/trace` of its
/// own. One install regenerates shared package units plus every node, so two
/// members with independent trace homes would be two observers of the same
/// work — and two holders of a lock meant to be exclusive.
#[test]
fn activation_reads_the_selected_node_and_stores_at_the_workspace_root() {
    let user = UserScratch::new();

    // Row 1 — selected ROOT, root declares it: traced.
    let one = workspace(&user, true, false);
    let root = install_root(&user, one.path(), &[]);
    assert!(
        trace_member(&root).is_some(),
        "the selected root's own declaration activates",
    );
    assert_eq!(run_directories(one.path()).len(), 1);

    // Row 2 — selected MEMBER, only the root declares it: OFF.
    let two = workspace(&user, true, false);
    let member = two.path().join("member");
    let report = install_root(&user, &member, &[]);
    assert!(
        report.get("trace").is_none(),
        "the ROOT's declaration is not the selected node's: {report}",
    );
    assert!(!trace_dir(two.path()).exists(), "and nothing was stored");
    assert!(!trace_dir(&member).exists());

    // Row 3 — selected MEMBER declares it: traced, AT THE WORKSPACE ROOT.
    let three = workspace(&user, false, true);
    let member = three.path().join("member");
    let report = install_root(&user, &member, &[]);
    let trace = trace_member(&report).expect("the member's own declaration activates");
    let run_id = trace["run_id"].as_str().unwrap();
    assert_eq!(
        run_directories(three.path()),
        vec![run_id.to_string()],
        "the run lives at the canonical workspace root",
    );
    assert!(
        !trace_dir(&member).exists(),
        "and the member never gets a trace tree of its own",
    );
    assert!(
        !trace_support::trace_lock_exists(&member),
        "nor a lock of its own",
    );

    // Row 4 — a DEPENDENCY declares it and nobody else: OFF.
    let four = workspace(&user, false, false);
    let registry = tempfile::tempdir().unwrap();
    trace_support::publish_tracing_package(registry.path());
    let member = four.path().join("member");
    let output = user
        .vibe()
        .args(["install", "org.trace/dep@=0.1.0", "--json", "--registry"])
        .arg(registry.path())
        .arg("--path")
        .arg(&member)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !trace_dir(four.path()).exists() && !trace_dir(&member).exists(),
        "a dependency's `[compile] trace` activates nothing for its host",
    );

    // Row 5 — nobody declares it, but the CLI flag does: traced at the root.
    let five = workspace(&user, false, false);
    let member = five.path().join("member");
    let report = install_root(&user, &member, &["--trace-compile"]);
    let trace = trace_member(&report).expect("the flag alone activates");
    assert_eq!(
        run_directories(five.path()),
        vec![trace["run_id"].as_str().unwrap().to_string()],
        "and still stores at the workspace root, not beside the member",
    );
    assert!(!trace_dir(&member).exists());
}

/// A `phase:clean` script that succeeds and then leaves the workspace
/// unloadable: the clean epoch ran, the strict post-wipe rediscovery refuses,
/// and nothing traced was ever opened.
///
/// The ordering is the whole point. The identity is chosen before the wipe (as
/// it always was) but the SESSION may only open after it — a recorder opened
/// first would have had its own lock and index deleted by the very clean it
/// belongs to. So a rediscovery failure costs no trace artifact at all.
#[test]
fn a_post_clean_rediscovery_failure_leaves_no_trace_artifact() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    // A member that exists now and whose manifest the clean-phase script
    // corrupts, so the post-wipe load has something real to refuse.
    let member = project.path().join("member");
    fs::create_dir_all(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[workspace]\nmembers = [\"member\"]\n");
    text.push_str("\n[[extension]]\nid='sabotage'\npoint='phase:clean'\nhandler={ kind = \"script\", base = \"scripts/sabotage\" }\n");
    fs::write(&manifest, text).unwrap();

    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/sabotage.sh"),
        "printf '[package\\nbroken\\n' > member/vibe.toml\n\
         printf '%s' '{\"artifacts\":[],\"envelope\":1,\"message\":\"ok\",\"status\":\"ok\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/sabotage.ps1"),
        "Set-Content -LiteralPath member/vibe.toml -Value \"[package`nbroken\"\n\
         '{\"artifacts\":[],\"envelope\":1,\"message\":\"ok\",\"status\":\"ok\",\"tasks\":[]}' | Set-Content -LiteralPath $env:VIBE_REPLY -NoNewline\n",
    )
    .unwrap();

    let output = user
        .vibe()
        .args([
            "clean",
            "validate",
            "--trace-compile",
            "--json",
            "--offline",
        ])
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "the corrupted member makes the post-clean load refuse: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("member") || stderr.contains("vibe.toml"),
        "and the refusal names what it could not read: {stderr}",
    );
    assert!(
        !trace_dir(project.path()).exists(),
        "no trace tree — the session had not opened yet",
    );
    assert!(
        !trace_support::trace_lock_exists(project.path()),
        "and no cooperative lock",
    );
    assert!(
        documents(&output.stdout)
            .iter()
            .all(|doc| doc["command"] != "install" && doc["command"] != "lifecycle"),
        "and no traced command root was emitted: {}",
        String::from_utf8_lossy(&output.stdout),
    );
}

/// A real failing `phase:build` script under `--quiet --trace-compile`: one
/// line, the same exit code as its untraced twin, and no captured handler
/// streams anywhere near it.
#[test]
fn a_quiet_traced_phase_failure_is_one_line_with_the_same_exit_code() {
    let build = |flags: &[&str]| {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        fs::create_dir_all(project.path().join("scripts")).unwrap();
        fs::write(
            project.path().join("scripts/fail.sh"),
            "printf PHASE-OUT\nprintf PHASE-ERR >&2\nexit 29\n",
        )
        .unwrap();
        fs::write(
            project.path().join("scripts/fail.ps1"),
            "Write-Output PHASE-OUT\n[Console]::Error.Write('PHASE-ERR')\nexit 29\n",
        )
        .unwrap();
        let manifest = project.path().join("vibe.toml");
        let mut text = fs::read_to_string(&manifest).unwrap();
        text.push_str(
            "\n[[extension]]\nid='fatal'\npoint='phase:build'\nhandler={ kind = \"script\", base = \"scripts/fail\" }\n",
        );
        fs::write(&manifest, text).unwrap();

        let output = user
            .vibe()
            .args(["build", "--quiet", "--assume-yes", "--offline"])
            .args(flags)
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "the script exits 29");
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        (combined, output.status.code(), project)
    };

    let (off, off_code, _off_project) = build(&[]);
    let (on, on_code, on_project) = build(&["--trace-compile"]);

    assert_eq!(
        off.lines().count(),
        1,
        "quiet's one-line law holds with the trace off: {off:?}",
    );
    assert_eq!(on.lines().count(), 1, "and STILL holds with it on: {on:?}",);
    assert!(
        on.contains("compile trace failed"),
        "the suffix says the trace finalised failed: {on:?}",
    );
    assert!(
        !on.contains("PHASE-OUT") && !on.contains("PHASE-ERR"),
        "and quiet never gains the handler's captured streams: {on:?}",
    );
    assert_eq!(off_code, on_code, "the exit code is unchanged");

    let runs = run_directories(on_project.path());
    assert_eq!(runs.len(), 1, "one run: {runs:?}");
    let index = index_of(on_project.path(), &runs[0]);
    assert!(
        matches!(
            index.status,
            vibe_wire::generated::compiler_trace_index::e1::index::RunStatus::Failed
        ),
        "and the on-disk index is terminal failed",
    );
    assert_eq!(
        index.failure.as_deref(),
        Some("command failed"),
        "with the FIXED words and nothing of the script's output",
    );
    assert!(
        !trace_support::all_trace_bytes(on_project.path()).contains("PHASE-ERR"),
        "no captured stream reached any trace file",
    );
}

/// A VALIDATE-ONLY chain must consume the one workspace result too.
///
/// Validate has no install phase, so nothing downstream used to consume the
/// prepared state: it was dropped, validate reported OK, and the ritual
/// planner discovered the tree again. This test repairs the broken sibling
/// AFTER the command's own preparation would have happened — the second read
/// would therefore succeed — and requires the FIRST failure anyway.
///
/// The mutation that makes it fail is deleting the consume-or-refuse step at
/// the top of the phase boundary.
#[test]
fn validate_only_returns_the_first_workspace_failure_even_after_a_repair() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[workspace]\nmembers = [\"member\"]\n");
    fs::write(&manifest, text).unwrap();

    let member = project.path().join("member");
    fs::create_dir_all(&member).unwrap();
    // Broken sibling: the selected manifest is fine, the TREE is not.
    fs::write(member.join("vibe.toml"), "[package\nbroken\n").unwrap();

    let output = user
        .vibe()
        .args(["validate", "--trace-compile", "--json", "--offline"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    // The repair lands after the command has finished, which is the closest a
    // black-box test gets to "between the two reads". The assertion that
    // matters is the one below it: the command refused, and it refused with
    // the tree error rather than sailing past validate.
    fs::write(
        member.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    assert!(
        !output.status.success(),
        "a validate-only chain must refuse the workspace it could not load: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("workspace") || stderr.contains("vibe.toml"),
        "and name what it could not read: {stderr}",
    );
    // A requested trace makes the historically silent failure observable, so
    // a root MAY appear — but it is a FAILED one, and it never reports a
    // validate step that passed.
    for root in documents(&output.stdout)
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
    {
        assert_eq!(root["ok"], false, "the root reports the refusal: {root}");
        assert!(
            root["steps"]
                .as_array()
                .is_none_or(|steps| steps.iter().all(|step| step["status"] != "ok")),
            "and no step claims a validate that never ran: {root}",
        );
    }
    assert!(
        !trace_dir(project.path()).exists(),
        "and no trace opened for a run whose tree never loaded",
    );
}

/// Rows a prerequisite install measured survive a LATER phase failure.
///
/// The failing dispatch carries a draft of its own — it knows about the phase
/// rows it ran and nothing about the install that preceded it. Deleting the
/// prefix merge in the phase boundary drops every earlier successful row and
/// reports a run that "did nothing" when it had already installed a package
/// and run its slot contribution.
#[test]
fn a_later_phase_failure_keeps_the_prerequisite_rows_before_its_own() {
    let user = UserScratch::new();
    let registry = tempfile::tempdir().unwrap();
    // A dependency whose post-install hook SUCCEEDS: the prerequisite row.
    let package = registry.path().join("org.pre").join("ok").join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.pre'\nname='ok'\nkind='tool'\nversion='0.1.0'\n\n\
         [hooks]\npost-install='hooks/ok'\n",
    )
    .unwrap();
    fs::write(package.join("hooks/ok.sh"), "printf PRE-OK\n").unwrap();
    fs::write(package.join("hooks/ok.ps1"), "Write-Output PRE-OK\n").unwrap();

    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    trace_support::declare_static_dependency(project.path(), "tool:org.pre/ok", "=0.1.0");

    // And an authored phase handler that FAILS, after that row exists.
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/fail.sh"),
        "printf LATE-ERR >&2\nexit 29\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/fail.ps1"),
        "[Console]::Error.Write('LATE-ERR')\nexit 29\n",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(
        "\n[[extension]]\nid='late'\npoint='phase:build'\nhandler={ kind = \"script\", base = \"scripts/fail\" }\n",
    );
    fs::write(&manifest, text).unwrap();

    let output = user
        .vibe()
        .args(["build", "--trace-compile", "--json", "--assume-yes"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "the phase handler exits 29");

    let docs = documents(&output.stdout);
    let root = docs
        .iter()
        .find(|doc| doc["command"] == "lifecycle")
        .unwrap_or_else(|| panic!("one failed Lifecycle root: {docs:#?}"));
    assert_eq!(root["ok"], false);

    let rows = root["contributions"].as_array().expect("rows");
    let failing = rows
        .iter()
        .position(|row| row["status"] == "fail")
        .unwrap_or_else(|| panic!("the failing phase row is present: {rows:#?}"));
    assert!(
        failing > 0,
        "the prerequisite install's rows come FIRST, before the failure: {rows:#?}",
    );
    assert!(
        rows[..failing].iter().any(|row| row["point"]
            .as_str()
            .is_some_and(|p| p.starts_with("slot:"))),
        "and at least one of them is the install's own slot row: {rows:#?}",
    );
}
