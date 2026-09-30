//! Host-issued local target admission. The value carries pinned roots and
//! compares trusted facts; actual effects require the separate locked carrier.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-PHYSICAL-TARGET-FENCE");

use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_safefs::{FileIdentity, Project};
use vibe_wire::behaviour::deploy_records::validate_operation_binding;
use vibe_wire::generated::shared::{
    EnvironmentOperationBinding, EnvironmentOperationBindingRootBinding,
    EnvironmentOperationBindingTargetMode as Mode, EnvironmentOperationBindingTargetOs as Os,
    EnvironmentOperationBindingTargetRole as Role,
};

use super::model::{DeployExecution, deploy_state_home};
use crate::mechanism::vibebin::bound::{BoundDestinationLock, BoundVibeBinError};

/// Facts selected by a trusted host. The nonce is an owner-retained recovery
/// coordinate; neither a path nor a file ID can replace that owner decision.
pub(crate) struct TrustedLocalDeployFacts<'a> {
    pub host_context_id: &'a str,
    pub instance_ref: &'a str,
    pub selected_generation_ref: Option<&'a str>,
    pub principal_ref: &'a str,
    pub admission_epoch: u32,
    pub binding_nonce: &'a str,
    /// Only an explicit trusted-owner decision permits a symlink/reparse
    /// alias fallback. Direct no-follow pinning is the default.
    pub allow_alias_resolution: bool,
    pub target_mode: Mode,
    pub target_role: Role,
    pub target_os: Os,
    pub target_architecture: &'a str,
    /// Owner-supplied target claims, not native ABI/libc discovery.
    pub target_abi: Option<&'a str>,
    pub target_libc: Option<&'a str>,
    pub project_root: &'a Path,
    pub settings_root: &'a Path,
    pub user_home: &'a Path,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LocalAdmissionError {
    #[error("local bound deploy supports only explicit UserProfile mode")]
    UnsupportedMode,
    #[error("local bound deploy supports only produced-tool runtime role")]
    UnsupportedRole,
    #[error("selected target OS or architecture differs from the current host")]
    HostPlatformMismatch,
    #[error("trusted field {field} is invalid")]
    InvalidFacts { field: &'static str },
    #[error("{role} root could not be pinned: {reason}")]
    RootUnavailable { role: &'static str, reason: String },
    #[error("{role} root no longer names the admitted held directory")]
    RootChanged { role: &'static str },
    #[error("execution field {field} differs from host admission")]
    ExecutionMismatch { field: &'static str },
    #[error("request binding differs from the host-issued binding")]
    BindingMismatch,
    #[error(transparent)]
    Destination(#[from] BoundVibeBinError),
    #[error(
        "named root drifted after destination-lock setup ({reason}); .vibe/bin/store infrastructure may have been created"
    )]
    PostLockObservation { reason: String },
}

struct RootWitness {
    role: &'static str,
    supplied: PathBuf,
    held: Project,
    identity: FileIdentity,
    alias_resolved: bool,
    alias_note: Option<String>,
}

fn observed_identity(
    role: &'static str,
    supplied: &Path,
    alias_resolved: bool,
) -> Result<FileIdentity, LocalAdmissionError> {
    let named = if alias_resolved {
        std::fs::canonicalize(supplied).map_err(|_| LocalAdmissionError::RootChanged { role })?
    } else {
        supplied.to_path_buf()
    };
    Project::open(&named)
        .and_then(|value| value.root_identity())
        .map_err(|_| LocalAdmissionError::RootChanged { role })
}

fn is_alias(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Distinguish a real symlink/reparse alias from an unrelated no-follow
/// failure before the trusted owner's opt-in canonicalization is used.
fn has_alias_component(path: &Path) -> Result<bool, std::io::Error> {
    let mut current = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Prefix(value) => current.push(value.as_os_str()),
            Component::RootDir => current.push(std::path::MAIN_SEPARATOR_STR),
            Component::Normal(value) => {
                current.push(value);
                if is_alias(&std::fs::symlink_metadata(&current)?) {
                    return Ok(true);
                }
            }
            Component::CurDir | Component::ParentDir => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "dot component",
                ));
            }
        }
    }
    Ok(false)
}

