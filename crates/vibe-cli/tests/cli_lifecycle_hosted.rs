//! `vibe create` under a HOSTING agent — park, stop, resume, spend nothing.
//!
//! The oracle is the same hit-counting loopback provider the paid create e2e
//! uses, reached through the production `vibe-llm` transport. Every case here
//! asserts `hits() == 0`: "the hosted branch never constructs a provider" is a
//! measurement, not a claim. A configured, reachable endpoint is present in
//! every fixture precisely so that a regression which fell through to the paid
//! path would SUCCEED and be caught by the counter, rather than failing for
//! the unrelated reason that no provider was configured.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

/// One agent row plus a builtin sentinel AFTER it in the same phase. The
/// sentinel is the mutation detector: remove the stop and it runs.
const ONE_ROW: &str = r#"
[[extension]]
id = "produce-docs"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/guide.md", kind = "file", accept = "non-empty file" },
]

[[extension]]
id = "after-agent"
point = "phase:create"
handler = { kind = "builtin", name = "log" }
config = { message = "SENTINEL-AFTER-AGENT" }
"#;

/// Two agent rows in one phase: they park SEQUENTIALLY across resumes,
/// because the second row's envelope may depend on the first's artifacts.
const TWO_ROWS: &str = r#"
[[extension]]
id = "produce-guide"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/guide.md", kind = "file", accept = "non-empty file" },
]

[[extension]]
id = "produce-reference"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/reference.md", kind = "file", accept = "non-empty file" },
]
"#;

const GOOD_RESULT: &str = r#"{"outputs":[{"path":"docs/guide.md","content":"paid\n"}]}"#;

fn project(extensions: &str) -> (UserScratch, tempfile::TempDir) {
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
    patched.push_str(extensions);
    fs::write(&manifest_path, patched).unwrap();

    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite the declared documentation files.\n",
    )
    .unwrap();
    (user, project)
}

/// A hosted invocation: `--agent-mode agent`, JSON unless told otherwise.
fn create(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = user.vibe();
    command
        .args(["create", "--assume-yes", "--path"])
        .arg(project)
        .args(extra);
    command.output().unwrap()
}

fn hosted_json(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["--json", "--agent-mode", "agent"];
    args.extend_from_slice(extra);
    create(user, project, &args)
}

use common::hosted_slot::documents;

fn report(bytes: &[u8]) -> LifecycleReport {
    let documents = documents(bytes);
    serde_json::from_value(documents.last().expect("one report document").clone()).unwrap()
}

