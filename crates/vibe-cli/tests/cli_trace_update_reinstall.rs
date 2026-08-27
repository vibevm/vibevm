//! R3.4 — WHEN `vibe update` and `vibe reinstall` record a compile trace, and
//! what a disabled run of either leaves behind.
//!
//! Every test drives the real binary. Activation is a property of the command
//! as invoked — which manifest it consulted, which root it locked — and a unit
//! test of the truth table would prove none of that.
//!
//! The member cases are the sharp ones. `vibe reinstall` bubbles to the
//! absolute workspace root and regenerates the whole tree, so its lock, its
//! boot artifacts, its lifecycle state and its trace all belong to that root.
//! But the ACTIVATION vote and the report's identity belong to the node the
//! operator actually invoked. Collapsing the two in either direction is a real
//! defect: a root `[compile] trace = true` would start paying for an observer a
//! member never asked for, and two different member invocations would produce
//! indistinguishable documents.

mod common;
mod trace_support;

use std::fs;
use std::path::Path;

use common::UserScratch;
use serde_json::Value;
use trace_support::{
    clap_rejected, declare_trace, normalise_project, quiet_stdout, reinstall_json,
    reinstall_output, run_directories, trace_dir, trace_lock_exists, trace_member, update_json,
    update_output,
};

// ---------------------------------------------------------------- 1. grammar