impl RootWitness {
    fn open(
        role: &'static str,
        supplied: &Path,
        allow_alias_resolution: bool,
    ) -> Result<Self, LocalAdmissionError> {
        let unavailable = |reason: String| LocalAdmissionError::RootUnavailable { role, reason };
        if !supplied.is_absolute() || supplied.to_str().is_none() {
            return Err(unavailable("root must be an absolute UTF-8 path".into()));
        }
        for part in supplied.components() {
            match part {
                Component::Normal(value) => {
                    let value = value
                        .to_str()
                        .ok_or_else(|| unavailable("root component is not UTF-8".into()))?;
                    vibe_safefs::ensure_safe_component(value).map_err(|error| {
                        unavailable(format!("unsafe root component: {error:#}"))
                    })?;
                }
                Component::CurDir | Component::ParentDir => {
                    return Err(unavailable("root contains a dot component".into()));
                }
                Component::Prefix(_) | Component::RootDir => {}
            }
        }
        let (held, alias_resolved, alias_note) = match Project::open(supplied) {
            Ok(project) => (project, false, None),
            Err(primary) => {
                if !allow_alias_resolution {
                    return Err(unavailable(format!(
                        "{primary:#}; alias resolution was not admitted"
                    )));
                }
                if !has_alias_component(supplied).map_err(|error| {
                    unavailable(format!("{primary:#}; inspecting alias: {error}"))
                })? {
                    return Err(unavailable(format!(
                        "{primary:#}; no symlink/reparse alias was observed"
                    )));
                }
                // A trusted owner explicitly supplied a symlinked/reparse home.
                // Resolve once for usability, then pin no-follow and compare
                // the original alias again. This remains a weaker observation.
                let resolved = std::fs::canonicalize(supplied).map_err(|error| {
                    unavailable(format!("{primary:#}; alias resolution: {error}"))
                })?;
                if resolved == supplied {
                    return Err(unavailable(format!("{primary:#}; no alias resolved")));
                }
                let project = Project::open(&resolved)
                    .map_err(|error| unavailable(format!("resolved alias: {error:#}")))?;
                (
                    project,
                    true,
                    Some(format!(
                        "trusted {role} alias used a canonicalized path after no-follow open refused: {primary:#}; named identity is rechecked but hostile-writer CAS is unavailable"
                    )),
                )
            }
        };
        let identity = held
            .root_identity()
            .map_err(|error| unavailable(format!("{error:#}")))?;
        let value = Self {
            role,
            supplied: supplied.to_path_buf(),
            held,
            identity,
            alias_resolved,
            alias_note,
        };
        value.reobserve()?;
        Ok(value)
    }

    fn reobserve(&self) -> Result<(), LocalAdmissionError> {
        if observed_identity(self.role, &self.supplied, self.alias_resolved)? != self.identity {
            return Err(LocalAdmissionError::RootChanged { role: self.role });
        }
        Ok(())
    }
}

