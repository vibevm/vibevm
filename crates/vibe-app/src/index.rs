//! The one inventory, `<settings>/applications/index.json`
//! (`vibe-user-applications/7`): domain model, wire conversion and the
//! atomic save. The writer writes the **lowest** protocol that carries what
//! the home actually retains, so a home with no template provenance and no
//! migration record keeps writing `/6` and every existing PATH proxy goes
//! on reading it (`PROP-059#proxy-index-upgrade`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#index");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::{
    IconDecl, InstallerMethod, InstallerOptionDecl, LauncherDecl, PublicCommandDecl,
    RequirementScope, ShellInterpreter, StepPhase, VersionFileDecl,
};
use vibe_wire::generated::application::e6::index as wire;

use crate::error::StateError;
use crate::settings::{SettingsRoot, io, write_atomic};

/// The protocol tag of the current index format.
pub const INDEX_PROTOCOL: &str = "vibe-user-applications/7";
/// The protocol a home keeps writing while it retains no template
/// provenance, no declared migration entry and no migration outcome
/// (`PROP-059#proxy-index-upgrade`).
const PATHS_PROTOCOL: &str = "vibe-user-applications/6";
const EVIDENCE_PROTOCOL: &str = "vibe-user-applications/5";
const FOUR_PROTOCOL: &str = "vibe-user-applications/4";
const LEGACY_PROTOCOL: &str = "vibe-user-applications/3";

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
    /// Captured package source display facts; absent means unknown history.
    pub source_evidence: Option<vibe_wire::generated::shared::SourceEvidence>,
    /// Captured value-free secret origins; absent/empty legacy state is unknown history.
    pub secret_origins:
        BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>,
    /// Original common input, semantic basis and effective data; absent in legacy records.
    pub configuration: Option<crate::configuration::ConfigurationRecord>,
    /// An unfinished operation whose completion must precede installed shortcuts.
    pub pending: Option<vibe_wire::generated::shared::ApplicationPendingTarget>,
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
    /// The payload directories this installation asks to be exposed whole
    /// on PATH, in search order (`PROP-059#path-directories`).
    pub path_dirs: Vec<String>,
    /// The version files this installation's package declares
    /// (`PROP-075#VERSION-FILE-FORMATS`). Persisted **beside** the index in
    /// `applications/version-files.json`, not inside it, so a home that
    /// declares one does not force a new index protocol on PATH proxies
    /// that cannot read it (`PROP-059#proxy-index-upgrade`): [`Index::load`]
    /// fills this field from that sidecar and every publication rewrites it
    /// from these records. [`Index::parse`], which has no home to read,
    /// leaves it empty.
    pub version_file: Vec<VersionFileDecl>,
    /// The template this version's manifest was rendered from, when one
    /// produced it (`PROP-075#TEMPLATE-LOCK-RECORD`). Absent for an
    /// ordinary published package, which is what keeps a home without a
    /// rendered installation at `vibe-user-applications/6`.
    pub template: Option<InstalledTemplate>,
    /// The `[[application.migration]]` entries this version's package
    /// declares, retained so an offer after `install -g` / `update -g` /
    /// `app use` and a later `--only ID` need no package tree
    /// (`PROP-075#MIGRATION-DECLARATION`).
    pub migrations: Vec<MigrationEntryRecord>,
    /// The migrations already carried **into** this installation, oldest
    /// first (`PROP-075#MIGRATION-FAILURE`).
    pub migrated: Vec<MigrationOutcomeRecord>,
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

impl Installation {
    pub(crate) fn identity_record(&self) -> wire::Installation {
        wire_form::identity_record(self)
    }
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

/// The whole inventory: installations by key and each slot's state.
///
/// ```
/// use vibe_app::{EventKind, HomeEvent, Index, SettingsRoot, SlotState};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let mut index = Index::load(&settings).unwrap();
/// assert!(index.installations.is_empty());
/// index.slots.insert("editor".into(), SlotState::default());
/// index.save(&settings, &HomeEvent::new(EventKind::Updated)).unwrap();
/// assert_eq!(Index::load(&settings).unwrap(), index);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct Index {
    pub installations: BTreeMap<String, Installation>,
    pub slots: BTreeMap<String, SlotState>,
}

impl Index {
    /// Read the index; an absent file is an empty inventory.
    pub fn load(settings: &SettingsRoot) -> Result<Self, StateError> {
        Self::load_with_protocol(settings).map(|(index, _)| index)
    }

