//! The outermost lifecycle lease, end to end (R7.4 §2.1 / §10 item 4).
//!
//! `.vibe/lifecycle.lock` is the workspace-global single-writer record's
//! guard. Every mutating CLI surface acquires it BEFORE a run id, state row,
//! outbox byte or destructive clean, and a second holder receives a typed,
//! PROP-054-cited refusal that changes nothing but the infrastructure lock
//! file's own name.
//!
//! The holder here is the TEST process, through the real
//! `vibe_lifecycle::LifecycleLease::acquire`; the contenders are real `vibe`
//! subprocesses. What each case asserts is the refusal's TOTALITY: prompt,
//! typed, and byte-identical tree.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use common::UserScratch;
use vibe_lifecycle::LifecycleLease;

/// A 32-hex identity the bait state may legitimately carry.
const RUN_ID: &str = "00112233445566778899aabbccddeeff";

/// The canonical absolute root the subprocess's own `resolve_project_root`
/// would produce — one spelling, so the test's lease and the child's acquire
/// contend for the SAME lock file.
fn canonical(project: &Path) -> PathBuf {
    let resolved = project.canonicalize().unwrap();
    let text = resolved.to_string_lossy().into_owned();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

fn hold(project: &Path) -> LifecycleLease {
    LifecycleLease::acquire(&canonical(project)).expect("the fixture root is leasable")
}

/// One init'd project — every verb's locator only needs its `vibe.toml`; the
/// Busy refusal fires before anything else is read.
fn project(user: &UserScratch) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    user.init_project(dir.path());
    dir
}

/// Run one `vibe` verb as a subprocess, returning its output and how long it
/// took. Every case here expects the verb to have REFUSED, so failure output
/// is returned rather than asserted at this layer.
///
/// `reinstall` spells its target positionally; every other verb takes
/// `--path`.
fn run(
    user: &UserScratch,
    project: &Path,
    args: &[&str],
) -> (std::process::Output, std::time::Duration) {
    let mut command = user.vibe();
    let positional = args.first().is_some_and(|first| *first == "reinstall");
    for arg in args {
        command.arg(arg);
    }
    if positional {
        command.arg(project);
    } else {
        command.arg("--path").arg(project);
    }
    command.arg("--assume-yes");
    let started = Instant::now();
    let output = command.output().unwrap();
    (output, started.elapsed())
}

/// The typed refusal a contended verb must produce: non-zero exit, the lock
/// named, the spec cited — and nothing else claimed.
fn assert_busy(output: &std::process::Output, elapsed: std::time::Duration, verb: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("lifecycle.lock"),
        "{verb}: the refusal names the lock: {stderr}"
    );
    assert!(
        stderr.contains("PROP-054"),
        "{verb}: the refusal cites the state-home law: {stderr}"
    );
    assert!(
        stderr.contains("before any run id"),
        "{verb}: the refusal states its totality: {stderr}"
    );
    assert!(
        !output.status.success(),
        "{verb}: a Busy refusal is a non-zero exit"
    );
    assert!(
        elapsed.as_secs() < 30,
        "{verb}: a nonblocking refusal is prompt, took {elapsed:?}"
    );
}

/// A stable digest of everything under the project EXCEPT the lease's own
/// lock file — the "tree byte-identical except the permitted infrastructure"
/// oracle. Paths are sorted, so the digest does not depend on readdir order.
fn tree_digest(root: &Path) -> u64 {
    fn walk(dir: &Path, prefix: &str, into: &mut BTreeMap<String, u64>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let path = entry.path();
            if path.is_dir() {
                walk(&path, &rel, into);
            } else {
                // The one permitted artifact: the lock `try_lock` itself
                // creates. Everything else — state, outbox, scratch run
                // directories — is a mutation a Busy verb must not make.
                if rel == ".vibe/lifecycle.lock" {
                    continue;
                }
                let bytes = fs::read(&path).unwrap_or_default();
                let mut hash = fxhash_rel(&rel);
                hash = hash.wrapping_mul(31).wrapping_add(bytes.len() as u64);
                hash = hash.wrapping_mul(31).wrapping_add(fxhash_bytes(&bytes));
                into.insert(rel, hash);
            }
        }
    }
    fn fxhash_rel(text: &str) -> u64 {
        text.bytes().fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(0x100000001b3)
        })
    }
    fn fxhash_bytes(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0x9e3779b97f4a7c15, |hash, byte| {
            (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
        })
    }
    let mut files = BTreeMap::new();
    walk(root, "", &mut files);
    files
        .into_iter()
        .fold(0x517ce66b527dc1cd, |digest, (_, hash)| {
            (digest ^ hash).wrapping_mul(0x100000001b3)
        })
}

