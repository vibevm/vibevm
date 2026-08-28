//! The lifecycle owner's mapping of a NEUTRAL resume failure.
//!
//! The prerequisite install transports the measurement without naming a family,
//! because the same `execute_prepared` is also `vibe install`'s body and
//! `vibe update --all`'s delegate. Here the family is the lifecycle one, and
//! this command already has a mechanism for choosing it — the fallback in
//! `execute_after_open` — so the only thing missing is the rows.
//!
//! This drives the production helper and then the SAME fallback/classifier
//! shape the boundary uses, so the two halves cannot agree in isolation and
//! disagree in production.

use super::*;
use crate::failure::{MeasuredFailure, carry};

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

fn slot_row(key: &str, status: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: key.into(),
        point: "slot:post-install".into(),
        handler: "builtin".into(),
        provider: "org.demo/tools".into(),
        tier: "dependency".into(),
        status: status.into(),
        message: None,
        version: None,
        reference: "spec://org.demo/tools".into(),
        flagged: false,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
        slot_target: None,
    }
}

fn phase_row(key: &str) -> LifecycleContributionReport {
    LifecycleContributionReport {
        flagged: None,
        handler: "builtin".into(),
        key: key.into(),
        message: None,
        stderr: None,
        stderr_truncated: None,
        stdout: None,
        stdout_truncated: None,
        phase: "clean".into(),
        point: "phase:clean".into(),
        provider: "org.demo/tools".into(),
        reference: None,
        slot_target: None,
        status: "ok".into(),
        tier: "dependency".into(),
        version: None,
    }
}

/// A resume failure from the prerequisite install joins THIS command's
/// accumulator in chronology, and the boundary then reports one lifecycle root.
///
/// Every half of this is a separate way to lose the truth: dropping the helper
/// call empties the resumed rows; putting them in front loses the clean epoch's
/// chronology; carrying a root here would emit an install-shaped document where
/// a phase verb has always emitted a lifecycle-shaped one; and letting the
/// neutral wrapper escape hands `main` an error whose downcast is not the
/// command's.
#[test]
fn a_neutral_resume_failure_joins_the_prefix_and_reports_one_lifecycle_root() {
    // Whatever this command had already measured before the prerequisite ran.
    let mut measured = Measured {
        contributions: vec![phase_row("clean:earlier")],
    };
    let transported = carry(MeasuredFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        evidence: Measurement::Slot {
            progress: Box::new(vibe_install::InstallProgress::fresh(vec![".".into()])),
            reports: vec![
                slot_row("resumed:ok", "ok"),
                slot_row("resumed:fail", "fail"),
            ],
            packages_resolved: 4,
        },
        emit_machine_failure: false,
    });

    let error = absorb_resume_failure(transported, &mut measured);

    assert!(
        error.downcast_ref::<Sentinel>().is_some(),
        "the ORIGINAL object reaches the boundary",
    );
    assert!(
        !crate::failure::is_carried::<crate::failure::Measurement>(&error),
        "and the neutral wrapper does not escape",
    );
    assert_eq!(
        measured
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["clean:earlier", "resumed:ok", "resumed:fail"],
        "prefix first, then the resumed rows, in order",
    );

    // The SAME fallback shape `run_phases` builds for an uncarried error.
    assert!(
        original_is(&error),
        "the original object survives classification"
    );
    assert_eq!(
        format!("{error:#}"),
        "finishing the parked slot run: the resumed row refused",
    );
    let values = LifecycleValues::failed(
        "build",
        vec!["validate".into(), "install".into(), "build".into()],
        "build",
        measured.contributions.clone(),
    );
    assert!(!values.ok);
    assert_eq!(
        values
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["clean:earlier", "resumed:ok", "resumed:fail"],
        "exactly one lifecycle value set, carrying both passes in order",
    );
}

fn original_is(error: &anyhow::Error) -> bool {
    error.downcast_ref::<Sentinel>().is_some()
}

/// An error that is not a transported resume failure is returned exactly as it
/// arrived, and the accumulator is untouched.
#[test]
fn an_ordinary_error_passes_through_and_adds_no_rows() {
    let mut measured = Measured {
        contributions: vec![phase_row("clean:earlier")],
    };
    let error = absorb_resume_failure(anyhow::anyhow!("planning blew up"), &mut measured);
    assert_eq!(error.to_string(), "planning blew up");
    assert_eq!(measured.contributions.len(), 1);
}

