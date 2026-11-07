//! Three metadata files, bounded cooperative publication and checked undo.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO");

use std::path::{Path, PathBuf};

use vibe_core::manifest::Manifest;
use vibe_safefs::{OwnedFileReceipt, Pinned, Project, StableFileSnapshot};

use crate::error::{Error, Result};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
pub(super) struct FileGuard {
    path: PathBuf,
    project: Project,
    parent: Pinned,
    name: String,
    before: Option<StableFileSnapshot>,
    change: Option<Change>,
}

enum Change {
    Published(OwnedFileReceipt, usize),
    Removed,
}

fn failure(path: &Path, reason: impl std::fmt::Display) -> Error {
    vibe_workspace::WorkspaceError::Io {
        path: path.to_owned(),
        reason: format!("metadata publication/undo: {reason:#}; violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO; fix: inspect this path and retain an ordinary owned metadata file"),
    }.into()
}

impl FileGuard {
    fn capture(path: &Path) -> Result<Self> {
        let parent_path = path
            .parent()
            .ok_or_else(|| failure(path, "metadata has no parent"))?;
        let project = Project::open(parent_path).map_err(|e| failure(path, e))?;
        let parent = project.root_dir().map_err(|e| failure(path, e))?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| failure(path, "invalid metadata name"))?
            .to_owned();
        // The implementation allocates from observed metadata, not this ceiling.
        // No arbitrary document-size policy is introduced here.
        let before = project
            .read_file_snapshot_bounded_in(&parent, &name, usize::MAX - 1)
            .map_err(|e| failure(path, e))?;
        if let Some(before) = &before {
            std::str::from_utf8(&before.bytes)
                .map_err(|_| failure(path, "captured metadata is not UTF-8; left untouched"))?;
        }
        Ok(Self {
            path: path.to_owned(),
            project,
            parent,
            name,
            before,
            change: None,
        })
    }

    pub(super) fn previous_text(&self) -> Result<Option<&str>> {
        self.before
            .as_ref()
            .map(|s| {
                std::str::from_utf8(&s.bytes).map_err(|_| {
                    failure(&self.path, "captured metadata is not UTF-8; left untouched")
                })
            })
            .transpose()
    }

    fn observe(&self, cap: usize) -> Result<Option<StableFileSnapshot>> {
        self.project
            .read_file_snapshot_bounded_in(&self.parent, &self.name, cap)
            .map_err(|_| {
                failure(
                    &self.path,
                    "current name is changed, unsafe, oversized or unreadable; left untouched",
                )
            })
    }

    pub(super) fn publish(&mut self, after: Option<&str>) -> Result<()> {
        let cap = self.before.as_ref().map_or(0, |s| s.bytes.len());
        if self.observe(cap)? != self.before {
            return Err(failure(
                &self.path,
                "before-state changed before publication; left untouched",
            ));
        }
        let mode = self.before.as_ref().and_then(|s| s.unix_mode);
        match after {
            Some(text) => {
                if self
                    .before
                    .as_ref()
                    .is_some_and(|s| s.bytes == text.as_bytes())
                {
                    return Ok(());
                }
                match self.project.write_atomic_owned_in_with_mode(
                    &self.parent,
                    &self.name,
                    text.as_bytes(),
                    mode,
                ) {
                    Ok(published) => {
                        self.change = Some(Change::Published(published.receipt, text.len()))
                    }
                    Err(error) => {
                        self.change = error
                            .receipt
                            .map(|receipt| Change::Published(receipt, text.len()));
                        return Err(failure(&self.path, error.publication.into_report()));
                    }
                }
            }
            None if self.before.is_some() => {
                // This final observation-to-unlink window is cooperative, not CAS.
                self.change = Some(Change::Removed);
                self.project
                    .remove_file_in(&self.parent, &self.name)
                    .map_err(|e| failure(&self.path, e))?;
            }
            None => {}
        }
        Ok(())
    }

    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
    )]
    fn undo(&mut self) -> Result<()> {
        let Some(change) = self.change.take() else {
            return Ok(());
        };
        // Compare current state with owned evidence, not with an adopted reread.
        // Receipt matching below additionally checks known after length/digest.
        let after_size = match &change {
            Change::Published(_, size) => *size,
            Change::Removed => 0,
        };
        let cap = after_size.max(self.before.as_ref().map_or(0, |s| s.bytes.len()));
        let current = self.observe(cap)?;
        if current == self.before {
            return Ok(());
        }
        match change {
            Change::Published(receipt, _) => {
                if !current
                    .as_ref()
                    .is_some_and(|s| receipt.matches_snapshot(s))
                {
                    return Err(failure(
                        &self.path,
                        "rollback conflict: current name is not this publication's object/content/mode; left untouched",
                    ));
                }
                match &self.before {
                    Some(before) => {
                        self.project
                            .write_atomic_in_with_mode(
                                &self.parent,
                                &self.name,
                                &before.bytes,
                                before.unix_mode,
                            )
                            .map_err(|e| failure(&self.path, e.into_report()))?;
                    }
                    None => {
                        self.project
                            .remove_file_in(&self.parent, &self.name)
                            .map_err(|e| failure(&self.path, e))?;
                    }
                }
            }
            Change::Removed => {
                if current.is_some() {
                    return Err(failure(
                        &self.path,
                        "rollback conflict: removed name has a new occupant; left untouched",
                    ));
                }
                if let Some(before) = &self.before {
                    self.project
                        .publish_new_in_with_mode(
                            &self.parent,
                            &self.name,
                            &before.bytes,
                            before.unix_mode,
                        )
                        .map_err(|e| failure(&self.path, e.into_report()))?;
                }
            }
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.change = None;
    }
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        if let Err(error) = self.undo() {
            tracing::warn!(target: "vibe_install::apply", "checked metadata undo failed: {error}");
        }
    }
}