/// Every mutating surface, one contention case each: the verb runs in a
/// subprocess while THIS process holds the real lease, and must refuse typed
/// and total. The clean cases are load-bearing in a second way: their
/// refusal precedes any destructive wipe, which a lock taken late would not.
#[test]
fn every_mutating_verb_refuses_typed_and_total_under_contention() {
    let user = UserScratch::new();
    for (verb, args) in [
        ("validate", vec!["validate"]),
        ("create", vec!["create"]),
        ("direct install", vec!["install"]),
        ("update", vec!["update"]),
        ("reinstall", vec!["reinstall"]),
        ("clean-only", vec!["clean"]),
        ("clean-chain", vec!["clean", "validate"]),
    ] {
        let project = project(&user);
        // A tree that already carries real state and outbox bytes: the
        // refusal must leave them exactly as they are, not merely avoid
        // creating new ones.
        plant_bait_state(&project);
        let before = tree_digest(project.path());
        let lease = hold(project.path());
        let (output, elapsed) = run(&user, project.path(), &args);
        assert_busy(&output, elapsed, verb);
        drop(lease);
        let after = tree_digest(project.path());
        assert_eq!(
            before, after,
            "{verb}: a Busy refusal leaves the tree byte-identical except the lock file",
        );
        // The outbox the bait planted is covered by the digest above; what
        // is separately named is the one allocation a pre-refusal state read
        // would have made — a scratch run directory.
        assert!(
            fs::read_dir(project.path().join(".vibe/lifecycle"))
                .map(|entries| entries.count() == 0)
                .unwrap_or(true),
            "{verb}: no scratch run directory was allocated",
        );
    }
}

