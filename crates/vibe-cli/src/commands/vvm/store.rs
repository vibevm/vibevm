//! The version store: the install-root layout, distribution instances, the
//! live `current` pointer, and the `state.toml` inventory (PROP-019 §2.4,
//! §2.5). The store reads no ambient environment — the root is resolved at
//! the composition root and handed in.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#layout");

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use specmark::spec;
use thiserror::Error;

use super::model::{InstallRecord, State, VersionId};

#[path = "store_guard.rs"]
mod guard;
pub(crate) use guard::open_regular_no_follow;

#[derive(serde::Serialize, serde::Deserialize)]
struct ActivationJournal {
    current: Option<String>,
    previous: Option<String>,
}

/// The version-store layer's failure surface (PROP-019 §2.4, §2.5): reading,
/// parsing, or writing the on-disk inventory and the `current` pointer.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
pub enum StoreError {
    #[error(
        "reading the VVM inventory `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the install root is readable)"
    )]
    ReadState {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "the VVM inventory `{path}` is malformed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: repair or delete the corrupt state.toml)"
    )]
    ParseState { path: PathBuf, detail: String },

    #[error(
        "writing the VVM layout at `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the install root is writable)"
    )]
    WriteLayout {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "serialising the VVM inventory failed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: report this — the in-memory state is malformed)"
    )]
    Serialise { detail: String },

    #[error(
        "refusing to rewrite immutable local instance `{selector}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: allocate a new terminal #N instance instead)"
    )]
    ImmutableInstance { selector: String },

    #[error(
        "refusing unsafe VVM identity: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: use a relative git ref that remains below the versions root)"
    )]
    UnsafeIdentity { detail: String },

    #[error(
        "refusing unsafe VVM mutation path `{path}`: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: remove the symlink/reparse/special path component from the VVM store)"
    )]
    UnsafeMutationPath { path: PathBuf, detail: String },
}

/// The `vibe` binary's file name on this platform.
pub const BINARY_NAME: &str = if cfg!(windows) { "vibe.exe" } else { "vibe" };
/// The companion `vibe-index` binary's file name on this platform.
pub const INDEX_BINARY_NAME: &str = if cfg!(windows) {
    "vibe-index.exe"
} else {
    "vibe-index"
};

/// Owns the on-disk layout under `$VIBEVM_INSTALL_ROOT/opt` (PROP-019 §2.4).
#[derive(Debug, Clone)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
pub struct VersionStore {
    root: PathBuf,
}

