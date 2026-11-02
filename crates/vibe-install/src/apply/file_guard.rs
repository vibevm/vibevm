//! The project files an apply rewrites belong to the same undo as its slots.
//! An explicit install records its roots in `vibe.toml` before anything is
//! placed, and `vibe.lock` and its provenance sidecar are two writes, so a
//! failure anywhere before the last of them must put every one of these
//! files back as it was.

use std::path::{Path, PathBuf};

use crate::error::Result;

/// One file's bytes before this apply rewrote it — or the fact that it did
/// not exist — restored on drop unless [`FileGuard::disarm`] ran, so every
/// early return between the write and the end of the apply undoes it without
/// a branch of its own.
pub(super) struct FileGuard {
    armed: Option<(PathBuf, Option<Vec<u8>>)>,
}

impl FileGuard {
    /// A guard with nothing to restore: the apply does not rewrite the file.
    pub(super) fn idle() -> Self {
        Self { armed: None }
    }

    /// Remember `path`'s bytes, or its absence, before the apply rewrites it.
    pub(super) fn before_write(path: &Path) -> Result<Self> {
        let before = match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(vibe_workspace::WorkspaceError::Io {
                    path: path.to_path_buf(),
                    reason: error.to_string(),
                }
                .into());
            }
        };
        Ok(Self {
            armed: Some((path.to_path_buf(), before)),
        })
    }

    /// The install is durable: keep the rewritten file.
    pub(super) fn disarm(&mut self) {
        self.armed = None;
    }
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        let Some((path, before)) = self.armed.take() else {
            return;
        };
        let restored = match before {
            Some(bytes) => std::fs::write(&path, bytes),
            None => match std::fs::remove_file(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            },
        };
        if let Err(error) = restored {
            // A drop cannot fail the command, and the install already failed
            // with its own error; the log names what is left behind.
            tracing::warn!(
                target: "vibe_install::apply",
                "could not restore {} after the failed install: {error}",
                path.display()
            );
        }
    }
}