/// A successful clean keeps the live lock's NAME: `.vibe` survives the wipe,
/// so the next command acquires the very lock this one held — and the clean
/// removed only its already-specified derived targets, never the project's
/// own files.
#[test]
fn a_successful_clean_keeps_the_lock_named_and_the_next_command_acquires_it() {
    let user = UserScratch::new();
    let project = project(&user);
    // Give the clean something real to remove: an empty-world install
    // regenerates the boot lanes.
    let (installed, _) = run(&user, project.path(), &["install"]);
    assert!(
        installed.status.success(),
        "the empty-world install precedes the clean: {}",
        String::from_utf8_lossy(&installed.stderr),
    );
    let (output, _) = run(&user, project.path(), &["clean"]);
    assert!(
        output.status.success(),
        "clean succeeds uncontended: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let lock = project.path().join(".vibe/lifecycle.lock");
    assert!(lock.is_file(), "the live lock's name survives the wipe");
    assert!(
        project.path().join("vibe.toml").is_file(),
        "the manifest survives"
    );
    // The next command acquires it — and this time the test process holds
    // nothing, so the acquire inside the child really happens.
    let (output, _) = run(&user, project.path(), &["validate"]);
    assert!(
        output.status.success(),
        "the next command acquires the surviving lock: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

/// The post-acquisition snapshot law, at the seam the lease makes testable:
/// while THIS process holds the lease, a hosted resume over planted
/// adoption-bait state refuses without consuming a byte of it — and once the
/// lease is released, the SAME invocation adopts the exact planted identity,
/// proving the state read happens on the leased side of the refusal, never
/// on a pre-lease snapshot a concurrent writer could have replaced.
#[test]
fn a_busy_resume_never_consumes_bait_state_and_the_released_rerun_adopts_it() {
    let user = UserScratch::new();
    let project = project(&user);
    plant_bait_state(&project);
    let bait_bytes = fs::read(project.path().join(".vibe/lifecycle.toml")).unwrap();

    let lease = hold(project.path());
    let (output, elapsed) = run(
        &user,
        project.path(),
        &["--agent-mode", "agent", "validate"],
    );
    assert_busy(&output, elapsed, "the contended resume");
    drop(lease);
    assert_eq!(
        fs::read(project.path().join(".vibe/lifecycle.toml")).unwrap(),
        bait_bytes,
        "the refusal consumed nothing the bait had written",
    );

    // Released: the same invocation now reads the state it finds on disk —
    // under the lease it has just taken — and ADOPTS the planted identity
    // rather than minting a fresh one beside it.
    let (output, _) = run(
        &user,
        project.path(),
        &["--agent-mode", "agent", "validate"],
    );
    assert!(
        output.status.success(),
        "the uncontended resume completes: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let state = fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap();
    assert!(
        state.contains(&format!("run_id = \"{RUN_ID}\"")),
        "the planted identity was adopted, not displaced:\n{state}"
    );
}

/// A traced command takes the lease FIRST and the compile-trace lock SECOND —
/// the allowed order — and completes with both released. This is the CLI-seam
/// half of the order law the lease unit tests pin from below: the forward
/// order is viable end to end, and nothing in it blocks or deadlocks.
#[test]
fn a_traced_command_takes_the_lease_before_the_trace_lock_and_completes() {
    let user = UserScratch::new();
    let project = project(&user);
    let (output, _) = run(&user, project.path(), &["validate", "--trace-compile"]);
    assert!(
        output.status.success(),
        "lifecycle then compile-trace, in order, works: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        project.path().join(".vibe/lifecycle.lock").is_file(),
        "the lease was really taken"
    );
    // And both are released: this process can take the lease now.
    drop(hold(project.path()));
}

/// A valid state whose run header matches `vibe validate` in agent mode and
/// carries one phase-scoped parked row — adoption bait for the resume case.
fn plant_bait_state(project: &tempfile::TempDir) {
    let key = "__host__/demo#produce";
    let task = vibe_lifecycle::outbox_task_path(RUN_ID, key).unwrap();
    let body = format!(
        "schema = 1\n\
         [run]\nrequested = 'validate'\nchain = ['validate']\n\
         started = '2026-08-28T00:00:00Z'\nrun_id = '{RUN_ID}'\nselected = '.'\n\
         [execution.'{key}']\n\
         phase = 'validate'\nfingerprint = 'sha256:x'\nstatus = 'delegated'\n\
         duration_ms = 4\nscope = 'phase'\n\
         artifacts = [{{ id = 'a', kind = 'text', path = 'out/produced.md' }}]\n\
         tasks = ['{task}']\n"
    );
    let root = project.path();
    // `task` is already the whole project-relative outbox path.
    fs::create_dir_all(root.join(&task).parent().unwrap()).unwrap();
    fs::write(root.join(".vibe/lifecycle.toml"), body).unwrap();
    fs::write(
        root.join(&task),
        "# parked task\n\nProduce the declared output.\n",
    )
    .unwrap();
}

/// A two-node workspace: an init'd root with one `[package]` member. The
/// member is what the commands are invoked on; the WORKSPACE root is what
/// they must lease — state is workspace-global, so a member invocation that
/// locked the member directory would leave two members free to write the
/// same `.vibe/lifecycle.toml` through two different locks.
fn workspace_with_member(user: &UserScratch) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    user.init_project(dir.path());
    let manifest_path = dir.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str("\n[workspace]\nmembers = [\"app\"]\n");
    fs::write(&manifest_path, manifest).unwrap();
    let member = dir.path().join("app");
    fs::create_dir_all(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"app\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    (dir, member)
}

/// A member invocation leases the WORKSPACE root, not the member: the lock
/// file lands at the root both members share, and contending the ROOT — not
/// the member — is what refuses the member's command.
#[test]
fn a_member_invocation_leases_the_workspace_root() {
    let user = UserScratch::new();
    let (workspace, member) = workspace_with_member(&user);

    // Uncontended: the member's command succeeds and leaves the lock named
    // at the workspace root — never inside the member, where a second member
    // would not contend it.
    let (output, _) = run(&user, &member, &["validate"]);
    assert!(
        output.status.success(),
        "the member's validate succeeds uncontended: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        workspace.path().join(".vibe/lifecycle.lock").is_file(),
        "the lease is named at the WORKSPACE root"
    );
    assert!(
        !member.join(".vibe/lifecycle.lock").exists(),
        "and never inside the member"
    );

    // Contended: holding the WORKSPACE root's lease refuses a command
    // invoked from the member — one workspace, one writer, regardless of
    // which node the operator typed the command at.
    let lease = hold(workspace.path());
    let (output, elapsed) = run(&user, &member, &["validate"]);
    assert_busy(&output, elapsed, "the member's contended validate");
    drop(lease);
}
