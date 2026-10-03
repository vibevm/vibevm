//! Narrow submission and cold attachment to fixed retained mechanism owners.
//!
//! The caller supplies a held lifecycle lease and host-retained C.1 token,
//! trusted scope and key. No inspection result grants an effect. Deploy
//! submission consumes a separate source-bound host admission; cold inspection
//! requires a freshly reissued read-only grant with held physical roots.

use std::path::Path;

use chrono::Utc;
use thiserror::Error;
use vibe_core::manifest::{Manifest, MechanismKey, MechanismRoutes, TargetOs};
use vibe_extension_registry::MechanismRegistry;
use vibe_safefs::StableFileSnapshot;
use vibe_wire::generated::deploy_receipt::DeployReceipt;
use vibe_wire::generated::shared::{
    EnvironmentOperationBindingTargetMode as Mode, EnvironmentOperationBindingTargetOs as Os,
    EnvironmentOperationBindingTargetRole as Role,
};

use crate::mechanism::build::journal::{self as build, BuildJournalError, BuildJournalInspection};
use crate::mechanism::deploy::binding::{AdmittedLocalDeployBinding, TrustedLocalDeployFacts};
use crate::mechanism::deploy::bridge::{
    self as deploy, BoundDeployBridgeError, BoundDeployBridgeInspection,
};
use crate::mechanism::package::journal::{
    self as package, StaticFileJournalError, StaticFileJournalInspection,
};
use crate::{
    BuildExecution, DeployExecution, LifecycleLease, SubmissionError, SubmissionNamespace,
    SubmissionReservations, SubmissionScope,
};