impl VersionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        VersionStore { root: root.into() }
    }

    /// Prove that a mutation target is contained by this store and that no
    /// existing component below its canonical root is a symlink, reparse
    /// point, or special file. The trusted root is created component-wise on
    /// first use; callers invoke this immediately before create/rename/delete.
    pub(crate) fn guard_mutation_path(&self, path: &Path) -> Result<(), StoreError> {
        guard::guard_mutation_path(&self.root, path).map_err(|source| {
            StoreError::UnsafeMutationPath {
                path: path.to_path_buf(),
                detail: source.to_string(),
            }
        })
    }

    /// The stronger guard for opaque/recursive mutators (Cargo, Git and
    /// recursive deletion): reject redirects or special objects anywhere in
    /// an already-existing target tree before handing it off.
    pub(crate) fn guard_mutation_tree(&self, path: &Path) -> Result<(), StoreError> {
        self.guard_mutation_path(path)?;
        guard::guard_existing_tree(path).map_err(|source| StoreError::UnsafeMutationPath {
            path: path.to_path_buf(),
            detail: source.to_string(),
        })
    }

    /// `<root>/bin` — the shim directory that goes on PATH (PROP-019 §2.5).
    pub fn shim_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// `<root>/vibevm` — the data root.
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("vibevm")
    }

    /// `<root>/vibevm/versions`.
    pub fn versions_dir(&self) -> PathBuf {
        self.data_dir().join("versions")
    }

    /// `<root>/vibevm/versions/<kind>/<id>` — the parent of a version's
    /// instance dirs.
    pub fn version_id_dir(&self, id: &VersionId) -> PathBuf {
        self.versions_dir().join(id.path_segment())
    }

    /// `<root>/vibevm/versions/<kind>/<id>/<instance>` — one immutable
    /// distribution instance (PROP-019 §2.4, §2.15).
    pub fn instance_dir(&self, id: &VersionId, instance: u64) -> PathBuf {
        self.version_id_dir(id).join(instance.to_string())
    }

    /// The modern bundle's private executable directory.
    pub fn instance_bin_dir(&self, id: &VersionId, instance: u64) -> PathBuf {
        self.instance_dir(id, instance).join("bin")
    }

    /// The `vibe` binary inside a specific instance.
    pub fn binary_path(&self, id: &VersionId, instance: u64) -> PathBuf {
        let modern = self.instance_bin_dir(id, instance).join(BINARY_NAME);
        let legacy = self.instance_dir(id, instance).join(BINARY_NAME);
        if modern.exists() || !legacy.exists() {
            modern
        } else {
            legacy
        }
    }

    /// The companion binary in a modern bundle instance.
    pub fn index_binary_path(&self, id: &VersionId, instance: u64) -> PathBuf {
        self.instance_bin_dir(id, instance).join(INDEX_BINARY_NAME)
    }

    /// The exact source snapshot owned by a modern binary instance.
    pub fn instance_source_dir(&self, id: &VersionId, instance: u64) -> PathBuf {
        self.instance_dir(id, instance).join("source")
    }

    /// `<root>/vibevm/build` — the shared cargo `--target-dir` (PROP-019
    /// §2.7); never the source tree's own `target/`.
    pub fn build_dir(&self) -> PathBuf {
        self.data_dir().join("build")
    }

    /// `<root>/vibevm/src/.mirror` — the shared managed clone, fetched and
    /// checked out per build (PROP-019 §2.16).
    pub fn mirror_dir(&self) -> PathBuf {
        self.data_dir().join("src").join(".mirror")
    }

    /// `<root>/vibevm/state.toml`.
    pub fn state_path(&self) -> PathBuf {
        self.data_dir().join("state.toml")
    }

    /// `<root>/vibevm/current` — the live pointer to the active instance dir
    /// (PROP-019 §2.5).
    pub fn current_path(&self) -> PathBuf {
        self.data_dir().join("current")
    }

    /// `<root>/vibevm/previous` — the immediate local rollback pointer.
    pub fn previous_path(&self) -> PathBuf {
        self.data_dir().join("previous")
    }

    fn activation_journal_path(&self) -> PathBuf {
        self.data_dir().join("activation.pending.toml")
    }

    /// Load the inventory, defaulting to empty on a fresh machine.
    pub fn load_state(&self) -> Result<State, StoreError> {
        let path = self.state_path();
        let Some(text) = self.read_text_file(&path, 4 * 1024 * 1024)? else {
            return Ok(State::default());
        };
        let state: State = toml::from_str(&text).map_err(|e| StoreError::ParseState {
            path: path.clone(),
            detail: e.to_string(),
        })?;
        for record in &state.installs {
            record
                .version_id()
                .validate()
                .map_err(|error| StoreError::ParseState {
                    path: path.clone(),
                    detail: error.to_string(),
                })?;
        }
        Ok(state)
    }

    /// Write the inventory atomically (tmp + rename).
    pub fn save_state(&self, state: &State) -> Result<(), StoreError> {
        let dir = self.data_dir();
        self.guard_mutation_path(&dir)?;
        fs::create_dir_all(&dir).map_err(|source| StoreError::WriteLayout {
            path: dir.clone(),
            source,
        })?;
        let text = toml::to_string(state).map_err(|e| StoreError::Serialise {
            detail: e.to_string(),
        })?;
        let path = self.state_path();
        self.guard_mutation_path(&path)?;
        guard::atomic_replace(&path, text.as_bytes())
            .map_err(|source| StoreError::WriteLayout { path, source })?;
        Ok(())
    }

    /// Allocate the next monotonic instance number (PROP-019 §9.4).
    pub fn alloc_instance(&self) -> Result<u64, StoreError> {
        let mut state = self.load_state()?;
        let after_existing = state
            .installs
            .iter()
            .map(|record| record.instance)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let n = state.next_instance.max(after_existing).max(1);
        state.next_instance = n + 1;
        self.save_state(&state)?;
        Ok(n)
    }

    /// Record one immutable local instance. Replaying the exact same record is
    /// idempotent; changing metadata behind an existing terminal `#N` refuses.
    pub fn record_install(&self, record: InstallRecord) -> Result<(), StoreError> {
        record
            .version_id()
            .validate()
            .map_err(|error| StoreError::UnsafeIdentity {
                detail: error.to_string(),
            })?;
        let mut state = self.load_state()?;
        if let Some(existing) = state.installs.iter().find(|existing| {
            existing.version_id() == record.version_id() && existing.instance == record.instance
        }) {
            if existing == &record {
                return Ok(());
            }
            return Err(StoreError::ImmutableInstance {
                selector: record.selector().to_string(),
            });
        }
        state.installs.push(record);
        self.save_state(&state)
    }

    /// All recorded instances of a version id.
    pub fn instances_of(&self, id: &VersionId) -> Result<Vec<InstallRecord>, StoreError> {
        Ok(self
            .load_state()?
            .installs
            .into_iter()
            .filter(|r| &r.version_id() == id)
            .collect())
    }

    fn read_text_file(&self, path: &Path, maximum: u64) -> Result<Option<String>, StoreError> {
        guard::read_bounded_utf8(path, maximum).map_err(|source| StoreError::ReadState {
            path: path.to_path_buf(),
            source,
        })
    }

    fn read_pointer_file(&self, path: &Path) -> Result<Option<PathBuf>, StoreError> {
        let Some(text) = self.read_text_file(path, 64 * 1024)? else {
            return Ok(None);
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(trimmed)))
        }
    }

    fn read_activation_journal(&self) -> Result<Option<ActivationJournal>, StoreError> {
        let path = self.activation_journal_path();
        let Some(text) = self.read_text_file(&path, 64 * 1024)? else {
            return Ok(None);
        };
        toml::from_str(&text)
            .map(Some)
            .map_err(|error| StoreError::ParseState {
                path,
                detail: error.to_string(),
            })
    }

    fn logical_pointer(&self, current: bool) -> Result<Option<PathBuf>, StoreError> {
        if let Some(journal) = self.read_activation_journal()? {
            let journal_current = journal.current.map(PathBuf::from);
            let journal_previous = journal.previous.map(PathBuf::from);
            for path in [journal_current.as_deref(), journal_previous.as_deref()]
                .into_iter()
                .flatten()
            {
                self.require_inventoried_instance(path)?;
            }
            return Ok(if current {
                journal_current
            } else {
                journal_previous
            });
        }
        let path = if current {
            self.read_pointer_file(&self.current_path())?
        } else {
            self.read_pointer_file(&self.previous_path())?
        };
        if let Some(path) = &path {
            self.require_inventoried_instance(path)?;
        }
        Ok(path)
    }

    /// The active instance dir as named by the `current` file (PROP-019 §2.5).
    pub fn read_current(&self) -> Result<Option<PathBuf>, StoreError> {
        self.logical_pointer(true)
    }

    /// The instance dir saved for an immediate local rollback.
    pub fn read_previous(&self) -> Result<Option<PathBuf>, StoreError> {
        self.logical_pointer(false)
    }

    fn write_pointer(&self, name: &str, instance_dir: &Path) -> Result<(), StoreError> {
        let dir = self.data_dir();
        self.guard_mutation_path(&dir)?;
        fs::create_dir_all(&dir).map_err(|source| StoreError::WriteLayout {
            path: dir.clone(),
            source,
        })?;
        let path = dir.join(name);
        self.guard_mutation_path(&path)?;
        guard::atomic_replace(&path, format!("{}\n", instance_dir.display()).as_bytes())
            .map_err(|source| StoreError::WriteLayout { path, source })?;
        Ok(())
    }

    fn clear_pointer(&self, path: &Path) -> Result<(), StoreError> {
        self.guard_mutation_path(path)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(StoreError::WriteLayout {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    fn apply_pointer(&self, name: &str, value: Option<&Path>) -> Result<(), StoreError> {
        match value {
            Some(path) => self.write_pointer(name, path),
            None => self.clear_pointer(&self.data_dir().join(name)),
        }
    }

    fn preflight_file_leaf(&self, path: &Path) -> Result<(), StoreError> {
        guard::preflight_file_leaf(&self.root, path).map_err(|source| {
            StoreError::UnsafeMutationPath {
                path: path.to_path_buf(),
                detail: source.to_string(),
            }
        })
    }

    fn write_activation(
        &self,
        current: Option<&Path>,
        previous: Option<&Path>,
    ) -> Result<(), StoreError> {
        if let Some(path) = current {
            self.require_inventoried_instance(path)?;
        }
        if let Some(path) = previous {
            self.require_inventoried_instance(path)?;
        }
        for destination in [
            self.activation_journal_path(),
            self.current_path(),
            self.previous_path(),
        ] {
            self.preflight_file_leaf(&destination)?;
        }
        let journal = ActivationJournal {
            current: current.map(|path| path.display().to_string()),
            previous: previous.map(|path| path.display().to_string()),
        };
        let text = toml::to_string(&journal).map_err(|error| StoreError::Serialise {
            detail: error.to_string(),
        })?;
        let path = self.activation_journal_path();
        self.guard_mutation_path(&self.data_dir())?;
        self.guard_mutation_path(&path)?;
        fs::create_dir_all(self.data_dir()).map_err(|source| StoreError::WriteLayout {
            path: self.data_dir(),
            source,
        })?;
        guard::atomic_replace(&path, text.as_bytes()).map_err(|source| {
            StoreError::WriteLayout {
                path: path.clone(),
                source,
            }
        })?;
        self.apply_pointer("current", current)?;
        self.apply_pointer("previous", previous)?;
        self.clear_pointer(&path)
    }

    pub(crate) fn recover_activation_locked(&self) -> Result<(), StoreError> {
        let path = self.activation_journal_path();
        let Some(journal) = self.read_activation_journal()? else {
            return Ok(());
        };
        let current = journal.current.as_deref().map(Path::new);
        let previous = journal.previous.as_deref().map(Path::new);
        if let Some(path) = current {
            self.require_inventoried_instance(path)?;
        }
        if let Some(path) = previous {
            self.require_inventoried_instance(path)?;
        }
        for destination in [path.clone(), self.current_path(), self.previous_path()] {
            self.preflight_file_leaf(&destination)?;
        }
        self.apply_pointer("current", current)?;
        self.apply_pointer("previous", previous)?;
        self.clear_pointer(&path)
    }

    /// Repoint `current` atomically and remember the displaced valid instance
    /// as the immediate rollback target.
    pub fn write_current(&self, instance_dir: &Path) -> Result<(), StoreError> {
        let mut previous = self
            .previous()?
            .map(|record| self.instance_dir(&record.version_id(), record.instance));
        if let Some(old) = self.active()?
            && !guard::same_path(
                &self.instance_dir(&old.version_id(), old.instance),
                instance_dir,
            )
        {
            previous = Some(self.instance_dir(&old.version_id(), old.instance));
        }
        self.write_activation(Some(instance_dir), previous.as_deref())
    }

    /// Repair activation around destructive removal without recording the
    /// soon-to-be-deleted current instance as rollback history.
    pub fn reset_activation(
        &self,
        current: Option<&Path>,
        previous: Option<&Path>,
    ) -> Result<(), StoreError> {
        self.write_activation(current, previous)
    }

    fn require_inventoried_instance(&self, path: &Path) -> Result<InstallRecord, StoreError> {
        if !guard::absolute_normal_path(path) {
            return Err(StoreError::ParseState {
                path: path.to_path_buf(),
                detail: "pointer must be an absolute path with only normal components".to_string(),
            });
        }
        let record = self
            .load_state()?
            .installs
            .into_iter()
            .find(|record| {
                guard::lexical_path_eq(
                    &self.instance_dir(&record.version_id(), record.instance),
                    path,
                )
            })
            .ok_or_else(|| StoreError::ParseState {
                path: path.to_path_buf(),
                detail: "pointer does not name an exact inventoried instance".to_string(),
            })?;
        let expected = self.instance_dir(&record.version_id(), record.instance);
        self.guard_mutation_path(&expected)?;
        let metadata = match fs::symlink_metadata(&expected) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(record),
            Err(source) => {
                return Err(StoreError::ReadState {
                    path: expected.clone(),
                    source,
                });
            }
        };
        if !metadata.file_type().is_dir() {
            return Err(StoreError::ParseState {
                path: expected,
                detail: "inventoried instance path is not a directory".to_string(),
            });
        }
        Ok(record)
    }

    /// Find the inventory record whose immutable instance dir is `path`.
    pub fn record_at(&self, path: &Path) -> Result<Option<InstallRecord>, StoreError> {
        let record = self.load_state()?.installs.into_iter().find(|record| {
            guard::lexical_path_eq(
                &self.instance_dir(&record.version_id(), record.instance),
                path,
            )
        });
        if let Some(record) = record {
            self.guard_mutation_path(&self.instance_dir(&record.version_id(), record.instance))?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// The installed instance the `current` file points at, if any.
    pub fn active(&self) -> Result<Option<InstallRecord>, StoreError> {
        let Some(home) = self.read_current()? else {
            return Ok(None);
        };
        self.require_inventoried_instance(&home).map(Some)
    }

    /// The recorded immediate rollback instance, if the sidecar is present
    /// and still names an inventoried local payload.
    pub fn previous(&self) -> Result<Option<InstallRecord>, StoreError> {
        let Some(home) = self.read_previous()? else {
            return Ok(None);
        };
        self.require_inventoried_instance(&home).map(Some)
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