fn opaque(domain: &'static [u8], values: &[&str]) -> String {
    let mut hash = Sha256::new();
    hash.update(domain);
    for value in values {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    format!("sha256:{:x}", hash.finalize())
}

fn host_os() -> Option<Os> {
    match std::env::consts::OS {
        "windows" => Some(Os::Windows),
        "linux" => Some(Os::Linux),
        "macos" => Some(Os::Macos),
        _ => None,
    }
}

/// A generated DTO alone cannot be promoted to this non-cloneable carrier.
pub(crate) struct AdmittedLocalDeployBinding {
    binding: EnvironmentOperationBinding,
    project: RootWitness,
    settings: RootWitness,
    home: RootWitness,
}

impl AdmittedLocalDeployBinding {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-PHYSICAL-TARGET-FENCE",
        r = 1
    )]
    pub(crate) fn issue(facts: TrustedLocalDeployFacts<'_>) -> Result<Self, LocalAdmissionError> {
        if facts.target_mode != Mode::UserProfile {
            return Err(LocalAdmissionError::UnsupportedMode);
        }
        if facts.target_role != Role::ProducedToolRuntime {
            return Err(LocalAdmissionError::UnsupportedRole);
        }
        if host_os() != Some(facts.target_os.clone())
            || facts.target_architecture != std::env::consts::ARCH
        {
            return Err(LocalAdmissionError::HostPlatformMismatch);
        }
        for (field, value) in [
            ("host_context_id", facts.host_context_id),
            ("instance_ref", facts.instance_ref),
            ("principal_ref", facts.principal_ref),
            ("binding_nonce", facts.binding_nonce),
        ] {
            if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
                return Err(LocalAdmissionError::InvalidFacts { field });
            }
        }
        if facts.admission_epoch == 0 {
            return Err(LocalAdmissionError::InvalidFacts {
                field: "admission_epoch",
            });
        }
        let project =
            RootWitness::open("project", facts.project_root, facts.allow_alias_resolution)?;
        let settings = RootWitness::open(
            "settings",
            facts.settings_root,
            facts.allow_alias_resolution,
        )?;
        let home = RootWitness::open("user_home", facts.user_home, facts.allow_alias_resolution)?;
        let root_ref = |role: &'static str, path: &Path| -> Result<String, LocalAdmissionError> {
            let path = path
                .to_str()
                .ok_or(LocalAdmissionError::InvalidFacts { field: "root_path" })?;
            Ok(opaque(
                b"vibevm/local-root-ref/e1",
                &[
                    facts.host_context_id,
                    facts.instance_ref,
                    facts.binding_nonce,
                    role,
                    path,
                ],
            ))
        };
        let project_ref = root_ref("project", facts.project_root)?;
        let settings_ref = root_ref("settings", facts.settings_root)?;
        let home_ref = root_ref("user-home", facts.user_home)?;
        let epoch = facts.admission_epoch.to_string();
        let mode = serde_json::to_string(&facts.target_mode).map_err(|_| {
            LocalAdmissionError::InvalidFacts {
                field: "target_mode",
            }
        })?;
        let role = serde_json::to_string(&facts.target_role).map_err(|_| {
            LocalAdmissionError::InvalidFacts {
                field: "target_role",
            }
        })?;
        let os = serde_json::to_string(&facts.target_os)
            .map_err(|_| LocalAdmissionError::InvalidFacts { field: "target_os" })?;
        let binding_id = opaque(
            b"vibevm/local-binding/e1",
            &[
                facts.host_context_id,
                facts.instance_ref,
                facts.selected_generation_ref.unwrap_or(""),
                facts.principal_ref,
                facts.binding_nonce,
                &epoch,
                &mode,
                &role,
                &os,
                facts.target_architecture,
                facts.target_abi.unwrap_or(""),
                facts.target_libc.unwrap_or(""),
                &project_ref,
                &settings_ref,
                &home_ref,
            ],
        );
        let binding = EnvironmentOperationBinding {
            binding_epoch: 1,
            admission_epoch: facts.admission_epoch,
            binding_id,
            host_context_id: facts.host_context_id.into(),
            instance_ref: facts.instance_ref.into(),
            selected_generation_ref: facts.selected_generation_ref.map(str::to_owned),
            target_mode: facts.target_mode,
            target_role: facts.target_role,
            target_os: facts.target_os,
            target_architecture: facts.target_architecture.into(),
            target_abi: facts.target_abi.map(str::to_owned),
            target_libc: facts.target_libc.map(str::to_owned),
            principal_ref: facts.principal_ref.into(),
            storage_binding_ref: settings_ref.clone(),
            root_bindings: vec![
                EnvironmentOperationBindingRootBinding {
                    root_role: "project".into(),
                    capability_ref: project_ref,
                },
                EnvironmentOperationBindingRootBinding {
                    root_role: "settings".into(),
                    capability_ref: settings_ref,
                },
                EnvironmentOperationBindingRootBinding {
                    root_role: "user-home".into(),
                    capability_ref: home_ref,
                },
            ],
        };
        validate_operation_binding(&binding)
            .map_err(|error| LocalAdmissionError::InvalidFacts { field: error.field })?;
        Ok(Self {
            binding,
            project,
            settings,
            home,
        })
    }

    pub(crate) fn binding(&self) -> &EnvironmentOperationBinding {
        &self.binding
    }
    pub(crate) fn project(&self) -> &Project {
        &self.project.held
    }
    pub(crate) fn settings(&self) -> &Project {
        &self.settings.held
    }
    pub(crate) fn user_home(&self) -> &Project {
        &self.home.held
    }
    pub(crate) fn alias_warnings(&self) -> Vec<&str> {
        [&self.project, &self.settings, &self.home]
            .into_iter()
            .filter_map(|root| root.alias_note.as_deref())
            .collect()
    }

    pub(crate) fn reobserve_roots(&self) -> Result<(), LocalAdmissionError> {
        self.project.reobserve()?;
        self.settings.reobserve()?;
        self.home.reobserve()
    }

    /// Body paths and principal are compared before provider selection or
    /// destination-lock infrastructure. No field is inferred from the body.
    pub(crate) fn preflight(
        &self,
        execution: &DeployExecution<'_>,
        selected: &EnvironmentOperationBinding,
        principal: &str,
    ) -> Result<(), LocalAdmissionError> {
        if selected != &self.binding {
            return Err(LocalAdmissionError::BindingMismatch);
        }
        if principal != self.binding.principal_ref {
            return Err(LocalAdmissionError::ExecutionMismatch { field: "principal" });
        }
        for (field, actual, wanted) in [
            (
                "project_root",
                execution.project_root,
                self.project.supplied.as_path(),
            ),
            (
                "settings_root",
                execution.settings_root,
                self.settings.supplied.as_path(),
            ),
            (
                "user_home",
                execution.user_home,
                self.home.supplied.as_path(),
            ),
        ] {
            if actual != wanted {
                return Err(LocalAdmissionError::ExecutionMismatch { field });
            }
        }
        if execution.state_home != deploy_state_home(&self.settings.supplied) {
            return Err(LocalAdmissionError::ExecutionMismatch {
                field: "state_home",
            });
        }
        self.reobserve_roots()
    }

    /// Consume the SAME settings Project into the existing vibe-bin physical
    /// lock. No second lock file is acquired; the provider borrows this owner.
    pub(crate) fn acquire_destination(
        self,
    ) -> Result<LockedLocalDeployBinding, LocalAdmissionError> {
        self.reobserve_roots()?;
        let Self {
            binding,
            project,
            settings,
            home,
        } = self;
        let RootWitness {
            role,
            supplied,
            held,
            identity,
            alias_resolved,
            alias_note,
        } = settings;
        let destination = BoundDestinationLock::acquire(held)?;
        let locked = LockedLocalDeployBinding {
            binding,
            project,
            home,
            settings: NamedRootWitness {
                role,
                supplied,
                identity,
                alias_resolved,
                alias_note,
            },
            destination,
        };
        locked.reobserve_before_commit().map_err(|error| {
            LocalAdmissionError::PostLockObservation {
                reason: error.to_string(),
            }
        })?;
        Ok(locked)
    }
}

