//! Storage libraries: `<settings>/storage.toml`, the implicit default library
//! `<settings>/opt`, `vibe-library.toml` markers and availability observed at
//! the moment of use.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#availability");

mod edit;
mod relocate;

pub use edit::{add, attach, detach, listing};
pub use relocate::{Moved, gc, relocate};

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_core::manifest::is_portable_token;

use crate::error::StateError;
use crate::settings::{SettingsRoot, io, same_place, write_atomic};

/// The id of the implicit library at `<settings>/opt`.
pub const DEFAULT_LIBRARY: &str = "default";

/// The id of the library at the install root store the user home's
/// applications follow (`PROP-019#install-root`): listed in `storage.toml`
/// on the first installation that lands there, so the PATH proxy and every
/// older vibe resolve it like any configured library.
pub const INSTALL_ROOT_LIBRARY: &str = "install-root";

/// The marker file that proves a directory is the library it claims to be.
pub const LIBRARY_MARKER: &str = "vibe-library.toml";

/// One configured library: a stable id and an absolute directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries")]
pub struct Library {
    pub id: String,
    pub path: PathBuf,
}

/// The `<settings>/storage.toml` document. The default library is implicit
/// and never listed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries")]
pub struct StorageConfig {
    #[serde(default, rename = "library", skip_serializing_if = "Vec::is_empty")]
    pub libraries: Vec<Library>,
}

/// The `vibe-library.toml` marker inside a library directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries")]
pub struct LibraryMarker {
    pub id: String,
    pub created: String,
}

/// Availability as observed now; never cached.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#availability")]
pub enum Availability {
    Available,
    Unavailable(String),
}

/// The libraries of one settings root: the default plus every configured row.
///
/// ```
/// use vibe_app::{Availability, SettingsRoot, Storage, DEFAULT_LIBRARY};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let storage = Storage::load(&settings).unwrap();
/// assert!(matches!(storage.availability(DEFAULT_LIBRARY), Availability::Unavailable(_)));
/// let root = storage.usable(DEFAULT_LIBRARY).unwrap();
/// assert_eq!(root, dir.path().join("opt"));
/// assert_eq!(storage.availability(DEFAULT_LIBRARY), Availability::Available);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries")]
pub struct Storage {
    default_path: PathBuf,
    config: StorageConfig,
    storage_path: PathBuf,
    install: Option<InstallLibrary>,
}

/// How the install root store relates to `storage.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallLibrary {
    store: PathBuf,
    /// The `install-root` row is in memory only, written on first use.
    unlisted: bool,
    /// The listed `install-root` row names another directory: the install
    /// root changed after applications were installed under the old one.
    listed_elsewhere: Option<PathBuf>,
}

impl Storage {
    /// Read `storage.toml` (absent means only the default library).
    pub fn load(settings: &SettingsRoot) -> Result<Self, StateError> {
        let path = settings.storage_path();
        let config = match fs::read_to_string(&path) {
            Ok(text) => {
                toml::from_str::<StorageConfig>(&text).map_err(|error| StateError::Storage {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                })?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => StorageConfig::default(),
            Err(source) => return Err(io("reading", &path, source)),
        };
        validate(&config).map_err(|reason| StateError::Storage {
            path: path.display().to_string(),
            reason,
        })?;
        let mut config = config;
        let install = settings.install_root().map(|store| {
            let listed = config
                .libraries
                .iter()
                .find(|library| library.id == INSTALL_ROOT_LIBRARY);
            let listed_elsewhere = listed
                .filter(|library| !same_place(&library.path, store))
                .map(|library| library.path.clone());
            let unlisted = listed.is_none();
            if unlisted {
                config.libraries.push(Library {
                    id: INSTALL_ROOT_LIBRARY.into(),
                    path: store.to_path_buf(),
                });
            }
            InstallLibrary {
                store: store.to_path_buf(),
                unlisted,
                listed_elsewhere,
            }
        });
        Ok(Self {
            default_path: settings.default_library_path(),
            config,
            storage_path: path,
            install,
        })
    }

    /// The library a new owned installation lands in when none is named:
    /// `install-root` when the applications follow an install root other
    /// than the settings root, else `default`. An `install-root` row that
    /// still names an earlier install root is refused rather than mixed.
    ///
    /// ```
    /// let dir = tempfile::tempdir().unwrap();
    /// let plain = vibe_app::SettingsRoot::new(&dir.path().join("settings"));
    /// assert_eq!(vibe_app::Storage::load(&plain).unwrap().fresh_library().unwrap(), "default");
    /// let work = plain.with_install_root(&dir.path().join("Work/opt"));
    /// let storage = vibe_app::Storage::load(&work).unwrap();
    /// assert_eq!(storage.fresh_library().unwrap(), "install-root");
    /// assert_eq!(storage.usable("install-root").unwrap(), dir.path().join("Work/opt"));
    /// assert!(std::fs::read_to_string(work.storage_path()).unwrap().contains("install-root"));
    /// ```
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root")]
    pub fn fresh_library(&self) -> Result<&'static str, StateError> {
        match &self.install {
            None => Ok(DEFAULT_LIBRARY),
            Some(InstallLibrary {
                store,
                listed_elsewhere: Some(listed),
                ..
            }) => Err(StateError::InstallRootMoved {
                listed: listed.display().to_string(),
                store: store.display().to_string(),
            }),
            Some(_) => Ok(INSTALL_ROOT_LIBRARY),
        }
    }

    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Every library, the default first.
    pub fn libraries(&self) -> Vec<Library> {
        let mut all = vec![Library {
            id: DEFAULT_LIBRARY.into(),
            path: self.default_path.clone(),
        }];
        all.extend(self.config.libraries.iter().cloned());
        all
    }