pub(super) struct MetadataFiles {
    pub manifest: Option<FileGuard>,
    pub lock: FileGuard,
    pub provenance: FileGuard,
}

/// Publish the admitted legacy requires seed through the existing owned-file
/// receipt. Failed planning never reaches this boundary; changed source bytes
/// still refuse instead of being overwritten by a stale prepared snapshot.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub(crate) fn publish_admitted_manifest(
    root: &Path,
    expected: &Manifest,
    replacement: &Manifest,
) -> Result<()> {
    let path = root.join(Manifest::FILENAME);
    let mut guard = FileGuard::capture(&path)?;
    let text = guard.previous_text()?;
    let observed = text.and_then(|text| Manifest::parse_str(text).ok());
    let matches = observed
        .as_ref()
        .map(|observed| vibe_workspace::authored_manifests_equal(observed, expected))
        .transpose()
        .map_err(|e| failure(&path, e))?
        .unwrap_or(false);
    if !matches {
        return Err(failure(
            &path,
            "selected manifest changed before admitted migration publication",
        ));
    }
    let rendered = replacement.render_with_comments(text)?;
    guard
        .publish(Some(&rendered))
        .map_err(|error| combine(error, [&mut guard]))?;
    guard.disarm();
    Ok(())
}

impl MetadataFiles {
    pub fn capture(
        selected: &Path,
        lock: &Path,
        planned: &Manifest,
        changed: bool,
    ) -> Result<Self> {
        let path = selected.join(Manifest::FILENAME);
        let manifest = changed.then(|| FileGuard::capture(&path)).transpose()?;
        let source_bytes;
        let text = match &manifest {
            Some(guard) => guard.previous_text()?,
            None => {
                let project = Project::open(selected).map_err(|e| failure(&path, e))?;
                source_bytes = project
                    .read_source_bytes(Manifest::FILENAME)
                    .map_err(|e| failure(&path, e))?;
                source_bytes
                    .as_ref()
                    .map(|bytes| {
                        std::str::from_utf8(bytes).map_err(|_| {
                            failure(&path, "captured manifest is not UTF-8; left untouched")
                        })
                    })
                    .transpose()?
            }
        };
        let captured = text.and_then(|s| Manifest::parse_str(s).ok());
        let matches = captured
            .as_ref()
            .map(|observed| vibe_workspace::authored_manifests_equal(observed, planned))
            .transpose()
            .map_err(|e| failure(&path, e))?
            .unwrap_or(false);
        if !matches {
            return Err(failure(
                &path,
                "selected manifest recognized semantics changed since planning; replan before applying",
            ));
        }
        let lock_guard = FileGuard::capture(lock)?;
        let provenance = FileGuard::capture(&vibe_core::lock_provenance::provenance_path(lock))?;
        Ok(Self {
            manifest,
            lock: lock_guard,
            provenance,
        })
    }

    pub fn check_lock_path(&self, path: &Path) -> Result<()> {
        if self.lock.path != path {
            return Err(failure(
                path,
                "workspace lock parent changed since preparation",
            ));
        }
        Ok(())
    }

    pub fn undo_records(&mut self, primary: Error) -> Error {
        combine(primary, [&mut self.lock, &mut self.provenance])
    }

    pub fn undo_all(&mut self, primary: Error) -> Error {
        let primary = self.undo_records(primary);
        match &mut self.manifest {
            Some(guard) => combine(primary, [guard]),
            None => primary,
        }
    }

    pub fn disarm(&mut self) {
        if let Some(manifest) = &mut self.manifest {
            manifest.disarm();
        }
        self.lock.disarm();
        self.provenance.disarm();
    }
}

fn combine<const N: usize>(primary: Error, guards: [&mut FileGuard; N]) -> Error {
    let conflicts: Vec<_> = guards
        .into_iter()
        .filter_map(|guard| guard.undo().err())
        .map(|e| e.to_string())
        .collect();
    if conflicts.is_empty() {
        primary
    } else {
        Error::RollbackFailed {
            primary: Box::new(primary),
            reason: conflicts.join("; "),
        }
    }
}

/// Replace only a captured known pre-configuration lock with an admitted empty
/// supported world. A newly readable/rich/future file is never overwritten.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
pub(crate) fn publish_empty_legacy_lock(
    root: &Path,
    expected_schema: u32,
    replacement: &vibe_core::manifest::Lockfile,
) -> Result<()> {
    let path = root.join(vibe_core::manifest::Lockfile::FILENAME);
    let mut guard = FileGuard::capture(&path)?;
    let text = guard
        .previous_text()?
        .ok_or_else(|| failure(&path, "legacy lock disappeared before regeneration"))?;
    if !(1..=6).contains(&expected_schema)
        || !replacement.packages.is_empty()
        || !replacement.remaps.is_empty()
        || !replacement.configurations.is_empty()
        || !replacement.meta.root_dependencies.is_empty()
    {
        return Err(failure(
            &path,
            "legacy empty-world publication requires a known old epoch and empty admitted replacement",
        ));
    }
    match vibe_core::manifest::Lockfile::parse_str(text) {
        Err(vibe_core::Error::UnsupportedLockfile { found, .. }) if found == expected_schema => {}
        _ => {
            return Err(failure(
                &path,
                "captured lock is no longer the known legacy schema; retain its current state",
            ));
        }
    }
    let rendered = replacement.render_with_comments(Some(text))?;
    guard
        .publish(Some(&rendered))
        .map_err(|error| combine(error, [&mut guard]))?;
    guard.disarm();
    Ok(())
}
