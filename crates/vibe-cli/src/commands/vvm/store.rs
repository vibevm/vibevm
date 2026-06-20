//! The version store: the install-root layout, distribution instances, the
//! live `current` pointer, and the `state.toml` inventory (PROP-019 §2.4,
//! §2.5). The store reads no ambient environment — the root is resolved at
//! the composition root and handed in.

specmark::scope!("spec://vibevm/common/PROP-019#layout");

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use specmark::spec;
use thiserror::Error;

use super::model::{InstallRecord, State, VersionId};

/// The version-store layer's failure surface (PROP-019 §2.4, §2.5): reading,
/// parsing, or writing the on-disk inventory and the `current` pointer.
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub enum StoreError {
    #[error(
        "reading the VVM inventory `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#layout; \
          fix: ensure the install root is readable)"
    )]
    ReadState {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "the VVM inventory `{path}` is malformed: {detail} \
         (violates spec://vibevm/common/PROP-019#layout; \
          fix: repair or delete the corrupt state.toml)"
    )]
    ParseState { path: PathBuf, detail: String },

    #[error(
        "writing the VVM layout at `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#layout; \
          fix: ensure the install root is writable)"
    )]
    WriteLayout {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "serialising the VVM inventory failed: {detail} \
         (violates spec://vibevm/common/PROP-019#layout; \
          fix: report this — the in-memory state is malformed)"
    )]
    Serialise { detail: String },
}

/// The `vibe` binary's file name on this platform.
pub const BINARY_NAME: &str = if cfg!(windows) { "vibe.exe" } else { "vibe" };

/// Owns the on-disk layout under `$VIBEVM_INSTALL_ROOT/opt` (PROP-019 §2.4).
#[derive(Debug, Clone)]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub struct VersionStore {
    root: PathBuf,
}

