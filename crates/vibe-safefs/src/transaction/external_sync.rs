specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY");

/// Observe/refuse the retained-image data-sync boundary in native tests only.
/// True injects failure BEFORE the real file sync; None clears the observer.
#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_before_retained_file_sync(callback: Option<Box<dyn FnMut() -> bool>>) {
    BEFORE_RETAINED_SYNC.with(|armed| *armed.borrow_mut() = callback);
}
#[cfg(any(test, feature = "inject-failures"))]
thread_local! {
    static BEFORE_RETAINED_SYNC: std::cell::RefCell<Option<Box<dyn FnMut() -> bool>>> =
        std::cell::RefCell::new(None);
}
fn retained_sync_refused() -> bool {
    #[cfg(any(test, feature = "inject-failures"))]
    {
        return BEFORE_RETAINED_SYNC
            .with(|armed| armed.borrow_mut().as_mut().is_some_and(|check| check()));
    }
    #[cfg(not(any(test, feature = "inject-failures")))]
    {
        false
    }
}

fn sync_stable_external_file(
    root: &Pinned,
    relative: &str,
    cap: usize,
    expected: &crate::StableFileSnapshot,
) -> Result<DirectoryDurability> {
    let view = project_view(root)?;
    let (holder, name) = view
        .holder_of(root, relative)?
        .context("retained image parent is missing")?;
    let display = holder.join(&name);
    let mut options = cap_options();
    options.read(true);
    // FlushFileBuffers requires a write-capable handle on Windows. This open
    // neither creates nor truncates, and no content write follows it.
    #[cfg(windows)]
    options.write(true);
    let file = holder.dir.open_with(&name, &options)?.into_std();
    verify_regular_single_link(&file, &display)?;
    if file_identity(&file, &display)? != expected.identity {
        bail!("retained image handle differs from the approved physical identity");
    }
    let before = view
        .read_file_snapshot_bounded_in(&holder, &name, cap)?
        .context("retained image is missing before synchronization")?;
    if &before != expected {
        bail!("retained image differs from the approved snapshot before synchronization");
    }
    if retained_sync_refused() {
        bail!("retained image data synchronization refused before file sync");
    }
    file.sync_all()
        .with_context(|| format!("synchronizing retained image `{}`", display.display()))?;
    verify_regular_single_link(&file, &display)?;
    if file_identity(&file, &display)? != expected.identity {
        bail!("retained image identity changed across synchronization");
    }
    let after = view
        .read_file_snapshot_bounded_in(&holder, &name, cap)?
        .context("retained image is missing after synchronization")?;
    if &after != expected {
        bail!("retained image differs from the approved snapshot after synchronization");
    }
    Ok(sync_directory(&holder))
}
