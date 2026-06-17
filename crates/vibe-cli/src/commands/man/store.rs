//! The version store: the `$VIBEVM_ROOT` layout and the `state.toml`
//! inventory (PROP-019 §2.4). The store reads no ambient environment — the
//! root is resolved at the composition root and handed in (§2.1).

specmark::scope!("spec://vibevm/common/PROP-019#layout");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use specmark::spec;

use super::model::{InstallRecord, State, VersionId};

/// The `vibe` binary's file name on this platform.
pub const BINARY_NAME: &str = if cfg!(windows) { "vibe.exe" } else { "vibe" };

/// Owns the on-disk layout under `$VIBEVM_ROOT` (PROP-019 §2.4).
#[derive(Debug, Clone)]
#[spec(implements = "spec://vibevm/common/PROP-019#layout")]
pub struct VersionStore {
    root: PathBuf,
}

impl VersionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        VersionStore { root: root.into() }
    }

    /// `<root>/vibevm` — the data root holding versions, sources, state.
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("vibevm")
    }

    /// `<root>/vibevm/versions`.
    pub fn versions_dir(&self) -> PathBuf {
        self.data_dir().join("versions")
    }

    /// `<root>/vibevm/build` — the shared cargo `--target-dir` for builds,
    /// kept out of both the source tree's `target/` and the running binary's
    /// path so a build never relinks a live `vibe.exe` (PROP-019 §2.7).
    pub fn build_dir(&self) -> PathBuf {
        self.data_dir().join("build")
    }

    /// `<root>/bin` — the shim directory that goes on PATH (PROP-019 §2.5).
    pub fn shim_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// `<root>/vibevm/src/<kind>/<id>` — a version's source tree (clone
    /// path; gc-able, PROP-019 §2.4).
    pub fn src_dir(&self, id: &VersionId) -> PathBuf {
        self.data_dir().join("src").join(id.path_segment())
    }

    /// Drop a version from the inventory (no-op if absent, PROP-019 §2.9).
    pub fn forget(&self, id: &VersionId) -> Result<()> {
        let mut state = self.load_state()?;
        let before = state.installs.len();
        state.installs.retain(|r| &r.version_id() != id);
        if state.installs.len() != before {
            self.save_state(&state)?;
        }
        Ok(())
    }

    /// `<root>/vibevm/versions/<kind>/<id>` — the prefix `VIBEVM_HOME`
    /// points at when this version is active (PROP-019 §2.5).
    pub fn version_prefix(&self, id: &VersionId) -> PathBuf {
        self.versions_dir().join(id.path_segment())
    }

    /// The installed `vibe` binary for a version.
    pub fn binary_path(&self, id: &VersionId) -> PathBuf {
        self.version_prefix(id).join(BINARY_NAME)
    }

    /// `<root>/vibevm/state.toml`.
    pub fn state_path(&self) -> PathBuf {
        self.data_dir().join("state.toml")
    }

    /// Load the inventory, returning the empty default when the file is
    /// absent (a fresh machine).
    pub fn load_state(&self) -> Result<State> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(State::default());
        }
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing `{}`", path.display()))
    }

    /// Write the inventory atomically (tmp + rename) so a crash mid-write
    /// never truncates `state.toml`.
    pub fn save_state(&self, state: &State) -> Result<()> {
        let dir = self.data_dir();
        fs::create_dir_all(&dir).with_context(|| format!("creating `{}`", dir.display()))?;
        let text = toml::to_string(state).context("serialising VVM state")?;
        let path = self.state_path();
        let tmp = dir.join("state.toml.tmp");
        fs::write(&tmp, text).with_context(|| format!("writing `{}`", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("renaming into `{}`", path.display()))?;
        Ok(())
    }

    /// Upsert an install record, replacing any existing entry with the same
    /// canonical id (PROP-019 §2.7).
    pub fn record_install(&self, record: InstallRecord) -> Result<()> {
        let mut state = self.load_state()?;
        let id = record.version_id();
        state.installs.retain(|r| r.version_id() != id);
        state.installs.push(record);
        self.save_state(&state)
    }

    /// The installed version whose prefix matches `active_home` (the
    /// `VIBEVM_HOME` value), if any (PROP-019 §2.5).
    pub fn active(&self, active_home: Option<&Path>) -> Result<Option<InstallRecord>> {
        let Some(home) = active_home else {
            return Ok(None);
        };
        for record in self.load_state()?.installs {
            if same_path(&self.version_prefix(&record.version_id()), home) {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }
}

/// Compare two paths for identity, canonicalising when both exist so that
/// separator, `.`, and symlink differences do not cause a false miss.
fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::man::model::{InstallRecord, Kind, State};
    use specmark::verifies;

    fn write_state(store: &VersionStore, state: &State) {
        fs::create_dir_all(store.data_dir()).unwrap();
        fs::write(store.state_path(), toml::to_string(state).unwrap()).unwrap();
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn layout_paths_are_namespaced_by_kind() {
        let store = VersionStore::new("/opt");
        let tag = VersionId::new(Kind::Tag, "1.2.3");
        let expect = PathBuf::from("/opt")
            .join("vibevm")
            .join("versions")
            .join("tag")
            .join("1.2.3");
        assert_eq!(store.version_prefix(&tag), expect);
        assert_eq!(store.binary_path(&tag), expect.join(BINARY_NAME));
        assert_eq!(
            store.state_path(),
            PathBuf::from("/opt").join("vibevm").join("state.toml")
        );
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#layout", r = 1)]
    fn load_state_defaults_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        assert!(store.load_state().unwrap().installs.is_empty());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#introspection", r = 1)]
    fn active_matches_the_prefix_named_by_vibevm_home() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Branch, "main");
        write_state(
            &store,
            &State {
                installs: vec![InstallRecord {
                    kind: Kind::Branch,
                    id: "main".into(),
                    commit: "abc".into(),
                    toolchain: "rustc 1.93.0".into(),
                    profile: "debug".into(),
                    installed_at: "2026-06-17T00:00:00Z".into(),
                }],
            },
        );
        let prefix = store.version_prefix(&id);
        fs::create_dir_all(&prefix).unwrap();

        // The prefix VIBEVM_HOME names is the active one.
        let active = store.active(Some(&prefix)).unwrap().unwrap();
        assert_eq!(active.version_id(), id);
        // A different prefix → no active match.
        assert!(store.active(Some(tmp.path())).unwrap().is_none());
        // No VIBEVM_HOME → nothing active.
        assert!(store.active(None).unwrap().is_none());
    }
}
