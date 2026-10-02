//! Internal, narrowly protected lifecycle submission through the incumbent runner.
//!
//! Only Validate and a one-node empty-world Install are admitted. General
//! package install, providers, members, trace and prior-current-state cases
//! remain unavailable.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail, ensure};
use vibe_core::manifest::TargetOs;
use vibe_core::manifest::{ExtensionHandler, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_lifecycle::agent::{
    AgentCompletion, AgentRequest, PromptRequest, ResolvedPrompt, SelectedWorldPromptResolver,
};
use vibe_lifecycle::process::{
    PackageTreeWitness, SelectedManifestLockWitness, capture_package_tree_witness,
    recheck_package_tree_witness, recheck_selected_manifest_lock,
};
use vibe_lifecycle::process::{
    ProcessRunner, PromptSourceWitness, ScriptSourceWitness, SourceSnapshotError,
    SystemProcessRunner, capture_prompt_source_witness, capture_script_source_witness,
    recheck_prompt_source_witness, recheck_script_source_witness,
};
use vibe_lifecycle::{
    AgentBackend, ExistingSubmissionInspection, JournalAdmission, LifecycleLease,
    LifecycleOutcomeInspection, LifecycleOutcomeRecoveryReason, LifecycleReportPayload,
    LifecycleRunJournalObservation, LifecycleStateStore, Phase, RunIdentity, RunMetadata,
    SubmissionNamespace, SubmissionReservations, SubmissionScope, create_lifecycle_outcome_marker,
    inspect_reserved_lifecycle_run, inspect_retained_lifecycle_outcome,
    publish_install_entered_stage, publish_script_source_stage, retain_lifecycle_outcome,
    select_run_identity_for_reservation,
};
use vibe_lifecycle::{ExtensionProvider, HandlerExecution};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_outcome_marker::{
    LifecycleOutcomeMarker, LifecycleOutcomeMarkerEffectState as EffectState,
    LifecycleOutcomeMarkerOutcome as MarkerOutcome, ReportEvidenceUnavailableReason,
};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::Timestamp;
use vibe_workspace::hooks::{InterpreterProbe, SystemProbe};

use super::lifecycle_plan::{
    LifecyclePreEffectPlanInput, canonical_lifecycle_pre_effect_plan,
    canonical_one_package_fixture_request,
};
use super::lifecycle_plan::{
    PromptPlanSource, ScriptPlanSource, canonical_one_agent_fixture_request,
    canonical_one_script_fixture_request,
};
use super::{JournalState, SubmissionFailure, SubmissionJournal, SubmissionOutcome, submit};
use crate::RitualPlan;
use crate::dispatch::ProtectedScriptExecution;
use crate::failure::Measurement;
use crate::values::LifecycleValues;

// The target OS changes lifecycle behavior and belongs in this fixed
// request. `observed_at` is execution evidence: retries retain the first
// report timestamp instead of treating a fresh clock reading as new intent.
pub(crate) const FIXED_OPERATION: &str = "rust:vibe-lifecycle:validate";

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
    dead_code,
    reason = "private host integration atom; no public surface yet"
)]
mod live_owner;
mod sequential_runs;
#[allow(
    unused_imports,
    reason = "internal Install adapter is exercised by focused tests"
)]
pub(crate) use install::submit_empty_world_install;
#[cfg(test)]
pub(crate) use install::submit_one_package_fixture_install;
pub(in crate::machine_submission) use live_owner::evidence::cold_snapshot;
pub use live_owner::{
    BindingSummary, EvidenceBasis, FixedOperationStatus, FixedOwnerEvidenceSnapshot,
    HostControlCapability, HostOwnedFixedOperation, ObservationHandle, ObservedPhase, ObservedWork,
    Readiness,
};

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

use super::lifecycle_decisions::submission::profile::AdmittedProfile;

struct ScriptFixturePort<'a> {
    profile: Option<&'a AdmittedProfile>,
    admitted: OneScriptValidateAdmission,
    process: &'a dyn ProcessRunner,
    probe: &'a dyn InterpreterProbe,
    after_stage: Option<&'a dyn Fn()>,
    after_effect: Option<&'a dyn Fn() -> Result<()>>,
    ready: Option<&'a live_owner::ReadyObserver>,
}

struct AgentFixturePort<'a> {
    admitted: OneAgentValidateAdmission,
    backend: Arc<WitnessedAgentBackend>,
    before_run: Option<&'a dyn Fn()>,
    after_effect: Option<&'a dyn Fn() -> Result<()>>,
}

/// Test-controlled, credential-free resolver that serves only the preflight
/// prompt bytes. The paid half is a canary and must never run on a park.
struct WitnessedAgentBackend {
    source: PromptSourceWitness,
    resolved: ResolvedPrompt,
    address: String,
    resolves: Arc<AtomicUsize>,
    completes: Arc<AtomicUsize>,
}

