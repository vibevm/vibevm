//! R3.4 — RED 8: a compile trace across a HOSTED `vibe update` park and the
//! flagless resume that finishes it, SCOPED and WHOLE.
//!
//! The install family already proved a traced park suspends its run and a
//! sticky-bit resume reopens it (`cli_trace_hosted.rs`). What only update can
//! prove is that its OWN command surface keeps that contract: the parked root
//! is an `update` root carrying the member, the resume re-enters through
//! `vibe update` without the one-shot flag, and the run that finalises is the
//! one the park suspended — same id, same original start, appended events and
//! scopes, one dense global sequence, and exactly one run directory on disk.
//!
//! The fixture is the boot-bearing variant on purpose: a package with no
//! `[boot_snippet]` compiles nothing, so "the resume appended" would be a
//! number that cannot move and the strict-growth assertions would be vacuous.

mod common;
mod trace_support;

use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, add_version, assert_ok, declare_static_tools, documents, lifecycle_state,
    project_at, publish_slot_agent_with_boot, write_declared_output,
};
use common::{UserScratch, git_available};
use serde_json::Value;
use trace_support::{index_of, run_directories, trace_member};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;
use vibe_wire::generated::shared::Timestamp;

/// The untraced seed every case starts from: a DECLARED, CLI-mode install, so
/// the project holds a locked, materialised world whose one agent row was
/// legitimately paid for. `provider.hits()` after this is the baseline every
/// hosted invocation below must hold.
fn seed(user: &UserScratch, project: &Path) {
    let output = user
        .vibe()
        .args(["install", "--json", "--assume-yes"])
        .args(["--agent-mode", "cli"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_ok(&output);
}

/// No document beside the registered root: not a `compile-trace` echo, not a
/// loose run-id object.
fn no_standalone_trace(docs: &[Value]) {
    for doc in docs {
        assert_ne!(
            doc["command"].as_str(),
            Some("compile-trace"),
            "the member rides the registered root, never a document of its own: {docs:?}",
        );
        assert!(
            doc.get("run_id").is_none(),
            "a bare run id is not a report: {docs:?}",
        );
    }
}

/// The ONE registered terminal root of a completed (or unavailable) stream,
/// with every other document in that stream accounted for.
///
/// A terminal stream may legitimately carry known supplementary documents —
/// `<command>:plan`, `<command>:closure-diff` — and only those. Anything else
/// beside the expected root is a SECOND report: an extra `install` root next
/// to the `update` one, a bare run-id object, a standalone trace echo — each
/// fails here instead of passing as "some other document". The root must also
/// be the stream's LAST document: nothing reports after the terminal report.
fn sole_terminal_root(bytes: &[u8], command: &str) -> Value {
    let docs = documents(bytes);
    let mut roots: Vec<(usize, &Value)> = Vec::new();
    let supplements = [
        "install:plan",
        "lifecycle:plan",
        "install:closure-diff",
        "update:closure-diff",
    ];
    for (index, doc) in docs.iter().enumerate() {
        assert!(
            doc.get("run_id").is_none(),
            "a bare run id is not a report: {docs:?}",
        );
        let name = doc["command"].as_str().unwrap_or("");
        if !supplements.contains(&name) {
            roots.push((index, doc));
        }
    }
    let [(index, root)] = roots.as_slice() else {
        panic!("exactly one non-supplementary document — the registered root: {docs:?}");
    };
    assert_eq!(
        root["command"].as_str(),
        Some(command),
        "the terminal root is the expected command: {docs:?}",
    );
    assert_eq!(
        *index,
        docs.len() - 1,
        "the registered root is the LAST document in the stream: {docs:?}",
    );
    (*root).clone()
}

/// What a traced park leaves behind, saved for its resume to be measured
/// against.
struct Parked {
    run_id: String,
    started: Timestamp,
    state_started: String,
    events: usize,
    scopes: usize,
    task: String,
}

/// The shared traced-park matrix. Returns the facts the resume must be held
/// to, plus the parked root itself for the command-specific assertions.
fn assert_traced_park(
    project: &Path,
    output: &std::process::Output,
    command: &str,
    provider: &MockProvider,
    baseline: usize,
) -> (Parked, Value) {
    assert_ok(output);
    let docs = documents(&output.stdout);
    assert_eq!(
        docs.len(),
        1,
        "a park emits exactly one document — its registered root: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    no_standalone_trace(&docs);
    let report = docs[0].clone();
    assert_eq!(report["command"].as_str(), Some(command));
    let handoff = &report["delegation"];
    assert!(handoff.is_object(), "the slot row parked: {report}");
    let trace = trace_member(&report).expect("a parked traced run still reports its trace");
    assert_eq!(
        trace["status"], "running",
        "a park does not finish — it suspends: {trace}",
    );
    assert_eq!(trace["finalised"], false);
    let run_id = trace["run_id"]
        .as_str()
        .expect("the member names its run")
        .to_string();
    assert_eq!(
        handoff["run_id"].as_str(),
        Some(run_id.as_str()),
        "the trace belongs to the lifecycle run that parked",
    );

    let state = lifecycle_state(project);
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(run_id.as_str()),
        "member id = delegation id = lifecycle-state id: {state:?}",
    );
    assert!(
        state.run.compile_trace,
        "the request is sticky in the state, which is how a flagless resume keeps it",
    );
    assert!(
        state.run.slot_continuation.is_some(),
        "and the run records exactly what it owes: {state:?}",
    );

    let index = index_of(project, &run_id);
    assert!(matches!(index.status, RunStatus::Running));
    assert!(
        index.finished.is_none(),
        "no finish instant is invented for a suspended run",
    );
    assert!(!index.events.is_empty(), "the park really compiled");
    assert!(!index.scopes.is_empty(), "and really declared scopes");
    assert_eq!(
        run_directories(project),
        vec![run_id.clone()],
        "exactly one run directory owns this park",
    );

    let task = handoff["tasks"][0]
        .as_str()
        .expect("one owed task")
        .to_string();
    assert!(
        project.join(&task).is_file(),
        "the owed task exists for the hosting agent",
    );
    assert_eq!(provider.hits(), baseline, "parking adds no spend");
    (
        Parked {
            run_id,
            started: index.started,
            state_started: state.run.started,
            events: index.events.len(),
            scopes: index.scopes.len(),
            task,
        },
        report,
    )
}

/// The shared traced-resume matrix: same run, same original start, appended
/// history, one owner. `>=` is refused everywhere a count moves — a run that
/// reset to zero and rebuilt to the same count must not pass.
fn assert_traced_resume(
    project: &Path,
    output: &std::process::Output,
    command: &str,
    parked: &Parked,
    provider: &MockProvider,
    baseline: usize,
) -> Value {
    assert_ok(output);
    let root = sole_terminal_root(&output.stdout, command);
    assert!(
        root.get("delegation").is_none(),
        "the resume satisfied the park: {root}",
    );
    let trace = trace_member(&root).expect("the sticky bit kept tracing without a flag");
    assert_eq!(
        trace["run_id"].as_str(),
        Some(parked.run_id.as_str()),
        "the resume reopened the run it parked, not a fresh one",
    );
    assert_eq!(trace["status"], "ok");
    assert_eq!(trace["finalised"], true);

    let index = index_of(project, &parked.run_id);
    assert!(matches!(index.status, RunStatus::Ok));
    assert!(index.finished.is_some());
    assert_eq!(
        index.started, parked.started,
        "the ORIGINAL start survives the reopen",
    );
    assert!(
        index.events.len() > parked.events,
        "the resume APPENDS events to the suspended run: {} -> {}",
        parked.events,
        index.events.len(),
    );
    assert!(
        index.scopes.len() > parked.scopes,
        "and a scope occurrence too: {} -> {}",
        parked.scopes,
        index.scopes.len(),
    );
    let mut sequences: Vec<u32> = index.events.iter().map(|event| event.sequence).collect();
    sequences.sort_unstable();
    assert_eq!(
        sequences,
        (0..u32::try_from(sequences.len()).unwrap()).collect::<Vec<_>>(),
        "one dense global sequence across BOTH halves of the run",
    );
    let member_events = trace["events"]
        .as_str()
        .expect("a decimal count")
        .to_string();
    assert_eq!(
        member_events,
        index.events.len().to_string(),
        "the member counts exactly what the index holds",
    );

    let state = lifecycle_state(project);
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(parked.run_id.as_str()),
        "the resume ran under the ORIGINAL run id",
    );
    assert_eq!(
        state.run.started, parked.state_started,
        "and the state keeps the park's own start",
    );
    assert!(
        state.run.slot_continuation.is_none(),
        "the serviced continuation is cleared: {state:?}",
    );

    assert_eq!(
        run_directories(project),
        vec![parked.run_id.clone()],
        "still exactly one run directory — the resume minted no second owner",
    );
    assert!(
        !project.join(&parked.task).exists(),
        "the exact owned task is gone",
    );
    assert!(
        !project
            .join(".vibe/agentic/outbox")
            .join(&parked.run_id)
            .exists(),
        "and its proven-empty run directory is pruned",
    );
    assert_eq!(provider.hits(), baseline, "and neither does the resume pay");
    root
}

/// `vibe update org.demo/tools …` — the scoped grammar.
fn update_scoped(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    user.vibe()
        .args(["update", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// `vibe update --all …` — the whole-graph grammar.
fn update_all(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    user.vibe()
        .args(["update", "--all", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// SCOPED: a traced update parks as `update` (scoped, naming its package), and
/// the same command WITHOUT the flag resumes and finalises the same run.
#[test]
fn a_traced_scoped_update_parks_and_its_flagless_resume_finalises_the_run() {
    if !git_available() {
        eprintln!("skipping hosted traced update e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent_with_boot(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_static_tools(project.path());
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    // The seed ran in CLI mode and legitimately paid for its own row; every
    // assertion below is about ADDITIONAL spend.
    let baseline = provider.hits();
    assert!(
        run_directories(project.path()).is_empty(),
        "the untraced seed left no trace tree of its own",
    );
    add_version(&published, "slot:post-install", "0.1.1");

    let (parked, root) = assert_traced_park(
        project.path(),
        &update_scoped(&user, project.path(), &["--trace-compile"]),
        "update",
        &provider,
        baseline,
    );
    assert_eq!(root["scope"], "scoped", "the scoped shape");
    assert_eq!(
        root["packages"],
        serde_json::json!(["org.demo/tools"]),
        "naming exactly the targeted package",
    );

    write_declared_output(project.path());
    assert_traced_resume(
        project.path(),
        &update_scoped(&user, project.path(), &[]),
        "update",
        &parked,
        &provider,
        baseline,
    );
}

/// WHOLE: `vibe update --all` over a lock whose slot was removed keeps its own
/// identity, parks traced, and the flagless whole update resumes it.
#[test]
fn a_traced_whole_update_parks_and_its_flagless_resume_finalises_the_run() {
    if !git_available() {
        eprintln!("skipping hosted traced update e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent_with_boot(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_static_tools(project.path());
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();
    assert!(
        run_directories(project.path()).is_empty(),
        "the untraced seed left no trace tree of its own",
    );

    // A whole update delegates to install-from-manifest, which is freshness
    // based: with the lock still satisfying `[requires]`, nothing re-resolves
    // and no payload event fires. Removing the materialised slot is what makes
    // this run a real materialisation — the thing that reaches the hosted row.
    std::fs::remove_dir_all(
        project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools"),
    )
    .unwrap();

    let (parked, root) = assert_traced_park(
        project.path(),
        &update_all(&user, project.path(), &["--trace-compile"]),
        "update",
        &provider,
        baseline,
    );
    assert_eq!(root["scope"], "all", "the whole-graph shape");

    write_declared_output(project.path());
    assert_traced_resume(
        project.path(),
        &update_all(&user, project.path(), &[]),
        "update",
        &parked,
        &provider,
        baseline,
    );
}
