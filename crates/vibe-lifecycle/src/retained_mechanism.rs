//! Read-only cold attachment to three fixed, retained mechanism owners.
//!
//! The caller supplies a held lifecycle lease and host-retained C.1 token,
//! trusted scope and key. No result variant grants an effect. Deploy also
//! requires a freshly reissued host admission with held physical roots.

use std::path::Path;

use thiserror::Error;
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
use crate::{LifecycleLease, SubmissionError, SubmissionScope};

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
    Ok(match observed {
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
    })
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
/// These reconstruct only a read-only admission for the original fixed local
/// UserProfile/ProducedToolRuntime deployment.
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
        let admitted = AdmittedLocalDeployBinding::issue(TrustedLocalDeployFacts {
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
        .map_err(|error| RetainedMechanismError::Refused(error.to_string()))?;
        Ok(Self { admitted })
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
    Ok(match observed {
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
    })
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
