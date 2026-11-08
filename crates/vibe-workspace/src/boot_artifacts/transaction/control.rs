//! Cooperative failure at the existing reversible publication points.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use super::*;
use vibe_core::cancellation::CancellationToken;
struct StopAtBoundary<'a>(&'a CancellationToken);
impl FaultInjector for StopAtBoundary<'_> {
    fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError> {
        // Never veto entry recovery, rollback, or post-INDEX commit cleanup.
        // A stop there is serviced at the next owner's reversible boundary.
        if matches!(
            point,
            WritePoint::IndexWrite
                | WritePoint::StaticWrite
                | WritePoint::PostStagePreReplace
                | WritePoint::StaticReplace
                | WritePoint::PostStaticPreIndex
                | WritePoint::IndexReplace
        ) {
            self.0.checkpoint().map_err(|error| io_error(path, error))?;
        }
        Ok(())
    }
}
pub(in crate::boot_artifacts) fn write_controlled_with_selectors<T>(
    write: ArtifactWrite<'_>,
    control: Option<&CancellationToken>,
    selectors: impl FnOnce(&str) -> Result<T, WorkspaceError>,
) -> Result<T, WorkspaceError> {
    let Some(control) = control else {
        return write_production_with_selectors(write, selectors);
    };
    control
        .checkpoint()
        .map_err(|error| io_error(write.index_path, error))?;
    write_with_faults_and_selectors_detailed(write, &StopAtBoundary(control), selectors)
        .map_err(DetailedTransactionFailure::into_source)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct CancelAfterStatic<'a> {
        token: &'a CancellationToken,
        foreign: bool,
    }
    impl FaultInjector for CancelAfterStatic<'_> {
        fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError> {
            if point == WritePoint::PostStaticPreIndex {
                if self.foreign {
                    fs::write(path, b"FOREIGN").unwrap();
                }
                self.token.cancel();
            }
            StopAtBoundary(self.token).check(point, path)
        }
    }
    #[test]
    fn cancellation_between_static_and_index_uses_real_guarded_inverse() {
        for foreign in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let index = root.path().join("INDEX.md");
            let selected = root.path().join("STATIC.xml");
            let stale = root.path().join("STATIC.md");
            fs::write(&index, b"OLD-INDEX").unwrap();
            fs::write(&stale, b"OLD-STATIC").unwrap();
            let token = CancellationToken::new();
            let error = write_with_faults_and_selectors_detailed(
                ArtifactWrite {
                    index_path: &index,
                    index_bytes: b"NEW-INDEX",
                    static_path: &selected,
                    static_bytes: Some(b"NEW-STATIC"),
                    stale_path: &stale,
                },
                &CancelAfterStatic {
                    token: &token,
                    foreign,
                },
                |_| Ok(()),
            )
            .unwrap_err();
            assert!(token.acknowledged());
            assert_eq!(fs::read(&index).unwrap(), b"OLD-INDEX");
            assert_eq!(fs::read(&stale).unwrap(), b"OLD-STATIC");
            if foreign {
                assert_eq!(fs::read(&selected).unwrap(), b"FOREIGN");
                assert_eq!(
                    error.disposition(),
                    TransactionFailureDisposition::RollbackRecoveryIntent
                );
            } else {
                assert!(!selected.exists());
                assert_eq!(
                    error.disposition(),
                    TransactionFailureDisposition::RestoredBefore
                );
                assert!(!root.path().join(JOURNAL_NAME).exists());
                assert!(!root.path().join(ROLLBACK_JOURNAL_NAME).exists());
            }
        }
    }
}
