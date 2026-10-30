//! The one inventory, `<settings>/applications/index.json`
//! (`vibe-user-applications/3`): domain model, wire conversion and the
//! atomic save.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#index");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::{InstallerMethod, LauncherDecl, PublicCommandDecl, ShellInterpreter};
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
}

impl Method {
    pub fn of(installation: &InstallerMethod) -> Self {
        match installation {
            InstallerMethod::Shell { .. } => Self::Shell,
            InstallerMethod::Zip { .. } => Self::Zip,
            InstallerMethod::TarGz { .. } => Self::TarGz,
            InstallerMethod::Exe { .. } => Self::Exe,
            InstallerMethod::Msi { .. } => Self::Msi,
            InstallerMethod::SourceBuild { .. } => Self::SourceBuild,
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
        }
    }

    pub fn is_owned(self) -> bool {
        matches!(self, Self::Zip | Self::TarGz | Self::SourceBuild)
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
    pub commands: AuthoredCommands,
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
            interpreter: value.commands.interpreter.map(|value| match value {
                ShellInterpreter::PosixSh => wire::Interpreter::PosixSh,
                ShellInterpreter::WindowsCmd => wire::Interpreter::WindowsCmd,
                ShellInterpreter::WindowsPowershell => wire::Interpreter::WindowsPowershell,
            }),
        },
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
    };
    let location = match (
        method.is_owned(),
        record.library,
        record.directory,
        record.app_root,
    ) {
        (true, Some(library), Some(directory), None) => {
            if directory
                .split('/')
                .any(|part| part.is_empty() || part == "..")
            {
                return Err(format!("directory `{directory}` is not library-relative"));
            }
            Location::Owned { library, directory }
        }
        (false, None, None, Some(app_root)) => {
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
            interpreter: record.commands.interpreter.map(|value| match value {
                wire::Interpreter::PosixSh => ShellInterpreter::PosixSh,
                wire::Interpreter::WindowsCmd => ShellInterpreter::WindowsCmd,
                wire::Interpreter::WindowsPowershell => ShellInterpreter::WindowsPowershell,
            }),
        },
    })
}

fn path_text(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Index {
        let owned = Installation {
            slot: "editor".into(),
            package: PackageCoordinate {
                group: "org.example".into(),
                name: "editor".into(),
                version: "1.2.0".into(),
            },
            option: "linux-zip".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            method: Method::Zip,
            location: Location::Owned {
                library: "default".into(),
                directory: "apps/editor/org.example.editor@1.2.0".into(),
            },
            portable: true,
            launchers: vec![LauncherDecl {
                id: "main".into(),
                name: "Editor".into(),
                description: None,
                executable: "$app_bin/editor".into(),
                args: vec!["--fixed".into()],
                working_directory: Some("$project_root".into()),
                icon: None,
                terminal: true,
                default: true,
            }],
            public_commands: vec![PublicCommandDecl {
                name: "edit".into(),
                launcher: "main".into(),
            }],
            commands: AuthoredCommands::default(),
        };
        let external = Installation {
            package: PackageCoordinate {
                group: "org.example".into(),
                name: "editor-apt".into(),
                version: "2.0.0".into(),
            },
            option: "ubuntu-apt".into(),
            platform: "ubuntu".into(),
            method: Method::Shell,
            location: Location::External {
                app_root: PathBuf::from("/usr/lib/editor"),
            },
            portable: false,
            launchers: Vec::new(),
            public_commands: Vec::new(),
            commands: AuthoredCommands {
                uninstall: Some("sudo apt-get remove editor".into()),
                force_uninstall: Some("sudo apt-get purge editor".into()),
                update: Some("sudo apt-get install --only-upgrade editor".into()),
                interpreter: Some(ShellInterpreter::PosixSh),
            },
            ..owned.clone()
        };
        let mut index = Index::default();
        index
            .installations
            .insert(owned.package.key(), owned.clone());
        index.installations.insert(external.package.key(), external);
        index
            .slots
            .insert("editor".into(), Some(owned.package.key()));
        index
    }

    #[test]
    fn domain_and_wire_round_trip_the_corpus() {
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/application/e2/index.json");
        let document: wire::ApplicationIndex =
            serde_json::from_slice(&fs::read(corpus).unwrap()).unwrap();
        let index = Index::from_wire(document.clone()).unwrap();
        assert_eq!(index, sample());
        assert_eq!(index.to_wire(), document);
    }

    #[test]
    fn save_is_atomic_and_load_rejects_inconsistent_records() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        sample().save(&settings).unwrap();
        assert_eq!(Index::load(&settings).unwrap(), sample());
        let leftovers: Vec<_> = fs::read_dir(settings.applications_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());

        let mut broken = sample().to_wire();
        broken.slots.insert(
            "other".into(),
            wire::Slot {
                default: Some("org.example/editor@1.2.0".into()),
            },
        );
        assert!(
            Index::from_wire(broken)
                .unwrap_err()
                .contains("not an installation")
        );
        let mut wrong = sample().to_wire();
        for record in wrong.installations.values_mut() {
            record.app_root = Some("/also".into());
        }
        assert!(Index::from_wire(wrong).is_err());
        let mut old = sample().to_wire();
        old.protocol = "vibe-user-applications/2".into();
        assert!(Index::from_wire(old).unwrap_err().contains("protocol"));
    }
}
