//! One lock-only final publication using apply's existing owned file guard.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::file_guard::FileGuard;
use crate::error::Result;
use std::path::Path;
use vibe_core::manifest::Lockfile;

/// A captured lock cannot overwrite a changed pathname after source validation.
/// ```no_run
/// let publication = vibe_install::LockPublication::capture(std::path::Path::new("."))?;
/// let unchanged = publication.lockfile().clone();
/// assert!(!publication.publish(&unchanged)?);
/// # Ok::<(), vibe_install::Error>(())
/// ```
pub struct LockPublication {
    guard: FileGuard,
    original: Lockfile,
}
impl LockPublication {
    pub fn capture(root: &Path) -> Result<Self> {
        let guard = FileGuard::capture(&root.join(Lockfile::FILENAME))?;
        let text = guard
            .previous_text()?
            .ok_or_else(|| vibe_core::Error::InvalidLockfile {
                reason: "lock-only publication requires an existing lock".into(),
            })?;
        let original = Lockfile::parse_str(text)?;
        Ok(Self { guard, original })
    }
    pub fn lockfile(&self) -> &Lockfile {
        &self.original
    }
    pub fn publish(mut self, after: &Lockfile) -> Result<bool> {
        let before = self
            .guard
            .previous_text()?
            .ok_or_else(|| vibe_core::Error::InvalidLockfile {
                reason: "captured lock disappeared".into(),
            })?
            .to_owned();
        let text = if after == &self.original {
            before.clone()
        } else {
            after.render_with_comments(Some(&before))?
        };
        self.guard
            .publish(Some(&text))
            .map_err(|error| super::file_guard::combine(error, [&mut self.guard]))?;
        self.guard.disarm();
        Ok(text != before)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    #[test]
    fn unchanged_lock_preserves_exact_bytes_and_inode() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("vibe.lock");
        let lock = Lockfile::empty("fixture", "fixed");
        lock.write(&path).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_slice(b"\n# preserve spelling\n");
        std::fs::write(&path, &bytes).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        let publication = LockPublication::capture(root.path()).unwrap();
        let unchanged = publication.lockfile().clone();
        assert!(!publication.publish(&unchanged).unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
    }
    #[test]
    fn admitted_changed_lock_remains_published_after_guard_drops() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("vibe.lock");
        Lockfile::empty("before", "fixed").write(&path).unwrap();
        let publication = LockPublication::capture(root.path()).unwrap();
        let mut candidate = publication.lockfile().clone();
        candidate.meta.generated_by = "after".into();
        assert!(publication.publish(&candidate).unwrap());
        assert_eq!(Lockfile::read(&path).unwrap().meta.generated_by, "after");
    }
    #[test]
    fn foreign_same_bytes_replacement_survives_publication_refusal() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("vibe.lock");
        Lockfile::empty("fixture", "fixed").write(&path).unwrap();
        let publication = LockPublication::capture(root.path()).unwrap();
        let mut candidate = publication.lockfile().clone();
        candidate.meta.generated_by = "changed".into();
        let bytes = std::fs::read(&path).unwrap();
        let foreign = root.path().join("replacement");
        std::fs::write(&foreign, &bytes).unwrap();
        std::fs::rename(&foreign, &path).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        assert!(publication.publish(&candidate).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
    }
}
