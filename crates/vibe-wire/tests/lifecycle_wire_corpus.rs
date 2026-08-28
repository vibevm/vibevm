//! Authored golden documents for every parser-visible R2.4 lifecycle root.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::lifecycle::e1::context::{Context, RunAgentMode};
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};
use vibe_wire::generated::lifecycle_plan::LifecyclePlan;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/lifecycle/e1")
}

fn read<T: DeserializeOwned + Serialize>(name: &str) -> T {
    let bytes = std::fs::read(corpus().join(name)).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let value: T = serde_json::from_value(authored.clone()).unwrap();
    let round_trip = serde_json::to_value(&value).unwrap();
    assert_eq!(
        round_trip, authored,
        "{name} loses data on generated round-trip"
    );
    serde_json::from_value(round_trip).unwrap()
}

fn absolute_machine_path(path: &str) {
    assert!(!path.contains('\\'), "path is not machine JSON: {path}");
    let bytes = path.as_bytes();
    let absolute = path.starts_with('/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'/');
    assert!(absolute, "path is not absolute machine JSON: {path}");
}

#[test]
fn context_corpus_uses_epoch_one_closed_agent_mode_and_absolute_paths() {
    let context: Context = read("context.json");
    assert_eq!(context.envelope, 1);
    assert_eq!(context.run.agent_mode, RunAgentMode::Cli);
    for path in [
        &context.project.root,
        &context.project.manifest,
        &context.project.spec_roots[0],
        &context.world.lockfile,
        &context.world.deps_root,
        &context.world.packages[0].slot,
        &context.slot_target.as_ref().unwrap().root,
        &context.io.scratch,
    ] {
        absolute_machine_path(path);
    }
    let slot_target = context.slot_target.as_ref().unwrap();
    assert_eq!(slot_target.group, "org.demo");
    assert_eq!(slot_target.name, "target");
    assert_eq!(slot_target.version, "2.0.0");
    assert_eq!(slot_target.kind, "tool");
}

#[test]
fn context_rejects_incomplete_slot_target() {
    let mut context: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus().join("context.json")).unwrap()).unwrap();
    context["slot_target"]
        .as_object_mut()
        .unwrap()
        .remove("root");
    assert!(serde_json::from_value::<Context>(context).is_err());
}

#[test]
fn reply_corpus_uses_epoch_one_and_the_closed_ok_status() {
    let reply: Reply = read("reply.json");
    assert_eq!(reply.envelope, 1);
    assert_eq!(reply.status, ReplyStatus::Ok);
    assert!(reply.artifacts.is_empty());
    assert!(reply.tasks.is_empty());
}

#[test]
fn reply_rejects_unknown_fields_at_every_object_boundary() {
    let mut reply: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus().join("reply.json")).unwrap()).unwrap();
    reply["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Reply>(reply).is_err());

    let mut nested: serde_json::Value = serde_json::json!({
        "envelope": 1,
        "status": "ok",
        "artifacts": [{
            "id": "guide",
            "path": "docs/guide.md",
            "kind": "text",
            "unknown": true
        }],
        "tasks": []
    });
    assert!(serde_json::from_value::<Reply>(nested.clone()).is_err());
    nested["artifacts"][0]
        .as_object_mut()
        .unwrap()
        .remove("unknown");
    assert!(serde_json::from_value::<Reply>(nested).is_ok());
}

#[test]
fn report_delegation_is_one_typed_member_and_steps_end_at_the_parked_phase() {
    let report: LifecycleReport = read("report_parked.json");
    let delegation = report.delegation.as_ref().unwrap();
    assert_eq!(delegation.resume, "vibe create");
    assert_eq!(delegation.tasks.len(), 1);
    assert_eq!(report.steps.last().unwrap().status, "delegated");
    assert_eq!(
        report.contributions[0].status, "delegated",
        "the parked contribution reports the delegated status itself"
    );
}

#[test]
fn failed_report_corpus_has_one_fail_step_and_keeps_the_executed_contribution_prefix() {
    let report: LifecycleReport = read("report_failed.json");
    assert!(!report.ok);
    assert_eq!(report.steps.len(), 1);
    assert_eq!(report.steps[0].phase, "build");
    assert_eq!(report.steps[0].status, "fail");
    assert_eq!(
        report
            .contributions
            .iter()
            .map(|row| (row.key.as_str(), row.status.as_str()))
            .collect::<Vec<_>>(),
        [
            ("org.demo/project#first", "ok"),
            ("org.demo/project#stop", "fail")
        ]
    );
    assert!(report.delegation.is_none());
}