    pub fn path_of(&self, id: &str) -> Option<PathBuf> {
        if id == DEFAULT_LIBRARY {
            return Some(self.default_path.clone());
        }
        self.config
            .libraries
            .iter()
            .find(|library| library.id == id)
            .map(|library| library.path.clone())
    }

    /// A library is available when its path is a directory whose marker
    /// carries the configured id.
    pub fn availability(&self, id: &str) -> Availability {
        let Some(path) = self.path_of(id) else {
            return Availability::Unavailable(format!("library `{id}` is not configured"));
        };
        if !path.is_dir() {
            return Availability::Unavailable(format!(
                "library directory `{}` is missing",
                path.display()
            ));
        }
        match read_marker(&path) {
            Ok(marker) if marker.id == id => Availability::Available,
            Ok(marker) => Availability::Unavailable(format!(
                "library marker at `{}` names `{}`, not `{id}`",
                path.display(),
                marker.id
            )),
            Err(reason) => Availability::Unavailable(reason),
        }
    }

    /// The root of a library an operation is about to write into. The default
    /// library is created on first use; any other must already be available.
    pub fn usable(&self, id: &str) -> Result<PathBuf, StateError> {
        let path = self
            .path_of(id)
            .ok_or_else(|| StateError::UnknownLibrary { id: id.into() })?;
        let implicit = id == DEFAULT_LIBRARY
            || id == INSTALL_ROOT_LIBRARY
                && self
                    .install
                    .as_ref()
                    .is_some_and(|install| install.listed_elsewhere.is_none());
        if implicit && !path.join(LIBRARY_MARKER).exists() {
            create_library(&path, id)?;
        }
        if id == INSTALL_ROOT_LIBRARY
            && self
                .install
                .as_ref()
                .is_some_and(|install| install.unlisted)
        {
            let text = toml::to_string(&self.config).map_err(|error| StateError::Storage {
                path: self.storage_path.display().to_string(),
                reason: error.to_string(),
            })?;
            write_atomic(&self.storage_path, text.as_bytes())?;
        }
        match self.availability(id) {
            Availability::Available => Ok(path),
            Availability::Unavailable(reason) => Err(StateError::LibraryUnavailable {
                id: id.into(),
                reason,
            }),
        }
    }
}

/// Create a library directory and its marker.
pub fn create_library(path: &Path, id: &str) -> Result<(), StateError> {
    fs::create_dir_all(path.join("apps")).map_err(|source| io("creating", path, source))?;
    let marker = LibraryMarker {
        id: id.into(),
        created: chrono::Utc::now().to_rfc3339(),
    };
    let text = toml::to_string(&marker).map_err(|error| StateError::Storage {
        path: path.join(LIBRARY_MARKER).display().to_string(),
        reason: error.to_string(),
    })?;
    write_atomic(&path.join(LIBRARY_MARKER), text.as_bytes())
}

/// Replace `storage.toml` atomically with `config`.
fn save(settings: &SettingsRoot, config: &StorageConfig) -> Result<(), StateError> {
    let path = settings.storage_path();
    let text = toml::to_string(config).map_err(|error| StateError::Storage {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    write_atomic(&path, text.as_bytes())
}

fn read_marker(path: &Path) -> Result<LibraryMarker, String> {
    let file = path.join(LIBRARY_MARKER);
    let text = fs::read_to_string(&file)
        .map_err(|error| format!("library marker `{}` is unreadable: {error}", file.display()))?;
    toml::from_str(&text)
        .map_err(|error| format!("library marker `{}` is invalid: {error}", file.display()))
}

fn validate(config: &StorageConfig) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for library in &config.libraries {
        if !is_portable_token(&library.id) || library.id == DEFAULT_LIBRARY {
            return Err(format!(
                "library id `{}` must be a portable token other than `default`",
                library.id
            ));
        }
        if !ids.insert(library.id.as_str()) {
            return Err(format!("library id `{}` is listed twice", library.id));
        }
        if !library.path.is_absolute() {
            return Err(format!(
                "library `{}` path `{}` is not absolute",
                library.id,
                library.path.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_toml_lists_extra_libraries_and_observes_markers() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let extra = dir.path().join("disk");
        fs::write(
            settings.storage_path(),
            format!(
                "[[library]]\nid = \"usb\"\npath = {}\n",
                toml::Value::String(extra.display().to_string())
            ),
        )
        .unwrap();
        let storage = Storage::load(&settings).unwrap();
        assert_eq!(storage.libraries().len(), 2);
        assert!(
            matches!(storage.availability("usb"), Availability::Unavailable(reason) if reason.contains("missing"))
        );
        assert!(storage.usable("usb").is_err());
        create_library(&extra, "other").unwrap();
        assert!(
            matches!(storage.availability("usb"), Availability::Unavailable(reason) if reason.contains("names `other`"))
        );
        create_library(&extra, "usb").unwrap();
        assert_eq!(storage.availability("usb"), Availability::Available);
        assert_eq!(storage.usable("usb").unwrap(), extra);
        assert!(matches!(
            storage.usable("nope"),
            Err(StateError::UnknownLibrary { .. })
        ));
    }

    #[test]
    fn invalid_rows_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        for text in [
            "[[library]]\nid = \"default\"\npath = \"/x\"\n",
            "[[library]]\nid = \"a\"\npath = \"relative\"\n",
            "[[library]]\nid = \"a\"\npath = \"/x\"\n[[library]]\nid = \"a\"\npath = \"/y\"\n",
            "[[library]]\nid = \"a\"\npath = \"/x\"\nextra = 1\n",
        ] {
            fs::write(settings.storage_path(), text).unwrap();
            assert!(Storage::load(&settings).is_err(), "{text}");
        }
    }
}