impl AgentBackend for WitnessedAgentBackend {
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.resolves.fetch_add(1, Ordering::SeqCst);
        if request.address != self.address
            || request.provider_root != self.source.provider_root
            || request.provider_group != "org.demo"
            || request.provider_name != "demo"
            || !request.selected_world.is_empty()
        {
            return Err("protected prompt request differs from the fixed source".into());
        }
        recheck_prompt_source_witness(&self.source).map_err(|error| error.to_string())?;
        Ok(self.resolved.clone())
    }

    fn complete(&self, _: &AgentRequest) -> Result<AgentCompletion, String> {
        self.completes.fetch_add(1, Ordering::SeqCst);
        Err("protected hosted fixture must park before provider completion".into())
    }
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

/// Fixed, effect-free admission for the one-script protected Validate fixture.
pub(crate) struct OneScriptValidateAdmission {
    pub canonical_request: Vec<u8>,
    pub canonical_plan: Vec<u8>,
    pub source: ScriptSourceWitness,
    pub execution_key: String,
    pub provider: String,
}

const ONE_SCRIPT_VALIDATE_MANIFEST: &str = "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n";

pub(crate) const ONE_AGENT_VALIDATE_MANIFEST: &str = "[project]\ngroup = 'org.demo'\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'park'\npoint = 'phase:validate'\nhandler = { kind = 'agent', prompt = 'spec://org.demo/demo/common/PROMPT-001#root' }\n[extension.config]\noutputs = [{ path = 'docs/result.md', kind = 'file', accept = 'non-empty file' }]\n";
pub(crate) const ONE_AGENT_PROMPT_PATH: &str = "spec/common/PROMPT-001.md";
const ONE_AGENT_PROMPT_ADDRESS: &str = "spec://org.demo/demo/common/PROMPT-001#root";

/// Effect-free, fixed one-agent fixture admission. A future owner route may
/// reserve these exact bytes; this function does not publish a hosted task.
pub(crate) struct OneAgentValidateAdmission {
    pub canonical_request: Vec<u8>,
    pub canonical_plan: Vec<u8>,
    pub source: PromptSourceWitness,
    pub execution_key: String,
    pub provider: String,
    pub address: String,
    pub resolved_prompt: ResolvedPrompt,
}

pub(crate) fn admit_one_agent_validate_fixture(
    path: &Path,
    request: &DefaultLifecycleRequest,
    target_os: TargetOs,
) -> Result<OneAgentValidateAdmission> {
    admit_one_agent_validate_fixture_inner(path, request, target_os, true)
}