    /// Observe the persisted tag independently of lossless writer eligibility.
    pub(crate) fn load_with_protocol(settings: &SettingsRoot) -> Result<(Self, u8), StateError> {
        let path = settings.index_path();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Self::default(), 3));
            }
            Err(source) => return Err(io("reading", &path, source)),
        };
        let invalid = |reason: String| StateError::Index {
            path: path.display().to_string(),
            reason,
        };
        let (mut index, protocol) = Self::parse_with_protocol(&bytes).map_err(invalid)?;
        index.read_declared_version_files(settings);
        Ok((index, protocol))
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

    /// One record without derived reverse edges for retained operation identity.
    pub(crate) fn identity_record(&self, key: &str) -> Option<wire::Installation> {
        self.installations.get(key).map(wire_form::identity_record)
    }

    pub fn to_wire(&self) -> wire::ApplicationIndex {
        wire::ApplicationIndex {
            protocol: match self.minimum_reader() {
                7 => INDEX_PROTOCOL,
                6 => PATHS_PROTOCOL,
                5 => EVIDENCE_PROTOCOL,
                4 => FOUR_PROTOCOL,
                _ => LEGACY_PROTOCOL,
            }
            .into(),
            installations: self
                .installations
                .iter()
                .map(|(key, value)| (key.clone(), installation_to_wire(self, key, value)))
                .collect(),
            slots: self
                .slots
                .iter()
                .map(|(slot, state)| {
                    (
                        slot.clone(),
                        wire::Slot {
                            default: state.default.clone(),
                            off: state.off,
                            path_env: state.path_env.iter().map(wire_form::env_to_wire).collect(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn from_wire(document: wire::ApplicationIndex) -> Result<Self, String> {
        if document.protocol != INDEX_PROTOCOL
            && document.protocol != PATHS_PROTOCOL
            && document.protocol != EVIDENCE_PROTOCOL
            && document.protocol != FOUR_PROTOCOL
            && document.protocol != LEGACY_PROTOCOL
        {
            return Err(format!(
                "protocol `{}` is not `{INDEX_PROTOCOL}`",
                document.protocol
            ));
        }
        if document.protocol == LEGACY_PROTOCOL
            && document.installations.values().any(|r| {
                r.configuration.is_some() || r.pending.is_some() || !r.secret_origins.is_empty()
            })
        {
            return Err(
                "protocol3 cannot retain common configuration or pending operation state".into(),
            );
        }
        if (document.protocol == LEGACY_PROTOCOL || document.protocol == FOUR_PROTOCOL)
            && document
                .installations
                .values()
                .any(|r| r.source_evidence.is_some())
        {
            return Err("source evidence requires application index5".into());
        }
        if document.protocol != INDEX_PROTOCOL
            && document.protocol != PATHS_PROTOCOL
            && (document
                .installations
                .values()
                .any(|r| !r.path_dirs.is_empty())
                || document
                    .slots
                    .values()
                    .any(|slot| slot.off || !slot.path_env.is_empty()))
        {
            return Err(
                "exposed payload directories and slot off state require application index6".into(),
            );
        }
        if document.protocol != INDEX_PROTOCOL
            && document
                .installations
                .values()
                .any(|r| r.template.is_some() || !r.migrations.is_empty() || !r.migrated.is_empty())
        {
            return Err(
                "template provenance and migration records require application index7".into(),
            );
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
            .map(|(slot, value)| {
                (
                    slot,
                    SlotState {
                        default: value.default,
                        off: value.off,
                        path_env: value.path_env.into_iter().map(env_from_wire).collect(),
                    },
                )
            })
            .collect();
        for (slot, state) in &slots {
            if let Some(key) = &state.default
                && installations
                    .get(key)
                    .is_none_or(|record| &record.slot != slot)
            {
                return Err(format!(
                    "slot `{slot}` default `{key}` is not an installation of that slot"
                ));
            }
            if state
                .path_env
                .iter()
                .any(|entry| entry.op != EnvOp::PathPrepend)
            {
                return Err(format!(
                    "slot `{slot}` records a link-directory environment entry that is not a PATH prepend"
                ));
            }
        }
        Ok(Self {
            installations,
            slots,
        })
    }
}

mod env_record;
#[path = "index/launcher_wire.rs"]
mod launcher_wire;
#[path = "index/migration.rs"]
mod migration;
#[path = "index/protocol.rs"]
mod protocol;
mod publication;
#[path = "index/upgrade.rs"]
mod upgrade;
mod upgrade_five;
mod upgrade_four;
mod upgrade_six;
#[path = "index/version_files.rs"]
mod version_files;
#[path = "index/wire.rs"]
mod wire_form;

pub use env_record::{EnvEntry, EnvOp, ValueKind};
pub use migration::{
    InstalledTemplate, MigrationEntryRecord, MigrationOutcomeRecord, MigrationResult,
};
mod slots;
pub use slots::SlotState;
use wire_form::{env_from_wire, installation_from_wire, installation_to_wire};

#[cfg(test)]
#[path = "index/tests.rs"]
mod tests;
