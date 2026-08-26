//! Crash-released exclusive lock for one boot-artifact directory.

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::WorkspaceError;
use crate::safe_file::{self, FileIdentity};

use super::io_error;

pub(super) const LOCK_NAME: &str = ".vibe-boot-artifacts.lock";

pub(super) struct BootArtifactLock {
    file: File,
    path: PathBuf,
    identity: FileIdentity,
}

impl BootArtifactLock {
    pub(super) fn acquire(parent: &Path) -> Result<Self, WorkspaceError> {
        let path = parent.join(LOCK_NAME);
        let file = loop {
            safe_file::preflight_absent_or_regular(&path)
                .map_err(|error| io_error(&path, error))?;
            match safe_file::open_existing_read_write(&path) {
                Ok(file) => break file,
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    match safe_file::create_new_read_write(&path) {
                        Ok(file) => {
                            file.sync_all().map_err(|error| io_error(&path, error))?;
                            break file;
                        }
                        Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                        Err(error) => return Err(io_error(&path, error)),
                    }
                }
                Err(error) => return Err(io_error(&path, error)),
            }
        };
        let identity = safe_file::identity(&file).map_err(|error| io_error(&path, error))?;
        file.lock().map_err(|error| io_error(&path, error))?;
        if let Err(error) = safe_file::assert_path_identity(&path, identity) {
            let _ = file.unlock();
            return Err(io_error(&path, error));
        }
        Ok(Self {
            file,
            path,
            identity,
        })
    }

    pub(super) fn assert_current(&self) -> Result<(), WorkspaceError> {
        if safe_file::identity(&self.file).map_err(|error| io_error(&self.path, error))?
            != self.identity
        {
            return Err(io_error(
                &self.path,
                "open boot lock identity changed while held",
            ));
        }
        safe_file::assert_path_identity(&self.path, self.identity)
            .map_err(|error| io_error(&self.path, error))
    }
}

impl Drop for BootArtifactLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
