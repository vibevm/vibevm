//! Expected-snapshot guarded publication under a cooperating caller's lease.
//! The final comparison-to-rename window is not atomic destination-content CAS.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use crate::{DurableWrite, Project, PublishError, StableFileSnapshot};
#[derive(Debug)]
pub enum GuardedPublishError {
    Changed,
    Unavailable,
    Publication(PublishError),
}
impl Project {
    /// Caller holds its incumbent mutation lease. No-op bytes are still
    /// validated; a content/identity/mode drift refuses before staging.
    pub fn write_snapshot_guarded(
        &self,
        relative: &str,
        expected: &StableFileSnapshot,
        desired: &[u8],
    ) -> Result<DurableWrite, GuardedPublishError> {
        let cap = usize::try_from(expected.size)
            .map_err(|_| GuardedPublishError::Unavailable)?
            .checked_add(1)
            .ok_or(GuardedPublishError::Unavailable)?;
        let observed = self
            .read_file_snapshot_bounded(relative, cap)
            .map_err(|_| GuardedPublishError::Unavailable)?;
        if observed.as_ref() != Some(expected) {
            return Err(GuardedPublishError::Changed);
        };
        let root = self
            .root_dir()
            .map_err(|_| GuardedPublishError::Unavailable)?;
        self.write_atomic_durable_in_with_mode(&root, relative, desired, expected.unix_mode)
            .map_err(GuardedPublishError::Publication)
    }
}
#[cfg(test)]
mod m_16_d_3_cases {
    use super::*;
    #[test]
    fn expected_snapshot_refuses_drift_and_reports_landed_failure() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.md"), b"before").unwrap();
        let project = Project::open(root.path()).unwrap();
        let old = project
            .read_file_snapshot_bounded("a.md", 64)
            .unwrap()
            .unwrap();
        std::fs::write(root.path().join("a.md"), b"drift").unwrap();
        assert!(matches!(
            project.write_snapshot_guarded("a.md", &old, b"after"),
            Err(GuardedPublishError::Changed)
        ));
        assert_eq!(std::fs::read(root.path().join("a.md")).unwrap(), b"drift");
        let current = project
            .read_file_snapshot_bounded("a.md", 64)
            .unwrap()
            .unwrap();
        crate::fail_after_publish(Some("a.md"));
        let error = project
            .write_snapshot_guarded("a.md", &current, b"landed")
            .unwrap_err();
        crate::fail_after_publish(None);
        assert!(matches!(
            error,
            GuardedPublishError::Publication(PublishError {
                stage: crate::PublishStage::PossiblyPublished,
                ..
            })
        ));
        assert_eq!(std::fs::read(root.path().join("a.md")).unwrap(), b"landed");
    }
}
