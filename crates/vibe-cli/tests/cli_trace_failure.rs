//! R3.4 — what a FAILED traced command reports, and what it never leaks.
//!
//! Two properties dominate this file, and both concern a command that went
//! wrong:
//!
//! * the registered ROOT family a failure reports is a property of the site
//!   that measured it, not of the command name — `vibe install` reports a slot
//!   failure as an install root, and a phase verb's SUPPRESSED prerequisite
//!   reports at most one root in total;
//! * the trace records the FIXED words `command failed` and nothing else. A
//!   real command error can carry a script's captured stderr, so these tests
//!   plant a secret sentinel in exactly that channel and then search every
//!   byte the trace wrote.

mod common;
mod trace_support;

use common::UserScratch;
use trace_support::{documents, index_of, run_directories, trace_dir, trace_member};

/// A registry whose one package fails at the named hook point, printing
/// `sentinel` on the way out.
fn failing_package(registry: &std::path::Path, point: &str, sentinel: &str) {
    let package = registry.join("org.fail").join("hooked").join("v0.1.0");
    std::fs::create_dir_all(package.join("hooks")).unwrap();
    std::fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.fail'\nname='hooked'\nkind='tool'\nversion='0.1.0'\n\n\
             [hooks]\n{point}='hooks/fail'\n"
        ),
    )
    .unwrap();
    std::fs::write(
        package.join("hooks/fail.sh"),
        format!("echo {sentinel} 1>&2\nexit 3\n"),
    )
    .unwrap();
    std::fs::write(
        package.join("hooks/fail.ps1"),
        format!("[Console]::Error.Write('{sentinel}')\nexit 3\n"),
    )
    .unwrap();
}

fn install_failing(
    user: &UserScratch,
    project: &std::path::Path,
    registry: &std::path::Path,
    flags: &[&str],
) -> std::process::Output {
    user.vibe()
        .args(["install", "org.fail/hooked@=0.1.0"])
        .args(flags)
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

/// A direct `vibe install --json` whose slot row fails emits ONE install root
/// with `ok: false` — the family this site has always reported — and the trace
/// member rides that root rather than a document of its own.
#[test]
fn a_direct_slot_failure_reports_one_install_root_carrying_the_trace() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    failing_package(registry.path(), "pre-install", "SENTINEL-PRE");

    let output = install_failing(
        &user,
        project.path(),
        registry.path(),
        &["--trace-compile", "--json"],
    );
    assert!(!output.status.success(), "a failing pre-install hook fails");

    let docs = documents(&output.stdout);
    let roots: Vec<&serde_json::Value> = docs
        .iter()
        .filter(|doc| doc["command"] == "install")
        .collect();
    assert_eq!(roots.len(), 1, "exactly one registered root: {docs:#?}");
    assert_eq!(roots[0]["ok"], false);
    assert!(
        docs.iter().all(|doc| doc["command"] != "lifecycle"),
        "and no lifecycle root beside it: {docs:#?}",
    );
    assert!(
        roots[0].get("trace").is_some(),
        "the failed root carries the member — the trace finalised before it",
    );
}

/// The same failure under a phase verb, both ways round — the exact matrix.
///
/// A prerequisite install runs under `ctx.quiet_child()`, so trace-off emits
/// NO registered command root at all: only the plan previews survive. That is
/// characterised behaviour, and it is why the emission policy is a property of
/// the site rather than of the command.
///
/// With tracing requested, the generic `old-policy OR trace-requested` rule
/// makes that same Install-shaped draft observable exactly once, through the
/// outer root context — still with no Lifecycle root beside it, and still with
/// the same exit code and the same terminal error.
#[test]
fn the_suppressed_prerequisite_slot_failure_matrix_holds_both_ways() {
    let user = UserScratch::new();
    let registry = tempfile::tempdir().unwrap();
    failing_package(registry.path(), "pre-install", "SENTINEL-CHILD");

    let build = |flags: &[&str]| {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        trace_support::declare_static_dependency(project.path(), "tool:org.fail/hooked", "=0.1.0");
        let output = user
            .vibe()
            .arg("build")
            .args(["--json", "--assume-yes"])
            .args(flags)
            .arg("--registry")
            .arg(registry.path())
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "the prerequisite install failed");
        (output, project)
    };

    // ---- trace OFF: the old suppressed behaviour, unchanged --------------
    let (off, _off_project) = build(&[]);
    let off_docs = documents(&off.stdout);
    assert!(
        off_docs
            .iter()
            .all(|doc| doc["command"] != "install" && doc["command"] != "lifecycle"),
        "a suppressed prerequisite emits NO registered command root: {off_docs:#?}",
    );

    // ---- trace ON: the same draft becomes observable, once ---------------
    let (on, on_project) = build(&["--trace-compile"]);
    let on_docs = documents(&on.stdout);
    let install_roots: Vec<&serde_json::Value> = on_docs
        .iter()
        .filter(|doc| doc["command"] == "install")
        .collect();
    assert_eq!(
        install_roots.len(),
        1,
        "the requested trace makes it observable exactly once: {on_docs:#?}",
    );
    assert_eq!(install_roots[0]["ok"], false);
    assert!(
        on_docs.iter().all(|doc| doc["command"] != "lifecycle"),
        "and never as a Lifecycle root: {on_docs:#?}",
    );

    let trace = trace_member(install_roots[0]).expect("the requested trace reports itself");
    assert_eq!(trace["status"], "failed", "the run finalised failed");
    let runs = run_directories(on_project.path());
    assert_eq!(runs.len(), 1);
    assert!(
        index_of(on_project.path(), &runs[0]).scopes.is_empty(),
        "a pre-install failure compiles nothing, so the trace has zero scopes",
    );

    // ---- and the failure identity is the same either way -----------------
    assert_eq!(
        off.status.code(),
        on.status.code(),
        "the exit code does not move because an observer was asked for",
    );
    let off_tail = String::from_utf8_lossy(&off.stderr).into_owned();
    let on_tail = String::from_utf8_lossy(&on.stderr).into_owned();
    assert_eq!(
        off_tail, on_tail,
        "nor does the terminal error:\n off: {off_tail}\n on:  {on_tail}",
    );
    assert!(
        !on_tail.contains("FailedDraft") && !on_tail.contains("Carried"),
        "and the transport carrier never reaches the operator: {on_tail}",
    );
}

