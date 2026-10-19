//! Per-installation leases keep a launched payload alive during mutation.

use std::fs::{self, File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use vibe_safefs::{Project, ensure_no_follow_walk};

pub(super) fn path(settings_root: &Path, installation_key: &str) -> Result<std::path::PathBuf> {
    let settings_root = crate::commands::init::strip_unc_public(
        fs::canonicalize(settings_root).context("resolving application settings root for lease")?,
    );
    let settings = Project::open(&settings_root).context("opening application settings root")?;
    let applications = settings
        .dir(&["applications"], true)
        .context("opening application state for lease")?;
    applications
        .ensure_child("leases")
        .context("opening application leases directory")?;
    let digest = format!("{:x}", Sha256::digest(installation_key.as_bytes()));
    let path = settings_root
        .join("applications")
        .join("leases")
        .join(format!("{digest}.lock"));
    ensure_no_follow_walk(&settings_root, &path, true)?;
    Ok(path)
}

fn open_lease(settings_root: &Path, installation_key: &str) -> Result<File> {
    let path = path(settings_root, installation_key)?;
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("opening application lease `{installation_key}`"))
}

pub fn acquire_shared(settings_root: &Path, installation_key: &str) -> Result<File> {
    let file = open_lease(settings_root, installation_key)?;
    file.lock_shared()
        .with_context(|| format!("acquiring shared application lease `{installation_key}`"))?;
    Ok(file)
}

pub fn try_acquire_exclusive(settings_root: &Path, installation_key: &str) -> Result<Option<File>> {
    let file = open_lease(settings_root, installation_key)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error)
            .with_context(|| format!("acquiring exclusive application lease `{installation_key}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_lease_blocks_exclusive_for_same_installation() {
        let settings = tempfile::tempdir().unwrap();
        let shared = acquire_shared(settings.path(), "org.example/demo@1.0.0").unwrap();
        assert!(
            try_acquire_exclusive(settings.path(), "org.example/demo@1.0.0")
                .unwrap()
                .is_none()
        );
        assert!(
            try_acquire_exclusive(settings.path(), "org.example/demo@2.0.0")
                .unwrap()
                .is_some()
        );
        drop(shared);
        assert!(
            try_acquire_exclusive(settings.path(), "org.example/demo@1.0.0")
                .unwrap()
                .is_some()
        );
    }
}
