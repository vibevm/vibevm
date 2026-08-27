//! R3.4 — a compile trace across a hosted PARK, its resume, and displacement.
//!
//! A park is the one outcome where the trace deliberately does NOT finish: the
//! run is suspended, its index stays `running`, and the recorder drops so the
//! resume can take the cooperative lock back. Everything here is about that
//! seam being real rather than merely described:
//!
//! * the resume reuses the SAME run id and the SAME original start, and it
//!   does so WITHOUT the one-shot flag — the lifecycle's sticky bit is what
//!   carries the request across;
//! * events APPEND to the suspended run rather than resetting it;
//! * a displaced predecessor is terminalised where it stands, and a
//!   predecessor that never existed is not manufactured so that something
//!   could be superseded.
//!
//! Every destructive step here operates on a temporary project. Nothing in
//! this file touches the repository's own trace tree.

mod common;
mod trace_support;

use std::fs;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, documents, lifecycle_state, project_at, publish_slot_agent, sole_document,
    write_declared_output,
};
use common::{UserScratch, git_available};
use trace_support::{index_of, run_directories, trace_member};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

/// `vibe install org.demo/tools` under a hosting agent.
fn install(user: &UserScratch, project: &std::path::Path, extra: &[&str]) -> std::process::Output {
    user.vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// The same, from a DECLARED static requirement rather than a positional
/// pkgref — so the install really compiles a boot artifact and the trace has
/// scopes and events to append to.
fn install_declared(
    user: &UserScratch,
    project: &std::path::Path,
    extra: &[&str],
) -> std::process::Output {
    user.vibe()
        .args(["install", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// The shared hosted fixture declares extensions and no boot snippet, so a run
/// over it compiles NOTHING — which makes it useless for proving that a resume
/// appends rather than resets.
///
/// This one adds the missing half: a `[boot_snippet]` the consumer links
/// STATICALLY, so the node really compiles, the trace really records a scope,
/// and "the resume appended" is a number that can go up.
fn publish_slot_agent_with_boot(root: &std::path::Path, point: &str) {
    let source = root.join("src-boot");
    fs::create_dir_all(source.join("vibevm/vibespecs/common")).unwrap();
    fs::create_dir_all(source.join("boot")).unwrap();
    common::run_git(&source, &["init", "--initial-branch=main"]);
    common::run_git(&source, &["config", "user.email", "t@example.com"]);
    common::run_git(&source, &["config", "user.name", "Test"]);
    fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "tools"
kind = "flow"
version = "0.1.0"

[boot_snippet]
source = "boot/40-tools.md"
category = "flow"

[[extension]]
id = "slot-produce"
point = "{point}"
handler = {{ kind = "agent", prompt = "spec://org.demo/tools/common/agent-prompt#root" }}
config.outputs = [
  {{ path = "docs/slot.md", kind = "file", accept = "non-empty file" }},
]

[[extension]]
id = "after-agent"
point = "{point}"
handler = {{ kind = "builtin", name = "log" }}
config = {{ message = "SENTINEL-AFTER-SLOT-AGENT" }}
"#
        ),
    )
    .unwrap();
    fs::write(
        source.join("boot/40-tools.md"),
        "# Tools {#root}\n\nTOOLS BOOT BODY\n",
    )
    .unwrap();
    fs::write(
        source.join("vibevm/vibespecs/common/agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the slot document. MARKER=SLOT\n",
    )
    .unwrap();
    fs::write(source.join("payload.txt"), "payload one\n").unwrap();
    common::run_git(&source, &["add", "-A"]);
    common::run_git(&source, &["commit", "-m", "org.demo/tools@0.1.0"]);
    common::run_git(&source, &["tag", "v0.1.0"]);

    let bare = root.join("org.demo.tools.git");
    common::run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    common::run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
}

/// Declare `org.demo/tools` with a STATIC link, which is what makes the node
/// compile at all.
fn declare_static_tools(project: &std::path::Path) {
    let manifest = project.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(
        "\n[requires]\npackages = { \"flow:org.demo/tools\" = { version = \"^0.1\", link = \"static\" } }\n",
    );
    fs::write(&manifest, text).unwrap();
}

/// The one run this project's trace tree contains.
fn sole_run_id(project: &std::path::Path) -> String {
    let runs = run_directories(project);
    assert_eq!(runs.len(), 1, "exactly one run directory: {runs:?}");
    runs.into_iter().next().unwrap()
}

/// A park suspends the trace; the SAME command, without the flag, resumes and
/// finalises the SAME run.
///
/// The absent flag on the second invocation is the point. The request survives
/// in the lifecycle state's sticky bit, so a resume keeps tracing even though
/// nobody asked again — and the run it reopens is the one it parked, proved by
/// the id and by the original `started` instant surviving unchanged.
#[test]
fn a_hosted_park_suspends_the_trace_and_the_flagless_resume_finalises_it() {
    if !git_available() {
        eprintln!("skipping hosted trace e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    publish_slot_agent_with_boot(outer.path(), "slot:post-install");
    let user = UserScratch::new();
    let project = project_at(&user, outer.path());
    declare_static_tools(project.path());
    configure_provider(&user, &provider.endpoint());

    // ---- the parking invocation, traced -------------------------------
    let parked = install_declared(&user, project.path(), &["--trace-compile"]);
    assert!(
        parked.status.success(),
        "a park is not a failure: {}",
        String::from_utf8_lossy(&parked.stderr),
    );
    let report = sole_document(&parked.stdout);
    assert!(
        report["delegation"].is_object(),
        "the post-install row parked: {report}",
    );
    let trace = trace_member(&report).expect("a parked traced run still reports its trace");
    assert_eq!(
        trace["status"], "running",
        "a park does not finish — it suspends: {trace}",
    );
    assert_eq!(trace["finalised"], false);

    let run_id = trace["run_id"].as_str().unwrap().to_string();
    assert_eq!(
        run_id,
        report["delegation"]["run_id"].as_str().unwrap(),
        "the trace belongs to the lifecycle run that parked",
    );
    assert_eq!(sole_run_id(project.path()), run_id);

    let parked_index = index_of(project.path(), &run_id);
    let started = parked_index.started;
    assert!(matches!(parked_index.status, RunStatus::Running));
    assert!(
        parked_index.finished.is_none(),
        "no finish instant is invented for a suspended run",
    );
    let parked_events = parked_index.events.len();
    let parked_scopes = parked_index.scopes.len();

    assert!(
        lifecycle_state(project.path()).run.compile_trace,
        "the request is sticky in the state, which is how a flagless resume keeps it",
    );

    // ---- the hosting agent does the work, and the SAME command resumes --
    write_declared_output(project.path());
    let resumed = install_declared(&user, project.path(), &[]);
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_report = documents(&resumed.stdout)
        .into_iter()
        .find(|doc| doc["command"] == "install")
        .expect("the resume emits its one install root");
    let resumed_trace =
        trace_member(&resumed_report).expect("the sticky bit kept tracing without a flag");
    assert_eq!(
        resumed_trace["run_id"].as_str().unwrap(),
        run_id,
        "the resume reopened the run it parked, not a fresh one",
    );
    assert_eq!(resumed_trace["status"], "ok");
    assert_eq!(resumed_trace["finalised"], true);

    let final_index = index_of(project.path(), &run_id);
    assert_eq!(
        final_index.started, started,
        "the ORIGINAL start survives the reopen",
    );
    assert!(matches!(final_index.status, RunStatus::Ok));
    assert!(final_index.finished.is_some());
    // The resume really COMPILES again — the post-install continuation
    // regenerates the node — so the run gains a scope occurrence and events.
    // Asserting `>=` would have passed on a run that reset to zero and
    // rebuilt to the same count, which is the failure this is here to catch.
    assert!(
        final_index.scopes.len() > parked_scopes,
        "the resume APPENDS a scope occurrence to the suspended run: {} -> {}",
        parked_scopes,
        final_index.scopes.len(),
    );
    assert!(
        final_index.events.len() > parked_events,
        "and its events append too: {} -> {}",
        parked_events,
        final_index.events.len(),
    );
    assert!(
        final_index
            .events
            .iter()
            .any(|event| usize::try_from(event.sequence).unwrap_or(0) >= parked_events),
        "with sequence numbers CONTINUING past the park, not restarting at 0",
    );
    let mut sequences: Vec<u32> = final_index.events.iter().map(|e| e.sequence).collect();
    sequences.sort_unstable();
    assert_eq!(
        sequences,
        (0..u32::try_from(sequences.len()).unwrap()).collect::<Vec<_>>(),
        "one dense global sequence across BOTH halves of the run",
    );
    assert_eq!(
        sole_run_id(project.path()),
        run_id,
        "and still exactly one run directory — the resume minted nothing",
    );
    // The lock's release is proved by the RESUME, not by the lock file's
    // absence: the file is a lock handle and outlives the guard by design.
    // A park that had kept the lock would have made this reopen
    // `unavailable`, and the run would have finished as a second, partial
    // history instead of the same one.
    assert_eq!(
        resumed_trace["status"], "ok",
        "the reopen took the cooperative lock the park released",
    );
    assert_eq!(provider.hits(), 0, "the hosted path never pays a provider");
}

/// `--force` displaces a state-proven parked traced run: the predecessor is
/// terminalised where it stands, and the fresh run takes over.
///
/// The point is that displacement CLOSES rather than abandons. Without it the
/// old index would read `running` forever, and nothing would ever make it
/// retention-eligible.
#[test]
fn a_forced_repark_terminalises_the_run_it_displaced() {
    if !git_available() {
        eprintln!("skipping hosted trace e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    publish_slot_agent(outer.path(), "slot:pre-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, outer.path());
    configure_provider(&user, &provider.endpoint());

    let first = install(&user, project.path(), &["--trace-compile"]);
    assert!(first.status.success());
    let first_id = trace_member(&sole_document(&first.stdout)).expect("traced")["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(matches!(
        index_of(project.path(), &first_id).status,
        RunStatus::Running
    ));

    let second = install(&user, project.path(), &["--trace-compile", "--force"]);
    assert!(second.status.success());
    let second_id = trace_member(&sole_document(&second.stdout)).expect("traced")["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(first_id, second_id, "a forced repark is a NEW run");

    let displaced = index_of(project.path(), &first_id);
    assert!(
        matches!(displaced.status, RunStatus::Failed),
        "the displaced run is terminal, not abandoned mid-flight",
    );
    assert!(
        displaced.finished.is_some(),
        "and it carries the injected finish instant",
    );
    assert_eq!(
        displaced.failure.as_deref(),
        Some("superseded by a later invocation of this workspace"),
        "with the FIXED structural reason, never a quotation of anything",
    );

    let current = index_of(project.path(), &second_id);
    assert!(
        matches!(current.status, RunStatus::Running),
        "and the new owner is the only one still running",
    );
    let runs = run_directories(project.path());
    assert!(
        runs.contains(&first_id) && runs.contains(&second_id) && runs.len() == 2,
        "both runs are on disk; only one is current: {runs:?}",
    );
    assert_eq!(provider.hits(), 0);
}

/// A predecessor the state proves but the disk no longer has is left alone.
///
/// Superseding is an existing-only reopen: a directory that is not there stays
/// not-there. Manufacturing a phantom terminal run merely so that something
/// could be marked superseded would invent history.
#[test]
fn a_missing_predecessor_is_never_manufactured_to_be_superseded() {
    if !git_available() {
        eprintln!("skipping hosted trace e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    publish_slot_agent(outer.path(), "slot:pre-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, outer.path());
    configure_provider(&user, &provider.endpoint());

    let first = install(&user, project.path(), &["--trace-compile"]);
    assert!(first.status.success());
    let first_id = trace_member(&sole_document(&first.stdout)).expect("traced")["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Remove exactly this temporary project's old run directory, while the
    // lifecycle state still proves the identity that owns it.
    let old = trace_support::trace_dir(project.path()).join(&first_id);
    assert!(old.is_dir());
    fs::remove_dir_all(&old).unwrap();
    assert!(
        lifecycle_state(project.path()).run.compile_trace,
        "the state still proves a traced predecessor — that is what makes this a test",
    );

    let second = install(&user, project.path(), &["--trace-compile", "--force"]);
    assert!(second.status.success());
    let second_id = trace_member(&sole_document(&second.stdout)).expect("traced")["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    assert!(
        !old.exists(),
        "the absent predecessor was not recreated so it could be superseded",
    );
    assert_eq!(
        run_directories(project.path()),
        vec![second_id.clone()],
        "only the new run exists",
    );
    assert!(matches!(
        index_of(project.path(), &second_id).status,
        RunStatus::Running
    ));
    assert_eq!(provider.hits(), 0);
}