fn state(project: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(project.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn assert_ok(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        stdout(output),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// The headline: a first hosted invocation parks, publishes exactly one task,
/// reports one typed handoff, stops the chain at `create`, exits 0 — and
/// never reaches the provider.
#[test]
fn a_first_hosted_invocation_parks_stops_the_phase_and_spends_nothing() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(ONE_ROW);
    configure_provider(&user, &provider.endpoint());

    let output = hosted_json(&user, project.path(), &[]);
    assert_ok(&output);
    assert_eq!(provider.hits(), 0, "parking never constructs a provider");

    let documents = documents(&output.stdout);
    // THE contract: one TOTAL document, not "one that happens to carry the
    // handoff". A buffered plan preview printed beside it was the defect.
    assert_eq!(
        documents.len(),
        1,
        "hosted parking emits exactly one JSON document: {}",
        stdout(&output),
    );
    let report = report(&output.stdout);
    let handoff = report.delegation.as_ref().expect("one typed handoff");
    assert_eq!(handoff.resume, "vibe create");
    assert_eq!(handoff.tasks.len(), 1);
    assert!(
        handoff.tasks[0].starts_with(&format!(".vibe/agentic/outbox/{}/", handoff.run_id)),
        "the task lives under the reported run: {}",
        handoff.tasks[0],
    );
    assert!(
        project.path().join(&handoff.tasks[0]).is_file(),
        "the task document is durably published before the run reports it",
    );
    assert!(
        !stdout(&output).contains("```"),
        "JSON mode prints no fence: {}",
        stdout(&output),
    );
    assert!(
        documents[0].get("delegation").is_some(),
        "and that sole document is the one carrying the handoff",
    );

    // Steps end AT the parked phase, and the sentinel after the agent row is
    // the mutation detector for the stop.
    assert_eq!(report.steps.last().unwrap().phase, "create");
    assert_eq!(report.steps.last().unwrap().status, "delegated");
    assert!(
        !report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent")),
        "no later contribution runs on the parking invocation: {:?}",
        report.contributions,
    );
    assert!(
        !stdout(&output).contains("SENTINEL-AFTER-AGENT"),
        "the post-agent sentinel never ran",
    );

    let state = state(project.path());
    assert_eq!(state.run.run_id.as_deref(), Some(handoff.run_id.as_str()));
    let row = state
        .execution
        .values()
        .find(|row| row.status == ExecutionRecordStatus::Delegated)
        .expect("the parked row is checkpointed");
    assert_eq!(row.tasks, [handoff.tasks[0].clone()]);
    assert_eq!(row.artifacts.len(), 1, "the exact planned rows are awaited");
}

/// Park → the hosting agent performs the task → the SAME command resumes:
/// `ok`, zero provider calls, the owned task removed, and the contribution
/// that the park had stopped now runs.
#[test]
fn a_satisfied_resume_completes_the_chain_removes_the_task_and_spends_nothing() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(ONE_ROW);
    configure_provider(&user, &provider.endpoint());

    let parked = hosted_json(&user, project.path(), &[]);
    assert_ok(&parked);
    let handoff = report(&parked.stdout).delegation.unwrap();
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();

    // The hosting agent does the work the task describes.
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "hosted body\n").unwrap();

    let resumed = hosted_json(&user, project.path(), &[]);
    assert_ok(&resumed);
    assert_eq!(provider.hits(), 0, "the resume never pays either");
    let report = report(&resumed.stdout);
    assert!(
        report.delegation.is_none(),
        "a satisfied resume reports no outstanding handoff"
    );
    assert_eq!(
        report
            .contributions
            .iter()
            .find(|row| row.key.ends_with("produce-docs"))
            .expect("the agent row is reported")
            .status,
        "ok",
    );
    assert!(
        report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent") && row.status == "ok"),
        "the contribution the park had stopped now runs: {:?}",
        report.contributions,
    );
    assert_eq!(report.steps.last().unwrap().status, "ok");

    assert!(
        !project.path().join(&task).exists(),
        "only the state-owned task is removed"
    );
    assert!(
        !project
            .path()
            .join(".vibe/agentic/outbox")
            .join(&run_id)
            .exists(),
        "its proven-empty run directory is pruned",
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "hosted body\n",
        "the hosting agent's bytes are never rewritten",
    );
    let state = state(project.path());
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no park survives its own satisfied resume",
    );
}

/// Two agent rows park one at a time: the first stops the chain, and only
/// once it is satisfied does the second get its own task.
#[test]
fn multiple_hosted_rows_park_sequentially_across_resumes() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(TWO_ROWS);
    configure_provider(&user, &provider.endpoint());

    let first = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    });
    let first_handoff = first.delegation.unwrap();
    assert_eq!(first_handoff.tasks.len(), 1, "one row parks at a time");
    assert_eq!(
        first
            .contributions
            .iter()
            .filter(|row| row.status == "delegated")
            .count(),
        1,
        "the second agent row was not batched with the first",
    );

    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "first\n").unwrap();

    let second = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    });
    let second_handoff = second.delegation.expect("the second row now parks");
    assert_eq!(
        second_handoff.run_id, first_handoff.run_id,
        "the resume is the SAME run, so the second park inherits its identity",
    );
    assert_ne!(
        second_handoff.tasks, first_handoff.tasks,
        "the second row publishes its own task",
    );

    fs::write(project.path().join("docs/reference.md"), "second\n").unwrap();
    let third = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    });
    assert!(third.delegation.is_none(), "both rows are satisfied");
    assert_eq!(provider.hits(), 0, "three hosted invocations, zero spend");
}