fn admit_one_agent_validate_fixture_inner(
    path: &Path,
    request: &DefaultLifecycleRequest,
    target_os: TargetOs,
    fresh_only: bool,
) -> Result<OneAgentValidateAdmission> {
    ensure!(
        request.requested == Phase::Validate
            && request.agent_mode == RunAgentMode::Agent
            && !request.force
            && !request.trace_flag
            && request.assume_yes
            && request.install_inputs.packages.is_empty()
            && request.install_inputs.features.is_empty()
            && !request.install_inputs.no_default_features
            && !request.install_inputs.all_features
            && request.install_inputs.language.is_none()
            && !request.install_inputs.exact
            && !request.policy.offline
            && request.policy.slot_integrity == SlotIntegrity::TrustPresence
            && request.policy.spec_format_default.is_none(),
        "one-agent fixture refuses variable command or install inputs"
    );
    let leased = lease_default_lifecycle(path)?;
    let lease = leased.retain_lease();
    let selected_root = leased.selected_root();
    let (snapshot, witness) = SelectedManifest::read_witnessed(selected_root)?;
    let selection = snapshot.prepare();
    ensure!(
        selection.loaded_root() == Some(lease.root()),
        "agent fixture requires selected root"
    );
    ensure!(
        selection
            .loaded_workspace()
            .is_some_and(|world| world.iter_nodes().count() == 1),
        "agent fixture requires one workspace node"
    );
    ensure!(
        !selection.request(false),
        "agent fixture refuses manifest trace"
    );
    ensure!(
        selected_under_lease(&selection, &lease)? == ".",
        "agent fixture requires root node"
    );
    if fresh_only {
        ensure!(
            LifecycleStateStore::peek_with_lease(&lease)?.is_none(),
            "agent fixture refuses prior lifecycle header"
        );
    }
    let manifest = selection
        .parsed_ref()
        .context("agent fixture manifest did not parse")?;
    ensure!(
        manifest == &Manifest::parse_str(ONE_AGENT_VALIDATE_MANIFEST)?,
        "agent fixture refuses unreviewed manifest declarations"
    );
    install::require_empty_lock(&witness)?;
    let world = selection
        .loaded_workspace()
        .context("agent fixture workspace did not load")?;
    let ritual = crate::world::plan_default_prepared(selected_root, world, &[Phase::Validate])?;
    ensure!(
        ritual.executions().len() == 1,
        "agent fixture requires exactly one planned handler"
    );
    let planned = &ritual.executions()[0];
    ensure!(
        planned.phase == Phase::Validate.as_str(),
        "agent fixture planned foreign phase"
    );
    let row = &planned.row;
    let ExtensionHandler::Agent { prompt } = &row.declaration().handler else {
        bail!("agent fixture planned non-agent handler")
    };
    ensure!(
        prompt == ONE_AGENT_PROMPT_ADDRESS,
        "agent fixture prompt address differs"
    );
    let ExtensionProvider::Host(host) = row.provider() else {
        bail!("agent fixture planned dependency provider")
    };
    ensure!(
        host.root == selected_root,
        "agent fixture provider root differs"
    );
    let source = capture_prompt_source_witness(&host.root, ONE_AGENT_PROMPT_PATH)?;
    let text =
        std::str::from_utf8(&source.source.bytes).context("agent fixture prompt is not UTF-8")?;
    ensure!(
        text.starts_with("# Prompt {#root}\n") && text.trim().len() > "# Prompt {#root}".len(),
        "agent fixture prompt has no fixed root section/body"
    );
    ensure!(
        !["#embed", "#use", "#source"]
            .iter()
            .any(|directive| text.contains(directive)),
        "agent fixture prompt refuses composition directives"
    );
    let execution_key = HandlerExecution::from_row(row).key();
    let provider = row.provider().to_string();
    let address = prompt.clone();
    let resolved_prompt = SelectedWorldPromptResolver::new(lease.root())
        .resolve(&PromptRequest {
            address: address.clone(),
            provider_root: source.provider_root.clone(),
            provider_group: "org.demo".into(),
            provider_name: "demo".into(),
            selected_world: Default::default(),
        })
        .map_err(anyhow::Error::msg)?;
    ensure!(
        resolved_prompt.unsupported.is_empty(),
        "agent fixture prompt has unsupported composition"
    );
    let prompt_source = PromptPlanSource {
        witness: &source,
        execution_key: &execution_key,
        provider: &provider,
        address: &address,
    };
    let canonical_request = canonical_one_agent_fixture_request(&prompt_source, target_os)?;
    let canonical_plan = canonical_lifecycle_pre_effect_plan(&LifecyclePreEffectPlanInput {
        lease_root: lease.root(),
        selected_root,
        selected_node: ".",
        witness: &witness,
        workspace_witness: &witness,
        package_source: None,
        script_source: None,
        prompt_source: Some(prompt_source),
        requested: Phase::Validate,
        agent_mode: &request.agent_mode,
        trace_flag: false,
        effective_trace: false,
        force: false,
        assume_yes: true,
        policy: &request.policy,
        install_inputs: &request.install_inputs,
    })?;
    recheck_selected_manifest_lock(selected_root, &witness)?;
    recheck_prompt_source_witness(&source)?;
    Ok(OneAgentValidateAdmission {
        canonical_request,
        canonical_plan,
        source,
        execution_key,
        provider,
        address,
        resolved_prompt,
    })
}

fn require_one_script_request(request: &DefaultLifecycleRequest) -> Result<()> {
    ensure!(
        request.requested == Phase::Validate
            && request.agent_mode == RunAgentMode::Cli
            && !request.force
            && !request.trace_flag
            && request.assume_yes
            && request.install_inputs.packages.is_empty()
            && request.install_inputs.features.is_empty()
            && !request.install_inputs.no_default_features
            && !request.install_inputs.all_features
            && request.install_inputs.language.is_none()
            && !request.install_inputs.exact
            && !request.policy.offline
            && request.policy.slot_integrity == SlotIntegrity::TrustPresence
            && request.policy.spec_format_default.is_none(),
        "one-script fixture refuses variable command or install inputs"
    );
    Ok(())
}

pub(crate) fn admit_one_script_validate_fixture(
    path: &Path,
    request: &DefaultLifecycleRequest,
    target_os: TargetOs,
) -> Result<OneScriptValidateAdmission> {
    require_one_script_request(request)?;
    let leased = lease_default_lifecycle(path)?;
    preflight_one_script_under_lease(&leased, request, target_os)
}

