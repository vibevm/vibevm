//! Internal, narrowly protected lifecycle submission through the incumbent runner.
//!
//! Only Validate and a one-node empty-world Install are admitted. General
//! package install, providers, members, trace and prior-current-state cases
//! remain unavailable.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use vibe_core::manifest::TargetOs;
use vibe_core::user_config::SlotIntegrity;
use vibe_lifecycle::process::{
    PackageTreeWitness, SelectedManifestLockWitness, capture_package_tree_witness,
    recheck_package_tree_witness, recheck_selected_manifest_lock,
};
use vibe_lifecycle::{
    AgentBackend, ExistingSubmissionInspection, JournalAdmission, LifecycleLease,
    LifecycleOutcomeInspection, LifecycleOutcomeRecoveryReason, LifecycleReportPayload,
    LifecycleRunJournalObservation, LifecycleStateStore, Phase, RunIdentity, RunMetadata,
    SubmissionNamespace, SubmissionReservations, SubmissionScope, create_lifecycle_outcome_marker,
    inspect_reserved_lifecycle_run, inspect_retained_lifecycle_outcome,
    publish_install_entered_stage, retain_lifecycle_outcome, select_run_identity_for_reservation,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarker, LifecycleOutcomeMarkerEffectState as EffectState,
    LifecycleOutcomeMarkerOutcome as MarkerOutcome, ReportEvidenceUnavailableReason,
};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::Timestamp;

use super::lifecycle_plan::{
    LifecyclePreEffectPlanInput, canonical_lifecycle_pre_effect_plan,
    canonical_one_package_fixture_request,
};
use super::{JournalState, SubmissionFailure, SubmissionJournal, SubmissionOutcome, submit};
use crate::RitualPlan;

// The target OS changes lifecycle behavior and belongs in this fixed
// request. `observed_at` is execution evidence: retries retain the first
// report timestamp instead of treating a fresh clock reading as new intent.
const VALIDATE_LINUX: &[u8] =
    b"{\"operation\":\"lifecycle.validate\",\"schema\":1,\"target_os\":\"linux\"}";
const VALIDATE_MACOS: &[u8] =
    b"{\"operation\":\"lifecycle.validate\",\"schema\":1,\"target_os\":\"macos\"}";
const VALIDATE_WINDOWS: &[u8] =
    b"{\"operation\":\"lifecycle.validate\",\"schema\":1,\"target_os\":\"windows\"}";
const EMPTY_INSTALL_LINUX: &[u8] =
    b"{\"operation\":\"lifecycle.install-empty\",\"schema\":1,\"target_os\":\"linux\"}";
const EMPTY_INSTALL_MACOS: &[u8] =
    b"{\"operation\":\"lifecycle.install-empty\",\"schema\":1,\"target_os\":\"macos\"}";
const EMPTY_INSTALL_WINDOWS: &[u8] =
    b"{\"operation\":\"lifecycle.install-empty\",\"schema\":1,\"target_os\":\"windows\"}";
use crate::command::{
    DefaultLifecyclePorts, DefaultLifecycleRequest, LeasedDefaultLifecycle,
    lease_default_lifecycle, prepare_default_lifecycle_with_identity,
};
use crate::install::source_attestation::AttestingFactory;
use crate::install::{PreparedSelection, SelectedManifest};
use crate::phase::PhaseOutcome;
use crate::ports::{
    ConfirmGate, InstallNarration, InstallObserver, NoManifestMutation, PackageSource,
    PackageSourceBuild, PackageSourceFactory, RegistryEnvironment, RegistryEnvironmentSnapshot,
    RunObserver,
};
use crate::prelude::selected_under_lease;

mod install;
#[allow(
    unused_imports,
    reason = "internal Install adapter is exercised by focused tests"
)]
pub(crate) use install::submit_empty_world_install;
#[cfg(test)]
pub(crate) use install::submit_one_package_fixture_install;

/// A caller-controlled source port is admitted only by the test-only fixture
/// entry. The existing port cannot prove which tree its implementation serves;
/// this is not a production package-install authority.
struct FixtureSourcePort<'a> {
    source_root: &'a Path,
    factory: &'a dyn PackageSourceFactory,
    after_stage: Option<&'a dyn Fn()>,
    confirm: Option<&'a dyn ConfirmGate>,
}