struct NamedRootWitness {
    role: &'static str,
    supplied: PathBuf,
    identity: FileIdentity,
    alias_resolved: bool,
    alias_note: Option<String>,
}
impl NamedRootWitness {
    fn reobserve(&self) -> Result<(), LocalAdmissionError> {
        if observed_identity(self.role, &self.supplied, self.alias_resolved)? != self.identity {
            return Err(LocalAdmissionError::RootChanged { role: self.role });
        }
        Ok(())
    }
}

pub(crate) struct LockedLocalDeployBinding {
    binding: EnvironmentOperationBinding,
    project: RootWitness,
    home: RootWitness,
    settings: NamedRootWitness,
    destination: BoundDestinationLock,
}
impl LockedLocalDeployBinding {
    pub(crate) fn binding(&self) -> &EnvironmentOperationBinding {
        &self.binding
    }
    pub(crate) fn project(&self) -> &Project {
        &self.project.held
    }
    pub(crate) fn settings(&self) -> &Project {
        self.destination.settings()
    }
    pub(crate) fn user_home(&self) -> &Project {
        &self.home.held
    }
    pub(crate) fn destination(&self) -> &BoundDestinationLock {
        &self.destination
    }
    pub(crate) fn alias_warnings(&self) -> Vec<&str> {
        [
            self.project.alias_note.as_deref(),
            self.settings.alias_note.as_deref(),
            self.home.alias_note.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
    pub(crate) fn reobserve_before_commit(&self) -> Result<(), LocalAdmissionError> {
        self.project.reobserve()?;
        self.home.reobserve()?;
        self.settings.reobserve()?;
        if self
            .destination
            .settings()
            .root_identity()
            .map_err(|_| LocalAdmissionError::RootChanged { role: "settings" })?
            != self.settings.identity
        {
            return Err(LocalAdmissionError::RootChanged { role: "settings" });
        }
        Ok(())
    }
}
