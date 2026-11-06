//! Confirm actual retained-image synchronization, never infer it from bytes.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-RETAINED-SYNC");
use super::*;

#[cfg(any(test, feature = "inject-failures"))]
thread_local! {
    static BEFORE_RETAINED_SYNC: std::cell::RefCell<Option<Box<dyn FnMut() -> bool>>> =
        std::cell::RefCell::new(None);
}

fn retained_sync_refused() -> bool {
    #[cfg(any(test, feature = "inject-failures"))]
    {
        BEFORE_RETAINED_SYNC.with(|slot| slot.borrow_mut().as_mut().is_some_and(|hook| hook()))
    }
    #[cfg(not(any(test, feature = "inject-failures")))]
    {
        false
    }
}

impl ExternalStore {
    /// Test-only hook immediately before real retained-file sync. Returning true
    /// injects a failure; None clears it. No hook exists in ordinary builds.
    #[cfg(any(test, feature = "inject-failures"))]
    pub fn arm_before_retained_file_sync(callback: Option<Box<dyn FnMut() -> bool>>) {
        BEFORE_RETAINED_SYNC.with(|slot| *slot.borrow_mut() = callback);
    }

    /// Synchronize the exact retained regular single-link image without writing
    /// its contents. Bound and observation rules remain the incumbent reader's.
    /// The return value states actual parent-directory durability, not CAS.
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// use vibe_safefs::ExternalStore;
    /// let temp = tempfile::tempdir()?;
    /// let store = ExternalStore::open_or_create(&temp.path().join("state"))?;
    /// store.write_durable("before.bin", b"approved")?;
    /// let expected = store.read_stable_bounded("before.bin", 8)?.unwrap();
    /// let _parent_durability = store.sync_stable_file_bounded("before.bin", 8, &expected)?;
    /// assert_eq!(store.read_stable_bounded("before.bin", 8)?.unwrap(), expected);
    /// # Ok(()) }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-RETAINED-SYNC"
    )]
    pub fn sync_stable_file_bounded(
        &self,
        relative: &str,
        cap: usize,
        expected: &crate::StableFileSnapshot,
    ) -> Result<DirectoryDurability> {
        let view = project_view(&self.root)?;
        let (parents, name) = crate::split_relative(relative)?;
        let refs: Vec<_> = parents.iter().map(String::as_str).collect();
        let holder = view
            .dir(&refs, false)
            .context("retained image parent is unavailable")?;
        let display = holder.join(&name);
        let mut options = crate::file::regular_read_options();
        options.read(true);
        // FlushFileBuffers needs write access on Windows. This open neither
        // creates nor truncates; the method never writes content bytes.
        #[cfg(windows)]
        options.write(true);
        let file = holder.dir.open_with(&name, &options)?.into_std();
        verify_regular_single_link(&file, &display)?;
        if file_identity(&file, &display)? != expected.identity {
            bail!("retained image handle differs from approved identity");
        }
        let before = view
            .read_file_snapshot_bounded_in(&holder, &name, cap)?
            .context("retained image absent before synchronization")?;
        if &before != expected {
            bail!("retained image differs from approved snapshot before synchronization");
        }
        if retained_sync_refused() {
            bail!("retained image synchronization refused before file sync");
        }
        file.sync_all().context("synchronizing retained image")?;
        verify_regular_single_link(&file, &display)?;
        if file_identity(&file, &display)? != expected.identity {
            bail!("retained image handle changed across synchronization");
        }
        let after = view
            .read_file_snapshot_bounded_in(&holder, &name, cap)?
            .context("retained image absent after synchronization")?;
        if &after != expected {
            bail!("retained image differs from approved snapshot after synchronization");
        }
        Ok(sync_directory(&holder))
    }
}