struct FixtureRuntime<'a> {
    witness: PackageTreeWitness,
    factory: &'a dyn PackageSourceFactory,
    after_stage: Option<&'a dyn Fn()>,
    confirm: Option<&'a dyn ConfirmGate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetainedLifecycleResult {
    pub marker: LifecycleOutcomeMarker,
    pub report_bytes: Option<Vec<u8>>,
}

/// Caller supplies a host-issued namespace/scope and canonical request bytes.
/// This API is crate-internal and supports exactly one single-node Validate run.
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_validate_lifecycle(
    path: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_protected_lifecycle(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Phase::Validate,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn submit_protected_lifecycle(
    path: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    phase: Phase,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_protected_lifecycle_inner(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        phase,
        None,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn submit_protected_lifecycle_with_fixture<'a>(
    path: &Path,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    fixture: Option<FixtureSourcePort<'a>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_protected_lifecycle_inner(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Phase::Install,
        fixture,
    )
}

#[allow(clippy::too_many_arguments)]
fn submit_protected_lifecycle_inner<'a>(
    path: &Path,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    phase: Phase,
    fixture: Option<FixtureSourcePort<'a>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    ensure!(
        request.requested == phase,
        "protected slice request differs from its fixed phase"
    );
    ensure!(
        request.agent_mode == RunAgentMode::Cli,
        "protected slice only supports CLI mode"
    );
    ensure!(
        !request.force && !request.trace_flag,
        "protected slice refuses force and trace"
    );
    ensure!(
        request.assume_yes
            && request.policy.offline == fixture.is_some()
            && request.policy.slot_integrity
                == if fixture.is_some() {
                    SlotIntegrity::Verify
                } else {
                    SlotIntegrity::TrustPresence
                }
            && request.policy.spec_format_default.is_none(),
        "protected fixed request refuses variable confirmation or install policy"
    );
    ensure!(
        request.install_inputs.packages.is_empty()
            && request.install_inputs.features.is_empty()
            && !request.install_inputs.no_default_features
            && !request.install_inputs.all_features
            && request.install_inputs.language.is_none()
            && !request.install_inputs.exact,
        "protected slice refuses install inputs"
    );
    let leased = lease_default_lifecycle(path)?;
    let lease = leased.retain_lease();
    let fixture = fixture
        .map(|fixture| {
            ensure!(phase == Phase::Install, "fixture source is Install-only");
            let pin = vibe_core::PackageRef::parse(install::FIXTURE_PIN)?;
            let witness =
                capture_package_tree_witness(fixture.source_root, &pin, install::FIXTURE_FILES)?;
            Ok(FixtureRuntime {
                witness,
                factory: fixture.factory,
                after_stage: fixture.after_stage,
                confirm: fixture.confirm,
            })
        })
        .transpose()?;
    let canonical_request = if let Some(fixture) = &fixture {
        canonical_one_package_fixture_request(&fixture.witness, target_os)?
    } else {
        request_bytes(phase, target_os).to_vec()
    };
    if let Some(attached) =
        attach_existing_lifecycle(&lease, namespace, scope, key, &canonical_request)?
    {
        return Ok(attached);
    }
    let selected_root = leased.selected_root().to_path_buf();
    let (snapshot, witness) = SelectedManifest::read_witnessed(&selected_root)?;
    let selection = snapshot.prepare();
    ensure!(
        selection.loaded_root() == Some(lease.root()),
        "protected slice requires selected workspace root"
    );
    ensure!(
        selection
            .loaded_workspace()
            .is_some_and(|world| world.iter_nodes().count() == 1),
        "protected slice requires exactly one workspace node"
    );
    ensure!(
        !selection.request(false),
        "protected slice refuses manifest trace"
    );
    let selected = selected_under_lease(&selection, &lease)?;
    ensure!(selected == ".", "protected slice requires root node");
    if phase == Phase::Validate {
        install::require_empty_install_world(&selection, &witness)
            .context("protected Validate requires an empty handler-free world")?;
    }
    if phase == Phase::Install {
        if let Some(fixture) = &fixture {
            install::require_one_package_fixture_world(&selection, &witness, &fixture.witness)?;
            ensure!(
                LifecycleStateStore::peek_with_lease(&lease)?.is_none(),
                "one-package fixture refuses prior lifecycle state"
            );
        } else {
            install::require_empty_install_world(&selection, &witness)?;
        }
    }
    let chain: Vec<String> = vibe_lifecycle::inclusive_chain(phase)
        .iter()
        .map(ToString::to_string)
        .collect();
    let identity = select_run_identity_for_reservation(
        &lease,
        &selected_root,
        phase.as_str(),
        &chain,
        &selected,
        request.agent_mode.clone(),
        false,
        false,
        vibe_core::timestamp::now_utc(),
    )?;
    ensure!(
        !identity.adopted,
        "protected slice refuses legacy parked adoption"
    );
    let plan = canonical_lifecycle_pre_effect_plan(&LifecyclePreEffectPlanInput {
        lease_root: lease.root(),
        selected_root: &selected_root,
        selected_node: &selected,
        witness: &witness,
        workspace_witness: &witness,
        package_source: fixture.as_ref().map(|fixture| &fixture.witness),
        requested: phase,
        agent_mode: &request.agent_mode,
        trace_flag: false,
        effective_trace: identity.compile_trace,
        force: false,
        assume_yes: request.assume_yes,
        policy: &request.policy,
        install_inputs: &request.install_inputs,
    })?;
    recheck_selected_manifest_lock(&selected_root, &witness)?;
    let mut journal = ProtectedJournal {
        leased: Some(leased),
        selection: Some(selection),
        witness,
        identity,
        request,
        target_os,
        observed_at,
        namespace,
        scope,
        key,
        phase,
        canonical_plan: &plan,
        canonical_request: canonical_request.clone(),
        fixture,
    };
    submit(
        lease,
        namespace,
        scope,
        key,
        &canonical_request,
        &plan,
        &mut journal,
    )
    .map_err(anyhow::Error::new)
}

