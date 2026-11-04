//! Package-owned user-application declaration.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

mod environment;
mod icon_files;
mod launcher;
mod launcher_validation;
mod requirement;
mod site;
mod step_validation;
mod steps;
mod sugar;
mod template;
mod validation;
mod wizard;
mod wizard_validation;

pub use environment::{StepEnvironment, StepEnvironmentError, step_env_name};
pub use icon_files::{IconFinding, icon_findings};
pub use launcher::{
    ARG_PLACEHOLDERS, IconDecl, IconSet, IconSlot, LauncherActionDecl, LauncherKind,
    LauncherSource, MENU_CATEGORIES, png_dimensions,
};
pub use requirement::{
    RequirementDecl, RequirementScope, RequirementTable, application_requirements,
};
pub use steps::{
    AgentMode, ArchiveFormatDecl, Condition, ExternalResult, FeatureDecl, NativeFormatDecl, Need,
    OwnedWord, ParamEquals, ResultDecl, StepDecl, StepKind, StepOrigin, StepPhase,
};
pub use sugar::{result_of, steps_of};
pub use template::{StepVar, expand_step_template};
pub use wizard::{
    ChoiceDecl, LicenseDecl, ParameterDecl, ParameterType, ParameterValue, ProfileDecl,
};

use std::collections::BTreeMap;
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
    /// The declared size in bytes (`PROP-065#sizes`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
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
///
/// `name`, `executable` and `kind` are required, except in a row of an
/// option with `launchers_from` that overrides a launcher derived from a
/// desktop entry field by field (`PROP-059#desktop-entries`); an installed
/// row is always complete.
///
/// ```
/// use vibe_core::manifest::{LauncherDecl, LauncherKind};
///
/// let row: LauncherDecl = toml::from_str(
///     "id = \"edit\"\nname = \"Editor\"\nexecutable = \"bin/edit\"\nkind = \"tui\"",
/// )
/// .unwrap();
/// assert_eq!(row.effective_kind(), LauncherKind::Tui);
/// assert!(row.is_terminal() && !row.shows_in_menu());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launch-points")]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LauncherDecl {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub executable: String,
    /// `gui`, `tui` or `command`, declared and never inferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<LauncherKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    /// Derived from `kind`; when spelled it must agree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<bool>,
    #[serde(default)]
    pub default: bool,
    /// Listed in a graphical client's start menu (`PROP-059#menu`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generic_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_class: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mime_types: Vec<String>,
    /// Localised names by language tag (`Name[xx]`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub names: BTreeMap<String, String>,
    /// Localised descriptions by language tag (`Comment[xx]`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub descriptions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<LauncherActionDecl>,
    /// Authored or derived from a desktop entry; never parsed.
    #[serde(skip)]
    pub source: LauncherSource,
}

impl LauncherDecl {
    /// The declared kind, or the kind `terminal` implies for a row written
    /// before kinds existed.
    pub fn effective_kind(&self) -> LauncherKind {
        self.kind
            .unwrap_or_else(|| LauncherKind::from_terminal(self.terminal.unwrap_or(false)))
    }

    /// Whether the launcher needs a terminal: the spelled value, else the
    /// kind's.
    pub fn is_terminal(&self) -> bool {
        self.terminal
            .unwrap_or_else(|| self.effective_kind().terminal())
    }

    /// Whether the launcher appears in a graphical client's menu; `false`
    /// unless the author opted in.
    pub fn shows_in_menu(&self) -> bool {
        self.menu.unwrap_or(false)
    }

    /// Whether `name`, `executable` and `kind` are all present.
    pub fn is_complete(&self) -> bool {
        !self.name.trim().is_empty() && !self.executable.is_empty() && self.kind.is_some()
    }
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
    /// Common definitions owned by this installer option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<crate::manifest::ConfigurationDecl>,
    /// Named feature sets (`PROP-065#profiles`); only with `steps`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<ProfileDecl>,
    /// Typed questions (`PROP-065#parameters`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterDecl>,
    /// A license the user accepts before anything is fetched
    /// (`PROP-065#license`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<LicenseDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepDecl>,
    /// Application packages the installed option needs
    /// (`PROP-065#app-dependencies`); `runtime` scope unless stated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<RequirementDecl>,
    /// Globs relative to the application root naming desktop entries the
    /// launchers are derived from (`PROP-059#desktop-entries`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launchers_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launchers: Vec<LauncherDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_commands: Vec<PublicCommandDecl>,
}

/// Strict package-only `[application]` metadata: the slot name, the
/// package-wide portability default, the application icon and the menu
/// defaults (`PROP-059#menu`).
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
    /// The human-readable group a client shows this application's menu
    /// entries under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_group: Option<String>,
    /// Default categories of every launcher that names none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    /// Default keywords of every launcher that names none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// A package-relative image, or an icon set, for the application as a
    /// whole (`PROP-065#icon`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconDecl>,
}

fn portable_default() -> bool {
    true
}

fn is_portable_default(value: &bool) -> bool {
    *value
}

pub(crate) use wizard_validation::validate_parameter;
