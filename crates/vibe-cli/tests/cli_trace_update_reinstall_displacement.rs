//! R3.4 — a TRACE-DISABLED `vibe update` / `vibe reinstall` displacing a
//! state-proven parked traced install (REDs 19 and 22).
//!
//! The predecessor here is a traced `vibe install` that parked at its
//! post-install slot row under a hosting agent: its trace index honestly says
//! `running`, and the lifecycle state proves both halves a displacement needs
//! — a delegated row and the sticky trace bit. The displacing commands are
//! their OWN requested verbs (`update` / `reinstall`, never `install`), carry
//! NO `--trace-compile`, and keep `--agent-mode agent`, so the identity
//! selector cannot adopt: the prior run is displaced, not resumed.
//!
//! What that owes the operator is the ruling under test:
//!
//! * a CLEAN supersession says so in exactly ONE bounded stderr line —
//!   `vibe: warning: the displaced trace run `<id>` was finalised:
//!   superseded by a later invocation of this workspace` — after every
//!   writer warning, with the current command's own registered root carrying
//!   neither a `trace` nor a `notices` member (the trace-disabled wire format
//!   of both verbs is unchanged; the notice is a diagnostic, not a document
//!   key), and with the predecessor's index terminal `failed` under the FIXED
//!   superseded word;
//! * a predecessor whose recorded start is not RFC 3339 is refused in ONE
//!   bounded line and left EXACTLY as it is — still `running`, no finish, no
//!   failure, no phantom run manufactured so that something could be
//!   superseded.
//!
//! Every case drives the real binary: displacement is a fact about which
//! manifest was consulted, which root was locked and which state was read,
//! and none of that exists inside a unit test. The counting loopback provider
//! is configured and reachable throughout, so a fall-through to the paid path
//! would be caught by the counter rather than masked by "no provider".

mod common;
mod trace_support;

use std::fs;
use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, documents, lifecycle_state, project_at, publish_slot_agent, sole_document,
};
use common::{UserScratch, git_available};
use trace_support::{index_of, run_directories, trace_member};
use vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

/// What a displaced run's index records as its failure — FIXED text, never a
/// quotation of anything the displaced command produced.
const SUPERSEDED_FAILURE: &str = "superseded by a later invocation of this workspace";

/// The non-timestamp the invalid-start matrix writes into the persisted
/// state's `run.started`.
const BAD_START: &str = "not-an-rfc3339-time";

/// The exact body of the one structural notice a clean finalised
/// supersession emits — fixed prose, the validated run id its only variable.
fn structural_body(run_id: &str) -> String {
    format!(
        "the displaced trace run `{run_id}` was finalised: superseded by a later invocation \
         of this workspace"
    )
}

/// One fresh hosted fixture per case: `org.demo/tools@0.1.0` published with a
/// slot agent at `slot:post-install`, a counting loopback provider
/// configured, and a TRACED parking install under `--agent-mode agent` that
/// stops after the durable install.
///
/// Everything the later displacing invocation still needs is kept alive here:
/// the scratch settings home, the project, the published bare registry and
/// the provider whose hit count the matrices read at the end.
struct Parked {
    user: UserScratch,
    project: tempfile::TempDir,
    /// The parked run's trace id — the predecessor both matrices displace.
    run_id: String,
    provider: MockProvider,
    /// Keeps the published bare registry on disk for the displacing command;
    /// never read, held only for its lifetime.
    _registry: tempfile::TempDir,
}