/// Read-only existing-key route before any post-effect manifest/lock snapshot.
/// It returns only the original retained run's outcome; it never treats the
/// current files as that run's pre-effect plan or grants another dispatch.
fn attach_existing_lifecycle(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    canonical_request: &[u8],
) -> Result<Option<SubmissionOutcome<RetainedLifecycleResult>>> {
    let inspected = SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        canonical_request,
    )?;
    match inspected {
        ExistingSubmissionInspection::Absent => Ok(None),
        ExistingSubmissionInspection::RetiredUnreserved => {
            Ok(Some(SubmissionOutcome::Retired { run_id: None }))
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission: JournalAdmission::Pending,
            retired,
            ..
        } => Ok(Some(if retired {
            SubmissionOutcome::Retired {
                run_id: Some(domain_run_id),
            }
        } else {
            SubmissionOutcome::RecoveryRequired {
                run_id: domain_run_id,
            }
        })),
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission: JournalAdmission::Ready,
            reservation_ref: Some(reservation_ref),
            ..
        } => {
            let inspection =
                inspect_retained_lifecycle_outcome(lease, &domain_run_id, &reservation_ref)?;
            match inspection {
                LifecycleOutcomeInspection::Validated {
                    marker,
                    report_bytes,
                } => Ok(Some(SubmissionOutcome::Replayed {
                    run_id: domain_run_id,
                    outcome: RetainedLifecycleResult {
                        marker,
                        report_bytes,
                    },
                })),
                LifecycleOutcomeInspection::Absent => {
                    Err(anyhow::Error::new(SubmissionFailure::MissingJournal {
                        run_id: domain_run_id,
                    }))
                }
                LifecycleOutcomeInspection::RecoveryRequired { reason }
                    if matches!(
                        reason,
                        LifecycleOutcomeRecoveryReason::CurrentRunHeaderWithoutMarker
                            | LifecycleOutcomeRecoveryReason::MarkerMissing
                    ) =>
                {
                    Ok(Some(SubmissionOutcome::RecoveryRequired {
                        run_id: domain_run_id,
                    }))
                }
                LifecycleOutcomeInspection::RecoveryRequired { reason } => {
                    Err(anyhow::Error::new(SubmissionFailure::UncertainJournal {
                        run_id: domain_run_id,
                        reason: format!("{reason:?}"),
                    }))
                }
            }
        }
        ExistingSubmissionInspection::Reserved { domain_run_id, .. } => {
            Err(anyhow::Error::new(SubmissionFailure::UncertainJournal {
                run_id: domain_run_id,
                reason: "ready reservation has no correlation ref".into(),
            }))
        }
    }
}

