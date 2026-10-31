//! Package-owned user-application declaration.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

mod step_validation;
mod steps;
mod sugar;
mod template;
mod validation;

pub use steps::{
    AgentMode, ArchiveFormatDecl, Condition, ExternalResult, FeatureDecl, NativeFormatDecl, Need,
    OwnedWord, ResultDecl, StepDecl, StepKind, StepOrigin, StepPhase,
};
pub use sugar::{result_of, steps_of};
pub use template::{StepVar, expand_step_template};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

/// The interpreter that evaluates an authored shell command or recipe script.
/// Absent means the host default: `posix_sh` on Linux/macOS, `windows_cmd`
/// on Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum ShellInterpreter {
    PosixSh,
    WindowsCmd,
    WindowsPowershell,
}

/// One package-authored installation method. Selection belongs to the enclosing
/// `[[installer_options]]` row; no platform command is inferred from its name.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpreter: Option<ShellInterpreter>,
    },
    Zip {
        source: String,
        sha256: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_root: Option<PathBuf>,
    },
    TarGz {
        source: String,
        sha256: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_root: Option<PathBuf>,
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
    SourceBuild {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sources: Vec<SourceInputDecl>,
        recipe: RecipeDecl,
    },
}

impl InstallerMethod {
    /// The stable `installation.type` spelling of this method.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Shell { .. } => "shell",
            Self::Zip { .. } => "zip",
            Self::TarGz { .. } => "tar_gz",
            Self::Exe { .. } => "exe",
            Self::Msi { .. } => "msi",
            Self::SourceBuild { .. } => "source_build",
        }
    }

    /// Owned methods place their payload in a library directory vibe controls;
    /// external methods only register the `app_root` an installer produced.
    pub fn is_owned(&self) -> bool {
        matches!(
            self,
            Self::Zip { .. } | Self::TarGz { .. } | Self::SourceBuild { .. }
        )
    }
}

/// One `[[installation.sources]]` input of a `source_build` method.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#source-build")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInputDecl {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<bool>,
}

/// The `installation.recipe` of a `source_build` method: a package-relative
/// script run by the selected interpreter.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#source-build")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDecl {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<ShellInterpreter>,
    pub script: PathBuf,
}

/// One user-facing entry of an installed application. Paths remain templates
/// until launch, when the installed payload and optional project are known.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launch-points")]
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
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launch-points")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCommandDecl {
    pub name: String,
    pub launcher: String,
}

/// A named choice for one platform and CPU architecture. Multiple choices
/// may target the same pair; IDs distinguish them for selection and updates.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
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
    pub portable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_install: Option<String>,
    /// The single-method short spelling (`PROP-065#sugar`); exclusive with
    /// `steps`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installation: Option<InstallerMethod>,
    /// What the steps produce (`PROP-065#result`); only with `steps`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<FeatureDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launchers: Vec<LauncherDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_commands: Vec<PublicCommandDecl>,
}

/// Strict package-only `[application]` metadata: the slot name and the
/// package-wide portability default.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDecl {
    pub id: String,
    #[serde(
        default = "portable_default",
        skip_serializing_if = "is_portable_default"
    )]
    pub portable: bool,
}

fn portable_default() -> bool {
    true
}

fn is_portable_default(value: &bool) -> bool {
    *value
}