/// Incumbent v4 preflight under the caller's already-held outer lease.
pub(super) fn preflight_one_script_under_lease(
    leased: &LeasedDefaultLifecycle,
    request: &DefaultLifecycleRequest,
    target_os: TargetOs,
) -> Result<OneScriptValidateAdmission> {
    require_one_script_request(request)?;
    let lease = leased.retain_lease();
    let selected_root = leased.selected_root();
    let (snapshot, witness) = SelectedManifest::read_witnessed(selected_root)?;
    let selection = snapshot.prepare();
    ensure!(
        selection.loaded_root() == Some(lease.root()),
        "fixture requires selected root"
    );
    ensure!(
        selection
            .loaded_workspace()
            .is_some_and(|world| world.iter_nodes().count() == 1),
        "fixture requires one workspace node"
    );
    ensure!(!selection.request(false), "fixture refuses manifest trace");
    ensure!(
        selected_under_lease(&selection, &lease)? == ".",
        "fixture requires root node"
    );
    let manifest = selection
        .parsed_ref()
        .context("fixture manifest did not parse")?;
    ensure!(
        manifest == &Manifest::parse_str(ONE_SCRIPT_VALIDATE_MANIFEST)?,
        "fixture refuses unreviewed manifest declarations"
    );
    install::require_empty_lock(&witness)?;
    let world = selection
        .loaded_workspace()
        .context("fixture workspace did not load")?;
    let ritual = crate::world::plan_default_prepared(selected_root, world, &[Phase::Validate])?;
    ensure!(
        ritual.executions().len() == 1,
        "fixture requires exactly one planned handler"
    );
    let planned = &ritual.executions()[0];
    ensure!(
        planned.phase == Phase::Validate.as_str(),
        "fixture planned a foreign phase"
    );
    let row = &planned.row;
    ensure!(
        matches!(&row.declaration().handler, ExtensionHandler::Script { base } if base == Path::new("run")),
        "fixture planned a nonfixed handler"
    );
    let ExtensionProvider::Host(host) = row.provider() else {
        bail!("fixture planned a dependency provider")
    };
    ensure!(
        host.root == selected_root,
        "fixture provider root differs from selected root"
    );
    let script_relative = match target_os {
        TargetOs::Windows => "run.ps1",
        TargetOs::Linux | TargetOs::Macos => "run.sh",
    };
    let source = capture_script_source_witness(&host.root, script_relative)?;
    if target_os == TargetOs::Windows {
        match capture_script_source_witness(&host.root, "run.sh") {
            Ok(_) => bail!("Windows fixture refuses alternate run.sh selection"),
            Err(SourceSnapshotError::MissingScript) => {}
            Err(error) => return Err(error.into()),
        }
    }
    let execution_key = HandlerExecution::from_row(row).key();
    let provider = row.provider().to_string();
    let script = ScriptPlanSource {
        witness: &source,
        execution_key: &execution_key,
        provider: &provider,
    };
    let canonical_request = canonical_one_script_fixture_request(&script, target_os)?;
    let canonical_plan = canonical_lifecycle_pre_effect_plan(&LifecyclePreEffectPlanInput {
        lease_root: lease.root(),
        selected_root,
        selected_node: ".",
        witness: &witness,
        workspace_witness: &witness,
        package_source: None,
        script_source: Some(script),
        prompt_source: None,
        requested: Phase::Validate,
        agent_mode: &request.agent_mode,
        trace_flag: false,
        effective_trace: false,
        force: false,
        assume_yes: true,
        policy: &request.policy,
        install_inputs: &request.install_inputs,
    })?;
    recheck_selected_manifest_lock(selected_root, &witness)?;
    Ok(OneScriptValidateAdmission {
        canonical_request,
        canonical_plan,
        source,
        execution_key,
        provider,
    })
}

/// Internal fixed-script entry. It admits only the exact root manifest and
/// selected source; callers cannot supply a process or interpreter probe.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn submit_fixed_one_script_validate(
    path: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    ensure!(
        TargetOs::current() == Some(target_os),
        "fixed script target OS differs from the current host"
    );
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    static PROBE: SystemProbe = SystemProbe;
    submit_one_script_validate_internal(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        &PROCESS,
        &PROBE,
        None,
        None,
        None,
        None,
    )
}

/// Test-only counted process seam; the shipped entry above fixes both ports.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_one_script_validate_fixture<'a>(
    path: &Path,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    process: &'a dyn ProcessRunner,
    probe: &'a dyn InterpreterProbe,
    after_stage: Option<&'a dyn Fn()>,
    after_effect: Option<&'a dyn Fn() -> Result<()>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_one_script_validate_internal(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        process,
        probe,
        after_stage,
        after_effect,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn submit_one_script_validate_internal<'a>(
    path: &Path,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    process: &'a dyn ProcessRunner,
    probe: &'a dyn InterpreterProbe,
    after_stage: Option<&'a dyn Fn()>,
    after_effect: Option<&'a dyn Fn() -> Result<()>>,
    ready: Option<&'a live_owner::ReadyObserver>,
    profile: Option<&'a AdmittedProfile>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    let capture_request = match profile {
        Some(profile) => profile.source_capture_request(&request)?,
        None => request.clone(),
    };
    let admitted = admit_one_script_validate_fixture(path, &capture_request, target_os)?;
    submit_protected_lifecycle_inner(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Phase::Validate,
        None,
        Some(ScriptFixturePort {
            profile,
            admitted,
            process,
            probe,
            after_stage,
            after_effect,
            ready,
        }),
        None,
    )
}

