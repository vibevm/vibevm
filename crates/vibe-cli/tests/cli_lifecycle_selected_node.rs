//! Selected-node ownership of a parked run, through the real `vibe` binary
//! (R7.4 §2.3 / §9 rows 1–3).
//!
//! Two members of one workspace can present the IDENTICAL requested
//! phase/chain tuple; before A6 the second member could adopt the first
//! member's parked run id and look for its task under the wrong root. The
//! four-phase oracle below pins the whole ownership law end to end: the park
//! is authored by a NODE (state at the workspace root names it, the task
//! lives under that node's own root), a sibling's identical command — with or
//! without `--force` — is the typed ownership refusal before any mutation,
//! and the owning member's satisfied resume adopts the exact run and
//! completes it.
//!
//! The loopback provider is configured and reachable in every phase, and
//! every invocation must still keep its hit count at zero: "the hosted branch
//! and the refusals before it never construct a provider" is a measurement,
//! not a claim. The entry-alias law is deliberately NOT re-tested here —
//! `vibe-workspace` pins `node_rel_of`'s canonicalisation at unit cost.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::documents;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

const GOOD_RESULT: &str = r#"{"outputs":[{"path":"docs/guide.md","content":"paid\n"}]}"#;

/// One agent row plus a builtin sentinel AFTER it in the same phase. The
/// sentinel is the stop-detector on the park and the completion-detector on
/// the owning member's resume.
fn member_manifest(name: &str) -> String {
    format!(
        "[package]\ngroup = \"org.demo\"\nname = \"{name}\"\nkind = \"flow\"\n\
         version = \"0.1.0\"\npublish = false\n\
         \n[[extension]]\nid = \"produce-docs\"\npoint = \"phase:create\"\n\
         handler = {{ kind = \"agent\", \
         prompt = \"spec://org.demo/{name}/common/agent-prompt#root\" }}\n\
         config.outputs = [\n  \
         {{ path = \"docs/guide.md\", kind = \"file\", accept = \"non-empty file\" }},\n]\n\
         \n[[extension]]\nid = \"after-agent\"\npoint = \"phase:create\"\n\
         handler = {{ kind = \"builtin\", name = \"log\" }}\n\
         config = {{ message = \"SENTINEL-AFTER-AGENT\" }}\n",
    )
}

/// The member's own prompt document, carrying one distinguishing marker: the
/// address resolves from THAT member's real specs, never a sibling's or the
/// root's.
fn seed_prompt(member: &Path, marker: &str) {
    let specs = member.join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        format!("# Documentation prompt {{#root}}\n\nWrite the guide. MARKER={marker}\n"),
    )
    .unwrap();
}

/// One truthful workspace: an init'd root explicitly declaring BOTH members,
/// each member a valid selected-node manifest with the same phase/create
/// hosted-agent shape and its own real prompt document.
fn workspace(user: &UserScratch) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Agent"])
        .arg("--path")
        .arg(dir.path())
        .assert()
        .success();
    let root_manifest = dir.path().join("vibe.toml");
    let mut body = fs::read_to_string(&root_manifest).unwrap();
    body.push_str("\n[workspace]\nmembers = [\"members/a\", \"members/b\"]\n");
    fs::write(&root_manifest, body).unwrap();

    let mut members = Vec::new();
    for (name, marker) in [("a", "ALPHA"), ("b", "BETA")] {
        let member = dir.path().join("members").join(name);
        fs::create_dir_all(&member).unwrap();
        fs::write(member.join("vibe.toml"), member_manifest(name)).unwrap();
        seed_prompt(&member, marker);
        members.push(member);
    }
    let member_b = members.pop().expect("the last member");
    let member_a = members.pop().expect("the first member");
    (dir, member_a, member_b)
}

/// The hosted invocation every phase below drives: JSON, assume-yes, hosted
/// agent mode, at one member of the workspace.
fn create_at(user: &UserScratch, member: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = user.vibe();
    command.args(["create", "--json", "--assume-yes", "--agent-mode", "agent"]);
    command.args(extra);
    command.arg("--path").arg(member);
    command.output().unwrap()
}

fn report(bytes: &[u8]) -> LifecycleReport {
    let documents = documents(bytes);
    serde_json::from_value(documents.last().expect("one report document").clone()).unwrap()
}

fn decoded_state(state_path: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(state_path).unwrap()).unwrap()
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

