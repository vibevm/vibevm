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
        &context.io.scratch,
    ] {
        absolute_machine_path(path);
    }
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
fn plan_is_selection_only_and_precedes_the_distinct_outcome_shape() {
    let plan: LifecyclePlan = read("plan.json");
    let report: LifecycleReport = read("report.json");
    assert_eq!(plan.command, "lifecycle:plan");
    assert_eq!(report.command, "lifecycle");
    assert_eq!(plan.contributions.len(), report.contributions.len());
    assert_eq!(plan.contributions[0].key, report.contributions[0].key);
    assert_eq!(report.contributions[0].status, "ok");
    assert!(report.contributions[0].message.is_some());
}

#[test]
fn state_corpus_round_trips_generated_semantics_and_exact_build_chain() {
    let state: LifecycleState = read("state.json");
    assert_eq!(state.schema, 1);
    assert_eq!(
        state.run.chain,
        ["validate", "install", "generate", "build"]
    );
    assert_eq!(state.run.started, "2026-08-25T12:00:00Z");
    let row = &state.execution["org.demo/provider#announce"];
    assert_eq!(row.status, ExecutionRecordStatus::Ok);
    assert_eq!(row.duration_ms, 12);
    assert_eq!(row.artifacts[0].kind, "text");
}