fn request_bytes(phase: Phase, target_os: TargetOs) -> &'static [u8] {
    match (phase, target_os) {
        (Phase::Validate, TargetOs::Linux) => VALIDATE_LINUX,
        (Phase::Validate, TargetOs::Macos) => VALIDATE_MACOS,
        (Phase::Validate, TargetOs::Windows) => VALIDATE_WINDOWS,
        (Phase::Install, TargetOs::Linux) => EMPTY_INSTALL_LINUX,
        (Phase::Install, TargetOs::Macos) => EMPTY_INSTALL_MACOS,
        (Phase::Install, TargetOs::Windows) => EMPTY_INSTALL_WINDOWS,
        _ => unreachable!("protected entry points admit only Validate and Install"),
    }
}

struct ProtectedJournal<'a> {
    leased: Option<LeasedDefaultLifecycle>,
    selection: Option<PreparedSelection>,
    witness: SelectedManifestLockWitness,
    identity: RunIdentity,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    phase: Phase,
    canonical_plan: &'a [u8],
    canonical_request: Vec<u8>,
    fixture: Option<FixtureRuntime<'a>>,
}

impl ProtectedJournal<'_> {
    fn selected_root(&self) -> &Path {
        self.leased
            .as_ref()
            .expect("held before invoke")
            .selected_root()
    }

    fn recheck(&self) -> Result<()> {
        recheck_selected_manifest_lock(self.selected_root(), &self.witness)?;
        Ok(())
    }

    fn header(
        lease: &LifecycleLease,
        run_id: &str,
        expected_started: Option<&str>,
        phase: Phase,
    ) -> Result<()> {
        let state = LifecycleStateStore::peek_with_lease(lease)?
            .context("reserved lifecycle header is absent")?;
        let chain: Vec<String> = vibe_lifecycle::inclusive_chain(phase)
            .iter()
            .map(ToString::to_string)
            .collect();
        ensure!(
            state.run.run_id.as_deref() == Some(run_id)
                && state.run.requested == phase.as_str()
                && state.run.chain == chain
                && state.run.selected.as_deref() == Some(".")
                && !state.run.compile_trace
                && expected_started.is_none_or(|started| state.run.started == started),
            "reserved lifecycle header differs from protected identity"
        );
        Ok(())
    }

    fn reservation_ref(&self, lease: &LifecycleLease, run_id: &str) -> Result<String> {
        Ok(SubmissionReservations::reservation_ref(
            lease,
            self.namespace,
            self.scope,
            self.key,
            run_id,
        )?)
    }

    fn retain_uncertain(
        &self,
        lease: &LifecycleLease,
        run_id: &str,
        reservation_ref: &str,
    ) -> Result<RetainedLifecycleResult> {
        let marker = create_lifecycle_outcome_marker(
            run_id,
            reservation_ref,
            MarkerOutcome::Uncertain,
            EffectState::Uncertain,
            LifecycleReportPayload::Unavailable(ReportEvidenceUnavailableReason::NotProduced),
        )?;
        retained(lease, run_id, reservation_ref, &marker, None)
    }
}