/// A VALIDATE-ONLY chain proves lease/root/selected before it does state work.
///
/// This is the hole the install core's own gate could never cover: a
/// validate-only chain has no install phase, so it never enters
/// `execute_prepared` at all — and it still plans the world, surfaces the plan
/// and begins a tracked run whose state store and outbox are rooted at the
/// LEASE. Before the gate, a bundle naming one node and a `metadata.selected`
/// naming another would validate happily and then mint state under the wrong
/// identity.
///
/// The mutation this kills is deleting `ensure_selected` (or `ensure_root`)
/// from the top of the executed region: without it the run proceeds and returns
/// `Completed`, so the refusal below never happens.
mod validate_only_gate {
    use std::sync::Arc;

    use vibe_lifecycle::process::StreamMode;
    use vibe_lifecycle::{AgentBackend, Phase, RunMetadata};
    use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
    use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

    use crate::install::{InstallInputs, InstallPolicy, SelectedManifest, resolve_project_root};
    use crate::ports::{
        ConfirmGate, InstallNarration, InstallObserver, NoManifestMutation, PackageSource,
        PackageSourceBuild, PackageSourceFactory, RegistryEnvironment, RegistryEnvironmentSnapshot,
        RunObserver,
    };
    use crate::{PhaseOutcome, PhaseRun, RitualPlan, run_phases};

    struct Silent;

    impl RunObserver for Silent {
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

    impl InstallObserver for Silent {
        fn stream_mode(&self) -> StreamMode {
            StreamMode::Null
        }
        fn emit_machine_failure(&self) -> bool {
            false
        }
        fn narrate(&self, _event: InstallNarration<'_>) {}
        fn lane_sizes(&self, _root: &std::path::Path) -> Vec<(String, Option<u64>)> {
            Vec::new()
        }
        fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
            &NoPlanEvents
        }
        fn slot_observer(
            &self,
            _metadata: &RunMetadata,
        ) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
            Arc::new(NoSlotEvents)
        }
    }

    struct NoPlanEvents;
    impl vibe_install::PlanObserver for NoPlanEvents {
        fn on(&self, _event: vibe_install::PlanEvent) {}
    }

    struct NoSlotEvents;
    impl vibe_install::SlotLifecycleObserver for NoSlotEvents {
        fn observe(&self, _plan: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
            Ok(())
        }
        fn outcome(&self, _report: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
            Ok(())
        }
    }

    impl ConfirmGate for Silent {
        fn confirm_install(&self, _packages: usize) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// Both REFUSE: a validate-only chain must reach neither.
    impl RegistryEnvironment for Silent {
        fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
            anyhow::bail!("the registry environment was prepared");
        }
    }

    impl PackageSourceFactory for Silent {
        fn build(&self, _input: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
            anyhow::bail!("the package source was constructed");
        }
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn a_wrong_selected_node_refuses_a_validate_only_chain_before_any_state_work() {
        let dir = project();
        let root = resolve_project_root(dir.path()).unwrap();
        let lease =
            Arc::new(vibe_lifecycle::LifecycleLease::acquire(&root).expect("the fixture leases"));
        let selection = SelectedManifest::read(&root).prepare();
        let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);
        let metadata = RunMetadata {
            requested: "validate".to_string(),
            chain: vec!["validate".to_string()],
            offline: true,
            assume_yes: true,
            agent_mode: RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id: vibe_lifecycle::process::allocate_run_id(&root).unwrap(),
            started: vibe_core::timestamp::now_utc(),
            // The tree really maps this root to `"."`.
            selected: "members/other".to_string(),
        };
        let silent = Silent;

        let outcome = run_phases(PhaseRun {
            requested: Phase::Validate,
            phases: vec![Phase::Validate],
            chain: vec!["validate".to_string()],
            metadata,
            install_args: InstallInputs::default(),
            policy: InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            lease,
            selection,
            steps: Vec::new(),
            contributions: Vec::new(),
            notices: Vec::new(),
            observer: &silent,
            install_observer: &silent,
            confirm_gate: &silent,
            sources: &silent,
            environment: &silent,
            manifest_mutation: &NoManifestMutation,
            agent,
            trace: None,
        });

        let PhaseOutcome::Failed { original, .. } = outcome else {
            panic!("a selected-node mismatch can never validate, let alone write state");
        };
        let rendered = format!("{original:#}");
        assert!(
            original
                .downcast_ref::<vibe_lifecycle::LifecycleLeaseError>()
                .is_some(),
            "the refusal is the lease's own typed error: {rendered}",
        );
        assert!(
            rendered.contains("at phase execution"),
            "and it names the boundary it fired at: {rendered}",
        );
        // Nothing durable: no run directory was allocated under the state root
        // by THIS invocation's execution, because it never reached `begin`.
        assert!(
            !rendered.contains("the registry environment was prepared"),
            "and it fired before the registry epoch: {rendered}",
        );
    }
}