/// `--force` never inherits a parked identity: it allocates a fresh run and
/// reparks without probing, even with the declared outputs already on disk.
#[test]
fn force_reparks_under_a_fresh_run_without_probing() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(ONE_ROW);
    configure_provider(&user, &provider.endpoint());

    let parked = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    });
    let first = parked.delegation.unwrap();
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "hosted body\n").unwrap();

    let forced = report(&{
        let output = hosted_json(&user, project.path(), &["--force"]);
        assert_ok(&output);
        output.stdout
    });
    let second = forced.delegation.expect("--force reparks");
    assert_ne!(second.run_id, first.run_id, "a forced run is a NEW run");
    assert_eq!(provider.hits(), 0);
    assert!(
        project.path().join(&first.tasks[0]).exists(),
        "the earlier run's task is left as an honest orphan, not claimed",
    );
    let state = state(project.path());
    assert_eq!(state.run.run_id.as_deref(), Some(second.run_id.as_str()));
    assert_eq!(
        state
            .execution
            .values()
            .filter(|row| row.status == ExecutionRecordStatus::Delegated)
            .count(),
        1,
        "the fresh run carries only its own park",
    );
}

/// Human and quiet print EXACTLY ONE fenced `vibe-agent-tasks` block with the
/// run, the ordered tasks and the resume command — quiet included, because
/// suppressing narration is not a request to lose the work to be done.
#[test]
fn human_and_quiet_print_exactly_one_fenced_contract() {
    let provider = MockProvider::serving(GOOD_RESULT);
    for flag in ["--agent-mode", "--quiet"] {
        let (user, project) = project(ONE_ROW);
        configure_provider(&user, &provider.endpoint());
        let extra: Vec<&str> = if flag == "--quiet" {
            vec!["--quiet", "--agent-mode", "agent"]
        } else {
            vec!["--agent-mode", "agent"]
        };
        let output = create(&user, project.path(), &extra);
        assert_ok(&output);
        let text = stdout(&output);
        assert_eq!(
            text.matches("```vibe-agent-tasks").count(),
            1,
            "exactly one fence opens ({extra:?}): {text}",
        );
        assert_eq!(
            text.matches("```").count(),
            2,
            "and exactly one closes: {text}"
        );
        let run_id = state(project.path()).run.run_id.unwrap();
        assert!(text.contains(&format!("run: {run_id}")), "{text}");
        assert!(text.contains("resume: vibe create"), "{text}");
        assert!(
            text.contains(&format!("  - .vibe/agentic/outbox/{run_id}/task-")),
            "the ordered task list is part of the contract: {text}",
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(&text).is_err(),
            "human/quiet is not JSON",
        );
    }
    assert_eq!(provider.hits(), 0);
}

/// Mode resolution: `auto` + a resolved invoked-by parks; explicit `agent`
/// parks with no env at all; explicit `cli` wins over the env and pays.
#[test]
fn auto_env_explicit_agent_and_explicit_cli_over_env_all_resolve_as_specified() {
    let provider = MockProvider::serving(GOOD_RESULT);
    // (flag args, env invoked-by, parks?, cumulative provider hits)
    let cases: [(&[&str], Option<&str>, bool, usize); 4] = [
        // auto + VIBE_INVOKED_BY → agent.
        (&[], Some("claude-code"), true, 0),
        // auto with nothing hosting the process → cli, and it pays.
        (&[], None, false, 1),
        // explicit cli WINS over the env that would have inferred agent.
        (&["--agent-mode", "cli"], Some("claude-code"), false, 2),
        // explicit agent parks with no env at all.
        (&["--agent-mode", "agent"], None, true, 2),
    ];
    for (flags, env, parks, hits) in cases {
        let (user, dir) = project(ONE_ROW);
        configure_provider(&user, &provider.endpoint());
        let mut command = user.vibe();
        command
            .args(["create", "--json", "--assume-yes"])
            .args(flags)
            .arg("--path")
            .arg(dir.path());
        match env {
            Some(value) => {
                command.env("VIBE_INVOKED_BY", value);
            }
            None => {
                command.env_remove("VIBE_INVOKED_BY");
            }
        }
        let output = command.output().unwrap();
        assert_ok(&output);
        assert_eq!(
            report(&output.stdout).delegation.is_some(),
            parks,
            "flags {flags:?} with env {env:?}",
        );
        assert_eq!(provider.hits(), hits, "flags {flags:?} with env {env:?}");
    }
}

