//! The one inventory, `<settings>/applications/index.json`
//! (`vibe-user-applications/3`): domain model, wire conversion and the
//! atomic save.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#index");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::{
    IconDecl, InstallerMethod, InstallerOptionDecl, LauncherDecl, PublicCommandDecl,
    RequirementScope, ShellInterpreter, StepPhase,
};
use vibe_wire::generated::application::e2::index as wire;

use crate::error::StateError;
use crate::settings::{SettingsRoot, io, write_atomic};

/// The protocol tag of the current index format.
pub const INDEX_PROTOCOL: &str = "vibe-user-applications/3";

/// `group/name@version` of one package.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct PackageCoordinate {
    pub group: String,
    pub name: String,
    pub version: String,
}

impl PackageCoordinate {
    /// The installation key `group/name@version`.
    pub fn key(&self) -> String {
        format!("{}/{}@{}", self.group, self.name, self.version)
    }

    /// The library directory name `group.name@version`.
    pub fn directory_name(&self) -> String {
        format!("{}.{}@{}", self.group, self.name, self.version)
    }

    pub fn same_package(&self, other: &Self) -> bool {
        self.group == other.group && self.name == other.name
    }
}

/// The installation method recorded for an installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum Method {
    Shell,
    Zip,
    TarGz,
    Exe,
    Msi,
    SourceBuild,
    /// An option declared as `[[steps]]` (`PROP-065#steps`); its ownership
    /// follows the declared result.
    Steps,
}

impl Method {
    /// The method an option records: its `installation.type`, or `steps`.
    pub fn of(option: &InstallerOptionDecl) -> Self {
        match &option.installation {
            Some(InstallerMethod::Shell { .. }) => Self::Shell,
            Some(InstallerMethod::Zip { .. }) => Self::Zip,
            Some(InstallerMethod::TarGz { .. }) => Self::TarGz,
            Some(InstallerMethod::Exe { .. }) => Self::Exe,
            Some(InstallerMethod::Msi { .. }) => Self::Msi,
            Some(InstallerMethod::SourceBuild { .. }) => Self::SourceBuild,
            None => Self::Steps,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Zip => "zip",
            Self::TarGz => "tar_gz",
            Self::Exe => "exe",
            Self::Msi => "msi",
            Self::SourceBuild => "source_build",
            Self::Steps => "steps",
        }
    }
}

/// The outcome of one step (`PROP-065#execution`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#execution")]
pub enum StepOutcome {
    Done,
    Skipped,
    Failed,
    Delegated,
    /// Stopped by a signal (`PROP-065#cancellation`); resumable.
    Interrupted,
}

impl StepOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::Delegated => "delegated",
            Self::Interrupted => "interrupted",
        }
    }
}

/// A license accepted for an installation (`PROP-065#license`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#license")]
pub struct AcceptedLicense {
    pub name: String,
    /// RFC 3339 time of the acceptance.
    pub accepted_at: String,
}

/// One step outcome kept with an installation, with the uninstall data a
/// shell or native step recorded (`PROP-065#record`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#record")]
pub struct StepRecord {
    pub id: String,
    pub kind: String,
    /// The phase the step ran in; removal undoes `finish` steps first
    /// (`PROP-065#uninstall-tree`).
    pub phase: StepPhase,
    pub outcome: StepOutcome,
    /// `uninstall`, `force_uninstall` and `interpreter`; `update` unused.
    pub uninstall: AuthoredCommands,
}

impl AuthoredCommands {
    /// Whether an uninstall command of either variant is recorded.
    pub fn has_uninstall(&self) -> bool {
        self.uninstall.is_some() || self.force_uninstall.is_some()
    }
}

/// Where an installation's `app_root` lives. An owned root is derived from
/// its library and directory at every use; an external root is absolute.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#ownership")]
pub enum Location {
    Owned { library: String, directory: String },
    External { app_root: PathBuf },
}

