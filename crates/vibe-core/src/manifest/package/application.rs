//! Package-owned user-application declaration.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

mod validation;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::PackageRef;

/// The first user-application runtime vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
pub enum ApplicationRuntime {
    Node,
    Native,
}

/// One package-authored installation method. Selection belongs to the enclosing
/// `[[installer_options]]` row; no platform command is inferred from its name.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstallerMethod {
    Shell {
        app_root: String,
        install: String,
        force_install: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        update: Option<String>,
        uninstall: String,
        force_uninstall: String,
    },
    Zip {
        source: String,
        sha256: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_root: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        runners: Vec<ArchiveRunnerDecl>,
    },
    TarGz {
        source: String,
        sha256: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_root: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        runners: Vec<ArchiveRunnerDecl>,
    },
    Exe {
        source: String,
        sha256: String,
        app_root: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        arguments: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_arguments: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uninstall: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_uninstall: Option<String>,
    },
    Msi {
        source: String,
        sha256: String,
        app_root: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        arguments: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_arguments: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uninstall: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_uninstall: Option<String>,
    },
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRunnerDecl {
    pub path: PathBuf,
    pub name: String,
}

/// One user-facing entry of an installed application. Paths remain templates
/// until launch, when the installed payload and optional project are known.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LauncherDecl {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub executable: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub terminal: bool,
    #[serde(default)]
    pub default: bool,
}

/// A portable command claimed by this installer option. Its launcher is
/// resolved from the same option after installation, so different packages
/// and platforms may use different executables for the same public name.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCommandDecl {
    pub name: String,
    pub launcher: String,
}

/// A named choice for one platform and CPU architecture. Multiple choices
/// may target the same pair; IDs distinguish them for selection and updates.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallerOptionDecl {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub platform_name: String,
    pub arch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_install: Option<String>,
    pub installation: InstallerMethod,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launchers: Vec<LauncherDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_commands: Vec<PublicCommandDecl>,
}

/// Stable release-index locator for platform application distributions.
///
/// Artifact digests deliberately live in the release-owned index rather than
/// in source: publishing a binary must not mutate the commit it describes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#distribution")]
pub struct ApplicationDistributionDecl {
    pub repository: String,
    pub release_tag: String,
    pub index_asset: String,
}

/// A published bridge may delegate its application declaration to a mutable,
/// explicitly named Git branch. Each operation resolves that branch to an
/// immutable commit and records the resulting provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#sources")]
pub struct ApplicationSourceDecl {
    pub kind: ApplicationSourceKind,
    pub url: String,
    pub tracked_ref: String,
    pub registry_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#sources")]
pub enum ApplicationSourceKind {
    Git,
}

/// Strict package-only `[application]` metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
pub struct ApplicationDecl {
    pub id: String,
    pub installer_package: Option<PackageRef>,
    pub runtime: Option<ApplicationRuntime>,
    pub entry: Option<PathBuf>,
    pub commands: Vec<String>,
    pub distribution: Option<ApplicationDistributionDecl>,
}
