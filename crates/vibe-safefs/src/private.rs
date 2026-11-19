//! User-private filesystem capabilities for the explicitly managed local keyring.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use crate::{LockGuard, Pinned};
use anyhow::{Result, bail};

#[cfg(unix)]
fn owner(metadata: &std::fs::Metadata, mode: u32) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    // SAFETY: geteuid has no arguments, memory access or side effects.
    #[allow(unsafe_code)]
    let uid = unsafe { libc::geteuid() };
    if metadata.uid() != uid || metadata.permissions().mode() & 0o7777 != mode {
        bail!("private storage requires current-user ownership and exact private mode");
    }
    Ok(())
}

#[cfg(not(unix))]
fn owner(_: &std::fs::Metadata, _: u32) -> Result<()> {
    bail!("private storage protection is unavailable until current-user ACL admission is supported")
}

impl Pinned {
    /// Check ownership/mode on this held directory, never its ambient spelling.
    pub fn verify_user_private(&self) -> Result<()> {
        let file = self.dir.try_clone()?.into_std_file();
        let metadata = file.metadata()?;
        if !metadata.is_dir() {
            bail!("private storage is not a directory");
        }
        owner(&metadata, 0o700)
    }

    /// Trusted settings ancestor may be readable, but never foreign-owned or
    /// writable by another user. This is checked on the retained handle.
    pub fn verify_user_owned_parent(&self) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let metadata = self.dir.try_clone()?.into_std_file().metadata()?;
            #[allow(unsafe_code)]
            // SAFETY: geteuid reads the process effective identity only.
            let uid = unsafe { libc::geteuid() };
            if metadata.uid() != uid || metadata.permissions().mode() & 0o022 != 0 {
                bail!(
                    "private storage parent must be current-user owned and not group/world writable"
                );
            }
            Ok(())
        }
        #[cfg(not(unix))]
        bail!("private storage requires witnessed current-user ACLs")
    }

    /// Claim a fresh private revision directory; collisions are never reused.
    pub fn create_private_child_exclusive(&self, name: &str) -> Result<Self> {
        self.verify_user_private()?;
        crate::ensure_safe_component(name)?;
        #[cfg(unix)]
        {
            use cap_std::fs::DirBuilderExt;
            let mut builder = cap_std::fs::DirBuilder::new();
            builder.mode(0o700);
            self.dir.create_dir_with(name, &builder)?;
            let child = self.open_child(name)?;
            child.verify_user_private()?;
            Ok(child)
        }
        #[cfg(not(unix))]
        bail!("private storage requires witnessed current-user ACLs")
    }

    /// Open/create one private child. Mode is private at creation; existing
    /// insecure occupants are refused, never silently chmod-repaired.
    pub fn private_child(&self, name: &str, create: bool) -> Result<Option<Self>> {
        crate::ensure_safe_component(name)?;
        #[cfg(not(unix))]
        {
            let _ = create;
            bail!("private storage requires witnessed current-user ACLs");
        }
        #[cfg(unix)]
        {
            if let Some(child) = self.open_child_checked(name)? {
                child.verify_user_private()?;
                return Ok(Some(child));
            }
            if !create {
                return Ok(None);
            }
            use cap_std::fs::DirBuilderExt;
            let mut builder = cap_std::fs::DirBuilder::new();
            builder.mode(0o700);
            match self.dir.create_dir_with(name, &builder) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            let child = self.open_child(name)?;
            child.verify_user_private()?;
            Ok(Some(child))
        }
    }

    /// Metadata-only admission of one regular private leaf, rejecting hardlinks.
    pub fn verify_private_leaf(&self, name: &str, cap: u64) -> Result<Option<u64>> {
        self.verify_user_private()?;
        crate::ensure_safe_component(name)?;
        let file = match self
            .dir
            .open_with(name, crate::file::regular_read_options().read(true))
        {
            Ok(file) => file.into_std(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        crate::file::verify_regular_single_link(&file, &self.join(name))?;
        let metadata = file.metadata()?;
        owner(&metadata, 0o600)?;
        if metadata.len() > cap {
            bail!("private storage leaf exceeds its bound");
        }
        Ok(Some(metadata.len()))
    }

    /// Read protected metadata through the same held handle whose owner/mode
    /// was admitted; no check/reopen gap before bytes enter a parser.
    pub fn read_private_metadata(&self, name: &str, cap: usize) -> Result<Option<Vec<u8>>> {
        self.verify_user_private()?;
        crate::ensure_safe_component(name)?;
        let mut file = match self
            .dir
            .open_with(name, crate::file::regular_read_options().read(true))
        {
            Ok(file) => file.into_std(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        crate::file::verify_regular_single_link(&file, &self.join(name))?;
        owner(&file.metadata()?, 0o600)?;
        let snapshot =
            crate::file::bounded::read_held_snapshot_bounded(&mut file, self, name, cap)?;
        owner(&file.metadata()?, 0o600)?;
        Ok(Some(snapshot.bytes))
    }

    /// Lock directly under the verified private root, never creating `.vibe`.
    pub fn private_lock(&self, name: &str) -> Result<LockGuard> {
        self.verify_user_private()?;
        crate::ensure_safe_component(name)?;
        #[cfg(not(unix))]
        {
            bail!("private storage requires witnessed current-user ACLs");
        }
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            let mut options = crate::file::cap_options();
            options.read(true).write(true).create(true).mode(0o600);
            let file = self.dir.open_with(name, &options)?.into_std();
            crate::file::verify_regular_single_link(&file, &self.join(name))?;
            owner(&file.metadata()?, 0o600)?;
            file.lock()?;
            let guard = LockGuard { _file: file };
            let named = self
                .dir
                .open_with(name, crate::file::regular_read_options().read(true))?
                .into_std();
            let held = crate::file::identity::file_identity(&guard._file, &self.join(name))?;
            if held != crate::file::identity::file_identity(&named, &self.join(name))? {
                bail!("private storage lock identity changed");
            }
            Ok(guard)
        }
    }
}