/// Internal fixed-agent entry. The only backend is the credential-free
/// witnessed prompt resolver; no paid provider can be supplied by callers.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn submit_fixed_one_agent_validate(
    path: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    ensure!(
        TargetOs::current() == Some(target_os),
        "fixed agent target OS differs from the current host"
    );
    submit_one_agent_validate_internal(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
        None,
        None,
    )
}

/// Test-only counted resolver seam; the shipped entry owns its own counters.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_one_agent_validate_fixture<'a>(
    path: &Path,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    resolves: Arc<AtomicUsize>,
    completes: Arc<AtomicUsize>,
    before_run: Option<&'a dyn Fn()>,
    after_effect: Option<&'a dyn Fn() -> Result<()>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_one_agent_validate_internal(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        resolves,
        completes,
        before_run,
        after_effect,
    )
}

#[allow(clippy::too_many_arguments)]
fn submit_one_agent_validate_internal<'a>(
    path: &Path,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    resolves: Arc<AtomicUsize>,
    completes: Arc<AtomicUsize>,
    before_run: Option<&'a dyn Fn()>,
    after_effect: Option<&'a dyn Fn() -> Result<()>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    let admitted = admit_one_agent_validate_fixture_inner(path, &request, target_os, false)?;
    let backend = Arc::new(WitnessedAgentBackend {
        source: admitted.source.clone(),
        resolved: admitted.resolved_prompt.clone(),
        address: admitted.address.clone(),
        resolves,
        completes,
    });
    submit_protected_lifecycle_inner(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Phase::Validate,
        None,
        None,
        Some(AgentFixturePort {
            admitted,
            backend,
            before_run,
            after_effect,
        }),
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
        None,
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
        None,
        None,
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
    mut script: Option<ScriptFixturePort<'a>>,
    agent: Option<AgentFixturePort<'a>>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    ensure!(
        usize::from(fixture.is_some())
            + usize::from(script.is_some())
            + usize::from(agent.is_some())
            <= 1,
        "protected fixture sources cannot be combined"
    );
    ensure!(
        script.is_none() || phase == Phase::Validate,
        "script fixture is Validate-only"
    );
    ensure!(
        request.requested == phase,
        "protected slice request differs from its fixed phase"
    );
    ensure!(
        request.agent_mode
            == if agent.is_some() {
                RunAgentMode::Agent
            } else {
                RunAgentMode::Cli
            },
        "protected slice agent mode differs from fixed route"
    );
    match script.as_ref().and_then(|s| s.profile) {
        Some(profile) => {
            ensure!(!request.trace_flag, "protected slice refuses trace");
            ensure!(
                request.force == profile.force(),
                "approved force intent changed"
            );
        }
        None => ensure!(
            !request.force && !request.trace_flag,
            "protected slice refuses force and trace"
        ),
    }
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
    if let Some(ready) = script.as_ref().and_then(|s| s.ready) {
        ready.pin_root(&lease)?;
    }
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
    if let Some(script) = script.as_mut() {
        if let Some(profile) = script.profile {
            // Recapture actual v4 source and plan under this protected lease.
            let base = profile.source_capture_request(&request)?;
            script.admitted = preflight_one_script_under_lease(&leased, &base, target_os)?;
        }
    }
    let canonical_request = if let Some(agent) = &agent {
        agent.admitted.canonical_request.clone()
    } else if let Some(script) = &script {
        if let Some(profile) = script.profile {
            profile.request(&script.admitted.canonical_request)?
        } else {
            script.admitted.canonical_request.clone()
        }
    } else if let Some(fixture) = &fixture {
        canonical_one_package_fixture_request(&fixture.witness, target_os)?
    } else {
        request_bytes(phase, target_os).to_vec()
    };
    if let Some(attached) =
        attach_existing_lifecycle(&lease, namespace, scope, key, &canonical_request)?
    {
        if script.as_ref().is_some_and(|s| s.profile.is_some()) {
            if let Some(row) =
                SubmissionReservations::inspect_existing_key(&lease, namespace, scope, key)?
            {
                check_admitted_capsule(
                    &lease,
                    &row.domain_run_id,
                    &row.request_sha256,
                    &row.plan_sha256,
                )?;
            }
        }
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
    if let Some(agent) = &agent {
        ensure!(
            LifecycleStateStore::peek_with_lease(&lease)?.is_none(),
            "fresh agent fixture refuses prior lifecycle header"
        );
        let manifest = selection
            .parsed_ref()
            .context("agent fixture manifest did not parse")?;
        ensure!(
            manifest == &Manifest::parse_str(ONE_AGENT_VALIDATE_MANIFEST)?,
            "agent fixture manifest changed"
        );
        install::require_empty_lock(&witness)?;
        let world = selection
            .loaded_workspace()
            .context("agent fixture workspace did not load")?;
        let ritual =
            crate::world::plan_default_prepared(&selected_root, world, &[Phase::Validate])?;
        ensure!(ritual.executions().len() == 1, "agent fixture plan changed");
        let row = &ritual.executions()[0].row;
        ensure!(
            matches!(&row.declaration().handler, ExtensionHandler::Agent { prompt } if prompt == &agent.admitted.address)
                && HandlerExecution::from_row(row).key() == agent.admitted.execution_key
                && row.provider().to_string() == agent.admitted.provider,
            "agent fixture execution identity changed"
        );
        recheck_prompt_source_witness(&agent.admitted.source)?;
    } else if let Some(script) = &script {
        let manifest = selection
            .parsed_ref()
            .context("script fixture manifest did not parse")?;
        ensure!(
            manifest == &Manifest::parse_str(ONE_SCRIPT_VALIDATE_MANIFEST)?,
            "script fixture manifest changed"
        );
        install::require_empty_lock(&witness)?;
        let world = selection
            .loaded_workspace()
            .context("script fixture workspace did not load")?;
        let ritual =
            crate::world::plan_default_prepared(&selected_root, world, &[Phase::Validate])?;
        ensure!(
            ritual.executions().len() == 1,
            "script fixture plan changed"
        );
        let row = &ritual.executions()[0].row;
        ensure!(
            HandlerExecution::from_row(row).key() == script.admitted.execution_key
                && row.provider().to_string() == script.admitted.provider,
            "script fixture execution identity changed"
        );
        recheck_script_source_witness(&script.admitted.source)?;
    } else if phase == Phase::Validate {
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
        request.force,
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
        script_source: script.as_ref().map(|script| ScriptPlanSource {
            witness: &script.admitted.source,
            execution_key: &script.admitted.execution_key,
            provider: &script.admitted.provider,
        }),
        prompt_source: agent.as_ref().map(|agent| PromptPlanSource {
            witness: &agent.admitted.source,
            execution_key: &agent.admitted.execution_key,
            provider: &agent.admitted.provider,
            address: &agent.admitted.address,
        }),
        requested: phase,
        agent_mode: &request.agent_mode,
        trace_flag: false,
        effective_trace: identity.compile_trace,
        force: false,
        assume_yes: request.assume_yes,
        policy: &request.policy,
        install_inputs: &request.install_inputs,
    })?;
    if let Some(script) = &script {
        ensure!(
            plan == script.admitted.canonical_plan,
            "script fixture pre-effect plan changed"
        );
    }
    if let Some(agent) = &agent {
        ensure!(
            plan == agent.admitted.canonical_plan,
            "agent fixture pre-effect plan changed"
        );
    }
    recheck_selected_manifest_lock(&selected_root, &witness)?;
    let plan = if let Some(script) = &script {
        if let Some(profile) = script.profile {
            recheck_script_source_witness(&script.admitted.source)?;
            profile.guard(
                &lease,
                &script.admitted.canonical_request,
                &plan,
                &script.admitted.source,
            )?;
            profile.plan(&plan)?
        } else {
            plan
        }
    } else {
        plan
    };
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
        script,
        agent,
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

