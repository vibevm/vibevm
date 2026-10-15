//! Deterministic lifecycle vocabulary and request chaining for vibe.
//!
//! The default lifecycle is a fixed nine-phase ritual. Requesting a phase
//! includes every phase before it; the independent clean lifecycle can be
//! prepended to that chain.
//!
//! ```
//! use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase};
//!
//! assert_eq!(
//!     LifecycleRequest::Clean {
//!         then: Some(Phase::Build),
//!     }
//!     .steps(),
//!     vec![
//!         LifecycleStep::Clean,
//!         LifecycleStep::Default(Phase::Validate),
//!         LifecycleStep::Default(Phase::Install),
//!         LifecycleStep::Default(Phase::Generate),
//!         LifecycleStep::Default(Phase::Build),
//!     ],
//! );
//! ```
//!
//! The vocabulary remains available from this crate for compatibility even
//! though its single owner is now `vibe_core::lifecycle`:
//!
//! ```
//! use vibe_lifecycle::{CompilePoint, ExtensionPoint, Phase, PhasePoint};
//!
//! let phase = PhasePoint::Default(Phase::Build);
//! let point = ExtensionPoint::Phase(phase);
//! let core_point: vibe_core::lifecycle::ExtensionPoint = point;
//! assert_eq!(core_point.to_string(), "phase:build");
//! assert_eq!("compile:pass".parse(), Ok(CompilePoint::Pass));
//! ```
//!
//! Collection is pure: callers hand the crate an already selected installed
//! world in canonical lock order, then query the retained registry for a
//! subject-specific execution plan.
//!
//! ```
//! use std::path::PathBuf;
//! use vibe_core::{ContentHash, Group, PackageKind, PackageName};
//! use vibe_core::manifest::{ExtensionDecl, ExtensionHandler, ExtensionsControl};
//! use vibe_lifecycle::{
//!     DependencyExtensionSource, DependencyProvider, DependencyProviderId,
//!     ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
//!     SelectorSubject, collect_extensions,
//! };
//!
//! let id = DependencyProviderId::new(
//!     Group::parse("org.demo").unwrap(),
//!     PackageName::parse("tools").unwrap(),
//! );
//! let declaration = ExtensionDecl {
//!     id: "announce".into(),
//!     point: "phase:build".parse().unwrap(),
//!     handler: ExtensionHandler::Builtin { name: "log".into() },
//!     config: None, auto: None, inputs: None, applies_to: None,
//!     compiler_internals: None, pass: None, when: None,
//!     machine_operation_schema: None,
//! };
//! let world = ExtensionWorld {
//!     installed: vec![DependencyExtensionSource {
//!         provider: DependencyProvider {
//!             id, root: PathBuf::from("vibedeps/tools"), version: "1.0.0".into(),
//!             kind: PackageKind::Tool,
//!             content_hash: ContentHash::parse("sha256:aa").unwrap(),
//!         },
//!         declarations: vec![declaration],
//!         controls: ExtensionsControl::default(),
//!         mechanisms: Vec::new(),
//!     }],
//!     host: HostExtensionSource {
//!         provider: HostProvider {
//!             identity: HostIdentity::ungrouped_project("demo"), root: PathBuf::from("."),
//!             version: "0.1.0".into(), kind: None, content_hash: None,
//!         },
//!         declarations: Vec::new(), controls: ExtensionsControl::default(),
//!         mechanisms: Vec::new(),
//!     },
//!     effective_stack: None,
//! };
//! let registry = collect_extensions(world).unwrap();
//! let point = "phase:build".parse().unwrap();
//! assert_eq!(registry.plan(point, SelectorSubject::unscoped()).len(), 1);
//! ```

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

pub mod agent;
pub mod artifacts;
mod chain;
mod delegation;
mod execution;
pub mod handlers;
mod lease;
mod legacy_hooks;
mod mechanism;
pub mod native;
pub use legacy_hooks::SyntheticHookIdentity;
pub mod process;
mod registry;
pub mod retained_mechanism;
mod runner;
mod state;

