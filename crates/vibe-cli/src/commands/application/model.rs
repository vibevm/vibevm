//! Strict application dispatcher context, reply, and durable index models.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#context");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#reply");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Result, bail};
use vibe_wire::generated::application::e1::{
    context as context_wire, index as index_wire, reply as reply_wire,
};

#[path = "launcher.rs"]
mod launcher;
pub(super) use launcher::{launcher_belongs_to_command, launcher_kind};

pub const CONTEXT_PROTOCOL: &str = "vibe-application-context/1";
pub const RESULT_PROTOCOL: &str = "vibe-application-result/1";
pub const INDEX_PROTOCOL: &str = "vibe-user-applications/2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationOperation {
    Install,
    Update,
    Uninstall,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIdentity {
    pub group: String,
    pub name: String,
    pub version: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationIdentity {
    pub id: String,
    pub package: PackageIdentity,
    pub installer_package: PackageIdentity,
    pub commands: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationContext {
    pub protocol: String,
    pub operation: ApplicationOperation,
    pub application: ApplicationIdentity,
    pub settings_root: PathBuf,
    pub host_root: PathBuf,
    pub registry_root: Option<PathBuf>,
    pub vibe_executable: PathBuf,
    pub offline: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationStatus {
    Ready,
    Undeployed,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    Source,
    Distribution,
    Archive,
    Command,
    Shell,
    Zip,
    TarGz,
    Exe,
    Msi,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagementEntry {
    pub runtime: ManagementRuntime,
    pub entry: PathBuf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagementRuntime {
    Node,
    Builtin,
    Native,
    Command,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReply {
    pub protocol: String,
    pub operation: ApplicationOperation,
    pub application_id: String,
    pub status: ApplicationStatus,
    pub host_root: PathBuf,
    pub app_root: Option<PathBuf>,
    pub management: Option<ManagementEntry>,
    pub commands: Vec<String>,
    pub message: String,
    pub launchers: Vec<ApplicationLauncherOwnership>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationLauncherOwnership {
    pub destination: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationRecord {
    pub application: ApplicationIdentity,
    pub host_root: PathBuf,
    pub app_root: Option<PathBuf>,
    pub install_kind: Option<InstallKind>,
    pub selected_option_id: Option<String>,
    pub selected_platform: Option<String>,
    pub management: ManagementEntry,
    pub status: ApplicationStatus,
    pub provenance: Option<ApplicationProvenance>,
    pub launchers: Vec<ApplicationLauncherOwnership>,
    pub user_launchers: Vec<UserLauncher>,
    pub public_commands: Vec<PublicCommand>,
}

/// A command exposed by this provider, backed by one of its authored launch points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicCommand {
    pub name: String,
    pub launcher: String,
}

/// The durable global choice and owned public shims for one application slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternativeRecord {
    pub global_default: Option<String>,
    /// Names remain reserved after a provider is removed.
    pub reserved_commands: Vec<String>,
    pub runners: Vec<ApplicationLauncherOwnership>,
}

/// An authored launch point. Paths remain templates until launch time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLauncher {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub executable: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
    pub icon: Option<String>,
    pub terminal: bool,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationSourceObservation {
    pub url: String,
    pub tracked_ref: String,
    pub resolved_commit: String,
    pub source_tree: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationSelection {
    Source {
        commit: Option<String>,
        source_tree: Option<String>,
    },
    Binary {
        commit: String,
        source_tree: String,
        os: String,
        arch: String,
        asset_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationProvenance {
    pub available_source: Option<ApplicationSourceObservation>,
    pub selected: ApplicationSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationIndex {
    pub protocol: String,
    pub revision: String,
    /// Installed versions keyed by the exact `group/name@version` coordinate.
    pub applications: BTreeMap<String, ApplicationRecord>,
    /// Slot id to its global default, reserved aliases, and public runner ownership.
    pub alternatives: BTreeMap<String, AlternativeRecord>,
}

impl ApplicationContext {
    pub(crate) fn to_wire(&self) -> context_wire::ApplicationContext {
        context_wire::ApplicationContext {
            protocol: self.protocol.clone(),
            operation: match self.operation {
                ApplicationOperation::Install => context_wire::ApplicationOperation::Install,
                ApplicationOperation::Update => context_wire::ApplicationOperation::Update,
                ApplicationOperation::Uninstall => context_wire::ApplicationOperation::Uninstall,
            },
            application: context_identity(&self.application),
            settings_root: self.settings_root.clone(),
            host_root: self.host_root.clone(),
            registry_root: self.registry_root.clone(),
            vibe_executable: self.vibe_executable.clone(),
            offline: self.offline,
        }
    }
}

impl ApplicationReply {
    pub(crate) fn from_wire(value: reply_wire::ApplicationReply) -> Self {
        Self {
            protocol: value.protocol,
            operation: match value.operation {
                reply_wire::ApplicationOperation::Install => ApplicationOperation::Install,
                reply_wire::ApplicationOperation::Update => ApplicationOperation::Update,
                reply_wire::ApplicationOperation::Uninstall => ApplicationOperation::Uninstall,
            },
            application_id: value.application_id,
            status: match value.status {
                reply_wire::ApplicationStatus::Ready => ApplicationStatus::Ready,
                reply_wire::ApplicationStatus::Undeployed => ApplicationStatus::Undeployed,
                reply_wire::ApplicationStatus::Failed => ApplicationStatus::Failed,
            },
            host_root: value.host_root,
            app_root: value.app_root,
            management: value.management.map(|entry| ManagementEntry {
                runtime: match entry.runtime {
                    reply_wire::ManagementRuntime::Node => ManagementRuntime::Node,
                    reply_wire::ManagementRuntime::Builtin => ManagementRuntime::Builtin,
                    reply_wire::ManagementRuntime::Native => ManagementRuntime::Native,
                    reply_wire::ManagementRuntime::Command => ManagementRuntime::Command,
                },
                entry: entry.entry,
            }),
            commands: value.commands,
            message: value.message,
            launchers: value
                .launchers
                .into_iter()
                .map(|launcher| ApplicationLauncherOwnership {
                    destination: launcher.destination,
                    sha256: launcher.sha256,
                })
                .collect(),
        }
    }
}

impl ApplicationIndex {
    pub(crate) fn to_wire(&self) -> index_wire::ApplicationIndex {
        index_wire::ApplicationIndex {
            protocol: self.protocol.clone(),
            revision: self.revision.clone(),
            applications: self
                .applications
                .iter()
                .map(|(key, record)| (key.clone(), record_to_wire(record)))
                .collect(),
            alternatives: self
                .alternatives
                .iter()
                .map(|(slot, alternative)| {
                    (
                        slot.clone(),
                        index_wire::AlternativeRecord {
                            global_default: alternative.global_default.clone(),
                            reserved_commands: alternative.reserved_commands.clone(),
                            runners: alternative
                                .runners
                                .iter()
                                .map(|runner| index_wire::ApplicationLauncherOwnership {
                                    destination: runner.destination.clone(),
                                    sha256: runner.sha256.clone(),
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn from_wire(value: index_wire::ApplicationIndex) -> Result<Self> {
        Ok(Self {
            protocol: value.protocol,
            revision: value.revision,
            applications: value
                .applications
                .into_iter()
                .map(|(key, record)| Ok((key, record_from_wire(record)?)))
                .collect::<Result<_>>()?,
            alternatives: value
                .alternatives
                .into_iter()
                .map(|(slot, alternative)| {
                    (
                        slot,
                        AlternativeRecord {
                            global_default: alternative.global_default,
                            reserved_commands: alternative.reserved_commands,
                            runners: alternative
                                .runners
                                .into_iter()
                                .map(|runner| ApplicationLauncherOwnership {
                                    destination: runner.destination,
                                    sha256: runner.sha256,
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        })
    }
}

fn context_identity(value: &ApplicationIdentity) -> context_wire::ApplicationIdentity {
    context_wire::ApplicationIdentity {
        id: value.id.clone(),
        package: context_wire::PackageIdentity {
            group: value.package.group.clone(),
            name: value.package.name.clone(),
            version: value.package.version.clone(),
        },
        installer_package: context_wire::PackageIdentity {
            group: value.installer_package.group.clone(),
            name: value.installer_package.name.clone(),
            version: value.installer_package.version.clone(),
        },
        commands: value.commands.clone(),
    }
}

fn index_identity(value: &ApplicationIdentity) -> index_wire::ApplicationIdentity {
    index_wire::ApplicationIdentity {
        id: value.id.clone(),
        package: index_wire::PackageIdentity {
            group: value.package.group.clone(),
            name: value.package.name.clone(),
            version: value.package.version.clone(),
        },
        installer_package: index_wire::PackageIdentity {
            group: value.installer_package.group.clone(),
            name: value.installer_package.name.clone(),
            version: value.installer_package.version.clone(),
        },
        commands: value.commands.clone(),
    }
}

fn domain_identity(value: index_wire::ApplicationIdentity) -> ApplicationIdentity {
    ApplicationIdentity {
        id: value.id,
        package: PackageIdentity {
            group: value.package.group,
            name: value.package.name,
            version: value.package.version,
        },
        installer_package: PackageIdentity {
            group: value.installer_package.group,
            name: value.installer_package.name,
            version: value.installer_package.version,
        },
        commands: value.commands,
    }
}

fn record_to_wire(value: &ApplicationRecord) -> index_wire::ApplicationRecord {
    index_wire::ApplicationRecord {
        application: index_identity(&value.application),
        host_root: value.host_root.clone(),
        app_root: value.app_root.clone(),
        install_kind: value.install_kind.map(|kind| match kind {
            InstallKind::Source => index_wire::InstallKind::Source,
            InstallKind::Distribution => index_wire::InstallKind::Distribution,
            InstallKind::Archive => index_wire::InstallKind::Archive,
            InstallKind::Command => index_wire::InstallKind::Command,
            InstallKind::Shell => index_wire::InstallKind::Shell,
            InstallKind::Zip => index_wire::InstallKind::Zip,
            InstallKind::TarGz => index_wire::InstallKind::TarGz,
            InstallKind::Exe => index_wire::InstallKind::Exe,
            InstallKind::Msi => index_wire::InstallKind::Msi,
        }),
        selected_option_id: value.selected_option_id.clone(),
        selected_platform: value.selected_platform.clone(),
        management: index_wire::ManagementEntry {
            runtime: match value.management.runtime {
                ManagementRuntime::Node => index_wire::ManagementRuntime::Node,
                ManagementRuntime::Builtin => index_wire::ManagementRuntime::Builtin,
                ManagementRuntime::Native => index_wire::ManagementRuntime::Native,
                ManagementRuntime::Command => index_wire::ManagementRuntime::Command,
            },
            entry: value.management.entry.clone(),
        },
        status: match value.status {
            ApplicationStatus::Ready => index_wire::ApplicationStatus::Ready,
            ApplicationStatus::Undeployed => index_wire::ApplicationStatus::Undeployed,
            ApplicationStatus::Failed => index_wire::ApplicationStatus::Failed,
        },
        provenance: value.provenance.as_ref().map(provenance_to_wire),
        launchers: value
            .launchers
            .iter()
            .map(|launcher| index_wire::ApplicationLauncherOwnership {
                destination: launcher.destination.clone(),
                sha256: launcher.sha256.clone(),
            })
            .collect(),
        user_launchers: value
            .user_launchers
            .iter()
            .map(|launcher| index_wire::UserLauncher {
                id: launcher.id.clone(),
                name: launcher.name.clone(),
                description: launcher.description.clone(),
                executable: launcher.executable.clone(),
                args: launcher.args.clone(),
                working_directory: launcher.working_directory.clone(),
                icon: launcher.icon.clone(),
                terminal: launcher.terminal,
                default: launcher.default,
            })
            .collect(),
        public_commands: value
            .public_commands
            .iter()
            .map(|command| index_wire::PublicCommand {
                name: command.name.clone(),
                launcher: command.launcher.clone(),
            })
            .collect(),
    }
}

fn provenance_to_wire(value: &ApplicationProvenance) -> index_wire::ApplicationProvenance {
    let available_source =
        value
            .available_source
            .as_ref()
            .map(|source| index_wire::ApplicationSourceObservation {
                url: source.url.clone(),
                tracked_ref: source.tracked_ref.clone(),
                resolved_commit: source.resolved_commit.clone(),
                source_tree: source.source_tree.clone(),
            });
    let selected = match &value.selected {
        ApplicationSelection::Source {
            commit,
            source_tree,
        } => index_wire::ApplicationSelection {
            mode: index_wire::ApplicationSelectionMode::Source,
            commit: commit.clone(),
            source_tree: source_tree.clone(),
            os: None,
            arch: None,
            asset_sha256: None,
        },
        ApplicationSelection::Binary {
            commit,
            source_tree,
            os,
            arch,
            asset_sha256,
        } => index_wire::ApplicationSelection {
            mode: index_wire::ApplicationSelectionMode::Binary,
            commit: Some(commit.clone()),
            source_tree: Some(source_tree.clone()),
            os: Some(os.clone()),
            arch: Some(arch.clone()),
            asset_sha256: Some(asset_sha256.clone()),
        },
    };
    index_wire::ApplicationProvenance {
        available_source,
        selected,
    }
}

fn record_from_wire(value: index_wire::ApplicationRecord) -> Result<ApplicationRecord> {
    Ok(ApplicationRecord {
        application: domain_identity(value.application),
        host_root: value.host_root,
        app_root: value.app_root,
        install_kind: value.install_kind.map(|kind| match kind {
            index_wire::InstallKind::Source => InstallKind::Source,
            index_wire::InstallKind::Distribution => InstallKind::Distribution,
            index_wire::InstallKind::Archive => InstallKind::Archive,
            index_wire::InstallKind::Command => InstallKind::Command,
            index_wire::InstallKind::Shell => InstallKind::Shell,
            index_wire::InstallKind::Zip => InstallKind::Zip,
            index_wire::InstallKind::TarGz => InstallKind::TarGz,
            index_wire::InstallKind::Exe => InstallKind::Exe,
            index_wire::InstallKind::Msi => InstallKind::Msi,
        }),
        selected_option_id: value.selected_option_id,
        selected_platform: value.selected_platform,
        management: ManagementEntry {
            runtime: match value.management.runtime {
                index_wire::ManagementRuntime::Node => ManagementRuntime::Node,
                index_wire::ManagementRuntime::Builtin => ManagementRuntime::Builtin,
                index_wire::ManagementRuntime::Native => ManagementRuntime::Native,
                index_wire::ManagementRuntime::Command => ManagementRuntime::Command,
            },
            entry: value.management.entry,
        },
        status: match value.status {
            index_wire::ApplicationStatus::Ready => ApplicationStatus::Ready,
            index_wire::ApplicationStatus::Undeployed => ApplicationStatus::Undeployed,
            index_wire::ApplicationStatus::Failed => ApplicationStatus::Failed,
        },
        provenance: value.provenance.map(provenance_from_wire).transpose()?,
        launchers: value
            .launchers
            .into_iter()
            .map(|launcher| ApplicationLauncherOwnership {
                destination: launcher.destination,
                sha256: launcher.sha256,
            })
            .collect(),
        user_launchers: value
            .user_launchers
            .into_iter()
            .map(|launcher| UserLauncher {
                id: launcher.id,
                name: launcher.name,
                description: launcher.description,
                executable: launcher.executable,
                args: launcher.args,
                working_directory: launcher.working_directory,
                icon: launcher.icon,
                terminal: launcher.terminal,
                default: launcher.default,
            })
            .collect(),
        public_commands: value
            .public_commands
            .into_iter()
            .map(|command| PublicCommand {
                name: command.name,
                launcher: command.launcher,
            })
            .collect(),
    })
}

fn provenance_from_wire(value: index_wire::ApplicationProvenance) -> Result<ApplicationProvenance> {
    let available_source = value
        .available_source
        .map(|source| ApplicationSourceObservation {
            url: source.url,
            tracked_ref: source.tracked_ref,
            resolved_commit: source.resolved_commit,
            source_tree: source.source_tree,
        });
    let selected = match value.selected {
        index_wire::ApplicationSelection {
            mode: index_wire::ApplicationSelectionMode::Source,
            commit,
            source_tree,
            os: None,
            arch: None,
            asset_sha256: None,
        } => ApplicationSelection::Source {
            commit,
            source_tree,
        },
        index_wire::ApplicationSelection {
            mode: index_wire::ApplicationSelectionMode::Binary,
            commit: Some(commit),
            source_tree: Some(source_tree),
            os: Some(os),
            arch: Some(arch),
            asset_sha256: Some(asset_sha256),
        } => ApplicationSelection::Binary {
            commit,
            source_tree,
            os,
            arch,
            asset_sha256,
        },
        _ => bail!("application index selection fields do not match its mode"),
    };
    Ok(ApplicationProvenance {
        available_source,
        selected,
    })
}

impl Default for ApplicationIndex {
    fn default() -> Self {
        Self {
            protocol: INDEX_PROTOCOL.into(),
            revision: String::new(),
            applications: BTreeMap::new(),
            alternatives: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests;
