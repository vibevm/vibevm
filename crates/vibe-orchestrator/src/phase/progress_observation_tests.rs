//! Phase-fence observation over empty engine-owned phases.

use vibe_core::manifest::TargetOs;
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{Phase, RunMetadata};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use super::mechanism_wiring::{local_profile, run_deploying_on_observed};
use super::validate_only_gate::manifested;
use crate::ports::RunObserver;
use crate::{PhaseOutcome, RitualPlan};

#[derive(Default)]
struct FenceObserver {
    events: std::sync::Mutex<Vec<(String, String)>>,
}

impl FenceObserver {
    fn events(&self) -> Vec<(String, String)> {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl RunObserver for FenceObserver {
    fn observe_phase_started(&self, phase: &str) {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((phase.into(), "start".into()));
    }

    fn observe_phase_finished(&self, phase: &str, status: &str) {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((phase.into(), status.into()));
    }

    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }

    fn binary_quiet(&self) -> bool {
        true
    }

    fn emit_machine_failure(&self) -> bool {
        false
    }

    fn observe_plan(
        &self,
        _plan: &RitualPlan,
        _metadata: &RunMetadata,
        _emit_empty: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn observe_contribution(&self, _report: &LifecycleContributionReport) {}

    fn observe_untracked_failure(
        &self,
        _metadata: &RunMetadata,
        _phase: &str,
        _contributions: &[LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn empty_engine_phases_are_observed_around_their_fences_once() {
    let dir = manifested("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    let observer = FenceObserver::default();
    let outcome = run_deploying_on_observed(
        dir.path(),
        vec![Phase::Build, Phase::Verify, Phase::Package],
        None,
        TargetOs::Linux,
        &observer,
    );

    let PhaseOutcome::Completed(values) = outcome else {
        panic!("empty engine phases complete");
    };
    assert!(values.ok);
    assert_eq!(
        observer.events(),
        [
            ("build".into(), "start".into()),
            ("build".into(), "no-op".into()),
            ("verify".into(), "start".into()),
            ("verify".into(), "ok".into()),
            ("package".into(), "start".into()),
            ("package".into(), "no-op".into()),
        ],
    );
    assert_eq!(
        values
            .steps
            .iter()
            .map(|step| (step.phase.as_str(), step.status.as_str()))
            .collect::<Vec<_>>(),
        [("build", "no-op"), ("verify", "ok"), ("package", "no-op")],
    );
}

#[test]
fn an_empty_deploy_phase_is_observed_around_its_failing_fence_once() {
    let dir = manifested(concat!(
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
        "[[artifacts.build]]\nid = \"tool\"\nmechanism = \"build:cargo\"\n",
        "outputs = [{ id = \"tool.exe\", kind = \"executable\" }]\n\n",
        "[[deploy.target]]\nid = \"local\"\nartifact = \"tool.exe\"\n",
        "mechanism = \"deploy:vibe-bin\"\n\n",
        "[deploy.profiles.local]\ntargets = [\"local\"]\n",
    ));
    let observer = FenceObserver::default();
    let outcome = run_deploying_on_observed(
        dir.path(),
        vec![Phase::Deploy],
        Some(local_profile()),
        TargetOs::Linux,
        &observer,
    );

    let PhaseOutcome::Failed { original, .. } = outcome else {
        panic!("the deploy fence refuses the missing artifact record");
    };
    assert!(
        format!("{original:#}").contains("has no artifact record for"),
        "the engine-owned deploy fence really ran: {original:#}",
    );
    assert_eq!(
        observer.events(),
        [
            ("deploy".into(), "start".into()),
            ("deploy".into(), "fail".into()),
        ],
    );
}