#[test]
fn plan_is_selection_only_and_precedes_the_distinct_outcome_shape() {
    let plan: LifecyclePlan = read("plan.json");
    let report: LifecycleReport = read("report.json");
    assert_eq!(plan.command, "lifecycle:plan");
    assert_eq!(report.command, "lifecycle");
    assert_eq!(plan.contributions.len(), report.contributions.len());
    assert_eq!(plan.contributions[0].key, report.contributions[0].key);
    assert_eq!(report.contributions[0].status, "ok");
    assert_eq!(
        plan.contributions[0].reference.as_deref(),
        Some("hooks.post-install")
    );
    assert_eq!(
        report.contributions[0].reference,
        plan.contributions[0].reference
    );
    assert_eq!(report.contributions[0].flagged, Some(false));
    let planned_target = plan.contributions[0].slot_target.as_ref().unwrap();
    let reported_target = report.contributions[0].slot_target.as_ref().unwrap();
    assert_eq!(planned_target.name, "target");
    assert_eq!(reported_target.name, planned_target.name);
    assert_eq!(reported_target.root, planned_target.root);
    assert!(report.contributions[0].message.is_some());
    assert_eq!(
        report.contributions[0].stdout.as_deref(),
        Some("hello from build in demo\n")
    );
    assert_eq!(
        report.contributions[0].stderr.as_deref(),
        Some("warning: demo diagnostic\n")
    );
    assert_eq!(report.contributions[0].stdout_truncated, Some(false));
    assert_eq!(report.contributions[0].stderr_truncated, Some(false));
}

#[test]
fn state_corpus_round_trips_generated_semantics_and_exact_build_chain() {
    let state: LifecycleState = read("state.json");
    assert_eq!(state.schema, 1);
    assert_eq!(
        state.run.chain,
        ["validate", "install", "generate", "build", "test", "create"]
    );
    assert_eq!(state.run.requested, "create");
    assert_eq!(state.run.started, "2026-08-25T12:00:00Z");
    assert_eq!(
        state.run.run_id.as_deref(),
        Some("00112233445566778899aabbccddeeff"),
    );
    assert_eq!(
        state.run.selected.as_deref(),
        Some("members/tool"),
        "a delegated corpus is authored by a member node in the new epoch",
    );
    let row = &state.execution["org.demo/provider#announce"];
    assert_eq!(row.status, ExecutionRecordStatus::Ok);
    assert_eq!(row.duration_ms, 12);
    assert_eq!(row.artifacts[0].kind, "text");
    assert!(row.tasks.is_empty(), "an ok row carries no task files");
}

/// A delegated row is the one legal carrier of `tasks`, and the run header
/// answers with the identity those tasks live under. The round-trip above
/// proves the authored spelling survives; this pins the R7.3 additions on
/// their own terms — including that the corpus is COHERENT (the parked row's
/// phase is one the run header's requested chain actually contains, and its
/// task lives under the header's own run id), and that a pre-R7.3 file (no
/// `run_id`, no `tasks` anywhere) still parses through the strict reader.
/// The legacy file is idle (no delegated row), so it is also the pre-A6
/// compatibility proof: it carries no `selected` and reads it as `None`. A
/// delegated legacy state is deliberately NOT blessed here — A6's semantic
/// validator owns refusing that combination.
#[test]
fn state_carries_delegated_tasks_under_a_run_id_and_still_reads_pre_r73_files() {
    let state: LifecycleState = read("state.json");
    let row = &state.execution["org.demo/provider#draft-guide"];
    assert_eq!(row.status, ExecutionRecordStatus::Delegated);
    assert_eq!(row.tasks.len(), 1);
    assert!(
        state.run.chain.contains(&row.phase),
        "the parked row's phase must be part of the run this state records",
    );
    let run_id = state.run.run_id.as_deref().unwrap();
    assert_eq!(
        row.tasks[0],
        format!(".vibe/agentic/outbox/{run_id}/task-org.demo%2Fprovider%23draft-guide.md"),
        "the recorded task is the deterministic path for this run and execution key",
    );

    let pre_r73 = serde_json::json!({
        "schema": 1,
        "run": {
            "requested": "build",
            "chain": ["validate", "install", "build"],
            "started": "2026-08-20T09:00:00Z"
        },
        "execution": {
            "org.demo/provider#announce": {
                "phase": "build",
                "fingerprint": "sha256:0123456789abcdef",
                "status": "ok",
                "duration_ms": 12,
                "artifacts": []
            }
        }
    });
    let legacy: LifecycleState = serde_json::from_value(pre_r73).unwrap();
    assert_eq!(legacy.run.run_id, None);
    assert_eq!(
        legacy.run.selected, None,
        "a legacy idle state parses without `selected` and yields None",
    );
    assert!(legacy.execution.values().all(|row| row.tasks.is_empty()));
}