impl VersionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        VersionStore { root: root.into() }
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

    /// The `vibe` binary inside a specific instance.
    pub fn binary_path(&self, id: &VersionId, instance: u64) -> PathBuf {
        self.instance_dir(id, instance).join(BINARY_NAME)
    }

    /// `<root>/vibevm/build` — the shared cargo `--target-dir` (PROP-019
    /// §2.7); never the source tree's own `target/`.
    pub fn build_dir(&self) -> PathBuf {
        self.data_dir().join("build")
    }

    /// `<root>/vibevm/src/<kind>/<id>` — a managed clone (PROP-019 §2.16).
    pub fn src_dir(&self, id: &VersionId) -> PathBuf {
        self.data_dir().join("src").join(id.path_segment())
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

    /// Load the inventory, defaulting to empty on a fresh machine.
    pub fn load_state(&self) -> Result<State, StoreError> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(State::default());
        }
        let text = fs::read_to_string(&path).map_err(|source| StoreError::ReadState {
            path: path.clone(),
            source,
        })?;
        toml::from_str(&text).map_err(|e| StoreError::ParseState {
            path,
            detail: e.to_string(),
        })
    }

    /// Write the inventory atomically (tmp + rename).
    pub fn save_state(&self, state: &State) -> Result<(), StoreError> {
        let dir = self.data_dir();
        fs::create_dir_all(&dir).map_err(|source| StoreError::WriteLayout {
            path: dir.clone(),
            source,
        })?;
        let text = toml::to_string(state).map_err(|e| StoreError::Serialise {
            detail: e.to_string(),
        })?;
        let tmp = dir.join("state.toml.tmp");
        fs::write(&tmp, text).map_err(|source| StoreError::WriteLayout {
            path: tmp.clone(),
            source,
        })?;
        fs::rename(&tmp, self.state_path()).map_err(|source| StoreError::WriteLayout {
            path: self.state_path(),
            source,
        })?;
        Ok(())
    }

    /// Allocate the next monotonic instance number (PROP-019 §9.4).
    pub fn alloc_instance(&self) -> Result<u64, StoreError> {
        let mut state = self.load_state()?;
        let n = state.next_instance.max(1);
        state.next_instance = n + 1;
        self.save_state(&state)?;
        Ok(n)
    }

    /// Upsert an instance record (replacing any with the same id+instance).
    pub fn record_install(&self, record: InstallRecord) -> Result<(), StoreError> {
        let mut state = self.load_state()?;
        state
            .installs
            .retain(|r| !(r.version_id() == record.version_id() && r.instance == record.instance));
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

    /// Drop every instance record of a version id from the inventory (no-op
    /// if absent). Does not touch files.
    pub fn forget_id(&self, id: &VersionId) -> Result<(), StoreError> {
        let mut state = self.load_state()?;
        let before = state.installs.len();
        state.installs.retain(|r| &r.version_id() != id);
        if state.installs.len() != before {
            self.save_state(&state)?;
        }
        Ok(())
    }

    /// Drop a single instance record from the inventory (no-op if absent).
    pub fn forget_instance(&self, id: &VersionId, instance: u64) -> Result<(), StoreError> {
        let mut state = self.load_state()?;
        let before = state.installs.len();
        state
            .installs
            .retain(|r| !(&r.version_id() == id && r.instance == instance));
        if state.installs.len() != before {
            self.save_state(&state)?;
        }
        Ok(())
    }

    /// The active instance dir as named by the `current` file (PROP-019 §2.5).
    pub fn read_current(&self) -> Option<PathBuf> {
        let text = fs::read_to_string(self.current_path()).ok()?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    }

    /// Repoint `current` at an instance dir, atomically (PROP-019 §2.5).
    pub fn write_current(&self, instance_dir: &Path) -> Result<(), StoreError> {
        let dir = self.data_dir();
        fs::create_dir_all(&dir).map_err(|source| StoreError::WriteLayout {
            path: dir.clone(),
            source,
        })?;
        let tmp = dir.join("current.tmp");
        fs::write(&tmp, format!("{}\n", instance_dir.display())).map_err(|source| {
            StoreError::WriteLayout {
                path: tmp.clone(),
                source,
            }
        })?;
        fs::rename(&tmp, self.current_path()).map_err(|source| StoreError::WriteLayout {
            path: self.current_path(),
            source,
        })?;
        Ok(())
    }

    /// The installed instance the `current` file points at, if any.
    pub fn active(&self) -> Result<Option<InstallRecord>, StoreError> {
        let Some(home) = self.read_current() else {
            return Ok(None);
        };
        for record in self.load_state()?.installs {
            if same_path(
                &self.instance_dir(&record.version_id(), record.instance),
                &home,
            ) {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }
}

/// Compare two paths for identity, canonicalising when both exist.
fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::vvm::model::{InstallRecord, Kind, Origin, Profile};
    use specmark::verifies;

    fn rec(kind: Kind, id: &str, instance: u64) -> InstallRecord {
        InstallRecord {
            kind,
            id: id.into(),
            instance,
            commit: "c".into(),
            toolchain: "t".into(),
            profile: Profile::Debug,
            installed_at: "now".into(),
            origin: Origin::Managed,
            source_path: None,
        }
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn instance_paths_nest_under_kind_id_instance() {
        let store = VersionStore::new("/opt");
        let id = VersionId::new(Kind::Tag, "1.2.3");
        let expect = PathBuf::from("/opt")
            .join("vibevm")
            .join("versions")
            .join("tag")
            .join("1.2.3")
            .join("4");
        assert_eq!(store.instance_dir(&id, 4), expect);
        assert_eq!(store.binary_path(&id, 4), expect.join(BINARY_NAME));
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn alloc_instance_is_monotonic_from_one() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        assert_eq!(store.alloc_instance().unwrap(), 1);
        assert_eq!(store.alloc_instance().unwrap(), 2);
        assert_eq!(store.alloc_instance().unwrap(), 3);
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#activation", r = 1)]
    fn active_follows_the_current_pointer() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Branch, "main");
        store.record_install(rec(Kind::Branch, "main", 1)).unwrap();
        let inst = store.instance_dir(&id, 1);
        fs::create_dir_all(&inst).unwrap();

        assert!(store.active().unwrap().is_none(), "no current → no active");
        store.write_current(&inst).unwrap();
        let active = store.active().unwrap().unwrap();
        assert_eq!(active.version_id(), id);
        assert_eq!(active.instance, 1);
    }
}
