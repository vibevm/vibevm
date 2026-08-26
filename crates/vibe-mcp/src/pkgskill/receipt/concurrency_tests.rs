//! Real child-process proofs of cross-process serialization: OS crash
//! release of the package-skill lock and convergence of two contending
//! child reconciliations from one baseline. Children are the unit-test
//! executable itself (the established R3 idiom): the parent spawns it with a
//! test-name filter and a mode env, and the test's helper branch runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::nofollow::{LockGuard, Project};
use super::state::read_receipt;
use super::tests::{provider, seed};
use crate::pkgskill::{lower_project_skill_bindings, reconcile_project_skill_binding};

const LOCK_MODE: &str = "R8_CONCURRENCY_LOCK_HELPER";
const RECONCILE_MODE: &str = "R8_CONCURRENCY_RECONCILE_HELPER";
const POLL: Duration = Duration::from_millis(25);
const BUDGET: Duration = Duration::from_secs(10);
const LOCK_TEST: &str = "lock_process_death_releases_the_exact_file_lock";
const RECONCILE_TEST: &str = "two_child_reconciles_converge_from_one_baseline";

fn spawn_helper(test: &str, mode: (&str, String)) -> Child {
    let exe = std::env::current_exe().unwrap();
    Command::new(exe)
        .arg(test)
        .env(mode.0, mode.1)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the concurrency helper child")
}

/// A child is this same test binary under libtest, which CAPTURES its panic
/// output — so a contending child's reason never reaches the parent's stderr.
/// The helper therefore records its own failure beside the project, and the
/// parent quotes it. Without this a real refusal and a timing flake are
/// indistinguishable from `exit(101)` alone.
fn child_failures(root: &Path) -> String {
    let mut found = Vec::new();
    for skill in ["alpha", "beta"] {
        if let Ok(text) = fs::read_to_string(root.join(format!("child-{skill}.err"))) {
            found.push(format!("{skill}: {text}"));
        }
    }
    found.join(" | ")
}

fn wait_for_file(path: &Path) -> bool {
    let deadline = Instant::now() + BUDGET;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(POLL);
    }
    false
}

fn wait_for_lock(project: &Project) -> Option<LockGuard> {
    let deadline = Instant::now() + BUDGET;
    while Instant::now() < deadline {
        if let Some(guard) = project.try_lock(super::nofollow::LOCK_FILE).unwrap() {
            return Some(guard);
        }
        std::thread::sleep(POLL);
    }
    None
}

#[test]
fn lock_process_death_releases_the_exact_file_lock() {
    if let Ok(project_root) = std::env::var(LOCK_MODE) {
        lock_helper(&project_root);
        return;
    }
    let project = tempfile::tempdir().unwrap();
    let ready = project.path().join("ready.marker");
    let mut child = spawn_helper(
        LOCK_TEST,
        (LOCK_MODE, project.path().to_string_lossy().into_owned()),
    );
    // The parent proves its ready instrument actually fired.
    assert!(wait_for_file(&ready), "helper ready marker never appeared");
    let capability = Project::open(project.path()).unwrap();
    assert!(
        capability
            .try_lock(super::nofollow::LOCK_FILE)
            .unwrap()
            .is_none(),
        "the live child must hold the lock"
    );
    // Kill and reap exactly this child; only its death may release the lock.
    child.kill().expect("killing the exact helper child");
    let status = child.wait().expect("reaping the exact helper child");
    assert!(!status.success(), "the killed helper must not exit cleanly");
    // The marker still exists — the release came from process death, not
    // from marker deletion or the helper finishing on its own.
    assert!(ready.exists());
    let guard = wait_for_lock(&capability).expect("OS crash release within budget");
    drop(guard);
}

fn lock_helper(project_root: &str) {
    let project = Project::open(Path::new(project_root)).unwrap();
    let _guard = project.lock(super::nofollow::LOCK_FILE).unwrap();
    fs::write(Path::new(project_root).join("ready.marker"), "held").unwrap();
    // Hold the lock until the parent kills this exact process; bounded so an
    // orphaned helper cannot outlive the whole test run.
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
}

