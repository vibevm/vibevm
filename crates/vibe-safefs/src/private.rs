//! User-private filesystem capabilities for the explicitly managed local keyring.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use crate::{LockGuard, Pinned};
use anyhow::{Result, bail};

/// The effective user every private-storage ownership check compares against.
#[cfg(unix)]
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "the standard library exposes no effective-user query, and private storage must be owned by exactly that user; geteuid is the one infallible, argument-free libc read of it"
)]
#[allow(unsafe_code)]
fn effective_uid() -> u32 {
    // SAFETY: geteuid takes no arguments, touches no caller memory, cannot
    // fail and has no side effects.
    unsafe { libc::geteuid() }
}

#[cfg(unix)]
fn owner(metadata: &std::fs::Metadata, mode: u32) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.uid() != effective_uid() || metadata.permissions().mode() & 0o7777 != mode {
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
            if metadata.uid() != effective_uid() || metadata.permissions().mode() & 0o022 != 0 {
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
        Ok(self
            .private_child_recording(name, create)?
            .map(|(child, _)| child))
    }

    /// Reports only this invocation's successful private mkdir, after held-child
    /// admission. Existing children and a lost creation race report false.
    /// The caller still owns parent admission and durable genesis validation.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING")]
    pub fn private_child_recording(
        &self,
        name: &str,
        create: bool,
    ) -> Result<Option<(Self, bool)>> {
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
                return Ok(Some((child, false)));
            }
            if !create {
                return Ok(None);
            }
            use cap_std::fs::DirBuilderExt;
            let mut builder = cap_std::fs::DirBuilder::new();
            builder.mode(0o700);
            let created = match self.dir.create_dir_with(name, &builder) {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                Err(error) => return Err(error.into()),
            };
            let child = self.open_child(name)?;
            child.verify_user_private()?;
            Ok(Some((child, created)))
        }
    }

    /// Direct-name predicate on a held private root, without traversing entries.
    /// Coordination names are fixed by trusted composition, not user input.
    pub fn private_entries_are_only(&self, allowed: &[&str]) -> Result<bool> {
        self.verify_user_private()?;
        for entry in self.dir.entries()? {
            let entry = entry?;
            if !allowed
                .iter()
                .any(|name| entry.file_name() == std::ffi::OsStr::new(name))
            {
                return Ok(false);
            }
        }
        Ok(true)
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
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING")]
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

#[cfg(all(test, unix))]
mod tests {
    use crate::Project;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn private_creation_records_one_winner_without_repairing_existing_modes() {
        let temp = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o755))
            .tempdir()
            .unwrap();
        let root = Project::open(temp.path()).unwrap().root_dir().unwrap();
        root.verify_user_owned_parent().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let mut threads = vec![];
        for _ in 0..2 {
            let path = temp.path().to_owned();
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                let root = Project::open(&path).unwrap().root_dir().unwrap();
                barrier.wait();
                let (child, created) = root
                    .private_child_recording("private", true)
                    .unwrap()
                    .unwrap();
                assert_eq!(child.unix_mode().unwrap().unwrap() & 0o777, 0o700);
                created
            }));
        }
        barrier.wait();
        assert_eq!(
            threads
                .into_iter()
                .filter_map(|t| t.join().unwrap().then_some(()))
                .count(),
            1
        );
        assert!(
            !root
                .private_child_recording("private", true)
                .unwrap()
                .unwrap()
                .1
        );
        let private = root.private_child("private", false).unwrap().unwrap();
        assert!(private.private_entries_are_only(&[]).unwrap());
        std::fs::write(private.join("foreign-note"), b"KEEP").unwrap();
        assert!(!private.private_entries_are_only(&[]).unwrap());
        assert!(private.private_entries_are_only(&["foreign-note"]).unwrap());
        std::fs::create_dir(private.join("foreign-dir")).unwrap();
        assert!(!private.private_entries_are_only(&["foreign-note"]).unwrap());
        assert_eq!(
            std::fs::read(private.join("foreign-note")).unwrap(),
            b"KEEP"
        );
        std::fs::create_dir(temp.path().join("insecure")).unwrap();
        std::fs::set_permissions(
            temp.path().join("insecure"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(root.private_child_recording("insecure", true).is_err());
        assert_eq!(
            std::fs::metadata(temp.path().join("insecure"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
}