/// Historical status only. In particular there is no fresh Ready/dispatch
/// variant, and inspecting a prior key never invokes its provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetainedInspection<Completion, Partial> {
    Absent,
    Retired { run_id: Option<String> },
    RecoveryRequired { run_id: String },
    Uncertain { run_id: String, partial: Partial },
    Completed { run_id: String, result: Completion },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedPresence {
    Absent,
    Present,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedFile {
    Absent,
    Present { sha256: String, bytes: u64 },
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCompletion {
    pub output_relative: String,
    pub output_sha256: String,
    pub artifact_record: String,
    pub warning: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPartial {
    pub target_root: ObservedPresence,
    pub cargo_home: ObservedPresence,
    pub cargo_lock: ObservedFile,
    pub artifact_record: ObservedFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCompletion {
    pub output_sha256: String,
    pub artifact_record: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePartial {
    pub output_directory: ObservedPresence,
    pub output_file: ObservedFile,
    pub artifact_record: ObservedFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployCompletion {
    pub receipt: DeployReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployPartial {
    pub intent: String,
    pub receipt: String,
    pub sidecar: String,
}

pub type BuildInspection = RetainedInspection<BuildCompletion, BuildPartial>;
pub type PackageInspection = RetainedInspection<PackageCompletion, PackagePartial>;
pub type DeployInspection = RetainedInspection<DeployCompletion, DeployPartial>;

#[derive(Debug, Error)]
pub enum RetainedMechanismError {
    #[error(transparent)]
    Submission(#[from] SubmissionError),
    #[error("retained mechanism inspection refused: {0}")]
    Refused(String),
}

/// A failed fixed Cargo submission retains both the original refusal and the
/// owner's best available observation. Once an owner run exists, failure to
/// finish this call does not prove that Cargo was never started.
#[derive(Debug, Error)]
#[error("fixed Cargo submission failed: {source}")]
pub struct FixedCargoSubmitError {
    #[source]
    pub source: RetainedMechanismError,
    pub owner_observation: Option<BuildInspection>,
    pub effects_may_have_begun: bool,
}

fn build_error(error: BuildJournalError) -> RetainedMechanismError {
    match error {
        BuildJournalError::Submission(error) => error.into(),
        other => RetainedMechanismError::Refused(other.to_string()),
    }
}

fn package_error(error: StaticFileJournalError) -> RetainedMechanismError {
    match error {
        StaticFileJournalError::Submission(error) => error.into(),
        other => RetainedMechanismError::Refused(other.to_string()),
    }
}

fn deploy_error(error: BoundDeployBridgeError) -> RetainedMechanismError {
    match error {
        BoundDeployBridgeError::Submission(error) => error.into(),
        other => RetainedMechanismError::Refused(other.to_string()),
    }
}

fn build_presence(value: build::ObservedPresence) -> ObservedPresence {
    match value {
        build::ObservedPresence::Absent => ObservedPresence::Absent,
        build::ObservedPresence::Present => ObservedPresence::Present,
        build::ObservedPresence::Unavailable(reason) => ObservedPresence::Unavailable(reason),
    }
}

fn package_presence(value: package::ObservedPresence) -> ObservedPresence {
    match value {
        package::ObservedPresence::Absent => ObservedPresence::Absent,
        package::ObservedPresence::Present => ObservedPresence::Present,
        package::ObservedPresence::Unavailable(reason) => ObservedPresence::Unavailable(reason),
    }
}

fn build_file(value: build::ObservedFile) -> ObservedFile {
    match value {
        build::ObservedFile::Absent => ObservedFile::Absent,
        build::ObservedFile::Present { sha256, bytes } => ObservedFile::Present { sha256, bytes },
        build::ObservedFile::Unavailable(reason) => ObservedFile::Unavailable(reason),
    }
}

fn package_file(value: package::ObservedFile) -> ObservedFile {
    match value {
        package::ObservedFile::Absent => ObservedFile::Absent,
        package::ObservedFile::Present { sha256, bytes } => ObservedFile::Present { sha256, bytes },
        package::ObservedFile::Unavailable(reason) => ObservedFile::Unavailable(reason),
    }
}

/// Inspect the fixed offline Cargo owner. The lease must pin the original
/// root; its capsule also checks a same-host physical root correlation token.
pub fn inspect_fixed_cargo_build(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<BuildInspection, RetainedMechanismError> {
    let observed =
        build::inspect_fixed_cargo_cold(lease, namespace_token, scope, key).map_err(build_error)?;
    Ok(map_build_inspection(observed))
}

/// Submit the one fixed offline Cargo fixture through its existing journal
/// owner. The host supplies a held lease, issued namespace, trusted scope,
/// and already collected mechanism plane. Existing keys only inspect the
/// retained first-owner result and never run Cargo again.
pub fn submit_fixed_cargo_build(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
) -> Result<BuildInspection, FixedCargoSubmitError> {
    submit_fixed_cargo_build_with_owner_call(
        lease,
        namespace,
        scope,
        key,
        registry,
        routes,
        |execution| build::execute_fixed_cargo(lease, namespace, scope, key, execution),
    )
}

/// The private owner-call seam lets the journal's Ready/error boundary be
/// exercised without a Cargo subprocess. Production always supplies the
/// incumbent journal executor above.
pub(crate) fn submit_fixed_cargo_build_with_owner_call(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
    owner_call: impl FnOnce(&BuildExecution<'_>) -> Result<BuildJournalInspection, BuildJournalError>,
) -> Result<BuildInspection, FixedCargoSubmitError> {
    let targets = [build::fixed_target()];
    // The journal records this value only for a new owner. Existing-key
    // inspection ignores this retry clock and uses the retained first plan.
    let observed_at = Utc::now().to_rfc3339();
    let execution = BuildExecution {
        project_root: lease.root(),
        targets: &targets,
        registry,
        routes,
        build_root: BuildExecution::default_build_root(),
        offline: true,
        created_at: &observed_at,
    };
    let observed = owner_call(&execution).map_err(|error| {
        let source = build_error(error);
        let owner_observation = inspect_fixed_cargo_build(lease, namespace.id(), scope, key).ok();
        // The typed foreign-scope refusal is issued before this caller
        // can reserve an owner run; inspecting under that foreign scope
        // correctly refuses too. All other inspection failures stay
        // conservatively may-effect.
        let foreign_scope = matches!(
            &source,
            RetainedMechanismError::Submission(SubmissionError::ForeignScope)
        );
        let effects_may_have_begun = !foreign_scope
            && !matches!(
                owner_observation.as_ref(),
                Some(RetainedInspection::Absent | RetainedInspection::Retired { run_id: None })
            );
        FixedCargoSubmitError {
            source,
            owner_observation,
            effects_may_have_begun,
        }
    })?;
    Ok(map_build_inspection(Some(observed)))
}

fn map_build_inspection(observed: Option<BuildJournalInspection>) -> BuildInspection {
    match observed {
        None => RetainedInspection::Absent,
        Some(BuildJournalInspection::Retired { run_id }) => RetainedInspection::Retired { run_id },
        Some(BuildJournalInspection::RecoveryRequired { run_id })
        | Some(BuildJournalInspection::IntentReady { run_id }) => {
            RetainedInspection::RecoveryRequired { run_id }
        }
        Some(BuildJournalInspection::Uncertain { run_id, partial }) => {
            RetainedInspection::Uncertain {
                run_id,
                partial: BuildPartial {
                    target_root: build_presence(partial.target_root),
                    cargo_home: build_presence(partial.cargo_home),
                    cargo_lock: build_file(partial.cargo_lock),
                    artifact_record: build_file(partial.artifact_record),
                },
            }
        }
        Some(BuildJournalInspection::Completed {
            run_id,
            output_relative,
            output_sha256,
            artifact_record,
            warning,
        }) => RetainedInspection::Completed {
            run_id,
            result: BuildCompletion {
                output_relative,
                output_sha256,
                artifact_record,
                warning,
            },
        },
    }
}

/// Inspect the fixed local static-file owner without reading or copying its
/// current source. A missing or drifted result stays partial.
pub fn inspect_fixed_static_file_package(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<PackageInspection, RetainedMechanismError> {
    let observed = package::inspect_fixed_static_file_cold(lease, namespace_token, scope, key)
        .map_err(package_error)?;
    Ok(match observed {
        None => RetainedInspection::Absent,
        Some(StaticFileJournalInspection::Retired { run_id }) => {
            RetainedInspection::Retired { run_id }
        }
        Some(StaticFileJournalInspection::RecoveryRequired { run_id })
        | Some(StaticFileJournalInspection::IntentReady { run_id }) => {
            RetainedInspection::RecoveryRequired { run_id }
        }
        Some(StaticFileJournalInspection::Uncertain { run_id, partial }) => {
            RetainedInspection::Uncertain {
                run_id,
                partial: PackagePartial {
                    output_directory: package_presence(partial.output_directory),
                    output_file: package_file(partial.output_file),
                    artifact_record: package_file(partial.artifact_record),
                },
            }
        }
        Some(StaticFileJournalInspection::Completed {
            run_id,
            output_sha256,
            artifact_record,
        }) => RetainedInspection::Completed {
            run_id,
            result: PackageCompletion {
                output_sha256,
                artifact_record,
            },
        },
    })
}

/// Facts kept by the trusted host, never parsed from a request or capsule.
/// These identify the original fixed local UserProfile/ProducedToolRuntime
/// deployment. Values alone never grant either inspection or effects.
#[derive(Clone, Copy)]
pub struct DeployColdHostFacts<'a> {
    pub host_context_id: &'a str,
    pub instance_ref: &'a str,
    pub selected_generation_ref: Option<&'a str>,
    pub principal_ref: &'a str,
    pub admission_epoch: u32,
    pub binding_nonce: &'a str,
    pub allow_alias_resolution: bool,
    pub target_abi: Option<&'a str>,
    pub target_libc: Option<&'a str>,
    pub project_root: &'a Path,
    pub settings_root: &'a Path,
    pub user_home: &'a Path,
}

/// A host-reissued read capability. Its private carrier cannot be converted
/// by API callers into a destination lock or a deploy effect permit.
pub struct ReadOnlyDeployAdmission {
    admitted: AdmittedLocalDeployBinding,
}

impl ReadOnlyDeployAdmission {
    pub fn reissue(facts: DeployColdHostFacts<'_>) -> Result<Self, RetainedMechanismError> {
        Ok(Self {
            admitted: issue_deploy_binding(&facts)?,
        })
    }
}

fn issue_deploy_binding(
    facts: &DeployColdHostFacts<'_>,
) -> Result<AdmittedLocalDeployBinding, RetainedMechanismError> {
    let target_os = match std::env::consts::OS {
        "linux" => Os::Linux,
        "macos" => Os::Macos,
        "windows" => Os::Windows,
        other => {
            return Err(RetainedMechanismError::Refused(format!(
                "unsupported host OS {other}"
            )));
        }
    };
    AdmittedLocalDeployBinding::issue(TrustedLocalDeployFacts {
        host_context_id: facts.host_context_id,
        instance_ref: facts.instance_ref,
        selected_generation_ref: facts.selected_generation_ref,
        principal_ref: facts.principal_ref,
        admission_epoch: facts.admission_epoch,
        binding_nonce: facts.binding_nonce,
        allow_alias_resolution: facts.allow_alias_resolution,
        target_mode: Mode::UserProfile,
        target_role: Role::ProducedToolRuntime,
        target_os,
        target_architecture: std::env::consts::ARCH,
        target_abi: facts.target_abi,
        target_libc: facts.target_libc,
        project_root: facts.project_root,
        settings_root: facts.settings_root,
        user_home: facts.user_home,
    })
    .map_err(|error| RetainedMechanismError::Refused(error.to_string()))
}

const DEPLOY_MANIFEST_CAP: usize = 1024 * 1024;

/// A non-cloneable trusted-host permit bound to one key. A fresh key carries an
/// authored source witness for one effect; a reserved key can only cold-inspect
/// its retained capsule. Wire data cannot replace roots, profile, target or C1.
pub struct FixedDeployHostAdmission<'a> {
    lease: &'a LifecycleLease,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: String,
    execution: &'a DeployExecution<'a>,
    facts: DeployColdHostFacts<'a>,
    source: Option<StableFileSnapshot>,
    admitted: AdmittedLocalDeployBinding,
}

fn authored_fixed_deploy(
    lease: &LifecycleLease,
    execution: &DeployExecution<'_>,
) -> Result<StableFileSnapshot, RetainedMechanismError> {
    let source = lease
        .project()
        .read_file_snapshot_bounded("vibe.toml", DEPLOY_MANIFEST_CAP)
        .map_err(|error| RetainedMechanismError::Refused(error.to_string()))?
        .ok_or_else(|| {
            RetainedMechanismError::Refused("authored deploy manifest is absent".into())
        })?;
    let text = std::str::from_utf8(&source.bytes).map_err(|error| {
        RetainedMechanismError::Refused(format!("deploy manifest is not UTF-8: {error}"))
    })?;
    let manifest = Manifest::parse_str(text).map_err(|error| {
        RetainedMechanismError::Refused(format!("authored deploy manifest: {error}"))
    })?;
    let section = manifest.deploy.as_ref().ok_or_else(|| {
        RetainedMechanismError::Refused("authored deploy section is absent".into())
    })?;
    let os = TargetOs::current()
        .ok_or_else(|| RetainedMechanismError::Refused("host deploy OS is unsupported".into()))?;
    let projected = section
        .project_profile(&execution.selection.profile, os)
        .map_err(|error| {
            RetainedMechanismError::Refused(format!("authored deploy profile: {error}"))
        })?;
    if projected.active_targets() != execution.selection.targets.as_slice()
        || projected.active_targets().len() != 1
        || section.targets.len() != 1
        || execution.targets != section.targets.as_slice()
    {
        return Err(RetainedMechanismError::Refused(
            "host deploy selection or target differs from the authored profile/target".into(),
        ));
    }
    let target = &section.targets[0];
    let fixed_mechanism: MechanismKey = "deploy:vibe-bin".parse().expect("fixed deploy key");
    if target.mechanism != fixed_mechanism
        || target.when.is_some()
        || target
            .depends_on
            .as_ref()
            .is_some_and(|deps| !deps.is_empty())
    {
        return Err(RetainedMechanismError::Refused(
            "authored fixed deploy requires one unconditional independent vibe-bin target".into(),
        ));
    }
    Ok(source)
}

impl<'a> FixedDeployHostAdmission<'a> {
    /// The cooperative host supplies a held lease, C1 authority and physical
    /// roots. A captured authored manifest independently validates its plan.
    pub fn issue_from_trusted_host(
        lease: &'a LifecycleLease,
        namespace: &'a SubmissionNamespace,
        scope: &'a SubmissionScope,
        key: &str,
        execution: &'a DeployExecution<'a>,
        facts: DeployColdHostFacts<'a>,
        current_generation: &mut dyn FnMut() -> Result<Option<String>, crate::DeployError>,
    ) -> Result<Self, RetainedMechanismError> {
        lease
            .ensure_root(execution.project_root, "at fixed deploy admission")
            .map_err(|error| RetainedMechanismError::Refused(error.to_string()))?;
        if execution.project_root != facts.project_root
            || execution.settings_root != facts.settings_root
            || execution.user_home != facts.user_home
            || execution.state_home != crate::deploy_state_home(facts.settings_root)
            || scope.context != facts.host_context_id
            || scope.target != facts.instance_ref
            || scope.principal != facts.principal_ref
            || execution.package.is_some()
        {
            return Err(RetainedMechanismError::Refused(
                "fixed deploy roots, state home, package or C1 scope differ from trusted host"
                    .into(),
            ));
        }
        SubmissionReservations::validate_namespace(lease, namespace, scope)?;
        // An existing C1 key is read through its retained capsule, not the
        // present manifest. This also permits cold recovery after source drift.
        let existing = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?;
        let source = if existing.is_some() {
            None
        } else {
            Some(authored_fixed_deploy(lease, execution)?)
        };
        let admitted = issue_deploy_binding(&facts)?;
        if source.is_some() {
            let current = current_generation().map_err(|error| {
                RetainedMechanismError::Refused(format!("current deploy generation: {error}"))
            })?;
            if current.as_deref() != facts.selected_generation_ref {
                return Err(RetainedMechanismError::Refused(
                    "current deploy generation differs from host admission".into(),
                ));
            }
        }
        Ok(Self {
            lease,
            namespace,
            scope,
            key: key.into(),
            execution,
            facts,
            source,
            admitted,
        })
    }
}

/// An unsuccessful call retains the owner's actual run/partial truth when it
/// can be reobserved. Missing observation is never proof of no effects.
#[derive(Debug, Error)]
#[error("fixed bound deploy submission failed: {source}")]
pub struct FixedDeploySubmitError {
    #[source]
    pub source: RetainedMechanismError,
    pub known_run_id: Option<String>,
    pub owner_observation: Option<DeployInspection>,
    pub effects_may_have_begun: bool,
}

/// Submit only the captured, host-admitted one-target bound vibe-bin plan.
/// The incumbent deploy bridge is the sole C1 reservation/effect journal.
pub fn submit_fixed_bound_vibe_bin(
    admission: FixedDeployHostAdmission<'_>,
    key: &str,
    current_generation: &mut dyn FnMut() -> Result<Option<String>, crate::DeployError>,
) -> Result<DeployInspection, FixedDeploySubmitError> {
    let FixedDeployHostAdmission {
        lease,
        namespace,
        scope,
        key: captured_key,
        execution,
        facts,
        source,
        admitted,
    } = admission;
    if key != captured_key {
        return Err(FixedDeploySubmitError {
            source: RetainedMechanismError::Refused("fixed deploy permit key differs".into()),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: false,
        });
    }
    // A reserved key belongs to its original owner capsule. Current manifest
    // bytes cannot turn a same-key attach into a fresh effect or erase the
    // retained result; only a new key needs the fresh source witness below.
    let existing = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
        .map_err(|error| FixedDeploySubmitError {
            source: error.into(),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: true,
        })?;
    if let Some(existing) = existing {
        return match deploy::inspect_host_admitted_bound_vibe_bin_cold(
            lease,
            namespace.id(),
            scope,
            key,
            &admitted,
        ) {
            Ok(Some(observed)) => Ok(map_deploy_observation(Some(observed))),
            Ok(None) => Err(FixedDeploySubmitError {
                source: RetainedMechanismError::Refused(
                    "reserved deploy key has no cold owner observation".into(),
                ),
                known_run_id: Some(existing.domain_run_id),
                owner_observation: None,
                effects_may_have_begun: true,
            }),
            Err(error) => Err(FixedDeploySubmitError {
                source: deploy_error(error),
                known_run_id: Some(existing.domain_run_id),
                owner_observation: None,
                effects_may_have_begun: true,
            }),
        };
    }
    let Some(source) = source else {
        return Err(FixedDeploySubmitError {
            source: RetainedMechanismError::Refused(
                "existing deploy key lost its retained C1 reservation".into(),
            ),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: true,
        });
    };
    let changed = lease
        .project()
        .read_file_snapshot_bounded("vibe.toml", DEPLOY_MANIFEST_CAP)
        .map_err(|error| error.to_string());
    if !matches!(changed, Ok(Some(ref current)) if *current == source) {
        return Err(FixedDeploySubmitError {
            source: RetainedMechanismError::Refused(
                "authored deploy manifest changed before submit".into(),
            ),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: false,
        });
    }
    let binding = admitted.binding().clone();
    match deploy::submit_host_admitted_bound_vibe_bin(
        lease,
        namespace,
        scope,
        key,
        execution,
        &binding,
        admitted,
        facts.principal_ref,
        current_generation,
    ) {
        Ok(observed) => Ok(map_deploy_observation(Some(observed))),
        Err(error) => {
            let known_run_id =
                SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
                    .ok()
                    .flatten()
                    .map(|row| row.domain_run_id);
            let owner_observation = issue_deploy_binding(&facts)
                .ok()
                .and_then(|fresh| {
                    deploy::inspect_host_admitted_bound_vibe_bin_cold(
                        lease,
                        namespace.id(),
                        scope,
                        key,
                        &fresh,
                    )
                    .ok()
                })
                .map(map_deploy_observation);
            let known_run_id = known_run_id.or_else(|| match owner_observation.as_ref() {
                Some(
                    DeployInspection::Completed { run_id, .. }
                    | DeployInspection::Uncertain { run_id, .. }
                    | DeployInspection::RecoveryRequired { run_id },
                ) => Some(run_id.clone()),
                Some(DeployInspection::Retired { run_id }) => run_id.clone(),
                Some(DeployInspection::Absent) | None => None,
            });
            Err(FixedDeploySubmitError {
                source: deploy_error(error),
                known_run_id,
                owner_observation,
                effects_may_have_begun: true,
            })
        }
    }
}

/// Inspect the fixed bound vibe-bin owner through the freshly reissued held
/// project/settings/home grant. Capsule path text never opens a Project.
pub fn inspect_bound_vibe_bin_deploy(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    admission: &ReadOnlyDeployAdmission,
) -> Result<DeployInspection, RetainedMechanismError> {
    let observed = deploy::inspect_host_admitted_bound_vibe_bin_cold(
        lease,
        namespace_token,
        scope,
        key,
        &admission.admitted,
    )
    .map_err(deploy_error)?;
    Ok(map_deploy_observation(observed))
}

fn map_deploy_observation(observed: Option<BoundDeployBridgeInspection>) -> DeployInspection {
    match observed {
        None => RetainedInspection::Absent,
        Some(BoundDeployBridgeInspection::Retired { run_id }) => {
            RetainedInspection::Retired { run_id }
        }
        Some(BoundDeployBridgeInspection::RecoveryRequired { run_id }) => {
            RetainedInspection::RecoveryRequired { run_id }
        }
        Some(BoundDeployBridgeInspection::Uncertain { run_id, partial }) => {
            RetainedInspection::Uncertain {
                run_id,
                partial: DeployPartial {
                    intent: partial.intent,
                    receipt: partial.receipt,
                    sidecar: partial.sidecar,
                },
            }
        }
        Some(BoundDeployBridgeInspection::Completed {
            run_id, receipt, ..
        }) => RetainedInspection::Completed {
            run_id,
            result: DeployCompletion { receipt },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ReservationDecision, SubmissionReservations};

    fn scope() -> SubmissionScope {
        SubmissionScope {
            context: "host-context".into(),
            target: "instance-a".into(),
            principal: "owner".into(),
        }
    }

    #[test]
    fn cold_public_facade_retains_pending_run_and_refuses_foreign_scope() {
        let root = tempfile::tempdir().unwrap();
        let settings = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let trusted = scope();
        let (token, runs) = {
            let lease = LifecycleLease::acquire(root.path()).unwrap();
            let namespace =
                SubmissionReservations::issue_namespace(&lease, &trusted, 60_000).unwrap();
            let mut runs = Vec::new();
            for key in ["build", "package", "deploy"] {
                let run_id = crate::process::propose_run_id(lease.root()).unwrap();
                assert!(matches!(
                    SubmissionReservations::reserve(
                        &lease, &namespace, &trusted, key, b"request", b"plan", &run_id,
                    )
                    .unwrap(),
                    ReservationDecision::Created { .. }
                ));
                runs.push(run_id);
            }
            (namespace.id().to_owned(), runs)
        };
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        assert_eq!(
            inspect_fixed_cargo_build(&lease, &token, &trusted, "build").unwrap(),
            BuildInspection::RecoveryRequired {
                run_id: runs[0].clone()
            }
        );
        assert_eq!(
            inspect_fixed_static_file_package(&lease, &token, &trusted, "package").unwrap(),
            PackageInspection::RecoveryRequired {
                run_id: runs[1].clone()
            }
        );
        let grant = ReadOnlyDeployAdmission::reissue(DeployColdHostFacts {
            host_context_id: "host-context",
            instance_ref: "instance-a",
            selected_generation_ref: Some("generation-a"),
            principal_ref: "owner",
            admission_epoch: 1,
            binding_nonce: "nonce-a",
            allow_alias_resolution: false,
            target_abi: None,
            target_libc: None,
            project_root: root.path(),
            settings_root: settings.path(),
            user_home: home.path(),
        })
        .unwrap();
        assert_eq!(
            inspect_bound_vibe_bin_deploy(&lease, &token, &trusted, "deploy", &grant).unwrap(),
            DeployInspection::RecoveryRequired {
                run_id: runs[2].clone()
            }
        );
        let foreign = SubmissionScope {
            principal: "foreign".into(),
            ..trusted.clone()
        };
        assert!(matches!(
            inspect_fixed_cargo_build(&lease, &token, &foreign, "build"),
            Err(RetainedMechanismError::Submission(
                SubmissionError::ForeignScope
            ))
        ));
        assert!(!root.path().join("target").exists());
        assert!(!settings.path().join("bin").exists());
        assert!(!settings.path().join("store").exists());
    }
}