pub(super) fn inspect_fixed_existing(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
) -> Result<Option<FixedOperationStatus>> {
    let Some(row) = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?
    else {
        return Ok(None);
    };
    if let Err(error) = check_admitted_capsule(
        lease,
        &row.domain_run_id,
        &row.request_sha256,
        &row.plan_sha256,
    ) {
        return Ok(Some(FixedOperationStatus::RecoveryRequired {
            run_id: Some(row.domain_run_id),
            reason: format!("input capsule: {error:#}"),
        }));
    }
    let run_id = row.domain_run_id;
    let Some(reference) = row
        .reservation_ref
        .filter(|_| row.journal_admission == JournalAdmission::Ready)
    else {
        return Ok(Some(FixedOperationStatus::RecoveryRequired {
            run_id: Some(run_id),
            reason: "C1 journal not Ready".into(),
        }));
    };
    Ok(Some(
        match inspect_retained_lifecycle_outcome(lease, &run_id, &reference) {
            Ok(LifecycleOutcomeInspection::Validated {
                marker,
                report_bytes,
            }) => FixedOperationStatus::ResultAvailable {
                run_id,
                marker,
                report_bytes,
            },
            other => FixedOperationStatus::RecoveryRequired {
                run_id: Some(run_id),
                reason: format!("retained inspection: {other:?}"),
            },
        },
    ))
}