/// Authored commands an installation needs after its package is gone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct AuthoredCommands {
    pub uninstall: Option<String>,
    pub force_uninstall: Option<String>,
    pub update: Option<String>,
    pub interpreter: Option<ShellInterpreter>,
}

/// One installation record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct Installation {
    pub slot: String,
    pub package: PackageCoordinate,
    pub option: String,
    pub platform: String,
    pub arch: String,
    pub method: Method,
    pub location: Location,
    pub portable: bool,
    pub launchers: Vec<LauncherDecl>,
    pub public_commands: Vec<PublicCommandDecl>,
    /// The authored commands of a method-form option.
    pub commands: AuthoredCommands,
    /// The chosen features (`PROP-065#features`).
    pub features: Vec<String>,
    /// Chosen parameter values, never a secret one (`PROP-065#parameters`).
    pub parameters: BTreeMap<String, String>,
    /// The accepted license, when the option declares one.
    pub license: Option<AcceptedLicense>,
    /// The package-relative application icon or icon set (`PROP-065#icon`).
    pub icon: Option<IconDecl>,
    /// The application's menu group and default categories and keywords
    /// (`PROP-059#menu`).
    pub menu: ApplicationMenu,
    /// Step outcomes in step order (`PROP-065#record`).
    pub steps: Vec<StepRecord>,
    /// Exposure, marks, requirements, environment and update state.
    pub lifecycle: Lifecycle,
}

/// The `[application]` menu metadata an installation keeps after its package
/// is gone (`PROP-059#menu`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
pub struct ApplicationMenu {
    pub menu_group: Option<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
}

/// Why an installation exists (`PROP-065#uninstall-tree`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree")]
pub enum Mark {
    /// The user asked for it by name (`install`, `expose`, `app mark manual`).
    #[default]
    Manual,
    /// A requirement pulled it in; autoremove may remove it once nothing
    /// requires it.
    Auto,
}

impl Mark {
    /// The stable spelling (`manual`, `auto`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Auto => "auto",
        }
    }
}

/// One application package an installation requires
/// (`PROP-065#app-dependencies`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
pub struct Requirement {
    /// The authored `group/name@requirement`.
    pub package: String,
    /// The installation key it resolved to.
    pub key: String,
    pub scope: RequirementScope,
    /// The step that asked for it; `None` for an option-level requirement.
    pub step: Option<String>,
}

/// The operation of one durable environment change (`PROP-065#env-step`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub enum EnvOp {
    Set,
    Unset,
    PathPrepend,
}

impl EnvOp {
    /// The stable spelling (`set`, `unset`, `path_prepend`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Unset => "unset",
            Self::PathPrepend => "path_prepend",
        }
    }
}

/// The registry kind of a Windows value an env step replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub enum ValueKind {
    String,
    ExpandString,
}

/// One applied environment change, recorded so that `uninstall` and
/// `modify` remove exactly it (`PROP-065#env-step`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub struct EnvEntry {
    pub step: String,
    pub op: EnvOp,
    pub name: String,
    /// The expanded value; absent for `unset`.
    pub value: Option<String>,
    /// A Windows value this change replaced, restored on removal.
    pub previous: Option<String>,
    pub previous_kind: Option<ValueKind>,
}

/// The lifecycle state of an installation beyond its payload: hidden
/// exposure (`PROP-059#hidden-installations`), the manual/auto mark and
/// requirements (`PROP-065#uninstall-tree`, `#app-dependencies`), applied
/// environment entries (`#env-step`), and the hold / kept-previous state
/// of updates (`#update-and-rollback`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations")]
pub struct Lifecycle {
    pub hidden: bool,
    pub mark: Mark,
    pub requires: Vec<Requirement>,
    pub env: Vec<EnvEntry>,
    pub held: bool,
    /// The key this installation's update replaced.
    pub replaced: Option<String>,
    /// Kept hidden after an update until the next successful update of
    /// its slot; names the key that replaced it.
    pub superseded_by: Option<String>,
}