/// The flag parses on both commands, in every shape they accept — and clap
/// still rejects it where no compile happens.
#[test]
fn the_flag_parses_on_update_and_reinstall_and_nowhere_new() {
    let user = UserScratch::new();
    let accepted: [&[&str]; 4] = [
        &["update", "--trace-compile"],
        &["update", "--all", "--trace-compile"],
        &["reinstall", "--trace-compile"],
        &["reinstall", "--force", "--trace-compile"],
    ];
    for args in accepted {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        let output = user
            .vibe()
            .args(args)
            .args(["--offline", "--assume-yes"])
            .output()
            .unwrap();
        assert!(
            !clap_rejected(&output),
            "`vibe {}` must parse: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    // `vibe update <pkgref> --trace-compile` — the scoped grammar. It fails on
    // an empty project (nothing is installed), which is exactly the point: the
    // rejection must come from the COMMAND, not from clap.
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let scoped = user
        .vibe()
        .args(["update", "org.demo/absent", "--trace-compile"])
        .args(["--offline", "--assume-yes", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!clap_rejected(&scoped), "the scoped form parses too");

    // And nothing unrelated grew the flag.
    for verb in ["uninstall", "check", "outdated", "list"] {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        let output = user
            .vibe()
            .args([verb, "--trace-compile"])
            .arg(project.path())
            .output()
            .unwrap();
        assert!(
            clap_rejected(&output),
            "`vibe {verb}` compiles nothing and must not accept the flag: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

// ------------------------------------------------------------- 2. activation

/// The two halves of the truth table produce the SAME surface, on both
/// commands: the flag alone, and the selected manifest alone.
#[test]
fn the_flag_and_the_manifest_activate_identically() {
    let user = UserScratch::new();

    for declared in [false, true] {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        if declared {
            declare_trace(project.path());
        }
        let extra: &[&str] = if declared { &[] } else { &["--trace-compile"] };

        let updated = update_json(&user, project.path(), extra);
        let trace = trace_member(&updated).expect("update is traced either way");
        assert_eq!(trace["status"], "ok", "declared={declared}");
        assert_eq!(run_directories(project.path()).len(), 1);

        let reinstalled = reinstall_json(&user, project.path(), extra);
        let trace = trace_member(&reinstalled).expect("reinstall is traced either way");
        assert_eq!(trace["status"], "ok", "declared={declared}");
        assert_eq!(
            run_directories(project.path()).len(),
            2,
            "and the second command opened its own run: declared={declared}",
        );
    }
}

/// A DEPENDENCY that traces its own builds cannot switch either command on.
#[test]
fn a_dependency_manifest_activates_neither_command() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    trace_support::publish_tracing_package(registry.path());
    let installed = user
        .vibe()
        .args(["install", "org.trace/dep@=0.1.0", "--json", "--registry"])
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    assert!(!trace_dir(project.path()).exists());

    let updated = update_json(&user, project.path(), &[]);
    assert!(
        updated.get("trace").is_none(),
        "the installed dependency's own `[compile]` is not the host's vote",
    );
    let reinstalled = reinstall_json(&user, project.path(), &[]);
    assert!(reinstalled.get("trace").is_none());
    assert!(
        !trace_dir(project.path()).exists(),
        "and neither command opened a trace tree",
    );
}

// ------------------------------------------- 3. the member/root/storage matrix

/// A workspace root plus one member, each with its own `vibe.toml`.
fn workspace_with_member(user: &UserScratch) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    user.init_project(root.path());
    let manifest = root.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[workspace]\nmembers = [\"member\"]\n");
    fs::write(&manifest, text).unwrap();

    let member = root.path().join("member");
    fs::create_dir_all(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\n\
         version = \"0.1.0\"\n",
    )
    .unwrap();
    root
}

/// The `project` a report names, as a comparable path.
fn reported_project(report: &Value) -> String {
    report["project"].as_str().expect("a project").to_string()
}

/// Invoked through a member, BOTH commands trace — and everything durable is
/// still the workspace root's.
///
/// The three claims are independent and each has its own way of going wrong:
/// the member's manifest is the activation vote (read the wrong file and the
/// operator's `[compile] trace` is ignored); the report names the member (name
/// the workspace instead and two invocations become indistinguishable); the
/// trace tree lives at the workspace root (store it under the member and two
/// members hold independent locks over the same shared work).
#[test]
fn a_member_invocation_activates_from_the_member_and_stores_at_the_root() {
    let user = UserScratch::new();
    for forced in [false, true] {
        let root = workspace_with_member(&user);
        let member = root.path().join("member");
        declare_trace(&member);

        let extra: &[&str] = if forced { &["--force"] } else { &[] };
        let report = reinstall_json(&user, &member, extra);
        let trace = trace_member(&report)
            .unwrap_or_else(|| panic!("the MEMBER's manifest activated it: forced={forced}"));
        assert_eq!(trace["status"], "ok");
        assert_eq!(report["forced"], forced);

        assert_eq!(
            reported_project(&report),
            vibe_core::machine_json_path(&member),
            "the report names the invoked node: forced={forced}",
        );
        assert_ne!(
            reported_project(&report),
            vibe_core::machine_json_path(root.path()),
            "and NOT the workspace root it regenerated",
        );

        // Storage, lock and state are the workspace root's.
        assert_eq!(
            run_directories(root.path()).len(),
            1,
            "the run tree is at the workspace root: forced={forced}",
        );
        assert!(
            run_directories(&member).is_empty(),
            "and never under the member: forced={forced}",
        );
        assert!(
            trace_lock_exists(root.path()) && !member.join(".vibe/compile-trace.lock").exists(),
            "the cooperative lock belongs to the root",
        );
        // The boot artifacts prove the operational host: a member invocation
        // regenerates the WHOLE tree, root included.
        assert!(
            root.path().join("CLAUDE.md").is_file(),
            "the root's own boot artifacts were regenerated: forced={forced}",
        );
    }
}

/// The other direction: the ROOT declares tracing and the member does not. A
/// member invocation must stay untraced.
///
/// This is the one an "activate from `workspace.root_manifest`" implementation
/// gets wrong, and it gets it wrong silently — the run works, it just pays for
/// an observer nobody asked for and writes a diagnostic tree the operator
/// never requested.
#[test]
fn a_root_manifest_alone_cannot_activate_a_member_invocation() {
    let user = UserScratch::new();
    for forced in [false, true] {
        let root = workspace_with_member(&user);
        let member = root.path().join("member");
        declare_trace(root.path());

        let extra: &[&str] = if forced { &["--force"] } else { &[] };
        let report = reinstall_json(&user, &member, extra);
        assert!(
            report.get("trace").is_none(),
            "the member never asked: forced={forced}, report={report}",
        );
        assert!(
            !trace_dir(root.path()).exists(),
            "and nothing was written at the root either: forced={forced}",
        );
        assert!(!trace_dir(&member).exists());
    }
}

/// Invoked at the ROOT, the root's own manifest is the vote — the same read,
/// now naming the same node.
#[test]
fn a_root_invocation_activates_from_the_root_manifest() {
    let user = UserScratch::new();
    let root = workspace_with_member(&user);
    declare_trace(root.path());

    let report = reinstall_json(&user, root.path(), &[]);
    assert!(trace_member(&report).is_some());
    assert_eq!(
        reported_project(&report),
        vibe_core::machine_json_path(root.path()),
    );
    assert_eq!(run_directories(root.path()).len(), 1);
}

// -------------------------------------------------------- 4. disabled is inert

/// Trace off writes nothing at all — no directory, no lock — and each command's
/// old document is byte-identical apart from the member it does not carry.
#[test]
fn a_disabled_update_or_reinstall_writes_nothing_and_keeps_its_old_bytes() {
    let user = UserScratch::new();
    for command in ["update", "reinstall"] {
        let off_project = tempfile::tempdir().unwrap();
        user.init_project(off_project.path());
        let on_project = tempfile::tempdir().unwrap();
        user.init_project(on_project.path());

        let (off, mut on) = if command == "update" {
            (
                update_json(&user, off_project.path(), &[]),
                update_json(&user, on_project.path(), &["--trace-compile"]),
            )
        } else {
            (
                reinstall_json(&user, off_project.path(), &[]),
                reinstall_json(&user, on_project.path(), &["--trace-compile"]),
            )
        };

        assert!(
            !trace_dir(off_project.path()).exists(),
            "`vibe {command}` with tracing off writes no `.vibe/trace`",
        );
        assert!(
            !trace_lock_exists(off_project.path()),
            "and no cooperative lock either — disabled allocates nothing",
        );
        assert!(off.get("trace").is_none());
        assert!(
            !serde_json::to_string(&off).unwrap().contains("\"trace\""),
            "the key is absent from the wire, not merely null: {off}",
        );
        assert!(
            !serde_json::to_string(&off).unwrap().contains("notices"),
            "and this root has never had a notices member: {off}",
        );

        assert!(on.get("trace").is_some());
        let on_object = on.as_object_mut().unwrap();
        on_object.remove("trace");
        let mut off_normalised = off.clone();
        normalise_project(&mut off_normalised);
        let mut on_normalised = Value::Object(on_object.clone());
        normalise_project(&mut on_normalised);
        assert_eq!(
            off_normalised, on_normalised,
            "the trace member is the ONLY difference a disabled `vibe {command}` twin has",
        );
    }
}

/// Human and quiet surfaces with tracing OFF are exactly what they were.
///
/// The whole-graph update's silence is the load-bearing half: it has never
/// printed a completion summary on a terminal, and adding one would put a line
/// in front of every script that greps this command's stdout.
#[test]
fn disabled_human_and_quiet_surfaces_are_unchanged() {
    let user = UserScratch::new();

    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let quiet_update = quiet_stdout(
        &user
            .vibe()
            .args(["update", "--all", "--quiet", "--offline", "--assume-yes"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap(),
    );
    assert_eq!(
        quiet_update, "",
        "a whole update prints no completion line with tracing off: {quiet_update:?}",
    );

    let human_update = quiet_stdout(
        &user
            .vibe()
            .args(["update", "--all", "--offline", "--assume-yes"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap(),
    );
    assert!(
        !human_update.contains("Updated 0 packages"),
        "nor a human one: {human_update:?}",
    );

    let reinstalled = tempfile::tempdir().unwrap();
    user.init_project(reinstalled.path());
    let quiet_reinstall = quiet_stdout(&reinstall_quiet(&user, reinstalled.path(), &[]));
    assert_eq!(
        quiet_reinstall.lines().count(),
        1,
        "reinstall's quiet contract is one line: {quiet_reinstall:?}",
    );
    assert!(
        quiet_reinstall.starts_with("vibe reinstall: boot artifacts regenerated for"),
        "byte-for-byte the old line: {quiet_reinstall:?}",
    );
    assert!(
        !quiet_reinstall.contains("compile trace"),
        "with no suffix, because nothing was traced",
    );
}

/// Quiet + traced: still exactly one line per command, with the suffix on it.
#[test]
fn a_traced_quiet_run_gains_a_suffix_and_not_a_second_line() {
    let user = UserScratch::new();

    let updated = tempfile::tempdir().unwrap();
    user.init_project(updated.path());
    let update_line = quiet_stdout(
        &user
            .vibe()
            .args(["update", "--all", "--quiet", "--offline", "--assume-yes"])
            .arg("--trace-compile")
            .arg("--path")
            .arg(updated.path())
            .output()
            .unwrap(),
    );
    assert_eq!(
        update_line.lines().count(),
        1,
        "the traced whole update gets EXACTLY one line: {update_line:?}",
    );
    assert!(
        update_line.contains("compile trace ok"),
        "and it is the trace suffix that earned it: {update_line:?}",
    );

    let reinstalled = tempfile::tempdir().unwrap();
    user.init_project(reinstalled.path());
    let off = quiet_stdout(&reinstall_quiet(&user, reinstalled.path(), &[]));
    let on = quiet_stdout(&reinstall_quiet(
        &user,
        reinstalled.path(),
        &["--trace-compile"],
    ));
    assert_eq!(on.lines().count(), 1, "still one line: {on:?}");
    assert!(
        on.starts_with(off.trim_end()),
        "the suffix is APPENDED to the exact old line:\n off: {off:?}\n  on: {on:?}",
    );
    assert!(on.contains("compile trace ok"));
}

fn reinstall_quiet(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    user.vibe()
        .args(["reinstall", "--quiet", "--assume-yes"])
        .args(extra)
        .arg(project)
        .output()
        .unwrap()
}

/// Exactly ONE registered root per invocation, and never a standalone trace
/// document beside it.
#[test]
fn each_traced_command_emits_one_root_and_no_standalone_trace() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    for (label, output) in [
        (
            "update",
            update_output(&user, project.path(), &["--trace-compile"]),
        ),
        (
            "reinstall",
            reinstall_output(&user, project.path(), &["--trace-compile"]),
        ),
    ] {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let docs = trace_support::documents(&output.stdout);
        let roots: Vec<&Value> = docs.iter().filter(|doc| doc["command"] == label).collect();
        assert_eq!(roots.len(), 1, "one `{label}` root: {docs:#?}");
        assert!(
            docs.iter()
                .all(|doc| doc["command"] != "compile-trace" && doc.get("run_id").is_none()),
            "and no standalone trace object beside it: {docs:#?}",
        );
        assert!(
            trace_member(roots[0]).is_some(),
            "the member rides the ONE root",
        );
    }
}