pub use agent::{
    AgentBackend, AgentCompletion, AgentError, AgentRequest, AgentUsage, NoAgentBackend,
    PreparedAgent, PromptRequest,
};
pub use chain::{
    InvocationIntent, LifecycleRequest, LifecycleStep, inclusive_chain, invocation_chain,
};
pub use delegation::{
    Delegation, DelegationError, OUTBOX_RELATIVE, TASK_CAP, outbox_task_path, task_filename,
};
pub use execution::{
    BuiltinRegistry, ContributionOutcome, DispatchBatch, DispatchError, ExecutionSession,
    HandlerExecution, RunMetadata, SlotTarget,
};
pub use handlers::{
    NativeBackend, NativeBackendRequest, NoNativeBackend, NoPackageBindingBackend,
    PackageBindingArtifact, PackageBindingBackend, PackageBindingOutcome,
};
pub use lease::{LifecycleLease, LifecycleLeaseError};
pub use mechanism::package::{
    StaticFileJournalError, StaticFileJournalInspection, StaticFileObservedFile,
    StaticFileObservedPresence, StaticFilePartialEvidence, inspect_fixed_static_file_package,
    submit_fixed_static_file_package,
};
pub use mechanism::{
    ARTIFACT_RECORD_DIR, BuildError, BuildExecution, BuildOutcome, ClientExecutable,
    ClientExecutables, ClientProjectionError, ClientStateBinding, ClientStateOwner,
    ClientStateWitness, DEPLOY_STATE_DIR, DeployError, DeployExecution, DeployOutcome,
    DeployPlanReport, DeployProviderError, DeployResourcePlan, DeploySelection, DeployStatus,
    DeployedResource, DeploymentRow, LegacyArtifactEvidence, LegacyArtifactRead,
    LegacyArtifactReadError, MechanismError, PackageError, PackageExecution, PackageOutcome,
    PackagedArtifact, PreparedDeployQuery, PreparedDeployRow, ProducedArtifact, RecordError,
    RemovalOutcome, deploy_state_home, execute_bound_deploy_targets, execute_build_targets,
    execute_deploy_targets, execute_package_targets, list_deployments, plan_deploy_targets,
    prepare_deploy_query, prepare_deploy_query_with_client_state_owner, read_legacy_artifact,
    undeploy_targets,
};
// The mechanism PLANE, through the same compatibility door the extension
// plane already uses. The two executors above take a `MechanismRegistry`,
// so the orchestrator that calls them has to be able to collect and name
// one; the kernel type itself is `vibe-extension-registry`'s and is
// re-exported, never copied.
pub use registry::{
    CollectionError, CollectionNotice, ContributionTier, DependencyExtensionSource,
    DependencyProvider, DependencyProviderId, EffectiveManifestKind, ExecutableContribution,
    ExecutablePlan, ExtensionProvider, ExtensionRegistry, ExtensionRegistryRow, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, MechanismRegistry, MechanismRegistryRow,
    MechanismResolutionError, RegistryState, RegistryView, SelectorSubject, SyntheticPresetSource,
    collect_extensions, collect_extensions_with_presets, collect_mechanisms, lane_owner_host,
};
pub use runner::{
    ExecutionReuse, ExecutionTransition, FailedExecutionTransition, InvocationRefusal,
    LifecycleRun, LifecycleRunError, LifecycleRunHandle, REMOVED_DECLARATION,
    REMOVED_SLOT_DECLARATION, UNKNOWN_PROVENANCE, validate_invocation_prefix,
};
pub use state::{
    ExistingSubmissionInspection, FingerprintError, InstallEnteredStage, InstallStageError,
    JournalAdmission, LifecycleOutcomeInspection, LifecycleOutcomeRecoveryReason,
    LifecycleOutcomeStoreError, LifecycleReportPayload, LifecycleRunJournalObservation,
    LifecycleStateError, LifecycleStateStore, LifecycleTasksError, PostPublicationRecovery,
    ReservationDecision, RunIdentity, ScriptSourceStage, ScriptSourceStageError, SubmissionError,
    SubmissionNamespace, SubmissionReservations, SubmissionScope, SupersededTrace,
    create_lifecycle_outcome_marker, fingerprint_execution, fingerprint_execution_with,
    fingerprint_handler_execution, fingerprint_handler_execution_with,
    inspect_reserved_lifecycle_run, inspect_retained_lifecycle_outcome, pending_hosted_tasks,
    preparation_error_fingerprint, preparation_error_fingerprint_for_identity,
    publish_install_entered_stage, publish_script_source_stage, retain_lifecycle_outcome,
    select_run_identity, select_run_identity_for_reservation,
};
pub use vibe_core::lifecycle::{
    CompilePoint, CompilePointParseError, DEFAULT_PHASES, ExtensionPoint, ExtensionPointParseError,
    Phase, PhaseParseError, PhasePoint, PhasePointParseError, SlotPoint, SlotPointParseError,
};

pub use mechanism::generic;

pub use mechanism::client_commands;
