//! PID-based exclusive lock for the data dir.
//!
//! At server start, we attempt to create `<data-dir>/state/server.lock`
//! with `create_new(true)`. Success means we own the lock; failure
//! means another server (or a stale PID file) is in the way. The
//! operator's choices then are:
//!
//! - `vibe-index stop <data-dir>` — read the PID, signal the server
//!   to terminate, wait for the lock to disappear.
//! - manually remove the file if the previous server crashed.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use crate::error::{Error, Result};

const FILENAME: &str = "server.lock";

#[derive(Debug)]
pub struct ServerLock {
    path: PathBuf,
}

impl ServerLock {
    pub fn try_acquire(data_dir: &Path) -> Result<ServerLock> {
        let state_dir = data_dir.join("state");
        fs::create_dir_all(&state_dir).map_err(|e| Error::Io {
            path: state_dir.clone(),
            message: e.to_string(),
        })?;
        let path = state_dir.join(FILENAME);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Error::InvalidInput(format!(
                        "another vibe-index server already holds `{}` (stop it with \
                         `vibe-index stop` or remove the file if stale)",
                        path.display()
                    ))
                } else {
                    Error::Io {
                        path: path.clone(),
                        message: e.to_string(),
                    }
                }
            })?;
        writeln!(file, "{}", process::id()).map_err(|e| Error::Io {
            path: path.clone(),
            message: e.to_string(),
        })?;
        Ok(ServerLock { path })
    }

    pub fn read_pid(data_dir: &Path) -> Option<u32> {
        let path = data_dir.join("state").join(FILENAME);
        let s = fs::read_to_string(&path).ok()?;
        s.trim().parse().ok()
    }
}

impl Drop for ServerLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn try_acquire_succeeds_then_blocks_other() {
        let dir = tempdir().unwrap();
        let lock1 = ServerLock::try_acquire(dir.path()).unwrap();
        let err = ServerLock::try_acquire(dir.path()).unwrap_err();
        match err {
            Error::InvalidInput(m) => assert!(m.contains("already holds")),
            other => panic!("unexpected error: {other:?}"),
        }
        drop(lock1);
        // After drop, lock can be acquired again.
        let _lock2 = ServerLock::try_acquire(dir.path()).unwrap();
    }

    #[test]
    fn read_pid_returns_current_process() {
        let dir = tempdir().unwrap();
        let _lock = ServerLock::try_acquire(dir.path()).unwrap();
        assert_eq!(ServerLock::read_pid(dir.path()), Some(process::id()));
    }

    #[test]
    fn read_pid_returns_none_when_absent() {
        let dir = tempdir().unwrap();
        assert_eq!(ServerLock::read_pid(dir.path()), None);
    }
}