/// The typed ownership refusal as it must reach an operator: non-zero exit,
/// and stderr naming the owning node, the requesting node, the exact parked
/// run and the force/remedy law.
fn assert_foreign_park(output: &std::process::Output, run_id: &str) {
    assert!(
        !output.status.success(),
        "a foreign park is a non-zero exit:\n{}",
        stdout(output),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for needle in [
        "owned by workspace node `members/a`",
        "runs from node `members/b`",
        run_id,
        "not even under --force",
        "never force through it",
        "PROP-054",
    ] {
        assert!(
            stderr.contains(needle),
            "the refusal names `{needle}`:\n{stderr}",
        );
    }
}

/// The whole ownership law, one workspace, four phases: park at A; the
/// sibling's identical command refuses typed (without and with `--force`)
/// having mutated nothing anywhere; the owning member's satisfied resume
/// adopts the exact run and completes it — and the provider is never
/// constructed by any of it.
#[test]
fn a_park_belongs_to_the_node_that_created_it_end_to_end() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let user = UserScratch::new();
    configure_provider(&user, &provider.endpoint());
    let (workspace, member_a, member_b) = workspace(&user);

    // --- 1. `vibe create` at member A parks, authored by A ----------------
    let parked = create_at(&user, &member_a, &[]);
    assert_ok(&parked);
    assert_eq!(provider.hits(), 0, "parking never constructs a provider");
    let parked_report = report(&parked.stdout);
    let handoff = parked_report
        .delegation
        .as_ref()
        .expect("one typed handoff");
    assert_eq!(handoff.resume, "vibe create");
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();
    assert!(
        task.starts_with(&format!(".vibe/agentic/outbox/{run_id}/")),
        "the reported task is the deterministic member-relative path: {task}",
    );
    assert!(
        !parked_report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent")),
        "the park stops the chain before the sentinel: {:?}",
        parked_report.contributions,
    );

    // State is WORKSPACE-global and names its author; the task is A's own.
    let state_path = workspace.path().join(".vibe/lifecycle.toml");
    let state = decoded_state(&state_path);
    assert_eq!(state.run.selected.as_deref(), Some("members/a"));
    assert_eq!(state.run.run_id.as_deref(), Some(run_id.as_str()));
    let task_path = member_a.join(&task);
    assert!(task_path.is_file(), "the task is published under A's root");
    assert!(
        !workspace.path().join(".vibe/agentic").exists(),
        "never under the workspace root's outbox",
    );
    assert!(!member_b.join(".vibe").exists(), "and never under member B");

    let state_bytes = fs::read(&state_path).unwrap();
    let task_bytes = fs::read(&task_path).unwrap();

    // --- 2/3. the sibling's IDENTICAL command, without and with --force ---
    for label in ["unforced", "forced"] {
        let extra: &[&str] = if label == "forced" {
            ["--force"].as_slice()
        } else {
            [].as_slice()
        };
        let refused = create_at(&user, &member_b, extra);
        assert_foreign_park(&refused, &run_id);
        assert_eq!(provider.hits(), 0, "{label}: a refusal spends nothing");
        assert!(
            !member_b.join(".vibe").exists(),
            "{label}: the sibling's tree stays untouched — no state, scratch or outbox",
        );
        assert_eq!(
            fs::read(&state_path).unwrap(),
            state_bytes,
            "{label}: the workspace state is byte-identical",
        );
        assert_eq!(
            fs::read(&task_path).unwrap(),
            task_bytes,
            "{label}: A's owned task bytes are byte-identical",
        );
    }

    // --- 4. the owning member satisfies its declared output and resumes ---
    fs::create_dir_all(member_a.join("docs")).unwrap();
    fs::write(member_a.join("docs/guide.md"), "hosted body\n").unwrap();

    let resumed = create_at(&user, &member_a, &[]);
    assert_ok(&resumed);
    assert_eq!(
        provider.hits(),
        0,
        "four hosted invocations, zero provider constructions",
    );
    let report = report(&resumed.stdout);
    assert!(
        report.delegation.is_none(),
        "the satisfied resume owes nothing"
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
        "the post-agent sentinel the park had stopped now runs: {:?}",
        report.contributions,
    );
    assert_eq!(report.steps.last().expect("steps").status, "ok");

    assert!(
        !task_path.exists(),
        "only A's owned task is removed — nothing was touched anywhere else",
    );
    assert!(
        !member_a.join(".vibe/agentic/outbox").join(&run_id).exists(),
        "its proven-empty run directory is pruned",
    );
    assert_eq!(
        fs::read_to_string(member_a.join("docs/guide.md")).unwrap(),
        "hosted body\n",
        "the hosting agent's bytes are never rewritten",
    );

    // The durable header: the SAME run, still owned by A.
    let state = decoded_state(&state_path);
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(run_id.as_str()),
        "the resume adopted the exact parked run",
    );
    assert_eq!(state.run.selected.as_deref(), Some("members/a"));
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no park survives its own satisfied resume",
    );
}