#[test]
fn two_child_reconciles_converge_from_one_baseline() {
    if let Ok(spec) = std::env::var(RECONCILE_MODE) {
        reconcile_helper(&spec);
        return;
    }
    let project = tempfile::tempdir().unwrap();
    // Two disjoint provider sources and skill names under one project.
    let one = seed(project.path(), "one");
    let two = seed(project.path(), "two");
    let go = project.path().join("go.marker");
    let mut first = spawn_helper(
        RECONCILE_TEST,
        (
            RECONCILE_MODE,
            format!(
                "{}|{}|alpha",
                project.path().display(),
                one.path().display()
            ),
        ),
    );
    let mut second = spawn_helper(
        RECONCILE_TEST,
        (
            RECONCILE_MODE,
            format!("{}|{}|beta", project.path().display(), two.path().display()),
        ),
    );
    // Prove both helpers reached the barrier before releasing them through
    // one marker, so the test creates real lock contention rather than merely
    // two eventually successful sequential processes.
    assert!(wait_for_file(&project.path().join("ready-alpha.marker")));
    assert!(wait_for_file(&project.path().join("ready-beta.marker")));
    fs::write(&go, "go").unwrap();
    for child in [&mut first, &mut second] {
        let status = child.wait().expect("reaping a reconcile child");
        // Name the exit code: `2` is a malformed spec, `3` is the barrier
        // budget expiring, anything else is a real reconcile failure. A bare
        // "must succeed" makes a timing flake and a correctness break look
        // identical from the failure text alone.
        assert!(
            status.success(),
            "each contending reconcile child must succeed \
             (exit {:?}; 3 = barrier budget expired): {}",
            status.code(),
            child_failures(project.path()),
        );
    }
    // The parent proves its go instrument fired and both sides converged.
    assert!(go.exists());
    let project_cap = Project::open(project.path()).unwrap();
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    assert!(receipt.applying.is_none(), "no pending intent remains");
    let keys = receipt
        .binding
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        receipt.binding.len(),
        2,
        "both bindings committed: {keys:?}"
    );
    assert!(
        keys.iter()
            .any(|key| key == "@vibe/package/skill/org.example/one/alpha"),
        "{keys:?}"
    );
    assert!(
        keys.iter()
            .any(|key| key == "@vibe/package/skill/org.example/two/beta"),
        "{keys:?}"
    );
    assert_eq!(
        fs::read_to_string(project.path().join(".claude/skills/alpha/SKILL.md")).unwrap(),
        "body-one"
    );
    assert_eq!(
        fs::read_to_string(project.path().join(".claude/skills/beta/SKILL.md")).unwrap(),
        "body-two"
    );
    let mut skills = fs::read_dir(project.path().join(".claude/skills"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    skills.sort();
    assert_eq!(skills, vec!["alpha", "beta"], "no orphan target directory");
}

fn reconcile_helper(spec: &str) {
    let mut parts = spec.split('|');
    let (Some(project_root), Some(provider_root), Some(skill)) =
        (parts.next(), parts.next(), parts.next())
    else {
        std::process::exit(2);
    };
    let root = Path::new(project_root);
    fs::write(root.join(format!("ready-{skill}.marker")), "ready").unwrap();
    // Wait for the shared go marker so both children start together and
    // contend on the project's package-skill lock.
    let go = root.join("go.marker");
    let deadline = Instant::now() + BUDGET;
    while !go.exists() && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
    if !go.exists() {
        std::process::exit(3);
    }
    let package = PathBuf::from(provider_root);
    let name = if skill == "alpha" { "one" } else { "two" };
    let input = provider(&package, name, skill, &["claude"]);
    let outcome = lower_project_skill_bindings(root, vec![input])
        .and_then(|bindings| reconcile_project_skill_binding(root, &bindings[0]));
    if let Err(error) = outcome {
        let _ = fs::write(
            root.join(format!("child-{skill}.err")),
            format!("{error:#}"),
        );
        std::process::exit(4);
    }
}