/// A DIRECT install whose post-durability lifecycle handler fails reports a
/// LIFECYCLE root and explicitly no install root.
///
/// The root family belongs to the site that measured the failure, not to the
/// command that was typed. `vibe install` runs its `phase:install` ritual after
/// the world is durable, and a handler that fails there is a lifecycle failure
/// inside an install command.
#[test]
fn a_direct_tracked_handler_failure_reports_a_lifecycle_root() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    std::fs::create_dir_all(project.path().join("scripts")).unwrap();
    std::fs::write(
        project.path().join("scripts/fail.sh"),
        "printf HANDLER-OUT\nprintf HANDLER-ERR >&2\nexit 29\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join("scripts/fail.ps1"),
        "Write-Output HANDLER-OUT\n[Console]::Error.Write('HANDLER-ERR')\nexit 29\n",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    text.push_str(
        "\n[[extension]]\nid='post-durable'\npoint='phase:install'\nhandler={ kind = \"script\", base = \"scripts/fail\" }\n",
    );
    std::fs::write(&manifest, text).unwrap();

    let output = user
        .vibe()
        .args([
            "install",
            "--trace-compile",
            "--json",
            "--offline",
            "--assume-yes",
        ])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "the handler exits 29");

    let docs = documents(&output.stdout);
    let lifecycle: Vec<&serde_json::Value> = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .collect();
    assert_eq!(lifecycle.len(), 1, "exactly one Lifecycle root: {docs:#?}");
    assert_eq!(lifecycle[0]["ok"], false);
    assert!(
        docs.iter().all(|doc| doc["command"] != "install"),
        "and explicitly NO install root: {docs:#?}",
    );
    let trace = trace_member(lifecycle[0]).expect("the requested trace rides that root");
    assert_eq!(trace["status"], "failed");
    assert_eq!(
        index_of(project.path(), trace["run_id"].as_str().unwrap())
            .failure
            .as_deref(),
        Some("command failed"),
    );
    assert!(
        !trace_support::all_trace_bytes(project.path()).contains("HANDLER-ERR"),
        "the handler's captured stderr never reaches the trace",
    );
}

/// A PRE-COMPILE failure's trace says only the fixed sentence.
///
/// The hook fails at `pre-install`, before anything compiles, so this is the
/// zero-scope half of the secrecy proof: the index is terminal `failed`, its
/// only diagnostic is the fixed word, and the secret the command's error
/// carries in captured stderr is nowhere in the tree. The POST-compile half —
/// a run that really recorded events first — is
/// `a_direct_tracked_handler_failure_reports_a_lifecycle_root`, which plants
/// `HANDLER-ERR` after the world is durable.
#[test]
fn a_failed_run_records_only_the_fixed_words_and_never_the_command_error() {
    const SECRET: &str = "SUPERSECRETTOKEN9";
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    failing_package(registry.path(), "pre-install", SECRET);

    let output = install_failing(
        &user,
        project.path(),
        registry.path(),
        &["--trace-compile", "--json"],
    );
    assert!(!output.status.success());

    let runs = run_directories(project.path());
    assert_eq!(runs.len(), 1, "one command, one run: {runs:?}");
    let index = index_of(project.path(), &runs[0]);
    assert_eq!(
        index.failure.as_deref(),
        Some("command failed"),
        "the index records the FIXED diagnostic and nothing else",
    );
    assert!(
        !trace_support::all_trace_bytes(project.path()).contains(SECRET),
        "no byte the trace wrote may quote the command's error",
    );
    let docs = documents(&output.stdout);
    let root = docs
        .iter()
        .find(|doc| doc["command"] == "install")
        .expect("the failed root");
    let trace = trace_member(root).expect("a requested trace reports itself");
    assert!(
        !serde_json::to_string(trace).unwrap().contains(SECRET),
        "nor may the member: {trace}",
    );
}

