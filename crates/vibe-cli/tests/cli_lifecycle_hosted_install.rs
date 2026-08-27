//! The install/slot seam under a HOSTING agent, and the destructive clean
//! runner's explicit refusal.
//!
//! A `slot:` agent row is legal, so the hosted branch has to reach the slot
//! call site too. One fixture declares it at `slot:pre-install` — the earliest
//! slot point, so "parks BEFORE materialisation and before the lockfile
//! barrier" is provable from the tree rather than merely asserted — and one at
//! `slot:post-install`, where the park represents real partial progress the
//! report must not erase.
//!
//! The evidence is the same hit-counting loopback provider the paid slot e2e
//! uses: a configured, reachable endpoint is present precisely so a regression
//! that fell through to the paid path would be caught by the counter rather
//! than by an unrelated "no provider configured" failure.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::agent_provider::{MockProvider, configure_provider};
use common::{UserScratch, git_available, run_git, write_project_with_per_package_registry};
use vibe_wire::generated::install_report::InstallReport;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecordScope, ExecutionRecordStatus, LifecycleState,
};

const RESULT: &str = r#"{"outputs":[{"path":"docs/slot.md","content":"paid slot body\n"}]}"#;

fn slot_agent_source(root: &Path, point: &str) -> PathBuf {
    let source = root.join("src-slot-agent");
    fs::create_dir_all(source.join("vibevm/vibespecs/common")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "tools"
kind = "flow"
version = "0.1.0"

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
        source.join("vibevm/vibespecs/common/agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the slot document. MARKER=SLOT-0.1.0\n",
    )
    .unwrap();
    fs::write(source.join("payload.txt"), "payload one\n").unwrap();
    run_git(&source, &["add", "-A"]);
    run_git(&source, &["commit", "-m", "org.demo/tools@0.1.0"]);
    run_git(&source, &["tag", "v0.1.0"]);
    source
}

fn publish(root: &Path, source: &Path) -> PathBuf {
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
    root.to_path_buf()
}

fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

fn documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// An explicit `vibe install --json` under a hosting agent still emits ONE
/// generated machine document, and that document carries the handoff as an
/// additive typed member. The park happens at the slot seam — before the
/// post-barrier work — and calls no provider.
#[test]
fn a_hosted_slot_row_parks_before_post_barrier_work_and_reports_one_document() {
    if !git_available() {
        eprintln!("skipping hosted slot-agent CLI e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(RESULT);
    let outer = tempfile::tempdir().unwrap();
    let source = slot_agent_source(outer.path(), "slot:pre-install");
    let registry = publish(outer.path(), &source);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    configure_provider(&user, &provider.endpoint());

    let output = user
        .vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "a durable handoff exits 0: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(
        provider.hits(),
        0,
        "the hosted slot branch never constructs a provider"
    );

    let documents = documents(&output.stdout);
    // THE contract: one TOTAL document. A slot plan preview, a per-row echo
    // and a lifecycle report beside the install report were all extra.
    assert_eq!(
        documents.len(),
        1,
        "hosted parking emits exactly one JSON document: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let carriers: Vec<&serde_json::Value> = documents.iter().collect();
    assert!(
        carriers[0].get("delegation").is_some(),
        "and it is the document carrying the handoff",
    );
    // The registered root format for `vibe install --json` does not change
    // with runtime status: a parked run is still a `cli-install-report`, with
    // the handoff as its additive typed member.
    let report: InstallReport = serde_json::from_value((*carriers[0]).clone()).unwrap();
    assert_eq!(report.command, "install", "the root format is unchanged");
    assert!(report.ok, "a durable handoff keeps ok=true");
    assert!(
        !report.project.is_empty(),
        "the install report still names its project"
    );
    assert!(
        serde_json::from_value::<vibe_wire::generated::lifecycle_report::LifecycleReport>(
            (*carriers[0]).clone()
        )
        .is_err(),
        "and it is NOT a lifecycle report wearing the install command's name",
    );
    let handoff = report.delegation.as_ref().unwrap();
    assert_eq!(handoff.tasks.len(), 1);
    assert_eq!(handoff.resume, "vibe install");
    assert!(
        project.path().join(&handoff.tasks[0]).is_file(),
        "the slot task is durably published: {}",
        handoff.tasks[0],
    );
    assert!(
        report.materialised.is_empty(),
        "a pre-install park materialised nothing, and says so: {:?}",
        report,
    );
    assert!(
        !report.complete,
        "and it says the apply did NOT finish: {:?}",
        report,
    );
    assert!(
        !stdout(&output).contains("```"),
        "JSON mode prints no fence"
    );

    // The post-barrier work did NOT happen: the paid slot output is absent
    // and the lockfile barrier was never crossed for this install.
    assert!(
        !project.path().join("docs/slot.md").exists(),
        "a parked slot row produces no output of its own",
    );
    let lock = fs::read_to_string(project.path().join("vibe.lock")).unwrap_or_default();
    assert!(
        !lock.contains("tools"),
        "the park stopped the install before its lockfile barrier: {lock}",
    );
    let slot = project
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.demo.tools")
        .join("0.1.0");
    assert!(
        !slot.join("payload.txt").exists(),
        "a pre-install park precedes the slot's own payload materialisation: {:?}",
        fs::read_dir(&slot)
            .map(|entries| entries
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>())
            .unwrap_or_default(),
    );
}

/// A park AFTER the materialise pass represents real partial progress. The
/// document must say so: `installed` names the dependency slots this run
/// actually materialised, never a false `[]`. Human mode prints exactly one
/// fenced handoff and no "completed" summary.
#[test]
fn a_post_install_park_reports_the_dependency_progress_it_really_made() {
    if !git_available() {
        eprintln!("skipping hosted slot-agent CLI e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(RESULT);
    let outer = tempfile::tempdir().unwrap();
    let source = slot_agent_source(outer.path(), "slot:post-install");
    let registry = publish(outer.path(), &source);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    configure_provider(&user, &provider.endpoint());

    let json = user
        .vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        json.status.success(),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    assert_eq!(provider.hits(), 0);
    let carriers = documents(&json.stdout);
    assert_eq!(
        carriers.len(),
        1,
        "still exactly one TOTAL document: {}",
        String::from_utf8_lossy(&json.stdout),
    );
    let report: InstallReport = serde_json::from_value(carriers[0].clone()).unwrap();
    assert!(
        !report.materialised.is_empty(),
        "the slot was materialised before the park; reporting nothing would be false",
    );
    assert!(
        report.materialised[0].contains("tools"),
        "the record names the slot it materialised: {:?}",
        report,
    );
    // Two facts, deliberately not the same fact. The MATERIALISATION finished
    // — the slot list above proves it — but the COMMAND did not: it is parked
    // waiting on the hosting agent, and will be resumed. `complete` is the
    // command's, so it is false here even though nothing about the apply is
    // partial.
    assert!(
        !report.complete,
        "a parked command is never complete, however much it materialised: {:?}",
        report,
    );
    assert!(
        report.delegation.is_some(),
        "and that is exactly why: the run is waiting on a handoff: {:?}",
        report,
    );

    // Human mode on a second, independent project: one fence, no false
    // "completed" summary.
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    let human = user
        .vibe()
        .args(["install", "org.demo/tools", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        human.status.success(),
        "{}",
        String::from_utf8_lossy(&human.stderr)
    );
    let text = stdout(&human);
    assert_eq!(
        text.matches("```vibe-agent-tasks").count(),
        1,
        "exactly one fence: {text}",
    );
    assert!(text.contains("resume: vibe install"), "{text}");
    assert!(
        text.contains("parked for the hosting agent"),
        "the summary says parked: {text}",
    );
    assert!(
        !text.contains("package(s) materialised into vibedeps/"),
        "and never claims the install completed: {text}",
    );
    assert_eq!(provider.hits(), 0);
}

/// The headline of repair #2: a POST-install park writes the lock first, so
/// its resume lands on the fresh fast path. The same command must rebuild the
/// slot run from the persisted target set, finish it, clear the continuation
/// and only THEN let anything downstream proceed.
#[test]
fn a_post_install_park_resumes_through_the_fresh_lock_and_clears_its_continuation() {
    if !git_available() {
        eprintln!("skipping hosted slot-agent CLI e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(RESULT);
    let outer = tempfile::tempdir().unwrap();
    let source = slot_agent_source(outer.path(), "slot:post-install");
    let registry = publish(outer.path(), &source);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    configure_provider(&user, &provider.endpoint());

    let install = |user: &UserScratch| {
        user.vibe()
            .args(["install", "org.demo/tools", "--json", "--assume-yes"])
            .args(["--agent-mode", "agent"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    };

    let parked = install(&user);
    assert!(
        parked.status.success(),
        "{}",
        String::from_utf8_lossy(&parked.stderr)
    );
    let parked_docs = documents(&parked.stdout);
    assert_eq!(parked_docs.len(), 1, "one total document on the park");
    let report: InstallReport = serde_json::from_value(parked_docs[0].clone()).unwrap();
    let handoff = report.delegation.expect("the post-install row parked");
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();
    assert!(project.path().join(&task).is_file());

    // The lock is now FRESH, and the state carries both the live slot-scoped
    // park and the continuation that names the exact target set.
    let state: LifecycleState =
        toml::from_str(&fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap())
            .unwrap();
    let continuation = state
        .run
        .slot_continuation
        .as_ref()
        .expect("the slot run recorded what it owes");
    assert_eq!(
        continuation.targets.len(),
        1,
        "exactly the payload-event target set, not every installed package",
    );
    assert_eq!(continuation.targets[0].name, "tools");
    assert!(
        state.execution.values().any(|row| {
            row.status == ExecutionRecordStatus::Delegated
                && row.scope == Some(ExecutionRecordScope::Slot)
        }),
        "the park is tagged with its typed scope, not inferred from a key",
    );
    // The mutation detector: the row AFTER the agent must not have run.
    assert!(
        !report
            .contributions
            .iter()
            .any(|row| row.key.contains("#after-agent")),
        "no post-barrier row runs on the parking invocation: {:?}",
        report.contributions,
    );

    // The hosting agent does the work; the SAME command resumes.
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(
        project.path().join("docs/slot.md"),
        "hosted slot body
",
    )
    .unwrap();

    let resumed = install(&user);
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    assert_eq!(provider.hits(), 0, "neither invocation paid a provider");
    // HARD assertions: the resume must SATISFY the park, not repark.
    let resumed_docs = documents(&resumed.stdout);
    let root_docs: Vec<&serde_json::Value> = resumed_docs
        .iter()
        .filter(|document| document.get("command") == Some(&serde_json::json!("install")))
        .collect();
    assert_eq!(
        root_docs.len(),
        1,
        "exactly one `cli-install-report` on the resume: {}",
        String::from_utf8_lossy(&resumed.stdout),
    );
    let resumed_report: InstallReport = serde_json::from_value(root_docs[0].clone()).unwrap();
    assert!(
        resumed_report.delegation.is_none(),
        "the resume SATISFIED the park; nothing is still owed: {resumed_report:?}",
    );
    assert_eq!(provider.hits(), 0, "and it paid for nothing");

    let after: LifecycleState =
        toml::from_str(&fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap())
            .unwrap();
    assert_eq!(
        after.run.run_id.as_deref(),
        Some(run_id.as_str()),
        "the resume ran under the ORIGINAL run id",
    );
    assert!(
        after
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no delegated slot record remains: {after:?}",
    );
    assert!(
        after
            .execution
            .values()
            .any(|row| row.status == ExecutionRecordStatus::Ok),
        "the row is ok, not merely gone: {after:?}",
    );
    assert!(
        after.run.slot_continuation.is_none(),
        "and the continuation is cleared: {after:?}",
    );
    assert!(
        !project.path().join(&task).exists(),
        "the exact owned task is gone",
    );
    assert!(
        !project
            .path()
            .join(".vibe/agentic/outbox")
            .join(&run_id)
            .exists(),
        "and its proven-empty run directory is pruned",
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/slot.md")).unwrap(),
        "hosted slot body
",
        "the hosting agent's bytes are never rewritten",
    );
    assert!(
        resumed_report
            .contributions
            .iter()
            .any(|row| row.key.contains("#after-agent") && row.status == "ok"),
        "the downstream row runs only AFTER the park was satisfied: {:?}",
        resumed_report.contributions,
    );
}

/// The destructive clean runner is state-blind. Under a hosting agent it must
/// neither pay a provider nor wipe: it refuses explicitly, naming the row.
#[test]
fn the_destructive_clean_runner_refuses_a_hosted_agent_row_without_wiping() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Agent"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let mut patched = String::new();
    for line in manifest.lines() {
        let line = if line.starts_with("name = ") && !line.contains("demo") {
            "name = \"demo\"".to_string()
        } else {
            line.to_string()
        };
        patched.push_str(&line);
        patched.push('\n');
        if line.starts_with("name = ") && !patched.contains("group = ") {
            patched.push_str("group = \"org.demo\"\n");
        }
    }
    patched.push_str(
        r#"
[[extension]]
id = "clean-agent"
point = "phase:clean"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/clean.md", kind = "file", accept = "non-empty file" },
]
"#,
    );
    fs::write(&manifest_path, patched).unwrap();
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the clean document.\n",
    )
    .unwrap();
    configure_provider(&user, &provider.endpoint());

    let sentinel = project
        .path()
        .join(vibe_core::layout::current_vibedeps_root());
    fs::create_dir_all(&sentinel).unwrap();
    fs::write(sentinel.join("witness.txt"), "kept\n").unwrap();

    // The scratch child set, byte-for-byte, before and after: allocating a run
    // directory is itself a mutation an unsupported invocation must not make.
    let before_scratch = scratch_children(project.path());
    let output = user
        .vibe()
        .args(["clean", "--assume-yes", "--agent-mode", "agent", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "an unsupported hosted clean is an explicit refusal, not a silent success",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("clean-agent"),
        "the refusal names the row it cannot host: {stderr}",
    );
    assert!(
        stderr.contains("neither paid a provider nor removed anything"),
        "and says exactly what it did not do: {stderr}",
    );
    assert_eq!(provider.hits(), 0, "the refusal spent nothing");
    assert_eq!(
        scratch_children(project.path()),
        before_scratch,
        "and it allocated NO scratch run directory: the refusal precedes          `select_run_identity`, which mints one on its fresh branch",
    );
    assert!(
        sentinel.join("witness.txt").exists(),
        "and wiped nothing: the refusal precedes the destructive step",
    );
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Every child of `.vibe/lifecycle/`, sorted — the scratch run directories a
/// run allocates.
fn scratch_children(project: &Path) -> Vec<String> {
    let base = project.join(".vibe/lifecycle");
    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