impl SubmissionJournal for ProtectedJournal<'_> {
    type Outcome = RetainedLifecycleResult;

    fn proposed_run_id(&self) -> String {
        self.identity.run_id.clone()
    }

    fn inspect(
        &mut self,
        lease: &LifecycleLease,
        run_id: &str,
    ) -> Result<JournalState<Self::Outcome>, String> {
        let admission = SubmissionReservations::journal_admission(
            lease,
            self.namespace,
            self.scope,
            self.key,
            run_id,
        )
        .map_err(|error| error.to_string())?;
        if admission == JournalAdmission::Pending {
            return match inspect_reserved_lifecycle_run(lease, run_id) {
                Ok(LifecycleRunJournalObservation::Absent) => Ok(JournalState::Absent),
                Ok(LifecycleRunJournalObservation::RecoveryRequired { .. }) => {
                    // A retry may have proposed a different fresh ID; it can
                    // prepare only the stored ID and never invokes it again.
                    let started =
                        (run_id == self.identity.run_id).then_some(self.identity.started.as_str());
                    Self::header(lease, run_id, started, self.phase)
                        .map_err(|error| error.to_string())?;
                    Ok(JournalState::Prepared)
                }
                Ok(LifecycleRunJournalObservation::Uncertain { .. }) => Ok(
                    JournalState::Uncertain("unattributed current lifecycle header".into()),
                ),
                Err(error) => Ok(JournalState::Uncertain(error.to_string())),
            };
        }
        let reservation_ref = self
            .reservation_ref(lease, run_id)
            .map_err(|error| error.to_string())?;
        match inspect_retained_lifecycle_outcome(lease, run_id, &reservation_ref)
            .map_err(|error| error.to_string())?
        {
            LifecycleOutcomeInspection::Validated {
                marker,
                report_bytes,
            } => Ok(JournalState::Complete(RetainedLifecycleResult {
                marker,
                report_bytes,
            })),
            LifecycleOutcomeInspection::Absent => Ok(JournalState::Absent),
            LifecycleOutcomeInspection::RecoveryRequired { reason }
                if matches!(
                    reason,
                    LifecycleOutcomeRecoveryReason::CurrentRunHeaderWithoutMarker
                        | LifecycleOutcomeRecoveryReason::MarkerMissing
                ) =>
            {
                Ok(JournalState::Prepared)
            }
            LifecycleOutcomeInspection::RecoveryRequired { reason } => {
                Ok(JournalState::Uncertain(format!("{reason:?}")))
            }
        }
    }

    fn initialize(&mut self, lease: Arc<LifecycleLease>, run_id: &str) -> Result<(), String> {
        let result = (|| -> Result<()> {
            self.recheck()?;
            ensure!(
                LifecycleStateStore::peek_with_lease(&lease)?.is_none(),
                "protected validate slice refuses any prior lifecycle header"
            );
            let store = LifecycleStateStore::begin(
                lease.clone(),
                self.phase.as_str().into(),
                vibe_lifecycle::inclusive_chain(self.phase)
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                self.identity.started.clone(),
                run_id.to_owned(),
                ".".into(),
                false,
            )?;
            ensure!(
                store.state().execution.is_empty(),
                "new protected header inherited execution rows"
            );
            drop(store);
            Self::header(&lease, run_id, Some(&self.identity.started), self.phase)
        })();
        result.map_err(|error| format!("{error:#}"))
    }

    fn invoke(
        &mut self,
        lease: Arc<LifecycleLease>,
        run_id: &str,
    ) -> Result<Self::Outcome, String> {
        let result = (|| -> Result<Self::Outcome> {
            ensure!(
                run_id == self.identity.run_id,
                "effect run ID differs from fresh proposal"
            );
            ensure!(
                SubmissionReservations::journal_admission(
                    &lease,
                    self.namespace,
                    self.scope,
                    self.key,
                    run_id,
                )? == JournalAdmission::Ready,
                "lifecycle journal is not ready"
            );
            let reservation_ref = self.reservation_ref(&lease, run_id)?;
            self.recheck()?;
            Self::header(&lease, run_id, Some(&self.identity.started), self.phase)?;
            if self.phase == Phase::Install {
                let root = self.selected_root();
                // This root-only slice has one witness for both the selected
                // node and the workspace lock home. Publication and readback
                // occur before the incumbent install can enter any port.
                let _entered = publish_install_entered_stage(
                    &lease,
                    self.namespace,
                    self.scope,
                    self.key,
                    run_id,
                    &self.canonical_request,
                    self.canonical_plan,
                    root,
                    &self.witness,
                    lease.root(),
                    &self.witness,
                )?;
            }
            if let Some(fixture) = &self.fixture {
                if let Some(after_stage) = fixture.after_stage {
                    after_stage();
                }
                recheck_package_tree_witness(&fixture.witness)?;
            }
            let leased = self.leased.take().context("leased stage consumed twice")?;
            let selection = self.selection.take().context("selection consumed twice")?;
            let prepared = prepare_default_lifecycle_with_identity(
                leased,
                self.request.clone(),
                selection,
                self.identity.clone(),
            )?;
            let attested_factory = self
                .fixture
                .as_ref()
                .map(|fixture| AttestingFactory::new(fixture.factory, &fixture.witness))
                .transpose()?;
            let ports = if self.phase == Phase::Validate {
                validate_ports()
            } else if let Some(factory) = attested_factory.as_ref() {
                install::one_package_fixture_ports(
                    factory,
                    self.fixture.as_ref().and_then(|f| f.confirm),
                )
            } else {
                install::empty_install_ports()
            };
            let outcome = prepared.run(ports, None, self.target_os, self.observed_at, None);
            match outcome {
                PhaseOutcome::Completed(values) => {
                    let report_bytes = serde_json::to_vec(&values.into_report(None))?;
                    match create_lifecycle_outcome_marker(
                        run_id,
                        &reservation_ref,
                        MarkerOutcome::Completed,
                        EffectState::Uncertain,
                        LifecycleReportPayload::Available(&report_bytes),
                    ) {
                        Ok(marker) => retained(
                            &lease,
                            run_id,
                            &reservation_ref,
                            &marker,
                            Some(&report_bytes),
                        ),
                        Err(_) => self.retain_uncertain(&lease, run_id, &reservation_ref),
                    }
                }
                PhaseOutcome::Parked(_) | PhaseOutcome::Failed { .. } => {
                    self.retain_uncertain(&lease, run_id, &reservation_ref)
                }
            }
        })();
        result.map_err(|error| format!("{error:#}"))
    }
}