// Immutable input evidence belongs to the incumbent protected journal, not a
// second result journal. Only the new profile publishes this capsule; default
// v4 bytes and its existing inspection route stay unchanged.
fn admitted_capsule(request: &[u8], plan: &[u8]) -> Result<Vec<u8>> {
    let value = serde_json::json!({
        "format": "vibevm.lifecycle.admitted-input/1",
        "request": serde_json::from_slice::<serde_json::Value>(request)?,
        "plan": serde_json::from_slice::<serde_json::Value>(plan)?,
    });
    let mut bytes = Vec::new();
    super::lifecycle_plan::write_canonical(&value, &mut bytes)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024 + 4096,
        "input capsule capacity"
    );
    Ok(bytes)
}
fn check_admitted_capsule(
    lease: &LifecycleLease,
    run_id: &str,
    request_sha256: &str,
    plan_sha256: &str,
) -> Result<()> {
    let home = lease
        .publication_journal_project()
        .subproject(&[".vibe", "lifecycle-outcomes"])?;
    let bytes = home
        .read_file_bounded(
            &format!("{run_id}.admitted-input.json"),
            2 * 1024 * 1024 + 4096,
        )?
        .context("admitted input capsule absent")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    ensure!(
        value["format"] == "vibevm.lifecycle.admitted-input/1",
        "input capsule format"
    );
    for (field, expected) in [("request", request_sha256), ("plan", plan_sha256)] {
        let mut canonical = Vec::new();
        super::lifecycle_plan::write_canonical(&value[field], &mut canonical)?;
        let actual = vibe_lifecycle::process::SelectedFileWitness::from_bytes(canonical);
        // Both the C1 inspection and SelectedFileWitness already include the
        // sha256: prefix. Compare the full digest of the read-back bytes once.
        ensure!(
            expected == actual.sha256,
            "input capsule {field} digest differs from C1"
        );
    }
    Ok(())
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
    script: Option<ScriptFixturePort<'a>>,
    agent: Option<AgentFixturePort<'a>>,
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
        if self.script.as_ref().is_some_and(|s| s.profile.is_some()) {
            let row = SubmissionReservations::inspect_existing_key(
                lease,
                self.namespace,
                self.scope,
                self.key,
            )
            .map_err(|error| error.to_string())?
            .ok_or("C1 absent")?;
            // Pending with no capsule is still Absent and can initialize only
            // the reserved journal; Ready must never bypass capsule proof.
            if row.journal_admission == JournalAdmission::Ready {
                check_admitted_capsule(
                    lease,
                    &row.domain_run_id,
                    &row.request_sha256,
                    &row.plan_sha256,
                )
                .map_err(|error| error.to_string())?;
            }
        }
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
                Err(error) => {
                    // The fixed owner independently re-reads and proves a
                    // distinct terminal predecessor; generic inspection stays
                    // strict and grants no transition authority.
                    match sequential_runs::inspect_previous_terminal(lease, run_id) {
                        Ok(()) => Ok(JournalState::Absent),
                        Err(proof) => Ok(JournalState::Uncertain(format!("{error}; {proof:#}"))),
                    }
                }
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
            if LifecycleStateStore::peek_with_lease(&lease)?.is_some() {
                sequential_runs::inspect_previous_terminal(&lease, run_id)?;
            }
            if self.script.as_ref().is_some_and(|s| s.profile.is_some()) {
                let bytes = admitted_capsule(&self.canonical_request, self.canonical_plan)?;
                let home = lease
                    .publication_journal_project()
                    .ensure_subproject(&[".vibe", "lifecycle-outcomes"])?;
                let directory = home.root_dir()?;
                let name = format!("{run_id}.admitted-input.json");
                match home.read_file_bounded_in(&directory, &name, 2 * 1024 * 1024 + 4096)? {
                    Some(prior) => ensure!(prior == bytes, "input capsule changed"),
                    None => home
                        .publish_new_in(&directory, &name, &bytes)
                        .map_err(|e| e.into_report())?,
                }
                let row = SubmissionReservations::inspect_existing_key(
                    &lease,
                    self.namespace,
                    self.scope,
                    self.key,
                )?
                .context("C1 absent")?;
                check_admitted_capsule(
                    &lease,
                    &row.domain_run_id,
                    &row.request_sha256,
                    &row.plan_sha256,
                )?;
            }
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
                store.state().run.slot_continuation.is_none() && store.delegated_rows().is_empty(),
                "new protected header inherited parked work"
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
            let script_stage = if let Some(script) = &self.script {
                recheck_script_source_witness(&script.admitted.source)?;
                if let Some(ready) = script.ready {
                    ready.admitted(
                        lease.clone(),
                        self.namespace,
                        self.scope,
                        self.key,
                        run_id,
                        &reservation_ref,
                        &self.canonical_request,
                        self.canonical_plan,
                        &script.admitted.source,
                        &self.witness,
                        &self.identity.started,
                    )?;
                    // Notification cannot admit source or identity drift.
                    self.recheck()?;
                    recheck_script_source_witness(&script.admitted.source)?;
                    Self::header(&lease, run_id, Some(&self.identity.started), self.phase)?;
                    if let Some(stopped) = ready.close_gate(&lease)? {
                        return Ok(stopped);
                    }
                }
                let stage = publish_script_source_stage(
                    &lease,
                    self.namespace,
                    self.scope,
                    self.key,
                    run_id,
                    &self.canonical_request,
                    self.canonical_plan,
                    Phase::Validate.as_str(),
                    &script.admitted.execution_key,
                    &script.admitted.provider,
                    &script.admitted.source.provider_root,
                    &script.admitted.source.script_relative,
                )?;
                stage.matches_witness(&script.admitted.source)?;
                if let Some(after_stage) = script.after_stage {
                    after_stage();
                }
                Self::header(&lease, run_id, Some(&self.identity.started), self.phase)?;
                Some(stage)
            } else {
                None
            };
            if let Some(fixture) = &self.fixture {
                if let Some(after_stage) = fixture.after_stage {
                    after_stage();
                }
                recheck_package_tree_witness(&fixture.witness)?;
            }
            if let Some(agent) = &self.agent {
                if let Some(before_run) = agent.before_run {
                    before_run();
                }
                Self::header(&lease, run_id, Some(&self.identity.started), self.phase)?;
                ensure!(
                    matches!(
                        SubmissionReservations::reserve(
                            &lease, self.namespace, self.scope, self.key,
                            &self.canonical_request, self.canonical_plan, run_id,
                        )?,
                        vibe_lifecycle::ReservationDecision::Existing { domain_run_id } if domain_run_id == run_id
                    ),
                    "agent fixture reservation changed before task publication"
                );
                ensure!(
                    SubmissionReservations::journal_admission(
                        &lease,
                        self.namespace,
                        self.scope,
                        self.key,
                        run_id,
                    )? == JournalAdmission::Ready,
                    "agent fixture journal is not ready before task publication"
                );
                recheck_prompt_source_witness(&agent.admitted.source)?;
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
            let mut ports = if self.phase == Phase::Validate {
                validate_ports()
            } else if let Some(factory) = attested_factory.as_ref() {
                install::one_package_fixture_ports(
                    factory,
                    self.fixture.as_ref().and_then(|f| f.confirm),
                )
            } else {
                install::empty_install_ports()
            };
            let live_observer = self
                .script
                .as_ref()
                .and_then(|script| script.ready)
                .map(live_owner::observation::LiveRun::new);
            if let Some(observer) = &live_observer {
                ports.observer = observer;
            }
            if let Some(agent) = &self.agent {
                ports.agent = agent.backend.clone();
            }
            let outcome = if let (Some(script), Some(stage)) = (&self.script, &script_stage) {
                prepared.run_protected_script(
                    ports,
                    self.target_os,
                    self.observed_at,
                    ProtectedScriptExecution {
                        stage,
                        process: script.process,
                        probe: script.probe,
                    },
                )
            } else {
                prepared.run(ports, None, self.target_os, self.observed_at, None)
            };
            if let Some(after_effect) = self.script.as_ref().and_then(|script| script.after_effect)
            {
                after_effect()?;
            }
            if let Some(after_effect) = self.agent.as_ref().and_then(|agent| agent.after_effect) {
                after_effect()?;
            }
            let deferred = self
                .script
                .as_ref()
                .and_then(|script| script.ready)
                .map(|ready| ready.close_deferred_gate())
                .transpose()?
                .flatten();
            match outcome {
                PhaseOutcome::Completed(values) => {
                    let report_bytes = serde_json::to_vec(&values.into_report(None))?;
                    if let Some(id) = deferred.as_deref() {
                        return self
                            .script
                            .as_ref()
                            .and_then(|script| script.ready)
                            .context("deferred owner missing")?
                            .retain_deferred_stop(&lease, id, &report_bytes);
                    }
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
                PhaseOutcome::Parked(values) => {
                    let report_bytes = serde_json::to_vec(&values.into_report(None))?;
                    match create_lifecycle_outcome_marker(
                        run_id,
                        &reservation_ref,
                        MarkerOutcome::Parked,
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
                PhaseOutcome::Failed {
                    measurement:
                        Measurement::Lifecycle {
                            rows,
                            stopped_phase,
                            requested,
                            chain,
                            verification,
                        },
                    ..
                } => {
                    let report_bytes = serde_json::to_vec(
                        &LifecycleValues::failed_with_verification(
                            &requested,
                            chain,
                            &stopped_phase,
                            rows,
                            verification.map(|item| *item),
                        )
                        .into_report(None),
                    )?;
                    match create_lifecycle_outcome_marker(
                        run_id,
                        &reservation_ref,
                        MarkerOutcome::Failed,
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
                PhaseOutcome::Failed { .. } => {
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