/// A parked row whose DECLARATION is removed must not strand the run.
///
/// Same-id adoption keeps delegated rows — that is how a resume finds its own
/// work — but the current plan will never visit a key whose declaration is
/// gone. The chosen policy is cancellation by exact state-owned cleanup, and
/// it is REPORTED: a run that silently completed over a live delegated row is
/// the failure this guards.
#[test]
fn a_removed_declaration_cancels_its_park_instead_of_stranding_the_run() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(ONE_ROW);
    configure_provider(&user, &provider.endpoint());

    let parked = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    })
    .delegation
    .expect("the row parks");
    assert!(project.path().join(&parked.tasks[0]).is_file());

    // The operator deletes the agent contribution, keeping the sentinel.
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let cut = manifest
        .find(
            "[[extension]]
id = \"produce-docs\"",
        )
        .unwrap();
    let keep_from = manifest
        .find(
            "[[extension]]
id = \"after-agent\"",
        )
        .unwrap();
    let patched = format!("{}{}", &manifest[..cut], &manifest[keep_from..]);
    fs::write(&manifest_path, patched).unwrap();

    let output = hosted_json(&user, project.path(), &[]);
    assert_ok(&output);
    let report = report(&output.stdout);
    assert!(
        report.delegation.is_none(),
        "nothing is owed once the declaration is gone: {report:?}",
    );
    assert!(
        report
            .contributions
            .iter()
            .any(|row| row.status == "cancelled" && row.key.ends_with("produce-docs")),
        "and the cancellation is REPORTED, never swallowed: {:?}",
        report.contributions,
    );
    assert!(
        report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent") && row.status == "ok"),
        "the run continues past the cancelled row: {:?}",
        report.contributions,
    );

    let state = state(project.path());
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no live delegated row survives a completed run: {state:?}",
    );
    assert!(
        !project.path().join(&parked.tasks[0]).exists(),
        "the exact state-owned task was removed",
    );
    assert!(
        !project
            .path()
            .join(".vibe/agentic/outbox")
            .join(&parked.run_id)
            .exists(),
        "and only its proven-empty run directory was pruned",
    );
    assert_eq!(provider.hits(), 0);
}

/// A changed prompt changes the fingerprint: the old task is not treated as
/// satisfied, and the row reparks under the same run.
#[test]
fn a_changed_prompt_reparks_rather_than_accepting_the_old_outputs() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(ONE_ROW);
    configure_provider(&user, &provider.endpoint());

    let first = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    })
    .delegation
    .unwrap();
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "hosted body\n").unwrap();

    // The prompt document the address resolves to changes.
    fs::write(
        project
            .path()
            .join("vibevm/vibespecs/common/agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite them DIFFERENTLY now.\n",
    )
    .unwrap();

    let reparked = report(&{
        let output = hosted_json(&user, project.path(), &[]);
        assert_ok(&output);
        output.stdout
    })
    .delegation
    .expect("a changed fingerprint may not accept the old outputs");
    assert_eq!(reparked.run_id, first.run_id, "the identity is unchanged");
    assert_eq!(
        reparked.tasks, first.tasks,
        "the deterministic task path is rewritten, not multiplied",
    );
    assert!(
        fs::read_to_string(project.path().join(&reparked.tasks[0]))
            .unwrap()
            .contains("DIFFERENTLY"),
        "the republished task carries the new prompt bytes",
    );
    assert_eq!(provider.hits(), 0);
}