/// A failed QUIET traced command is exactly one line, and that line is the
/// error with the trace suffix appended. Its trace-off twin is the exact old
/// line, and both exit the same way.
#[test]
fn a_failed_quiet_traced_command_is_one_line_with_a_suffix() {
    let user = UserScratch::new();
    let registry = tempfile::tempdir().unwrap();
    failing_package(registry.path(), "pre-install", "SENTINEL-QUIET");

    let run = |flags: &[&str]| {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        let mut all = vec!["--quiet"];
        all.extend_from_slice(flags);
        let output = install_failing(&user, project.path(), registry.path(), &all);
        assert!(!output.status.success());
        (
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.code(),
        )
    };

    let (off, off_code) = run(&[]);
    let (on, on_code) = run(&["--trace-compile"]);

    assert_eq!(
        off.lines().count(),
        1,
        "trace off is exactly one stderr line: {off:?}",
    );
    assert_eq!(
        on.lines().count(),
        1,
        "and trace on is STILL exactly one: {on:?}",
    );
    assert!(
        on.contains("compile trace"),
        "the suffix rides that one line: {on:?}",
    );
    assert!(
        on.starts_with(off.trim_end()),
        "and it is APPENDED to the exact old line:\n off: {off:?}\n on:  {on:?}",
    );
    assert_eq!(off_code, on_code, "the exit code is unchanged");
}

/// A chained clean whose post-wipe rediscovery fails must return BEFORE any
/// trace opens: the wipe already rewrote the tree, and a session opened over
/// the wreckage would leave a lock and a `running` index nobody can close.
#[test]
fn a_post_clean_rediscovery_failure_opens_no_trace() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    // A member the root declares and that does not exist: the tree will not
    // load, and the point is that the refusal costs no trace artifact.
    let manifest = project.path().join("vibe.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        format!("{text}\n[workspace]\nmembers = [\"absent-member\"]\n"),
    )
    .unwrap();

    let output = user
        .vibe()
        .args(["clean", "build", "--trace-compile", "--offline"])
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success(), "the declared member is missing");
    assert!(
        !trace_dir(project.path()).exists(),
        "and no trace tree was created for a run that could not name its root",
    );
    assert!(
        !trace_support::trace_lock_exists(project.path()),
        "nor a cooperative lock",
    );
}

/// Whole `vibe update` delegates to the install substrate but owns no trace
/// funnel yet, so a measured failure arrives CARRIED. It must be unwrapped at
/// that boundary.
///
/// Two things break if the carrier escapes to `main`: the historical Install
/// root this path has always emitted is suppressed (the carrier is not a
/// draft `main` knows how to render), and the exit code is computed from a
/// wrapper instead of the command's own typed error.
#[test]
fn whole_update_unwraps_the_carrier_and_keeps_its_historical_root() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    failing_package(registry.path(), "pre-install", "SENTINEL-UPDATE");
    trace_support::declare_static_dependency(project.path(), "tool:org.fail/hooked", "=0.1.0");

    // The same failure, once through `install` and once through whole
    // `update`: the second is the compatibility path under test, and the
    // first is the oracle it must match.
    let via_install = install_failing(&user, project.path(), registry.path(), &["--json"]);
    assert!(!via_install.status.success());

    // `vibe update` has no `--registry` flag: it re-resolves from what the
    // project declares, so the registry is wired into the manifest.
    let fresh = tempfile::tempdir().unwrap();
    user.init_project(fresh.path());
    trace_support::declare_static_dependency(fresh.path(), "tool:org.fail/hooked", "=0.1.0");
    let manifest = fresh.path().join("vibe.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    text.push_str(&format!(
        "
[[registry]]
name = \"fixture\"
url = \"file:///{}\"
",
        registry
            .path()
            .display()
            .to_string()
            .replace(std::path::MAIN_SEPARATOR, "/"),
    ));
    std::fs::write(&manifest, text).unwrap();
    let via_update = user
        .vibe()
        .args(["update", "--all", "--json", "--assume-yes"])
        .arg("--path")
        .arg(fresh.path())
        .output()
        .unwrap();
    assert!(!via_update.status.success(), "the slot row still fails");

    let docs = documents(&via_update.stdout);
    let install_roots = docs
        .iter()
        .filter(|doc| doc["command"] == "install")
        .count();
    assert_eq!(
        install_roots, 1,
        "the historical Install root is emitted exactly once: {docs:#?}",
    );

    let tail = String::from_utf8_lossy(&via_update.stderr).into_owned();
    assert!(
        !tail.contains("FailedDraft") && !tail.contains("Carried"),
        "the carrier never reaches stderr: {tail}",
    );
    assert_eq!(
        via_install.status.code(),
        via_update.status.code(),
        "and the exit identity is the substrate's, not a wrapper's",
    );
}
