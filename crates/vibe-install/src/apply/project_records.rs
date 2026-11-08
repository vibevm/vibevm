//! Shared lock/report publication delegates recovery to the existing file guards.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#provenance");
use super::file_guard::{FileGuard, MetadataFiles};
use crate::error::Result;
use std::path::Path;
use vibe_core::manifest::Lockfile;

/// Precompute shared report changes before slots mutate, retaining checked receipts.
/// ```no_run
/// # fn publish(root:&std::path::Path, before:&vibe_core::manifest::Lockfile, after:&vibe_core::manifest::Lockfile)->Result<(),vibe_install::Error> {
/// let records=vibe_install::ProjectRecordPublication::capture(root,before,after,&[])?;
/// records.publish(after)
/// # }
/// ```
pub struct ProjectRecordPublication {
    files: MetadataFiles,
    before: Lockfile,
    planned: Lockfile,
    report: Option<String>,
}
/// The same guarded before-state, captured before any source or slot mutation.
/// ```no_run
/// # fn prepare(root:&std::path::Path, before:&vibe_core::manifest::Lockfile)->Result<(),vibe_install::Error> {
/// let capture=vibe_install::ProjectRecordPublication::capture_before(root,before)?;
/// let records=capture.prepare(before,&[])?;
/// records.validate_before_publication()?;
/// # Ok(()) }
/// ```
pub struct ProjectRecordCapture {
    files: MetadataFiles,
    before: Lockfile,
}
impl ProjectRecordCapture {
    pub fn prepare(
        self,
        planned: &Lockfile,
        cached: &[&vibe_registry::CachedPackage],
    ) -> Result<ProjectRecordPublication> {
        let report = crate::record::merge_cached_provenance(
            cached,
            &self.before,
            planned,
            self.files.provenance.previous_text()?,
        )?;
        let publication = ProjectRecordPublication {
            files: self.files,
            before: self.before,
            planned: planned.clone(),
            report,
        };
        publication.validate_before_publication()?;
        Ok(publication)
    }
}
impl ProjectRecordPublication {
    pub fn capture_before(root: &Path, before: &Lockfile) -> Result<ProjectRecordCapture> {
        let path = root.join(Lockfile::FILENAME);
        let lock = FileGuard::capture(&path)?;
        let text = lock
            .previous_text()?
            .ok_or_else(|| vibe_core::Error::InvalidLockfile {
                reason: "project publication needs an existing lock".into(),
            })?;
        if Lockfile::parse_str(text)? != *before {
            return Err(vibe_core::Error::InvalidLockfile {
                reason: "project lock changed before publication preparation".into(),
            }
            .into());
        }
        let provenance = FileGuard::capture(&vibe_core::lock_provenance::provenance_path(&path))?;
        if let Some(text) = provenance.previous_text()? {
            vibe_core::lock_provenance::LockProvenance::parse_str(text)?;
        }
        Ok(ProjectRecordCapture {
            files: MetadataFiles {
                manifest: None,
                lock,
                provenance,
            },
            before: before.clone(),
        })
    }
    pub fn capture(
        root: &Path,
        before: &Lockfile,
        planned: &Lockfile,
        cached: &[&vibe_registry::CachedPackage],
    ) -> Result<Self> {
        Self::capture_before(root, before)?.prepare(planned, cached)
    }
    pub fn validate_before_publication(&self) -> Result<()> {
        self.files.lock.recheck()?;
        self.files.provenance.recheck()
    }
    /// Publish lock and shared report together, undoing owned metadata on failure.
    pub fn publish(mut self, after: &Lockfile) -> Result<()> {
        if !crate::record::same_selection_set(&self.planned, after) {
            return Err(vibe_core::Error::InvalidLockfile {
                reason: "source selection changed after report preparation".into(),
            }
            .into());
        }
        let old = self.files.lock.previous_text()?.map(str::to_owned);
        let text = if after == &self.before {
            old.unwrap_or_default()
        } else {
            after.render_with_comments(old.as_deref())?
        };
        let result = (|| {
            self.files.lock.publish(Some(&text))?;
            self.files.provenance.publish(self.report.as_deref())
        })();
        if let Err(error) = result {
            return Err(self.files.undo_records(error));
        }
        self.files.disarm();
        Ok(())
    }
    /// The uninstall owner already publishes/undoes its lock; publish this report
    /// last, before it declares metadata success, without acquiring a second lock writer.
    pub fn publish_provenance(mut self) -> Result<()> {
        if let Err(error) = self.files.provenance.publish(self.report.as_deref()) {
            return Err(self.files.undo_records(error));
        }
        self.files.disarm();
        Ok(())
    }
}
