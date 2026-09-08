specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::BTreeMap;

use super::sha256::digest;
use super::*;

#[derive(Default)]
struct MemoryStore {
    pending: Option<Journal>,
    snapshots: Vec<(SnapshotRecord, Vec<u8>)>,
    reports: Vec<TransactionReport>,
    events: Vec<String>,
    retired: bool,
    fail_snapshot_at: Option<usize>,
    fail_snapshot_after_write_at: Option<usize>,
    snapshot_calls: usize,
    fail_report_once: bool,
    fail_retire_once: bool,
    fail_journal_state: Option<TransactionState>,
}

impl TransactionStore for MemoryStore {
    fn prove_outside_project(&mut self, _: &str) -> Result<(), TransactionError> {
        self.events.push("external".into());
        Ok(())
    }
    fn lock_project(&mut self, _: &ProjectKey) -> Result<ProjectLock, TransactionError> {
        self.events.push("lock".into());
        Ok(ProjectLock::acquired())
    }
    fn pending(&mut self, _: &ProjectKey) -> Result<Option<Journal>, TransactionError> {
        Ok(self.pending.clone())
    }
    fn verify_snapshot_progress(
        &mut self,
        journal: &Journal,
    ) -> Result<SnapshotActiveObservation, TransactionError> {
        let complete = journal.snapshots_persisted;
        if self.snapshots.len() < complete || self.snapshots.len() > complete + 1 {
            return Err(TransactionError::Store(
                "snapshot progress/file count mismatch".into(),
            ));
        }
        for ((actual, bytes), expected) in self
            .snapshots
            .iter()
            .take(complete)
            .zip(&journal.snapshots[..complete])
        {
            if actual != expected || actual.sha256 != digest(bytes) {
                return Err(TransactionError::Store("snapshot content mismatch".into()));
            }
        }
        match journal.snapshot_active {
            None if self.snapshots.len() == complete => Ok(SnapshotActiveObservation::None),
            Some(active) if self.snapshots.len() == complete => {
                if active != complete {
                    return Err(TransactionError::Store(
                        "snapshot intent index mismatch".into(),
                    ));
                }
                Ok(SnapshotActiveObservation::Absent)
            }
            Some(active) if self.snapshots.len() == complete + 1 => {
                let (record, bytes) = &self.snapshots[complete];
                if active != complete
                    || record != &journal.snapshots[active]
                    || record.sha256 != digest(bytes)
                {
                    return Err(TransactionError::Store(
                        "active snapshot is not exact-present".into(),
                    ));
                }
                Ok(SnapshotActiveObservation::ExactPresent)
            }
            _ => Err(TransactionError::Store(
                "unjournaled snapshot data is present".into(),
            )),
        }
    }
    fn read_snapshot(
        &mut self,
        journal: &Journal,
        name: &str,
    ) -> Result<Vec<u8>, TransactionError> {
        let record = journal
            .snapshots
            .iter()
            .position(|record| record.name == name)
            .ok_or_else(|| TransactionError::Store("snapshot is not journaled".into()))?;
        if record >= journal.snapshots_persisted {
            return Err(TransactionError::Store(
                "snapshot is outside durable prefix".into(),
            ));
        }
        self.snapshots
            .iter()
            .find(|(candidate, _)| candidate.name == name)
            .map(|(_, bytes)| bytes.clone())
            .ok_or_else(|| TransactionError::Store("snapshot is absent".into()))
    }
    fn mint_transaction_id(&mut self, _: &ProjectKey) -> Result<TransactionId, TransactionError> {
        Ok(TransactionId("TX000001".into()))
    }
    fn verification_workspace_intent(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspaceIntent, TransactionError> {
        let mut material = b"vibe-scrape-verification-workspace-e1\0".to_vec();
        material.extend_from_slice(journal.project_key.0.as_bytes());
        material.push(0);
        material.extend_from_slice(journal.transaction_id.0.as_bytes());
        Ok(VerificationWorkspaceIntent {
            name: "v".into(),
            display_root: format!("C:/external/{}/v", journal.transaction_id.0),
            ownership_token: digest(&material).0,
        })
    }
    fn create_transaction(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspace, TransactionError> {
        if journal.revision != 0 {
            return Err(TransactionError::Store(
                "initial journal revision is not zero".into(),
            ));
        }
        let intent = journal
            .verification_workspace
            .clone()
            .ok_or_else(|| TransactionError::Store("workspace intent is absent".into()))?;
        let workspace = VerificationWorkspace {
            intent,
            directory_identity: hash("workspace-directory").0,
            entry_identity: hash("workspace-entry").0,
            project_identity_token: hash("workspace-project").0,
        };
        self.events.push("create".into());
        self.pending = Some(journal.clone());
        Ok(workspace)
    }
    fn persist_snapshot(
        &mut self,
        _: &TransactionId,
        record: &SnapshotRecord,
        bytes: &[u8],
    ) -> Result<(), TransactionError> {
        let index = self.snapshot_calls;
        self.snapshot_calls += 1;
        if self.fail_snapshot_at == Some(index) {
            return Err(TransactionError::Store(format!(
                "injected snapshot store failure {index}"
            )));
        }
        self.events.push(format!("snapshot/{}", record.name));
        self.snapshots.push((record.clone(), bytes.to_vec()));
        if self.fail_snapshot_after_write_at == Some(index) {
            self.fail_snapshot_after_write_at = None;
            return Err(TransactionError::Store(format!(
                "injected post-write snapshot failure {index}"
            )));
        }
        Ok(())
    }
    fn persist_journal(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        if self.fail_journal_state.as_ref() == Some(&journal.state) {
            self.fail_journal_state = None;
            return Err(TransactionError::Store(format!(
                "injected journal failure at {:?}",
                journal.state
            )));
        }
        if let Some(current) = &self.pending {
            let exact_retry = current.revision == journal.revision && current == journal;
            let next = current
                .revision
                .checked_add(1)
                .is_some_and(|revision| revision == journal.revision);
            if !exact_retry && !next {
                return Err(TransactionError::Store(
                    "journal revision is stale or skipped".into(),
                ));
            }
        }
        self.events.push(format!("journal/{:?}", journal.state));
        self.pending = Some(journal.clone());
        Ok(())
    }
    fn persist_report(
        &mut self,
        report: &TransactionReport,
        canonical_wire: &[u8],
    ) -> Result<(), TransactionError> {
        if self.fail_report_once {
            self.fail_report_once = false;
            return Err(TransactionError::Store("injected report failure".into()));
        }
        if canonical_wire.is_empty() {
            return Err(TransactionError::Store(
                "canonical report bytes are empty".into(),
            ));
        }
        self.events.push("report".into());
        self.reports.push(report.clone());
        Ok(())
    }
    fn retire_transaction(&mut self, _: &Journal) -> Result<(), TransactionError> {
        if self.fail_retire_once {
            self.fail_retire_once = false;
            return Err(TransactionError::Store("injected retire failure".into()));
        }
        self.events.push("retire".into());
        self.retired = true;
        self.pending = None;
        Ok(())
    }
}
