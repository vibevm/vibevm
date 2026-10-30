//! The user settings root and its single mutation lock.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#index");

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use specmark::spec;

use crate::error::StateError;

/// The current user's settings root (`~/.vibe` or `$VIBE_SETTINGS`), threaded
/// in by the caller; this crate never reads the environment itself.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// assert!(settings.index_path().ends_with("applications/index.json"));
/// let _lock = settings.lock().unwrap();
/// assert!(settings.lock().is_err(), "a second mutation is refused");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct SettingsRoot {
    root: PathBuf,
}

/// Held for the duration of one mutating command; dropping it releases the
/// operating-system file lock.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct MutationLock {
    _file: File,
}

impl SettingsRoot {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn applications_dir(&self) -> PathBuf {
        self.root.join("applications")
    }

    pub fn index_path(&self) -> PathBuf {
        self.applications_dir().join("index.json")
    }

    pub fn storage_path(&self) -> PathBuf {
        self.root.join("storage.toml")
    }

    pub fn default_library_path(&self) -> PathBuf {
        self.root.join("opt")
    }

    /// `<settings>/opt/bin`, where every reserved command has its PATH proxy.
    pub fn proxy_dir(&self) -> PathBuf {
        self.root.join("opt").join("bin")
    }

    /// Take the `applications/.mutation.lock` exclusive lock, creating the
    /// state directory on first use. A held lock is refused, never awaited.
    pub fn lock(&self) -> Result<MutationLock, StateError> {
        let dir = self.applications_dir();
        fs::create_dir_all(&dir).map_err(|source| io("creating", &dir, source))?;
        let path = dir.join(".mutation.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|source| io("opening", &path, source))?;
        file.try_lock().map_err(|_| StateError::Locked {
            path: path.display().to_string(),
        })?;
        Ok(MutationLock { _file: file })
    }
}

pub(crate) fn io(action: &'static str, path: &Path, source: std::io::Error) -> StateError {
    StateError::Io {
        action,
        path: path.display().to_string(),
        source,
    }
}

/// Replace `path` atomically: write a sibling, flush it, rename it over.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StateError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|source| io("creating", parent, source))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sibling = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = File::create(&sibling).map_err(|source| io("writing", &sibling, source))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| io("writing", &sibling, source))?;
    drop(file);
    fs::rename(&sibling, path).map_err(|source| {
        let _ = fs::remove_file(&sibling);
        io("replacing", path, source)
    })
}
