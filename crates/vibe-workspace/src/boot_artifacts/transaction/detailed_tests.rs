use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::*;

struct FailAt(WritePoint);

impl FaultInjector for FailAt {
    fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError> {
        if point == self.0 {
            Err(fault(path, format!("injected {point:?}")))
        } else {
            Ok(())
        }
    }
}

struct FailRollbackStart;

impl FaultInjector for FailRollbackStart {
    fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError> {
        if point == WritePoint::IndexReplace || point == WritePoint::RollbackStart {
            Err(fault(path, format!("injected {point:?}")))
        } else {
            Ok(())
        }
    }
}

struct FailPrimaryAndPostRestore;

impl FaultInjector for FailPrimaryAndPostRestore {
    fn check(&self, point: WritePoint, path: &Path) -> Result<(), WorkspaceError> {
        if point == WritePoint::IndexReplace || point == WritePoint::PostRollbackPreCleanup {
            Err(fault(path, format!("injected {point:?}")))
        } else {
            Ok(())
        }
    }
}

struct Fixture {
    root: TempDir,
    index: PathBuf,
    selected: PathBuf,
    stale: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().expect("tempdir");
        let index = root.path().join("INDEX.md");
        let selected = root.path().join("STATIC.xml");
        let stale = root.path().join("STATIC.md");
        fs::write(&index, b"OLD-INDEX").expect("old index");
        fs::write(&stale, b"OLD-STATIC").expect("old static");
        Self {
            root,
            index,
            selected,
            stale,
        }
    }

    fn write(&self) -> ArtifactWrite<'_> {
        ArtifactWrite {
            index_path: &self.index,
            index_bytes: b"NEW-INDEX",
            static_path: &self.selected,
            static_bytes: Some(b"NEW-STATIC"),
            stale_path: &self.stale,
        }
    }

    fn assert_old(&self) {
        assert_eq!(fs::read(&self.index).expect("index"), b"OLD-INDEX");
        assert_eq!(fs::read(&self.stale).expect("stale"), b"OLD-STATIC");
        assert!(!self.selected.exists());
    }

    fn assert_committed_before_stale_cleanup(&self) {
        assert_eq!(fs::read(&self.index).expect("index"), b"NEW-INDEX");
        assert_eq!(fs::read(&self.selected).expect("selected"), b"NEW-STATIC");
        assert_eq!(fs::read(&self.stale).expect("stale"), b"OLD-STATIC");
    }
}

#[test]
fn detailed_disposition_matrix_marks_real_transaction_control_points() {
    for point in [WritePoint::IndexWrite, WritePoint::StaticWrite] {
        let fixture = Fixture::new();
        let failure = write_with_faults_detailed(fixture.write(), &FailAt(point))
            .expect_err("uncommitted fault");
        assert_eq!(
            failure.disposition(),
            TransactionFailureDisposition::Uncommitted
        );
        fixture.assert_old();
    }
    for point in [
        WritePoint::PostStagePreReplace,
        WritePoint::StaticReplace,
        WritePoint::PostStaticPreIndex,
        WritePoint::IndexReplace,
    ] {
        let fixture = Fixture::new();
        let failure = write_with_faults_detailed(fixture.write(), &FailAt(point))
            .expect_err("restored fault");
        assert_eq!(
            failure.disposition(),
            TransactionFailureDisposition::RestoredBefore
        );
        fixture.assert_old();
    }
    for point in [
        WritePoint::PostIndexPreStaleCleanup,
        WritePoint::StaleRemove,
    ] {
        let fixture = Fixture::new();
        let failure = write_with_faults_detailed(fixture.write(), &FailAt(point))
            .expect_err("committed fault");
        assert_eq!(
            failure.disposition(),
            TransactionFailureDisposition::CommitRecoveryIntent
        );
        assert!(journal_path(fixture.root.path()).exists());
    }

    let fixture = Fixture::new();
    let failure = write_with_faults_detailed(
        fixture.write(),
        &FailAt(WritePoint::PostIntentPreRollForward),
    )
    .expect_err("post-intent fault");
    assert_eq!(
        failure.disposition(),
        TransactionFailureDisposition::Indeterminate
    );
    fixture.assert_old();
    assert!(journal_path(fixture.root.path()).exists());
}

#[test]
fn rollback_entry_and_indeterminate_states_are_typed_without_text_inference() {
    let fixture = Fixture::new();
    let rollback = write_with_faults_detailed(fixture.write(), &FailRollbackStart)
        .expect_err("rollback intent");
    assert_eq!(
        rollback.disposition(),
        TransactionFailureDisposition::RollbackRecoveryIntent
    );
    assert!(rollback_journal_path(fixture.root.path()).exists());

    let fixture = Fixture::new();
    let rollback = write_with_faults_detailed(fixture.write(), &FailPrimaryAndPostRestore)
        .expect_err("post-rollback cleanup fault");
    assert_eq!(
        rollback.disposition(),
        TransactionFailureDisposition::RollbackRecoveryIntent
    );
    fixture.assert_old();
    assert!(rollback_journal_path(fixture.root.path()).exists());

    let fixture = Fixture::new();
    fs::write(journal_path(fixture.root.path()), b"not-a-journal").expect("seed malformed intent");
    let entry =
        write_with_faults_detailed(fixture.write(), &NoFault).expect_err("entry recovery failure");
    assert_eq!(
        entry.disposition(),
        TransactionFailureDisposition::EntryRecoveryFailed
    );
}

#[test]
fn selector_refusal_is_the_exact_source_of_a_commit_recovery_intent() {
    let fixture = Fixture::new();
    let expected = fault(&fixture.index, "selector closure refusal").to_string();
    let failure = write_with_faults_and_selectors_detailed(fixture.write(), &NoFault, |_| {
        Err::<(), _>(fault(&fixture.index, "selector closure refusal"))
    })
    .expect_err("selector refusal");
    assert_eq!(
        failure.disposition(),
        TransactionFailureDisposition::CommitRecoveryIntent
    );
    assert_eq!(failure.source_error().to_string(), expected);
    fixture.assert_committed_before_stale_cleanup();
    assert!(journal_path(fixture.root.path()).exists());
}

#[test]
fn legacy_writer_erases_only_the_disposition_and_keeps_the_exact_source() {
    let fixture = Fixture::new();
    let detailed = write_with_faults_detailed(fixture.write(), &FailAt(WritePoint::IndexReplace))
        .expect_err("detailed fault");
    let expected = detailed.source_error().to_string();
    fixture.assert_old();
    let legacy = write_with_faults(fixture.write(), &FailAt(WritePoint::IndexReplace))
        .expect_err("legacy fault");
    assert_eq!(legacy.to_string(), expected);
}

fn fault(path: &Path, reason: impl Into<String>) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