fn park_traced_predecessor() -> Option<Parked> {
    if !git_available() {
        eprintln!("skipping displacement e2e: git not on PATH");
        return None;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let registry = tempfile::tempdir().unwrap();
    publish_slot_agent(registry.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, registry.path());
    configure_provider(&user, &provider.endpoint());

    let parked = user
        .vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(["--trace-compile"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        parked.status.success(),
        "a park is not a failure: {}",
        String::from_utf8_lossy(&parked.stderr),
    );
    let report = sole_document(&parked.stdout);
    assert!(
        report["delegation"].is_object(),
        "the post-install row parked for the hosting agent: {report}",
    );
    let trace = trace_member(&report).expect("a parked traced run still reports its trace");
    assert_eq!(trace["status"], "running", "a park suspends: {trace}");
    assert_eq!(trace["finalised"], false);
    let run_id = trace["run_id"].as_str().unwrap().to_string();
    assert_eq!(
        run_id,
        report["delegation"]["run_id"].as_str().unwrap(),
        "the trace belongs to the lifecycle run that parked",
    );

    // The state proves both halves the displacing invocation below needs: a
    // delegated row (the park is durable) and the STICKY trace bit (the park
    // was traced). Without the second, nothing would be superseded at all.
    let state = lifecycle_state(project.path());
    assert!(
        state.run.compile_trace,
        "the trace request is sticky: {state:?}"
    );
    assert!(
        state
            .execution
            .values()
            .any(|row| row.status == ExecutionRecordStatus::Delegated),
        "the delegated park is durable: {state:?}",
    );

    // The predecessor's own trace is a truthful suspension: running, no
    // terminal instant, and the only run directory on disk.
    let index = index_of(project.path(), &run_id);
    assert!(matches!(index.status, RunStatus::Running));
    assert!(
        index.finished.is_none(),
        "no finish was invented: {index:?}"
    );
    assert_eq!(run_directories(project.path()), vec![run_id.clone()]);
    assert_eq!(provider.hits(), 0, "the hosted park never pays a provider");
    Some(Parked {
        user,
        project,
        run_id,
        provider,
        _registry: registry,
    })
}

/// The changed requested command, update family: same agent mode, no trace
/// request — exactly the invocation whose differing `requested` verb makes
/// the selector displace rather than adopt.
fn displace_via_update(parked: &Parked) -> std::process::Output {
    parked
        .user
        .vibe()
        .args(["update", "--all", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(parked.project.path())
        .output()
        .unwrap()
}

/// The changed requested command, reinstall family. The project path is
/// POSITIONAL — that is reinstall's own grammar.
fn displace_via_reinstall(parked: &Parked) -> std::process::Output {
    parked
        .user
        .vibe()
        .args(["reinstall", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg(parked.project.path())
        .output()
        .unwrap()
}

/// The displacing command's ONE registered root: a completed,
/// delegation-free current-family document that carries neither a `trace`
/// member nor a `notices` member.
///
/// The two absences are the routing under test. The notice's one channel in
/// a trace-disabled JSON run is stderr — inventing a `notices` key here would
/// put a member on a registered format nobody agreed to, and a `trace` key
/// would claim a recorder a disabled run never opened.
fn assert_clean_root(output: &std::process::Output, family: &str) {
    let docs = documents(&output.stdout);
    let roots: Vec<&serde_json::Value> =
        docs.iter().filter(|doc| doc["command"] == family).collect();
    assert_eq!(roots.len(), 1, "EXACTLY one `{family}` root: {docs:#?}");
    let root = docs.last().expect("at least one document");
    assert_eq!(
        root["command"], family,
        "and the root is the LAST document on stdout: {docs:#?}",
    );
    let serialized = serde_json::to_string(root).unwrap();
    assert!(
        !serialized.contains("\"trace\""),
        "a trace-disabled root carries no trace member: {root:#?}",
    );
    assert!(
        !serialized.contains("\"notices\""),
        "and no notices member either — the notice is a stderr diagnostic: {root:#?}",
    );
    assert!(
        root.get("delegation").is_none(),
        "the displacing command completed without parking again: {root:#?}",
    );
    assert_eq!(root["complete"], true, "{root:#?}");
}

/// Every `vibe: warning: ` line on stderr — the one diagnostic channel that
/// exists in every mode and is never part of the document stream.
fn warning_lines(stderr: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| line.starts_with("vibe: warning: "))
        .map(|line| line.to_string())
        .collect()
}

/// Rewrite the persisted lifecycle state with ONLY the run's original start
/// replaced — deserialize, mutate the one field, serialize pretty — so the
/// state still proves the delegated park and the sticky bit, and the single
/// broken fact is the start the trace epoch cannot parse.
fn corrupt_displaced_start(project: &Path) {
    let path = project.join(".vibe/lifecycle.toml");
    let mut state = lifecycle_state(project);
    state.run.started = BAD_START.to_string();
    fs::write(&path, toml::to_string_pretty(&state).unwrap()).unwrap();
}

// ------------------------------------------------- RED 19: the clean matrix

/// Update family: a trace-disabled `vibe update --all` displaces the parked
/// traced install, proves the unabsorbable routing ONCE, and closes the
/// predecessor where it stands.
///
/// "Unabsorbable routing once" is the load-bearing claim: the update and
/// reinstall report formats declare no `notices` list, so the structural
/// notice must reach the operator through exactly one OTHER channel — one
/// bounded stderr line — rather than being folded into the document (a
/// member nobody agreed to) or dropped (the only account of the close).
#[test]
fn a_trace_disabled_update_displaces_a_parked_traced_install_cleanly() {
    let Some(parked) = park_traced_predecessor() else {
        return;
    };
    let output = displace_via_update(&parked);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_clean_root(&output, "update");

    let warnings = warning_lines(&output.stderr);
    assert_eq!(
        warnings.len(),
        1,
        "exactly one structural notice, on stderr: {warnings:?}",
    );
    assert_eq!(
        warnings[0],
        format!("vibe: warning: {}", structural_body(&parked.run_id)),
        "the exact body — fixed prose plus the run id, and nothing else",
    );
    assert!(
        structural_body(&parked.run_id).len() <= DIAGNOSTIC_CAP_BYTES,
        "the notice is whole-message bounded",
    );

    // No new run: the current command was never traced, so the trace tree
    // still holds exactly the predecessor.
    assert_eq!(
        run_directories(parked.project.path()),
        vec![parked.run_id.clone()],
    );
    let displaced = index_of(parked.project.path(), &parked.run_id);
    assert!(
        matches!(displaced.status, RunStatus::Failed),
        "the predecessor is terminal, not abandoned mid-flight: {displaced:?}",
    );
    assert!(displaced.finished.is_some(), "with its terminal instant");
    assert_eq!(
        displaced.failure.as_deref(),
        Some(SUPERSEDED_FAILURE),
        "the FIXED structural reason, never a quotation of anything",
    );
    assert_eq!(parked.provider.hits(), 0);
}

/// Reinstall family: the same changed-verb displacement through
/// `vibe reinstall`'s own grammar, its own root, and the same one-line
/// account.
#[test]
fn a_trace_disabled_reinstall_displaces_a_parked_traced_install_cleanly() {
    let Some(parked) = park_traced_predecessor() else {
        return;
    };
    let output = displace_via_reinstall(&parked);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_clean_root(&output, "reinstall");

    let warnings = warning_lines(&output.stderr);
    assert_eq!(warnings.len(), 1, "one structural notice: {warnings:?}");
    assert_eq!(
        warnings[0],
        format!("vibe: warning: {}", structural_body(&parked.run_id)),
    );
    assert!(structural_body(&parked.run_id).len() <= DIAGNOSTIC_CAP_BYTES);

    assert_eq!(
        run_directories(parked.project.path()),
        vec![parked.run_id.clone()],
        "no new run",
    );
    let displaced = index_of(parked.project.path(), &parked.run_id);
    assert!(matches!(displaced.status, RunStatus::Failed));
    assert!(displaced.finished.is_some());
    assert_eq!(displaced.failure.as_deref(), Some(SUPERSEDED_FAILURE));
    assert_eq!(parked.provider.hits(), 0);
}

// --------------------------------------------- RED 22: the invalid-start matrix

/// Update family: a predecessor whose recorded start cannot be parsed is
/// refused in one bounded line and left EXACTLY as it is.
///
/// The one broken fact is surgically planted — only `run.started` changes, so
/// the state still proves the traced delegated park and the displacement
/// really is attempted. What must not happen: a terminal write on a guessed
/// epoch, a phantom run, or a second line of output.
#[test]
fn an_invalid_displaced_start_on_update_refuses_once_and_touches_nothing() {
    let Some(parked) = park_traced_predecessor() else {
        return;
    };
    corrupt_displaced_start(parked.project.path());
    let output = displace_via_update(&parked);
    assert!(
        output.status.success(),
        "an unparsable predecessor start is a reason to leave the trace alone, \
         never a reason to fail: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_clean_root(&output, "update");

    let warnings = warning_lines(&output.stderr);
    assert_eq!(warnings.len(), 1, "one bounded refusal: {warnings:?}");
    let body = warnings[0]
        .strip_prefix("vibe: warning: ")
        .expect("the one warning line");
    assert!(body.contains(&parked.run_id), "it names the run: {body}");
    assert!(
        body.contains(BAD_START),
        "it quotes the exact bad start: {body}",
    );
    assert!(
        body.contains("not an RFC 3339"),
        "it says why nothing was done: {body}",
    );
    assert!(
        body.len() <= DIAGNOSTIC_CAP_BYTES,
        "the refusal is whole-message bounded: {}",
        body.len(),
    );

    assert_eq!(
        run_directories(parked.project.path()),
        vec![parked.run_id.clone()],
        "no new run and no phantom",
    );
    let predecessor = index_of(parked.project.path(), &parked.run_id);
    assert!(
        matches!(predecessor.status, RunStatus::Running),
        "the predecessor was left exactly as it is: {predecessor:?}",
    );
    assert!(predecessor.finished.is_none());
    assert!(predecessor.failure.is_none());
    assert_eq!(parked.provider.hits(), 0);
}

/// Reinstall family: the same surgical start corruption, through
/// `vibe reinstall`'s own grammar and root.
#[test]
fn an_invalid_displaced_start_on_reinstall_refuses_once_and_touches_nothing() {
    let Some(parked) = park_traced_predecessor() else {
        return;
    };
    corrupt_displaced_start(parked.project.path());
    let output = displace_via_reinstall(&parked);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_clean_root(&output, "reinstall");

    let warnings = warning_lines(&output.stderr);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let body = warnings[0]
        .strip_prefix("vibe: warning: ")
        .expect("the one warning line");
    assert!(body.contains(&parked.run_id));
    assert!(body.contains(BAD_START));
    assert!(body.contains("not an RFC 3339"));
    assert!(body.len() <= DIAGNOSTIC_CAP_BYTES, "{}", body.len());

    assert_eq!(
        run_directories(parked.project.path()),
        vec![parked.run_id.clone()],
        "no new run and no phantom",
    );
    let predecessor = index_of(parked.project.path(), &parked.run_id);
    assert!(matches!(predecessor.status, RunStatus::Running));
    assert!(predecessor.finished.is_none());
    assert!(predecessor.failure.is_none());
    assert_eq!(parked.provider.hits(), 0);
}
