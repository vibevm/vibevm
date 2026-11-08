//! Reversible slot staging and atomic durable-world recording.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#UNINSTALL-PREFLIGHT"
);

use super::plan::Plan;
use anyhow::{Context, Result, anyhow, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{Lockfile, Manifest};

fn reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
fn inspect(root: &Path, path: &Path, directory: bool) -> Result<bool> {
    let relative = path
        .strip_prefix(root)
        .context("uninstall path escapes workspace")?;
    let mut current = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    for (index, part) in components.iter().enumerate() {
        if !matches!(part, std::path::Component::Normal(_)) {
            bail!("unsafe uninstall path `{}`", path.display());
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if reparse(&meta) {
                    bail!(
                        "refusing uninstall through link/reparse path `{}`",
                        current.display()
                    );
                }
                let final_component = index + 1 == components.len();
                if (!final_component || directory) && !meta.is_dir() {
                    bail!(
                        "uninstall slot/ancestor `{}` is not a directory",
                        current.display()
                    );
                }
                if final_component && !directory && !meta.is_file() {
                    bail!("durable world path `{}` is not a file", current.display());
                }
                if !current.canonicalize()?.starts_with(root.canonicalize()?) {
                    bail!("uninstall path escapes workspace: `{}`", current.display());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && directory => return Ok(false),
            Err(e) => {
                return Err(e).with_context(|| format!("preflighting `{}`", current.display()));
            }
        }
    }
    Ok(true)
}
struct Recorded {
    path: PathBuf,
    original: Vec<u8>,
    replacement: Vec<u8>,
    changed: bool,
}
struct Staged {
    original: PathBuf,
    staged: PathBuf,
}
pub(super) fn apply(
    root: &Path,
    project: &Path,
    plan: &Plan,
    lock: &Lockfile,
    manifest: &Manifest,
) -> Result<()> {
    apply_recording(
        root,
        project,
        plan,
        lock,
        manifest,
        |source, destination| fs::rename(source, destination).map_err(Into::into),
    )
}
fn apply_recording(
    root: &Path,
    project: &Path,
    plan: &Plan,
    lock: &Lockfile,
    manifest: &Manifest,
    mut record_file: impl FnMut(&Path, &Path) -> Result<()>,
) -> Result<()> {
    let slots: Vec<_> = plan
        .removed
        .iter()
        .map(|row| root.join(super::plan::slot(row)))
        .collect();
    // All containment and type checks run before the first rename, including
    // slots later in the plan and the two durable-world destinations.
    for slot in &slots {
        inspect(root, slot, true)?;
    }
    let lock_path = root.join(Lockfile::FILENAME);
    let manifest_path = project.join(Manifest::FILENAME);
    inspect(root, &lock_path, false)?;
    inspect(root, &manifest_path, false)?;
    let before_lock = Lockfile::read(&lock_path)?;
    let mut report = Some(vibe_install::ProjectRecordPublication::capture(
        root,
        &before_lock,
        lock,
        &[],
    )?);
    let staging = tempfile::Builder::new()
        .prefix(".vibe-uninstall-")
        .tempdir_in(root)?;
    let mut records = Vec::new();
    let prepared_lock = staging.path().join("vibe.lock");
    fs::copy(&lock_path, &prepared_lock)?;
    lock.write(&prepared_lock)?;
    records.push(Recorded {
        original: fs::read(&lock_path)?,
        replacement: fs::read(&prepared_lock)?,
        path: lock_path,
        changed: false,
    });
    if plan.manifest_changed {
        let prepared_manifest = staging.path().join("vibe.toml");
        fs::copy(&manifest_path, &prepared_manifest)?;
        manifest.write(&prepared_manifest)?;
        records.push(Recorded {
            original: fs::read(&manifest_path)?,
            replacement: fs::read(&prepared_manifest)?,
            path: manifest_path,
            changed: false,
        });
    }
    let mut staged = Vec::new();
    let result: Result<()> = (|| {
        for (index, slot) in slots.iter().enumerate() {
            if !inspect(root, slot, true)? {
                continue;
            }
            let destination = staging.path().join(format!("slot-{index}"));
            fs::rename(slot, &destination)
                .with_context(|| format!("staging slot `{}`", slot.display()))?;
            staged.push(Staged {
                original: slot.clone(),
                staged: destination,
            });
        }
        for (index, record) in records.iter_mut().enumerate() {
            if fs::read(&record.path)? != record.original {
                bail!(
                    "durable world changed concurrently: `{}`",
                    record.path.display()
                );
            }
            let prepared = staging.path().join(format!("record-{index}"));
            fs::write(&prepared, &record.replacement)?;
            record_file(&prepared, &record.path)
                .with_context(|| format!("recording `{}`", record.path.display()))?;
            record.changed = true;
        }
        if let Some(report) = report.take() {
            report.publish_provenance()?;
        }
        Ok(())
    })();
    if let Err(cause) = result {
        let mut failures = Vec::new();
        for record in records.iter().rev().filter(|r| r.changed) {
            let restored: Result<()> = (|| {
                if fs::read(&record.path)? != record.replacement {
                    bail!("newer edits prevent restoring `{}`", record.path.display());
                }
                let restore = staging.path().join("restore-world");
                fs::write(&restore, &record.original)?;
                fs::rename(restore, &record.path)?;
                Ok(())
            })();
            if let Err(error) = restored {
                failures.push(error.to_string());
            }
        }
        for slot in staged.iter().rev() {
            let restored: Result<()> = (|| {
                if fs::symlink_metadata(&slot.original).is_ok() {
                    bail!(
                        "slot path occupied during rollback: `{}`",
                        slot.original.display()
                    );
                }
                inspect(root, slot.original.parent().context("slot parent")?, true)?;
                fs::rename(&slot.staged, &slot.original)?;
                Ok(())
            })();
            if let Err(error) = restored {
                failures.push(error.to_string());
            }
        }
        if !failures.is_empty() {
            let recovery = staging.keep();
            return Err(anyhow!(
                "{cause:#}; rollback incomplete: {}; retained recovery directory `{}`",
                failures.join("; "),
                recovery.display()
            ));
        }
        return Err(cause);
    }
    // Durable pruning is now authoritative. Cleanup failure preserves an
    // exact recovery location, never restores rows for removed slots.
    let recovery = staging.keep();
    fs::remove_dir_all(&recovery).with_context(|| {
        format!(
            "remaining world recorded; staged slot cleanup failed at `{}`",
            recovery.display()
        )
    })?;
    // Only empty package containers are ours to remove. Other versions and
    // untracked sibling files keep the container alive; never recurse here.
    for row in &plan.removed {
        if row.materialization.is_in_place() {
            continue;
        }
        let slot = root.join(super::plan::slot(row));
        let container = slot.parent().context("versioned slot container")?;
        if !inspect(root, container, true)? {
            continue;
        }
        match fs::remove_dir(container) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "packages removed; cleaning empty container `{}`",
                        container.display()
                    )
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
