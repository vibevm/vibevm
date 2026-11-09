//! Full-tree cooperative ownership witnesses for prepared native placements.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::io_err;
use crate::WorkspaceError;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use vibe_safefs::{OwnedFileReceipt, Pinned, Project, StableFileSnapshot};

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    identity: (u64, u64),
    mode: u32,
    kind: u8,
    bytes: Vec<u8>,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Tree(BTreeMap<PathBuf, Entry>);
impl Tree {
    pub(super) fn read(path: &Path) -> Result<Self, WorkspaceError> {
        fn visit(
            root: &Path,
            path: &Path,
            out: &mut BTreeMap<PathBuf, Entry>,
        ) -> Result<(), WorkspaceError> {
            let metadata = fs::symlink_metadata(path).map_err(|e| io_err(path, e))?;
            #[cfg(unix)]
            let (identity, mode) = {
                use std::os::unix::fs::MetadataExt;
                ((metadata.dev(), metadata.ino()), metadata.mode())
            };
            // Windows: creation and last-write times stand in for the inode
            // (PROP-073 `#NATIVE-WINDOWS`); a replaced file has new ones.
            #[cfg(windows)]
            let (identity, mode) = {
                use std::os::windows::fs::MetadataExt;
                (
                    (metadata.creation_time(), metadata.last_write_time()),
                    u32::from(metadata.permissions().readonly()),
                )
            };
            #[cfg(not(any(unix, windows)))]
            let (identity, mode) = ((0, 0), u32::from(metadata.permissions().readonly()));
            let (kind, bytes) = if metadata.is_dir() {
                (0, Vec::new())
            } else if metadata.is_file() {
                let mut file = fs::File::open(path).map_err(|e| io_err(path, e))?;
                let mut hash = Sha256::new();
                std::io::copy(&mut file, &mut hash).map_err(|e| io_err(path, e))?;
                (1, hash.finalize().to_vec())
            } else if metadata.file_type().is_symlink() {
                (
                    2,
                    fs::read_link(path)
                        .map_err(|e| io_err(path, e))?
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec(),
                )
            } else {
                return Err(WorkspaceError::SpecMaterialization {
                    path: path.into(),
                    reason: "prepared placement contains a special file".into(),
                });
            };
            out.insert(
                path.strip_prefix(root)
                    .map_err(|e| WorkspaceError::SpecMaterialization {
                        path: path.into(),
                        reason: e.to_string(),
                    })?
                    .into(),
                Entry {
                    identity,
                    mode,
                    kind,
                    bytes,
                },
            );
            if metadata.is_dir() {
                for entry in fs::read_dir(path).map_err(|e| io_err(path, e))? {
                    visit(root, &entry.map_err(|e| io_err(path, e))?.path(), out)?;
                }
            }
            Ok(())
        }
        let mut entries = BTreeMap::new();
        visit(path, path, &mut entries)?;
        Ok(Self(entries))
    }
    pub(super) fn same_content(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().all(|(path, e)| {
                other
                    .0
                    .get(path)
                    .is_some_and(|o| (e.mode, e.kind, &e.bytes) == (o.mode, o.kind, &o.bytes))
            })
    }
}

pub(super) struct Ignore {
    project: Project,
    parent: Pinned,
    before: Option<StableFileSnapshot>,
    receipt: Option<OwnedFileReceipt>,
    after_len: usize,
    after_bytes: Option<Vec<u8>>,
}
impl std::fmt::Debug for Ignore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CheckedIgnore")
    }
}
impl Ignore {
    pub(super) fn capture(root: &Path) -> Result<Self, WorkspaceError> {
        let fail = |e| WorkspaceError::SpecMaterialization {
            path: root.join(".gitignore"),
            reason: format!("checked ignore: {e}"),
        };
        let project = Project::open(root).map_err(|e| fail(format!("{e:#}")))?;
        let parent = project.root_dir().map_err(|e| fail(format!("{e:#}")))?;
        let before = project
            .read_file_snapshot_bounded_in(&parent, ".gitignore", usize::MAX - 1)
            .map_err(|e| fail(format!("{e:#}")))?;
        Ok(Self {
            project,
            parent,
            before,
            receipt: None,
            after_len: 0,
            after_bytes: None,
        })
    }
    pub(super) fn before_bytes(&self) -> Option<&[u8]> {
        self.before.as_ref().map(|s| s.bytes.as_slice())
    }
    pub(super) fn publish(&mut self, entry: &str) -> Result<(), String> {
        let previous = self
            .after_bytes
            .as_deref()
            .or_else(|| self.before_bytes())
            .unwrap_or_default();
        let previous = std::str::from_utf8(previous).map_err(|e| e.to_string())?;
        let now = self
            .project
            .read_file_snapshot_bounded_in(&self.parent, ".gitignore", usize::MAX - 1)
            .map_err(|e| format!("{e:#}"))?;
        let owned = match &self.receipt {
            Some(receipt) => now.as_ref().is_some_and(|s| receipt.matches_snapshot(s)),
            None => now == self.before,
        };
        if !owned {
            return Err(".gitignore changed before publication".into());
        }
        let Some(after) = crate::vibedeps::render_gitignore_entry(previous, entry) else {
            return Ok(());
        };
        match self.project.write_atomic_owned_in_with_mode(
            &self.parent,
            ".gitignore",
            after.as_bytes(),
            self.before.as_ref().and_then(|s| s.unix_mode),
        ) {
            Ok(written) => {
                self.receipt = Some(written.receipt);
                self.after_len = after.len();
                self.after_bytes = Some(after.into_bytes());
                Ok(())
            }
            Err(error) => {
                if let Some(receipt) = error.receipt {
                    self.receipt = Some(receipt);
                    self.after_len = after.len();
                    self.after_bytes = Some(after.into_bytes());
                }
                Err(format!("{}", error.publication.into_report()))
            }
        }
    }
    pub(super) fn undo(self) -> Result<(), String> {
        let Some(receipt) = self.receipt else {
            return Ok(());
        };
        let now = self
            .project
            .read_file_snapshot_bounded_in(
                &self.parent,
                ".gitignore",
                self.after_len
                    .max(self.before.as_ref().map_or(0, |s| s.bytes.len())),
            )
            .map_err(|e| format!("{e:#}"))?;
        if now == self.before {
            return Ok(());
        }
        if !now.as_ref().is_some_and(|s| receipt.matches_snapshot(s)) {
            return Err(".gitignore rollback conflict; foreign bytes preserved".into());
        }
        match self.before {
            Some(before) => self
                .project
                .write_atomic_in_with_mode(
                    &self.parent,
                    ".gitignore",
                    &before.bytes,
                    before.unix_mode,
                )
                .map(drop)
                .map_err(|e| format!("{}", e.into_report())),
            None => self
                .project
                .remove_file_in(&self.parent, ".gitignore")
                .map(|_| ())
                .map_err(|e| format!("{e:#}")),
        }
    }
}
