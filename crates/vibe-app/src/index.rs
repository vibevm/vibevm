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
    InstallerMethod, InstallerOptionDecl, LauncherDecl, PublicCommandDecl, ShellInterpreter,
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
}

impl StepOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::Delegated => "delegated",
        }
    }
}

/// One step outcome kept with an installation, with the uninstall data a
/// shell or native step recorded (`PROP-065#record`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#record")]
pub struct StepRecord {
    pub id: String,
    pub kind: String,
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
    /// Step outcomes in step order (`PROP-065#record`).
    pub steps: Vec<StepRecord>,
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

    pub fn to_wire(&self) -> wire::ApplicationIndex {
        wire::ApplicationIndex {
            protocol: INDEX_PROTOCOL.into(),
            installations: self
                .installations
                .iter()
                .map(|(key, value)| (key.clone(), installation_to_wire(value)))
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

fn installation_to_wire(value: &Installation) -> wire::Installation {
    let (library, directory, app_root) = match &value.location {
        Location::Owned { library, directory } => {
            (Some(library.clone()), Some(directory.clone()), None)
        }
        Location::External { app_root } => (None, None, Some(path_text(app_root))),
    };
    wire::Installation {
        slot: value.slot.clone(),
        package: wire::Package {
            group: value.package.group.clone(),
            name: value.package.name.clone(),
            version: value.package.version.clone(),
        },
        option: value.option.clone(),
        platform: value.platform.clone(),
        arch: value.arch.clone(),
        method: match value.method {
            Method::Shell => wire::Method::Shell,
            Method::Zip => wire::Method::Zip,
            Method::TarGz => wire::Method::TarGz,
            Method::Exe => wire::Method::Exe,
            Method::Msi => wire::Method::Msi,
            Method::SourceBuild => wire::Method::SourceBuild,
            Method::Steps => wire::Method::Steps,
        },
        library,
        directory,
        app_root,
        portable: value.portable,
        launchers: value
            .launchers
            .iter()
            .map(|launcher| wire::Launcher {
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
            .map(|command| wire::PublicCommand {
                name: command.name.clone(),
                launcher: command.launcher.clone(),
            })
            .collect(),
        commands: wire::AuthoredCommands {
            uninstall: value.commands.uninstall.clone(),
            force_uninstall: value.commands.force_uninstall.clone(),
            update: value.commands.update.clone(),
            interpreter: value.commands.interpreter.map(interpreter_to_wire),
        },
        features: value.features.clone(),
        steps: value
            .steps
            .iter()
            .map(|step| wire::StepRecord {
                id: step.id.clone(),
                kind: step.kind.clone(),
                outcome: match step.outcome {
                    StepOutcome::Done => wire::Outcome::Done,
                    StepOutcome::Skipped => wire::Outcome::Skipped,
                    StepOutcome::Failed => wire::Outcome::Failed,
                    StepOutcome::Delegated => wire::Outcome::Delegated,
                },
                uninstall: step.uninstall.uninstall.clone(),
                force_uninstall: step.uninstall.force_uninstall.clone(),
                interpreter: step.uninstall.interpreter.map(interpreter_to_wire),
            })
            .collect(),
    }
}

fn interpreter_to_wire(value: ShellInterpreter) -> wire::Interpreter {
    match value {
        ShellInterpreter::PosixSh => wire::Interpreter::PosixSh,
        ShellInterpreter::WindowsCmd => wire::Interpreter::WindowsCmd,
        ShellInterpreter::WindowsPowershell => wire::Interpreter::WindowsPowershell,
    }
}

fn interpreter_from_wire(value: wire::Interpreter) -> ShellInterpreter {
    match value {
        wire::Interpreter::PosixSh => ShellInterpreter::PosixSh,
        wire::Interpreter::WindowsCmd => ShellInterpreter::WindowsCmd,
        wire::Interpreter::WindowsPowershell => ShellInterpreter::WindowsPowershell,
    }
}

fn installation_from_wire(record: wire::Installation) -> Result<Installation, String> {
    let method = match record.method {
        wire::Method::Shell => Method::Shell,
        wire::Method::Zip => Method::Zip,
        wire::Method::TarGz => Method::TarGz,
        wire::Method::Exe => Method::Exe,
        wire::Method::Msi => Method::Msi,
        wire::Method::SourceBuild => Method::SourceBuild,
        wire::Method::Steps => Method::Steps,
    };
    let owned = match method {
        Method::Zip | Method::TarGz | Method::SourceBuild => Some(true),
        Method::Shell | Method::Exe | Method::Msi => Some(false),
        Method::Steps => None,
    };
    let location = match (owned, record.library, record.directory, record.app_root) {
        (Some(true) | None, Some(library), Some(directory), None) => {
            if directory
                .split('/')
                .any(|part| part.is_empty() || part == "..")
            {
                return Err(format!("directory `{directory}` is not library-relative"));
            }
            Location::Owned { library, directory }
        }
        (Some(false) | None, None, None, Some(app_root)) => {
            let app_root = PathBuf::from(app_root);
            if !app_root.is_absolute() {
                return Err("external appRoot is not absolute".into());
            }
            Location::External { app_root }
        }
        _ => {
            return Err(
                "owned methods need library and directory; external methods need only appRoot"
                    .into(),
            );
        }
    };
    Ok(Installation {
        slot: record.slot,
        package: PackageCoordinate {
            group: record.package.group,
            name: record.package.name,
            version: record.package.version,
        },
        option: record.option,
        platform: record.platform,
        arch: record.arch,
        method,
        location,
        portable: record.portable,
        launchers: record
            .launchers
            .into_iter()
            .map(|launcher| LauncherDecl {
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
        public_commands: record
            .public_commands
            .into_iter()
            .map(|command| PublicCommandDecl {
                name: command.name,
                launcher: command.launcher,
            })
            .collect(),
        commands: AuthoredCommands {
            uninstall: record.commands.uninstall,
            force_uninstall: record.commands.force_uninstall,
            update: record.commands.update,
            interpreter: record.commands.interpreter.map(interpreter_from_wire),
        },
        features: record.features,
        steps: record
            .steps
            .into_iter()
            .map(|step| StepRecord {
                id: step.id,
                kind: step.kind,
                outcome: match step.outcome {
                    wire::Outcome::Done => StepOutcome::Done,
                    wire::Outcome::Skipped => StepOutcome::Skipped,
                    wire::Outcome::Failed => StepOutcome::Failed,
                    wire::Outcome::Delegated => StepOutcome::Delegated,
                },
                uninstall: AuthoredCommands {
                    uninstall: step.uninstall,
                    force_uninstall: step.force_uninstall,
                    update: None,
                    interpreter: step.interpreter.map(interpreter_from_wire),
                },
            })
            .collect(),
    })
}

fn path_text(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "index/tests.rs"]
mod tests;