/// The whole inventory: installations by key and each slot's default key.
///
/// ```
/// use vibe_app::{Index, SettingsRoot};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let mut index = Index::load(&settings).unwrap();
/// assert!(index.installations.is_empty());
/// index.slots.insert("editor".into(), None);
/// index.save(&settings).unwrap();
/// assert_eq!(Index::load(&settings).unwrap(), index);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct Index {
    pub installations: BTreeMap<String, Installation>,
    pub slots: BTreeMap<String, Option<String>>,
}

impl Index {
    /// Read the index; an absent file is an empty inventory.
    pub fn load(settings: &SettingsRoot) -> Result<Self, StateError> {
        let path = settings.index_path();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => return Err(io("reading", &path, source)),
        };
        let invalid = |reason: String| StateError::Index {
            path: path.display().to_string(),
            reason,
        };
        let document: wire::ApplicationIndex =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        Self::from_wire(document).map_err(invalid)
    }

    /// Replace the index atomically (write a sibling, rename over it).
    pub fn save(&self, settings: &SettingsRoot) -> Result<(), StateError> {
        let path = settings.index_path();
        let mut bytes =
            serde_json::to_vec_pretty(&self.to_wire()).map_err(|error| StateError::Index {
                path: path.display().to_string(),
                reason: error.to_string(),
            })?;
        bytes.push(b'\n');
        write_atomic(&path, &bytes)
    }

    pub fn default_of(&self, slot: &str) -> Option<&str> {
        self.slots.get(slot).and_then(|value| value.as_deref())
    }

    /// The keys of the installations whose recorded requirements name
    /// `key`, with the scope of each requirement (`PROP-065#uninstall-tree`).
    pub fn required_by(&self, key: &str) -> Vec<(&str, RequirementScope)> {
        let mut found: Vec<(&str, RequirementScope)> = Vec::new();
        for (owner, record) in &self.installations {
            for requirement in &record.lifecycle.requires {
                if requirement.key == key && owner != key {
                    match found.iter_mut().find(|(seen, _)| *seen == owner.as_str()) {
                        Some((_, scope)) => *scope = (*scope).max(requirement.scope),
                        None => found.push((owner.as_str(), requirement.scope)),
                    }
                }
            }
        }
        found
    }

    pub fn to_wire(&self) -> wire::ApplicationIndex {
        wire::ApplicationIndex {
            protocol: INDEX_PROTOCOL.into(),
            installations: self
                .installations
                .iter()
                .map(|(key, value)| (key.clone(), installation_to_wire(self, key, value)))
                .collect(),
            slots: self
                .slots
                .iter()
                .map(|(slot, default)| {
                    (
                        slot.clone(),
                        wire::Slot {
                            default: default.clone(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn from_wire(document: wire::ApplicationIndex) -> Result<Self, String> {
        if document.protocol != INDEX_PROTOCOL {
            return Err(format!(
                "protocol `{}` is not `{INDEX_PROTOCOL}`",
                document.protocol
            ));
        }
        let mut installations = BTreeMap::new();
        for (key, record) in document.installations {
            let installation = installation_from_wire(record)
                .map_err(|reason| format!("installation `{key}`: {reason}"))?;
            if installation.package.key() != key {
                return Err(format!("installation key `{key}` differs from its package"));
            }
            installations.insert(key, installation);
        }
        let slots: BTreeMap<_, _> = document
            .slots
            .into_iter()
            .map(|(slot, value)| (slot, value.default))
            .collect();
        for (slot, default) in &slots {
            if let Some(key) = default
                && installations
                    .get(key)
                    .is_none_or(|record| &record.slot != slot)
            {
                return Err(format!(
                    "slot `{slot}` default `{key}` is not an installation of that slot"
                ));
            }
        }
        Ok(Self {
            installations,
            slots,
        })
    }
}

#[path = "index/launcher_wire.rs"]
mod launcher_wire;
#[path = "index/wire.rs"]
mod wire_form;

use wire_form::{installation_from_wire, installation_to_wire};

#[cfg(test)]
#[path = "index/tests.rs"]
mod tests;