fn retained(
    lease: &LifecycleLease,
    run_id: &str,
    reservation_ref: &str,
    marker: &LifecycleOutcomeMarker,
    report_bytes: Option<&[u8]>,
) -> Result<RetainedLifecycleResult> {
    match retain_lifecycle_outcome(lease, run_id, reservation_ref, marker, report_bytes)? {
        LifecycleOutcomeInspection::Validated {
            marker,
            report_bytes,
        } => Ok(RetainedLifecycleResult {
            marker,
            report_bytes,
        }),
        other => bail!("lifecycle outcome was not read back as validated: {other:?}"),
    }
}

struct SilentRun;
impl RunObserver for SilentRun {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Null
    }
    fn binary_quiet(&self) -> bool {
        true
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn observe_plan(&self, _: &RitualPlan, _: &RunMetadata, _: bool) -> Result<()> {
        Ok(())
    }
    fn observe_contribution(&self, _: &LifecycleContributionReport) {}
    fn observe_untracked_failure(
        &self,
        _: &RunMetadata,
        _: &str,
        _: &[LifecycleContributionReport],
    ) -> Result<()> {
        Ok(())
    }
}

struct SilentInstall;
impl InstallObserver for SilentInstall {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Null
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn narrate(&self, _: InstallNarration<'_>) {}
    fn lane_sizes(&self, _: &Path) -> Vec<(String, Option<u64>)> {
        Vec::new()
    }
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        &NoPlanEvents
    }
    fn slot_observer(&self, _: &RunMetadata) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(NoSlotEvents)
    }
}

struct NoPlanEvents;
impl vibe_install::PlanObserver for NoPlanEvents {
    fn on(&self, _: vibe_install::PlanEvent) {}
}
struct NoSlotEvents;
impl vibe_install::SlotLifecycleObserver for NoSlotEvents {
    fn observe(&self, _: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
        Ok(())
    }
    fn outcome(&self, _: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
        Ok(())
    }
}
struct RejectInstall;
impl ConfirmGate for RejectInstall {
    fn confirm_install(&self, _: usize) -> Result<()> {
        bail!("validate slice reached install confirmation")
    }
}
impl RegistryEnvironment for RejectInstall {
    fn prepare(&self) -> Result<RegistryEnvironmentSnapshot> {
        bail!("validate slice reached registry environment")
    }
}

impl PackageSourceFactory for RejectInstall {
    fn build(&self, _: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>> {
        bail!("validate slice reached package source")
    }
}

fn validate_ports<'a>() -> DefaultLifecyclePorts<'a> {
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);
    DefaultLifecyclePorts {
        observer: &SilentRun,
        install_observer: &SilentInstall,
        confirm_gate: &RejectInstall,
        sources: &RejectInstall,
        environment: &RejectInstall,
        manifest_mutation: &NoManifestMutation,
        agent,
    }
}

#[cfg(test)]
#[path = "lifecycle_adapter/tests.rs"]
mod tests;
