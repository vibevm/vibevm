//! Observed hosted lifecycle: a park cannot erase a checked metadata conflict.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO");

use super::*;
use std::sync::Arc;
use vibe_install::{
    SlotLifecycleObserver, SlotLifecyclePlan, SlotLifecycleReport, SlotLifecycleSeams,
};
use vibe_lifecycle::agent::{
    AgentBackend, AgentCompletion, AgentRequest, PromptRequest, ResolvedPrompt,
};
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleLease, LifecycleStateStore, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

struct PromptOnly;
impl AgentBackend for PromptOnly {
    fn resolve_prompt(&self, _: &PromptRequest) -> Result<ResolvedPrompt, String> {
        Ok(ResolvedPrompt {
            text: "Synthetic hosted task".into(),
            unsupported: vec![],
        })
    }
    fn complete(&self, _: &AgentRequest) -> Result<AgentCompletion, String> {
        panic!("hosted mode must never spawn provider work")
    }
}

struct Observer {
    path: PathBuf,
    change: bool,
}
impl SlotLifecycleObserver for Observer {
    fn observe(&self, _: &SlotLifecyclePlan) -> Result<(), String> {
        Ok(())
    }
    fn outcome(&self, report: &SlotLifecycleReport) -> Result<(), String> {
        if self.change && report.status == "delegated" {
            let published = fs::read_to_string(&self.path).unwrap();
            assert_ne!(
                published, BEFORE,
                "selected manifest must be published before park observation"
            );
            assert!(published.contains("org.vibevm/giant"));
            fs::write(
                &self.path,
                format!("{published}\n# OTHER_ACTOR_DURING_PARK\n"),
            )
            .unwrap();
        }
        Ok(())
    }
}

fn parked_apply(change: bool) {
    let (temp, mut source, selected) = super::recording::fixture(false);
    source.add_declaration("\n[[extension]]\nid='hosted'\npoint='slot:pre-install'\nhandler={kind='agent',prompt='spec://org.vibevm/giant/common/prompt#root'}\nconfig.outputs=[{path='docs/result.md',kind='file',accept='non-empty file'}]\n");
    let planned = super::recording::plan(&source, &selected);
    let lease = Arc::new(LifecycleLease::acquire(temp.path()).unwrap());
    let run_id = vibe_lifecycle::process::allocate_run_id(temp.path()).unwrap();
    let metadata = RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Agent,
        force: false,
        trace_compile: false,
        run_id: run_id.clone(),
        started: "2026-11-04T00:00:00Z".into(),
        selected: ".".into(),
    };
    let result = vibe_install::apply_with_spec_format_and_lifecycle_observed_traced_prepared(
        &source,
        *planned,
        SlotIntegrity::Verify,
        vibe_core::manifest::SpecFormat::Mixed,
        metadata,
        StreamMode::Capture,
        SlotLifecycleSeams {
            observer: Arc::new(Observer {
                path: selected.join("vibe.toml"),
                change,
            }),
            agent: Arc::new(PromptOnly),
            judgment: vibe_install::SlotJudgmentContext::consenting(),
        },
        lease.clone(),
        None,
    );
    let error = match result {
        Ok(_) => panic!("hosted row must stop apply"),
        Err(error) => error,
    };
    let state = LifecycleStateStore::peek_with_lease(&lease)
        .unwrap()
        .unwrap();
    let delegated: Vec<_> = state
        .execution
        .values()
        .filter(|row| row.status == ExecutionRecordStatus::Delegated)
        .collect();
    assert_eq!(delegated.len(), 1, "delegation state remains inspectable");
    assert!(state.run.slot_continuation.is_some());
    let task_dir = temp
        .path()
        .join(vibe_lifecycle::OUTBOX_RELATIVE)
        .join(&run_id);
    assert!(
        fs::read_dir(task_dir)
            .unwrap()
            .any(|entry| entry.unwrap().path().is_file())
    );
    println!(
        "PARKING_OBSERVATION {}",
        json!({"changed":change,"error":error.to_string(),"delegated_rows":delegated.len(),"run_id":run_id})
    );
    if change {
        assert!(
            fs::read_to_string(selected.join("vibe.toml"))
                .unwrap()
                .contains("OTHER_ACTOR_DURING_PARK")
        );
        let vibe_install::Error::SlotFailed {
            source, reports, ..
        } = error
        else {
            panic!("rollback conflict must remain a failure, got {error}");
        };
        assert!(matches!(
            *source,
            vibe_install::Error::RollbackFailed { .. }
        ));
        assert!(reports.iter().any(|r| r.status == "delegated"));
    } else {
        let vibe_install::Error::Delegated {
            delegation,
            reports,
            ..
        } = error
        else {
            panic!("normal park must remain delegated: {error}");
        };
        assert_eq!(delegation.run_id, run_id);
        assert!(!delegation.tasks.is_empty());
        assert!(reports.iter().any(|r| r.status == "delegated"));
        assert_eq!(
            fs::read_to_string(selected.join("vibe.toml")).unwrap(),
            BEFORE
        );
    }
}

#[test]
fn normal_hosted_park_remains_delegated_with_retained_task_state() {
    parked_apply(false);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn metadata_rollback_conflict_is_not_erased_by_a_hosted_park() {
    parked_apply(true);
}
