//! The application home (the user settings root or a project home) and its
//! single mutation lock.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#index");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#project-scope");

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use specmark::spec;

use crate::error::StateError;

/// An application home: the current user's settings root (`~/.vibe` or
/// `$VIBE_SETTINGS`) or a project home `<project>/vibevm/vibeopt`, threaded in
/// by the caller; this crate never reads the environment itself. Both have
/// the same layout; a project home keeps its downloads and build work
/// directories in the user home's `cache/`.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// assert!(settings.index_path().ends_with("applications/index.json"));
/// assert!(settings.app_sources_cache().ends_with("cache/app-sources"));
/// let _lock = settings.lock().unwrap();
/// assert!(settings.lock().is_err(), "a second mutation is refused");
///
/// let project = vibe_app::SettingsRoot::project(&dir.path().join("p"), &settings);
/// assert!(project.proxy_dir().ends_with("p/vibevm/vibeopt/opt/bin"));
/// assert_eq!(project.app_sources_cache(), settings.app_sources_cache());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct SettingsRoot {
    root: PathBuf,
    cache: PathBuf,
}

/// Held for the duration of one mutating command; dropping it releases the
/// operating-system file lock.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub struct MutationLock {
    _file: File,
}

/// The project home relative to its project root.
pub const PROJECT_HOME: [&str; 2] = ["vibevm", "vibeopt"];

impl SettingsRoot {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            cache: root.join("cache"),
        }
    }

    /// The project home `<project_root>/vibevm/vibeopt`, sharing the source
    /// cache and build directories of `user`.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#project-scope")]
    pub fn project(project_root: &Path, user: &SettingsRoot) -> Self {
        Self {
            root: PROJECT_HOME
                .iter()
                .fold(project_root.to_path_buf(), |path, part| path.join(part)),
            cache: user.cache.clone(),
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

    /// `<settings>/cache/app-sources`: verified `source_build` downloads,
    /// one `<sha256>/<filename>` per input (the user home's for a project).
    pub fn app_sources_cache(&self) -> PathBuf {
        self.cache.join("app-sources")
    }

    /// `<home>/cache/app-work` (a project home keeps its own work directories;
    /// only the sources cache is shared): one step work directory per
    /// installation key (`PROP-065#execution`); a failed sequence leaves
    /// its directory and `steps.json` here for inspection and `--resume`.
    pub fn app_work_dir(&self) -> PathBuf {
        self.root.join("cache").join("app-work")
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
