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

#[derive(Default)]
struct MemoryFs {
    candidate: Option<Vec<TreeEntry>>,
    output: Option<Vec<TreeEntry>>,
    output_occupied: bool,
    publish_unsupported: bool,
    output_third: bool,
    source_mutations: usize,
    quarantine: bool,
    quarantine_third: bool,
    steps: BTreeMap<String, StepWorld>,
    prepared_payloads: BTreeMap<String, Vec<u8>>,
    cleanup_failures: usize,
    candidate_creation: Option<ExclusiveTreeCreation>,
    quarantine_creation: Option<ExclusiveTreeCreation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum StepWorld {
    #[default]
    Before,
    After,
    Third,
}

impl TransactionFilesystem for MemoryFs {
    fn rebind_owned_tree(&mut self, _journal: &Journal) -> Result<(), TransactionError> {
        Ok(())
    }

    fn owned_tree_seal(
        &mut self,
        name: &str,
        _ownership_token: &str,
    ) -> Result<OwnedTreeSeal, TransactionError> {
        let entries = if name.contains("candidate") {
            self.candidate
                .as_ref()
                .or(self.output.as_ref())
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(OwnedTreeSeal {
            directory_identity: format!("owned/{name}"),
            manifest_digest: partial_manifest(entries.clone()).digest.0,
            entries: entries
                .into_iter()
                .map(|entry| OwnedEntrySeal {
                    identity: format!("identity/{}", entry.path),
                    path: entry.path,
                    kind: entry.kind,
                    sha256: entry.sha256,
                    bytes: entry.bytes,
                    mode: entry.mode,
                })
                .collect(),
        })
    }
    fn create_export_candidate(
        &mut self,
        _: &ExportPlan,
        _: &str,
        _: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        if let Some(outcome) = self.candidate_creation.take() {
            if !matches!(outcome, ExclusiveTreeCreation::NotCreated { .. }) {
                self.candidate = Some(Vec::new());
            }
            return Ok(outcome);
        }
        if self.candidate.is_some() {
            return Ok(ExclusiveTreeCreation::NotCreated {
                detail: "candidate occupied".into(),
            });
        }
        self.candidate = Some(Vec::new());
        Ok(ExclusiveTreeCreation::Owned)
    }
    fn apply_export_entry(
        &mut self,
        plan: &ExportPlan,
        _: &str,
        _: &str,
        entry: &ExportEntry,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        let sealed = plan
            .final_manifest
            .entries
            .iter()
            .find(|sealed| sealed.path == entry.target_path)
            .expect("validated manifest")
            .clone();
        self.candidate.as_mut().unwrap().push(sealed);
        if let Some(bytes) = prepared_after {
            self.prepared_payloads
                .insert(entry.target_path.clone(), bytes.to_vec());
        }
        Ok(())
    }
    fn observe_export_tree(
        &mut self,
        plan: &ExportPlan,
        slot: ExportTreeSlot,
        _: &str,
        _: &str,
    ) -> Result<OwnedTreeObservation, TransactionError> {
        if slot == ExportTreeSlot::Output && self.output_third {
            return Ok(OwnedTreeObservation::Third {
                detail: "extra descendant concurrent.txt".into(),
            });
        }
        let tree = match slot {
            ExportTreeSlot::Candidate => self.candidate.as_ref(),
            ExportTreeSlot::Output => self.output.as_ref(),
        };
        Ok(match tree {
            None => OwnedTreeObservation::Absent,
            Some(entries) if entries == &plan.final_manifest.entries => {
                OwnedTreeObservation::Exact(plan.final_manifest.clone())
            }
            Some(entries) => OwnedTreeObservation::Exact(partial_manifest(entries.clone())),
        })
    }
    fn publish_export_noreplace(
        &mut self,
        _: &ExportPlan,
        _: &str,
        _: &str,
    ) -> Result<(), TransactionError> {
        if self.publish_unsupported {
            return Err(TransactionError::AtomicNoReplaceUnsupported);
        }
        if self.output_occupied {
            return Err(TransactionError::OutputRace("raced occupant".into()));
        }
        self.output = self.candidate.take();
        Ok(())
    }
    fn unpublish_export(
        &mut self,
        _: &ExportPlan,
        _: &str,
        _: &str,
    ) -> Result<(), TransactionError> {
        if self.candidate.is_some() {
            return Err(TransactionError::ThirdState("candidate also exists".into()));
        }
        self.candidate = self.output.take();
        Ok(())
    }
    fn prepare_owned_tree_cleanup(
        &mut self,
        _: &Journal,
        name: &str,
        _: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
    ) -> Result<OwnedTreeCleanupPreparation, TransactionError> {
        let order = memory_cleanup_order(seal);
        if completed != &order[..completed.len().min(order.len())] {
            return Err(TransactionError::ThirdState(
                "cleanup progress is not canonical".into(),
            ));
        }
        if completed.len() == order.len() {
            return Ok(OwnedTreeCleanupPreparation::Complete);
        }
        let key = order[completed.len()].clone();
        let root = key == "root";
        let expected = if root {
            OwnedEntrySeal {
                path: name.to_owned(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
                identity: format!("root/{name}"),
            }
        } else {
            seal.entries
                .iter()
                .find(|entry| memory_cleanup_key(entry) == key)
                .expect("cleanup order derives from seal")
                .clone()
        };
        Ok(OwnedTreeCleanupPreparation::Intent(
            OwnedTreeCleanupIntent {
                intent_token: format!("intent/{key}"),
                progress_key: key,
                path: expected.path.clone(),
                expected,
                root,
            },
        ))
    }

    fn execute_owned_tree_cleanup(
        &mut self,
        _: &Journal,
        name: &str,
        _: &str,
        _: &OwnedTreeSeal,
        _: &[String],
        intent: &OwnedTreeCleanupIntent,
    ) -> Result<OwnedTreeCleanupCompletion, TransactionError> {
        if self.cleanup_failures != 0 {
            self.cleanup_failures -= 1;
            return Err(TransactionError::Filesystem(
                "injected cleanup refusal".into(),
            ));
        }
        let recovered = if name.contains("candidate") {
            if intent.root {
                self.candidate.replace(Vec::new()).is_none()
            } else {
                let Some(entries) = self.candidate.as_mut() else {
                    return Err(TransactionError::ThirdState(
                        "candidate disappeared before cleanup".into(),
                    ));
                };
                let before = entries.len();
                entries.retain(|entry| entry.path != intent.path);
                before == entries.len()
            }
        } else if intent.root {
            let recovered = !self.quarantine;
            self.quarantine = false;
            recovered
        } else {
            false
        };
        if intent.root && name.contains("candidate") {
            self.candidate = None;
        }
        Ok(OwnedTreeCleanupCompletion {
            progress_key: intent.progress_key.clone(),
            recovered_after_syscall: recovered,
        })
    }
    fn create_quarantine(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        if let Some(outcome) = self.quarantine_creation.take() {
            if !matches!(outcome, ExclusiveTreeCreation::NotCreated { .. }) {
                self.quarantine = true;
            }
            return Ok(outcome);
        }
        if self.quarantine {
            return Ok(ExclusiveTreeCreation::NotCreated {
                detail: "quarantine occupied".into(),
            });
        }
        self.quarantine = true;
        Ok(ExclusiveTreeCreation::Owned)
    }
    fn observe_step(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        step: &MutationStep,
    ) -> Result<SealedObservation, TransactionError> {
        Ok(
            match self.steps.get(&step.id).copied().unwrap_or_default() {
                StepWorld::Before => SealedObservation::Before,
                StepWorld::After => SealedObservation::After,
                StepWorld::Third => SealedObservation::Third {
                    detail: "concurrent bytes".into(),
                },
            },
        )
    }
    fn apply_step(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        step: &MutationStep,
        _: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        self.source_mutations += usize::from(step.kind != MutationKind::ContractExternalPreserve);
        self.steps.insert(step.id.clone(), StepWorld::After);
        Ok(())
    }
    fn rollback_step(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError> {
        self.steps.insert(step.id.clone(), StepWorld::Before);
        Ok(())
    }
    fn cleanup_unpublished_step_stage(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        _: &MutationStep,
    ) -> Result<(), TransactionError> {
        Ok(())
    }
    fn observe_quarantine_root(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
    ) -> Result<OwnedRootObservation, TransactionError> {
        Ok(if self.quarantine_third {
            OwnedRootObservation::Third {
                detail: "quarantine identity changed".into(),
            }
        } else if self.quarantine {
            OwnedRootObservation::ExactOwned
        } else {
            OwnedRootObservation::Absent
        })
    }
}

fn memory_cleanup_key(entry: &OwnedEntrySeal) -> String {
    format!(
        "{}:{}",
        match entry.kind {
            TreeEntryKind::File => "file",
            TreeEntryKind::Directory => "directory",
        },
        entry.path
    )
}

fn memory_cleanup_order(seal: &OwnedTreeSeal) -> Vec<String> {
    let mut files = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::File)
        .map(memory_cleanup_key)
        .collect::<Vec<_>>();
    files.sort();
    let mut directories = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .path
            .matches('/')
            .count()
            .cmp(&left.path.matches('/').count())
            .then_with(|| right.path.cmp(&left.path))
    });
    files.extend(directories.into_iter().map(memory_cleanup_key));
    files.push("root".into());
    files
}

#[derive(Default)]
struct AcceptingVerifier {
    fail: Option<VerificationPhase>,
    error: Option<VerificationPhase>,
    drift_view: Option<VerificationPhase>,
    drift_real: Option<VerificationPhase>,
    execute_calls: usize,
    calls: Vec<(
        VerificationPhase,
        VerificationRootKind,
        String,
        Digest,
        bool,
        Option<String>,
    )>,
}

impl TransactionVerifier for AcceptingVerifier {
    fn observe_phase_view(
        &mut self,
        _: &Journal,
        context: &VerificationContext<'_>,
    ) -> Result<TreeManifest, TransactionError> {
        if self.drift_view == Some(context.phase) {
            Ok(manifest("drift", vec![file("concurrent", "bytes")]))
        } else {
            Ok(context.expected_tree.clone())
        }
    }

    fn execute_verification(
        &mut self,
        _: &Journal,
        context: VerificationContext<'_>,
    ) -> Result<VerificationEvidence, TransactionError> {
        self.execute_calls += 1;
        if self.error == Some(context.phase) {
            return Err(TransactionError::Verification(format!(
                "injected {:?} execution/environment/protocol error",
                context.phase
            )));
        }
        self.calls.push((
            context.phase,
            context.root_kind,
            context.root_display.to_owned(),
            context.expected_tree.digest.clone(),
            context.same_display_path_required,
            context.contract_exemption.map(str::to_owned),
        ));
        let canonical_evidence = match context.phase {
            VerificationPhase::Before | VerificationPhase::AfterHealth => serde_json::to_vec(
                &serde_json::json!({
                    "phase": if context.phase == VerificationPhase::Before { "before" } else { "after" },
                    "plan_id": hash("health").0,
                    "checks": [],
                    "assurance_reduced": false,
                }),
            )
            .unwrap(),
            _ => format!("evidence/{:?}", context.phase).into_bytes(),
        };
        Ok(VerificationEvidence {
            accepted: self.fail != Some(context.phase),
            assurance: Assurance::Full,
            summary: format!("{:?}", context.phase),
            canonical_evidence,
        })
    }

    fn reprove_real_tree(
        &mut self,
        journal: &Journal,
        root_kind: VerificationRootKind,
        _root_display: &str,
    ) -> Result<TreeManifest, TransactionError> {
        let phase = match root_kind {
            VerificationRootKind::ExportFinal => VerificationPhase::FinalTree,
            VerificationRootKind::Source if journal.mode == TransactionMode::Export => {
                VerificationPhase::SourceUnchanged
            }
            VerificationRootKind::Source | VerificationRootKind::InPlaceView => {
                VerificationPhase::FinalTree
            }
        };
        if self.drift_real == Some(phase) {
            return Ok(manifest("real-drift", vec![file("foreign", "bytes")]));
        }
        Ok(match (&journal.execution, root_kind) {
            (PreparedMode::Export(plan), VerificationRootKind::ExportFinal) => {
                plan.final_manifest.clone()
            }
            (PreparedMode::Export(plan), VerificationRootKind::Source) => plan.source_tree.clone(),
            (PreparedMode::InPlace(plan), VerificationRootKind::InPlaceView) => {
                plan.after_tree.clone()
            }
            (PreparedMode::InPlace(plan), VerificationRootKind::Source) => plan.before_tree.clone(),
            _ => {
                return Err(TransactionError::Verification(
                    "invalid real-tree root".into(),
                ));
            }
        })
    }
}

#[cfg(windows)]
#[derive(Default)]
struct RealTreeVerifier;

#[cfg(windows)]
impl TransactionVerifier for RealTreeVerifier {
    fn observe_phase_view(
        &mut self,
        _journal: &Journal,
        context: &VerificationContext<'_>,
    ) -> Result<TreeManifest, TransactionError> {
        observe_real_tree(std::path::Path::new(context.root_display))
    }

    fn execute_verification(
        &mut self,
        _journal: &Journal,
        context: VerificationContext<'_>,
    ) -> Result<VerificationEvidence, TransactionError> {
        let canonical_evidence = match context.phase {
            VerificationPhase::Before | VerificationPhase::AfterHealth => serde_json::to_vec(
                &serde_json::json!({
                    "phase": if context.phase == VerificationPhase::Before { "before" } else { "after" },
                    "plan_id": hash("health").0,
                    "checks": [],
                    "assurance_reduced": context.phase == VerificationPhase::AfterHealth,
                }),
            )
            .unwrap(),
            _ => format!("real-tree/e1/{:?}", context.phase).into_bytes(),
        };
        Ok(VerificationEvidence {
            accepted: true,
            assurance: if context.phase == VerificationPhase::AfterHealth {
                Assurance::Reduced
            } else {
                Assurance::Full
            },
            summary: format!("real {:?} accepted", context.phase),
            canonical_evidence,
        })
    }

    fn reprove_real_tree(
        &mut self,
        _journal: &Journal,
        _root_kind: VerificationRootKind,
        root_display: &str,
    ) -> Result<TreeManifest, TransactionError> {
        observe_real_tree(std::path::Path::new(root_display))
    }
}

#[cfg(windows)]
fn observe_real_tree(root: &std::path::Path) -> Result<TreeManifest, TransactionError> {
    let observed = crate::health::tree::observe(root)
        .map_err(|error| TransactionError::Verification(error.to_string()))?;
    Ok(TreeManifest {
        digest: Digest(observed.tree_digest),
        entries: observed
            .entries
            .into_iter()
            .map(|entry| TreeEntry {
                path: entry.path,
                kind: match entry.kind {
                    crate::health::tree::TreeEntryKind::File => TreeEntryKind::File,
                    crate::health::tree::TreeEntryKind::Directory => TreeEntryKind::Directory,
                },
                sha256: entry.sha256.map(Digest),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    })
}

#[derive(Default)]
struct OneFault {
    target: Option<DurableBoundary>,
    fired: bool,
}

#[derive(Default)]
struct TraceFaults(Vec<DurableBoundary>);

impl FaultInjector for TraceFaults {
    fn boundary(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError> {
        self.0.push(boundary);
        Ok(())
    }
}

impl FaultInjector for OneFault {
    fn boundary(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError> {
        if !self.fired && self.target.as_ref() == Some(&boundary) {
            self.fired = true;
            return Err(TransactionError::FaultInjected(boundary));
        }
        Ok(())
    }
}

fn hash(label: &str) -> Digest {
    digest(label.as_bytes())
}

fn file(path: &str, label: &str) -> TreeEntry {
    TreeEntry {
        path: path.into(),
        kind: TreeEntryKind::File,
        sha256: Some(hash(label)),
        bytes: Some(label.len() as u64),
        mode: Some(0o644),
    }
}

fn directory(path: &str) -> TreeEntry {
    TreeEntry {
        path: path.into(),
        kind: TreeEntryKind::Directory,
        sha256: None,
        bytes: None,
        mode: Some(0o755),
    }
}

fn manifest(label: &str, entries: Vec<TreeEntry>) -> TreeManifest {
    TreeManifest {
        digest: hash(label),
        entries,
    }
}

fn partial_manifest(entries: Vec<TreeEntry>) -> TreeManifest {
    logical_tree_manifest(entries)
}

fn canonical_plan(mode: TransactionMode) -> Vec<u8> {
    let value = serde_json::json!({
        "assertions": [],
        "blockers": [],
        "command": "scrape",
        "contract": {
            "action": "delete-last",
            "contained": true,
            "display_path": "C:/source/vibevm/scrape/contract.toml",
            "sha256": hash("contract bytes").0,
        },
        "contract_boundary": {
            "kind": "delete-last",
            "empty_ancestors": ["vibevm/scrape", "vibevm"],
            "path": "vibevm/scrape/contract.toml",
        },
        "health_baseline": "strict",
        "health_limits": {
            "max_result_bytes": "1024",
            "max_stderr_bytes": "1024",
            "max_stdout_bytes": "1024",
            "termination_grace_seconds": 1,
        },
        "health_plan_id": hash("health").0,
        "healthchecks": [],
        "items": [],
        "mode": match mode { TransactionMode::Export => "export", TransactionMode::InPlace => "in-place" },
        "native_lock_changes": [],
        "plan_id": hash("plan").0,
        "project": { "display_root": "C:/source", "tree_digest": hash("tree").0 },
        "relocations": [],
        "rewrites": [],
        "schema": 1,
        "summary": {
            "delete_last": 0,
            "delete_modified": 0,
            "delete_unknown": 0,
            "delete_unmodified": 0,
            "keep": 0,
            "relocate": 0,
            "rewrite": 0,
        }
    });
    let plan: vibe_wire::generated::scrape::e1::plan::Plan = serde_json::from_value(value).unwrap();
    serde_json::to_vec(&plan).unwrap()
}

fn snapshots(mode: TransactionMode) -> Vec<Snapshot> {
    vec![
        Snapshot {
            kind: SnapshotKind::Contract,
            name: "contract".into(),
            bytes: b"contract bytes".to_vec(),
            mode: Some(0o644),
        },
        Snapshot {
            kind: SnapshotKind::CanonicalPlan,
            name: "plan".into(),
            bytes: canonical_plan(mode),
            mode: Some(0o600),
        },
        Snapshot {
            kind: SnapshotKind::CanonicalContract,
            name: "canonical-contract".into(),
            bytes: b"canonical contract value".to_vec(),
            mode: Some(0o600),
        },
        Snapshot {
            kind: SnapshotKind::Verifier,
            name: "verifier".into(),
            bytes: b"verifier bytes".to_vec(),
            mode: Some(0o755),
        },
        Snapshot {
            kind: SnapshotKind::PreparedAfter,
            name: "after/readme".into(),
            bytes: b"native readme\n".to_vec(),
            mode: Some(0o644),
        },
    ]
}

fn export_prepared() -> PreparedTransaction {
    let entries = vec![
        file("README.md", "native readme\n"),
        directory("docs"),
        file("docs/spec.md", "spec"),
        directory("src"),
        file("src/main.rs", "main"),
    ];
    PreparedTransaction {
        project_identity_token: "stable-project-identity".into(),
        project_display_root: "C:/source".into(),
        plan_id: hash("plan"),
        canonical_plan: canonical_plan(TransactionMode::Export),
        snapshots: snapshots(TransactionMode::Export),
        mode: PreparedMode::Export(Box::new(ExportPlan {
            output_identity: "absent-output-slot".into(),
            output_parent_identity: "output-parent-volume".into(),
            output_display_path: "C:/delivery".into(),
            output_name: "delivery".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![
                ExportEntry {
                    target_path: "README.md".into(),
                    kind: TreeEntryKind::File,
                    mode: Some(0o644),
                    payload: Some(ExportPayload::PreparedAfter {
                        snapshot_name: "after/readme".into(),
                    }),
                },
                ExportEntry {
                    target_path: "docs".into(),
                    kind: TreeEntryKind::Directory,
                    mode: Some(0o755),
                    payload: None,
                },
                ExportEntry {
                    target_path: "docs/spec.md".into(),
                    kind: TreeEntryKind::File,
                    mode: Some(0o644),
                    payload: Some(ExportPayload::Source {
                        source_path: "vibevm/spec.md".into(),
                        before: FileState {
                            sha256: hash("spec"),
                            bytes: 4,
                            mode: Some(0o644),
                        },
                    }),
                },
                ExportEntry {
                    target_path: "src".into(),
                    kind: TreeEntryKind::Directory,
                    mode: Some(0o755),
                    payload: None,
                },
                ExportEntry {
                    target_path: "src/main.rs".into(),
                    kind: TreeEntryKind::File,
                    mode: Some(0o644),
                    payload: Some(ExportPayload::Source {
                        source_path: "src/main.rs".into(),
                        before: FileState {
                            sha256: hash("main"),
                            bytes: 4,
                            mode: Some(0o644),
                        },
                    }),
                },
            ],
            source_tree: manifest(
                "source",
                vec![
                    file("README.md", "old readme"),
                    directory("src"),
                    file("src/main.rs", "main"),
                    directory("vibevm"),
                    file("vibevm/spec.md", "spec"),
                ],
            ),
            final_manifest: manifest("final", entries),
        })),
    }
}

fn transition(path: &str, before: PathState, after: PathState) -> PathTransition {
    PathTransition {
        location: Location::Project,
        path: path.into(),
        before,
        after,
    }
}

fn present(label: &str) -> PathState {
    PathState::File(FileState {
        sha256: hash(label),
        bytes: label.len() as u64,
        mode: Some(0o644),
    })
}

fn in_place_prepared() -> PreparedTransaction {
    let remove = MutationStep {
        id: "remove-metadata".into(),
        pair_id: None,
        kind: MutationKind::QuarantineFile,
        transitions: vec![
            transition("vibevm/data", present("data"), PathState::Absent),
            PathTransition {
                location: Location::Quarantine,
                path: "payload/vibevm/data".into(),
                before: PathState::Absent,
                after: present("data"),
            },
        ],
    };
    let contract = MutationStep {
        id: "contract-last".into(),
        pair_id: None,
        kind: MutationKind::ContractDeleteLast,
        transitions: vec![
            transition(
                "vibevm/scrape/contract.toml",
                present("contract bytes"),
                PathState::Absent,
            ),
            PathTransition {
                location: Location::Quarantine,
                path: "payload/vibevm/scrape/contract.toml".into(),
                before: PathState::Absent,
                after: present("contract bytes"),
            },
        ],
    };
    let contract_cleanup = MutationStep {
        id: "contract-ancestor-tree-park".into(),
        pair_id: None,
        kind: MutationKind::ContractAncestorTreePark,
        transitions: vec![
            transition(
                "vibevm",
                PathState::Tree(SubtreeState {
                    digest: hash("contract-ancestor-tree"),
                    root_mode: Some(0o755),
                    descendants: vec![SubtreeEntry {
                        relative_path: "scrape".into(),
                        kind: TreeEntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        mode: Some(0o755),
                    }],
                }),
                PathState::Absent,
            ),
            PathTransition {
                location: Location::Quarantine,
                path: "directories/contract-ancestors".into(),
                before: PathState::Absent,
                after: PathState::Tree(SubtreeState {
                    digest: hash("contract-ancestor-tree"),
                    root_mode: Some(0o755),
                    descendants: vec![SubtreeEntry {
                        relative_path: "scrape".into(),
                        kind: TreeEntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        mode: Some(0o755),
                    }],
                }),
            },
        ],
    };
    PreparedTransaction {
        project_identity_token: "stable-project-identity".into(),
        project_display_root: "C:/source".into(),
        plan_id: hash("plan"),
        canonical_plan: canonical_plan(TransactionMode::InPlace),
        snapshots: snapshots(TransactionMode::InPlace),
        mode: PreparedMode::InPlace(Box::new(InPlacePlan {
            quarantine_parent_identity: "same-volume-parent".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: vec![remove],
            contract: ContractCommit::DeleteLast {
                path: "vibevm/scrape/contract.toml".into(),
                empty_ancestors: vec!["vibevm/scrape".into(), "vibevm".into()],
            },
            contract_step: contract,
            contract_cleanup_step: Some(contract_cleanup),
            before_tree: manifest(
                "before",
                vec![
                    directory("vibevm"),
                    file("vibevm/data", "data"),
                    directory("vibevm/scrape"),
                    file("vibevm/scrape/contract.toml", "contract bytes"),
                ],
            ),
            pre_contract_tree: manifest(
                "pre-contract",
                vec![
                    directory("vibevm"),
                    directory("vibevm/scrape"),
                    file("vibevm/scrape/contract.toml", "contract bytes"),
                ],
            ),
            post_contract_tree: manifest(
                "post-contract",
                vec![directory("vibevm"), directory("vibevm/scrape")],
            ),
            after_tree: manifest("after", Vec::new()),
        })),
    }
}

fn complex_in_place_prepared() -> PreparedTransaction {
    let old = present("old cargo");
    let new = present("new cargo");
    let spec = SubtreeState {
        digest: hash("spec-tree"),
        root_mode: Some(0o755),
        descendants: vec![SubtreeEntry {
            relative_path: "guide.md".into(),
            kind: TreeEntryKind::File,
            sha256: Some(hash("spec")),
            bytes: Some(4),
            mode: Some(0o644),
        }],
    };
    let data = present("data");
    let capture = MutationStep {
        id: "capture-cargo".into(),
        pair_id: Some("cargo-pair".into()),
        kind: MutationKind::CaptureBeforeImage,
        transitions: vec![
            transition("Cargo.toml", old.clone(), old.clone()),
            PathTransition {
                location: Location::Quarantine,
                path: "before/Cargo.toml".into(),
                before: PathState::Absent,
                after: old.clone(),
            },
        ],
    };
    let rewrite = MutationStep {
        id: "rewrite-cargo".into(),
        pair_id: Some("cargo-pair".into()),
        kind: MutationKind::AtomicRewrite,
        transitions: vec![transition("Cargo.toml", old, new)],
    };
    let create_docs = MutationStep {
        id: "create-docs".into(),
        pair_id: None,
        kind: MutationKind::CreateRelocationParent,
        transitions: vec![transition(
            "docs",
            PathState::Absent,
            PathState::EmptyDirectory { mode: Some(0o755) },
        )],
    };
    let relocate = MutationStep {
        id: "relocate-spec".into(),
        pair_id: None,
        kind: MutationKind::Relocate,
        transitions: vec![
            transition(
                "vibevm/specs",
                PathState::Tree(spec.clone()),
                PathState::Absent,
            ),
            transition("docs/specs", PathState::Absent, PathState::Tree(spec)),
        ],
    };
    let remove = MutationStep {
        id: "remove-data".into(),
        pair_id: None,
        kind: MutationKind::QuarantineFile,
        transitions: vec![
            transition("vibevm/data", data.clone(), PathState::Absent),
            PathTransition {
                location: Location::Quarantine,
                path: "payload/vibevm/data".into(),
                before: PathState::Absent,
                after: data,
            },
        ],
    };
    let prune = MutationStep {
        id: "prune-empty".into(),
        pair_id: None,
        kind: MutationKind::PruneEmptyDirectory,
        transitions: vec![transition(
            "vibevm/empty",
            PathState::EmptyDirectory { mode: Some(0o755) },
            PathState::Absent,
        )],
    };
    let mut prepared = in_place_prepared();
    prepared.snapshots.push(Snapshot {
        kind: SnapshotKind::PreparedAfter,
        name: "after/rewrite-cargo".into(),
        bytes: b"new cargo".to_vec(),
        mode: Some(0o644),
    });
    let PreparedMode::InPlace(plan) = &mut prepared.mode else {
        unreachable!()
    };
    plan.steps = vec![capture, rewrite, create_docs, relocate, remove, prune];
    plan.before_tree = manifest(
        "complex-before",
        vec![
            file("Cargo.toml", "old cargo"),
            directory("vibevm"),
            file("vibevm/data", "data"),
            directory("vibevm/empty"),
            directory("vibevm/scrape"),
            file("vibevm/scrape/contract.toml", "contract bytes"),
            directory("vibevm/specs"),
            file("vibevm/specs/guide.md", "spec"),
        ],
    );
    plan.pre_contract_tree = manifest(
        "complex-pre-contract",
        vec![
            file("Cargo.toml", "new cargo"),
            directory("docs"),
            directory("docs/specs"),
            file("docs/specs/guide.md", "spec"),
            directory("vibevm"),
            directory("vibevm/scrape"),
            file("vibevm/scrape/contract.toml", "contract bytes"),
        ],
    );
    plan.post_contract_tree = manifest(
        "complex-post-contract",
        vec![
            file("Cargo.toml", "new cargo"),
            directory("docs"),
            directory("docs/specs"),
            file("docs/specs/guide.md", "spec"),
            directory("vibevm"),
            directory("vibevm/scrape"),
        ],
    );
    plan.after_tree = manifest(
        "complex-after",
        vec![
            file("Cargo.toml", "new cargo"),
            directory("docs"),
            directory("docs/specs"),
            file("docs/specs/guide.md", "spec"),
        ],
    );
    prepared
}

fn external_preserve_prepared() -> PreparedTransaction {
    let mut prepared = in_place_prepared();
    let PreparedMode::InPlace(plan) = &mut prepared.mode else {
        unreachable!()
    };
    plan.steps.clear();
    plan.contract = ContractCommit::ExternalPreserve;
    plan.contract_step = MutationStep {
        id: "external-preserve".into(),
        pair_id: None,
        kind: MutationKind::ContractExternalPreserve,
        transitions: Vec::new(),
    };
    plan.contract_cleanup_step = None;
    plan.before_tree = manifest("external", Vec::new());
    plan.pre_contract_tree = plan.before_tree.clone();
    plan.post_contract_tree = plan.before_tree.clone();
    plan.after_tree = plan.before_tree.clone();
    prepared
}

fn execute<I: FaultInjector>(
    prepared: PreparedTransaction,
    store: &mut MemoryStore,
    fs: &mut MemoryFs,
    verifier: &mut AcceptingVerifier,
    faults: &mut I,
) -> Result<TransactionReport, TransactionError> {
    let identity = prepared.project_identity_token.clone();
    let root = prepared.project_display_root.clone();
    Engine::new(store, fs, verifier, faults).execute_locked(&identity, &root, || Ok(prepared))
}

fn recover(
    store: &mut MemoryStore,
    fs: &mut MemoryFs,
    verifier: &mut AcceptingVerifier,
) -> Result<TransactionReport, TransactionError> {
    Engine::new(store, fs, verifier, &mut NoFaults).recover("stable-project-identity", "C:/source")
}

#[test]
fn project_key_is_deterministic_and_domain_separated() {
    assert_eq!(project_key("same"), project_key("same"));
    assert_ne!(project_key("same"), project_key("different"));
    assert_eq!(
        hash("abc").0,
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn snapshots_and_prepared_journal_are_durable_before_export_mutation() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".into(),
        }),
        fired: false,
    };
    let error = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    let first_journal = store
        .events
        .iter()
        .position(|event| event == "journal/Prepared")
        .unwrap();
    assert_eq!(
        store.events[..first_journal]
            .iter()
            .filter(|event| event.starts_with("snapshot/"))
            .count(),
        5
    );
    assert_eq!(fs.source_mutations, 0, "export never mutates source");
}

#[test]
fn export_restart_after_publication_rolls_back_exact_output() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-publish".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    assert!(fs.output.is_some() && fs.candidate.is_none());
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::Candidate
    );

    let report = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(fs.output.is_none() && fs.candidate.is_none());
    assert_eq!(fs.source_mutations, 0);
}

#[test]
fn ordinary_before_health_errors_terminal_refuse_without_pending_mutation() {
    for prepared in [export_prepared(), complex_in_place_prepared()] {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut verifier = AcceptingVerifier {
            error: Some(VerificationPhase::Before),
            ..AcceptingVerifier::default()
        };
        let report = execute(prepared, &mut store, &mut fs, &mut verifier, &mut NoFaults).unwrap();
        assert_eq!(report.outcome, Outcome::Refused);
        assert_eq!(report.cleanup, Cleanup::Complete);
        assert!(store.pending.is_none());
        assert_eq!(fs.source_mutations, 0);
        assert!(!fs.quarantine && fs.candidate.is_none() && fs.output.is_none());
    }
}

#[test]
fn export_third_state_refuses_without_overwriting_concurrent_data() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-publish".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    fs.output_third = true;
    let error = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
    assert!(fs.output.is_some(), "third-state output remains untouched");
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::RollbackFailed
    );
}

#[test]
fn published_output_disappearance_is_third_state_not_successful_rollback() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let prepared = export_prepared();
    let publish_index = match &prepared.mode {
        PreparedMode::Export(plan) => plan.entries.len() + 1,
        _ => unreachable!(),
    };
    let mut fault = OneFault {
        target: Some(DurableBoundary::StepCompletionPersisted {
            index: publish_index,
            id: "export/publish".into(),
        }),
        fired: false,
    };
    execute(
        prepared,
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    fs.output = None;
    let error = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
}

#[test]
fn rollback_failed_recovery_republishes_embedded_report_and_retries_store_failure() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut crash = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-publish".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut crash,
    )
    .unwrap_err();
    fs.output_third = true;
    let mut rollback_failed_crash = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::RollbackFailed,
        )),
        fired: false,
    };
    let first = Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut rollback_failed_crash,
    )
    .recover("stable-project-identity", "C:/source");
    assert!(matches!(first, Err(TransactionError::FaultInjected(_))));
    assert_eq!(
        store
            .pending
            .as_ref()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::RollbackFailed
    );
    store.fail_report_once = true;
    assert!(matches!(
        recover(&mut store, &mut fs, &mut AcceptingVerifier::default()),
        Err(TransactionError::Store(_))
    ));
    assert!(matches!(
        recover(&mut store, &mut fs, &mut AcceptingVerifier::default()),
        Err(TransactionError::ThirdState(_))
    ));
    assert_eq!(
        store.reports.last().unwrap().outcome,
        Outcome::RollbackFailed
    );
}

#[test]
fn complete_partial_preparation_retires_without_ephemeral_snapshot_files() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut data_crash = OneFault {
        target: Some(DurableBoundary::SnapshotDataPersisted { index: 0 }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut data_crash,
    )
    .unwrap_err();
    let mut terminal_crash = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::Complete,
        )),
        fired: false,
    };
    let first = Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut terminal_crash,
    )
    .recover("stable-project-identity", "C:/source");
    assert!(matches!(first, Err(TransactionError::FaultInjected(_))));
    store.snapshots.clear();
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(store.pending.is_none());
}

#[test]
fn destination_race_is_refused_and_raced_occupant_survives() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs {
        output_occupied: true,
        ..MemoryFs::default()
    };
    let report = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut OneFault::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(fs.output_occupied);
    assert!(fs.candidate.is_none());
    assert_eq!(fs.source_mutations, 0);
}

#[test]
fn unavailable_atomic_noreplace_publication_refuses_and_cleans_candidate() {
    let mut fs = MemoryFs {
        publish_unsupported: true,
        ..MemoryFs::default()
    };
    let report = execute(
        export_prepared(),
        &mut MemoryStore::default(),
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(fs.candidate.is_none() && fs.output.is_none());
}

#[test]
fn contract_boundary_is_rollback_capable_and_contract_is_restored() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::ContractBoundary(ContractBoundaryAction::DeleteLastMoved),
        )),
        fired: false,
    };
    execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::ContractBoundary(ContractBoundaryAction::DeleteLastMoved)
    );
    assert_eq!(fs.steps.get("contract-last"), Some(&StepWorld::After));
    let report = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert_eq!(fs.steps.get("contract-last"), Some(&StepWorld::Before));
    assert_eq!(fs.steps.get("remove-metadata"), Some(&StepWorld::Before));
}

#[test]
fn quarantine_creation_crash_is_inferred_and_reported_in_both_directions() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "quarantine-create".into(),
        }),
        fired: false,
    };
    execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    assert!(fs.quarantine);
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(!fs.quarantine);
    assert!(report.actual_mutations.iter().any(|actual| {
        actual.id == "in-place/quarantine"
            && actual.direction == MutationDirection::Apply
            && actual.status == MutationStatus::Applied
    }));
    assert!(report.actual_mutations.iter().any(|actual| {
        actual.id == "in-place/quarantine"
            && actual.direction == MutationDirection::Rollback
            && actual.status == MutationStatus::RolledBack
    }));
}

#[test]
fn in_place_third_state_never_overwrites_user_bytes() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "in-place-step-0".into(),
        }),
        fired: false,
    };
    execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    fs.steps.insert("remove-metadata".into(), StepWorld::Third);
    let error = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
    assert_eq!(fs.steps["remove-metadata"], StepWorld::Third);
}

#[test]
fn final_health_receives_final_path_and_exact_tree_seal() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let report = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut OneFault::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    let final_calls = verifier
        .calls
        .iter()
        .filter(|call| call.0 != VerificationPhase::Before)
        .collect::<Vec<_>>();
    assert!(final_calls.iter().all(|call| {
        call.1 == VerificationRootKind::ExportFinal
            && call.2 == "C:/delivery"
            && call.3 == hash("final")
            && !call.4
    }));
    assert_eq!(
        fs.prepared_payloads.get("README.md").map(Vec::as_slice),
        Some(b"native readme\n".as_slice())
    );
    let output = fs.output.as_ref().unwrap();
    assert!(output.iter().any(|entry| entry.path == "src/main.rs"));
    assert!(output.iter().any(|entry| entry.path == "docs/spec.md"));
    assert!(!output.iter().any(|entry| entry.path == ".git"));
}

#[test]
fn every_reachable_durable_boundary_accepts_deterministic_fault_injection() {
    #[derive(Clone, Copy)]
    enum Scenario {
        ExportSuccess,
        ExportRollback,
        ExportRace,
        InPlaceSuccess,
        InPlaceRollback,
        InPlaceExternalSuccess,
    }
    impl Scenario {
        fn prepared(self) -> PreparedTransaction {
            match self {
                Self::ExportSuccess | Self::ExportRollback | Self::ExportRace => export_prepared(),
                Self::InPlaceSuccess | Self::InPlaceRollback => complex_in_place_prepared(),
                Self::InPlaceExternalSuccess => external_preserve_prepared(),
            }
        }
        fn verifier(self) -> AcceptingVerifier {
            AcceptingVerifier {
                fail: match self {
                    Self::ExportRollback | Self::InPlaceRollback => {
                        Some(VerificationPhase::AfterHealth)
                    }
                    _ => None,
                },
                ..AcceptingVerifier::default()
            }
        }
        fn filesystem(self) -> MemoryFs {
            if matches!(self, Self::ExportRace) {
                MemoryFs {
                    output_occupied: true,
                    ..MemoryFs::default()
                }
            } else {
                MemoryFs::default()
            }
        }
    }

    let scenarios = [
        Scenario::ExportSuccess,
        Scenario::ExportRollback,
        Scenario::ExportRace,
        Scenario::InPlaceSuccess,
        Scenario::InPlaceRollback,
        Scenario::InPlaceExternalSuccess,
    ];
    let mut traces = Vec::new();
    for scenario in scenarios {
        let mut trace = TraceFaults::default();
        execute(
            scenario.prepared(),
            &mut MemoryStore::default(),
            &mut scenario.filesystem(),
            &mut scenario.verifier(),
            &mut trace,
        )
        .unwrap();
        traces.push((scenario, trace.0));
    }
    let unique = traces
        .iter()
        .flat_map(|(_, trace)| trace.iter().cloned())
        .collect::<Vec<_>>();
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::SnapshotPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepIntentPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepCompletionPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::VerificationCompleted(_)))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::CleanupCompleted))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepRollbackIntentPersisted { .. }))
    );
    assert!(
        unique
            .iter()
            .any(|b| matches!(b, DurableBoundary::StepRollbackCompletionPersisted { .. }))
    );
    assert!(unique.iter().any(|b| matches!(
        b,
        DurableBoundary::JournalPersisted(TransactionState::ContractBoundary(
            ContractBoundaryAction::ExternalPreserved
        ))
    )));

    for (scenario, boundaries) in traces {
        let mut boundaries = boundaries;
        boundaries.sort_by_key(|boundary| format!("{boundary:?}"));
        boundaries.dedup();
        for target in boundaries {
            let mut store = MemoryStore::default();
            let mut fs = scenario.filesystem();
            let mut verifier = scenario.verifier();
            let mut fault = OneFault {
                target: Some(target.clone()),
                fired: false,
            };
            let result = execute(
                scenario.prepared(),
                &mut store,
                &mut fs,
                &mut verifier,
                &mut fault,
            );
            assert!(fault.fired, "boundary was not reachable: {target:?}");
            assert!(matches!(result, Err(TransactionError::FaultInjected(_))));
            let Some(pending) = store.pending.as_ref() else {
                assert!(matches!(
                    target,
                    DurableBoundary::StoreProvedExternal | DurableBoundary::ProjectLockAcquired
                ));
                assert_eq!(fs.source_mutations, 0);
                continue;
            };
            let state_at_crash = pending.state.clone();
            let outcome_at_crash = pending.report.as_ref().map(|report| report.outcome);
            let settlement_at_crash = pending.settlement_intent;
            let recovered = recover(&mut store, &mut fs, &mut AcceptingVerifier::default())
                .unwrap_or_else(|error| {
                    panic!("recovering {target:?} from {state_at_crash:?}: {error}")
                });
            let expected = match state_at_crash {
                TransactionState::Preparing => Outcome::Refused,
                TransactionState::Verified | TransactionState::CleanupPending => Outcome::Verified,
                TransactionState::Complete => outcome_at_crash.expect("complete has report"),
                _ if settlement_at_crash == Some(Outcome::Refused) => Outcome::Refused,
                _ => Outcome::RolledBack,
            };
            assert_eq!(recovered.outcome, expected, "target {target:?}");
            assert_eq!(recovered.cleanup, Cleanup::Complete, "target {target:?}");
            assert!(!recovered.planned_mutations.is_empty());
            assert!(recovered.actual_mutations.iter().all(|actual| matches!(
                (actual.direction, actual.status),
                (MutationDirection::Apply, MutationStatus::Applied)
                    | (MutationDirection::Rollback, MutationStatus::RolledBack)
            )));
            assert!(store.pending.is_none(), "target {target:?}");
            match (scenario, expected) {
                (
                    Scenario::ExportSuccess | Scenario::ExportRollback | Scenario::ExportRace,
                    Outcome::Verified,
                ) => {
                    assert!(fs.output.is_some() && fs.candidate.is_none())
                }
                (Scenario::ExportSuccess | Scenario::ExportRollback | Scenario::ExportRace, _) => {
                    assert!(fs.output.is_none() && fs.candidate.is_none())
                }
                (
                    Scenario::InPlaceSuccess
                    | Scenario::InPlaceRollback
                    | Scenario::InPlaceExternalSuccess,
                    Outcome::Verified,
                ) => {
                    assert!(fs.steps.values().all(|state| *state == StepWorld::After));
                    assert!(!fs.quarantine);
                }
                (
                    Scenario::InPlaceSuccess
                    | Scenario::InPlaceRollback
                    | Scenario::InPlaceExternalSuccess,
                    _,
                ) => {
                    assert!(fs.steps.values().all(|state| *state == StepWorld::Before));
                    assert!(!fs.quarantine);
                }
            }
        }
    }
}

#[test]
fn cleanup_pending_survives_and_recovery_only_rolls_forward() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs {
        cleanup_failures: 1,
        ..MemoryFs::default()
    };
    let mut verifier = AcceptingVerifier::default();
    let report = execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut OneFault::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(report.cleanup, Cleanup::Pending);
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::CleanupPending
    );
    assert_eq!(fs.steps["remove-metadata"], StepWorld::After);
    assert_eq!(fs.steps["contract-last"], StepWorld::After);
    let pending_events = report.events.clone();

    let recovered = Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
    assert_eq!(recovered.outcome, Outcome::Verified);
    assert_eq!(recovered.cleanup, Cleanup::Complete);
    assert_eq!(
        &recovered.events[..pending_events.len()],
        pending_events.as_slice(),
        "cleanup-pending evidence must remain an append-only prefix"
    );
    assert_eq!(fs.steps["contract-last"], StepWorld::After);
    assert!(!fs.quarantine);
}

#[test]
fn cleanup_syscall_before_completion_checkpoint_is_replayed_from_exact_intent() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::CleanupMutationCompleted {
            progress_key: "root".into(),
        }),
        fired: false,
    };
    let error = execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    assert!(!fs.quarantine, "the root syscall occurred before the crash");
    let pending = store.pending.as_ref().unwrap();
    assert_eq!(
        pending
            .cleanup_wal
            .as_ref()
            .and_then(|wal| wal.active.as_ref())
            .map(|intent| intent.progress_key.as_str()),
        Some("root")
    );

    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(report.cleanup, Cleanup::Complete);
    assert!(
        report
            .events
            .iter()
            .any(|event| event.contains("recovered completed syscall for `root`"))
    );
}

#[test]
fn recovery_uses_journal_without_any_contract_source_input() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut verifier = AcceptingVerifier::default();
    let mut faults = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "in-place-step-0".into(),
        }),
        fired: false,
    };
    execute(
        in_place_prepared(),
        &mut store,
        &mut fs,
        &mut verifier,
        &mut faults,
    )
    .unwrap_err();
    let contract_record = store
        .pending
        .as_ref()
        .unwrap()
        .snapshots
        .iter()
        .find(|record| record.kind == SnapshotKind::Contract)
        .unwrap();
    assert_eq!(contract_record.sha256, digest(b"contract bytes"));
    Engine::new(&mut store, &mut fs, &mut verifier, &mut NoFaults)
        .recover("stable-project-identity", "C:/source")
        .unwrap();
}

#[test]
fn missing_safefs_directory_primitive_is_a_typed_blocker() {
    assert_eq!(
        RequiredPrimitive::StableProjectIdentityToken.required_api(),
        "vibe_safefs::Project::identity_token()"
    );
    assert!(
        RequiredPrimitive::ExternalNoFollowStoreAndLock
            .required_api()
            .contains("open_and_lock_project")
    );
    let PreparedMode::Export(plan) = export_prepared().mode else {
        unreachable!()
    };
    let error = SafefsCapabilityGap
        .publish_export_noreplace(&plan, "candidate", "owner")
        .unwrap_err();
    assert_eq!(
        error,
        TransactionError::MissingPrimitive(RequiredPrimitive::AtomicNoReplaceDirectoryRename)
    );
    assert!(error.to_string().contains("rename_child_noreplace_to"));
}

#[test]
fn active_export_step_accepts_exact_prefix_before_or_after_only() {
    for target in [
        DurableBoundary::StepIntentPersisted {
            index: 2,
            id: "export/entry/2/docs/spec.md".into(),
        },
        DurableBoundary::MutationCompleted {
            label: "export-entry-2".into(),
        },
    ] {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut verifier = AcceptingVerifier::default();
        let mut fault = OneFault {
            target: Some(target),
            fired: false,
        };
        execute(
            export_prepared(),
            &mut store,
            &mut fs,
            &mut verifier,
            &mut fault,
        )
        .unwrap_err();
        let report = recover(&mut store, &mut fs, &mut verifier).unwrap();
        assert_eq!(report.outcome, Outcome::RolledBack);
        assert!(fs.candidate.is_none() && fs.output.is_none());
    }
}

#[test]
fn exclusive_candidate_creation_distinguishes_all_ownership_outcomes() {
    let mut not_created = MemoryFs {
        candidate_creation: Some(ExclusiveTreeCreation::NotCreated {
            detail: "raced occupant".into(),
        }),
        ..MemoryFs::default()
    };
    let report = execute(
        export_prepared(),
        &mut MemoryStore::default(),
        &mut not_created,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(not_created.candidate.is_none());

    let mut store = MemoryStore::default();
    let mut uncertain = MemoryFs {
        candidate_creation: Some(ExclusiveTreeCreation::CreatedNotReopened {
            detail: "created but identity reopen failed".into(),
        }),
        ..MemoryFs::default()
    };
    execute(
        export_prepared(),
        &mut store,
        &mut uncertain,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert!(
        store.pending.is_some(),
        "partial ownership remains recoverable"
    );
    let report = recover(
        &mut store,
        &mut uncertain,
        &mut AcceptingVerifier::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(uncertain.candidate.is_none());

    let mut store = MemoryStore::default();
    let mut refused = MemoryFs {
        candidate_creation: Some(ExclusiveTreeCreation::NotCreated {
            detail: "occupied".into(),
        }),
        ..MemoryFs::default()
    };
    let mut fault = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::Complete,
        )),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut refused,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    let report = recover(&mut store, &mut refused, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
}

#[test]
fn preparation_journal_is_recoverable_at_creation_each_snapshot_and_store_failure() {
    let snapshot_count = export_prepared().snapshots.len();
    let mut targets = vec![DurableBoundary::TransactionCreated];
    for index in 0..snapshot_count {
        targets.extend([
            DurableBoundary::SnapshotIntentPersisted { index },
            DurableBoundary::SnapshotDataPersisted { index },
            DurableBoundary::SnapshotPersisted { index },
        ]);
    }
    for target in targets {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut fault = OneFault {
            target: Some(target),
            fired: false,
        };
        execute(
            export_prepared(),
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut fault,
        )
        .unwrap_err();
        assert_eq!(
            store.pending.as_ref().unwrap().state,
            TransactionState::Preparing
        );
        let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
        assert_eq!(report.outcome, Outcome::Refused);
        assert_eq!(fs.source_mutations, 0);
    }

    let mut store = MemoryStore {
        fail_snapshot_at: Some(2),
        ..MemoryStore::default()
    };
    let mut fs = MemoryFs::default();
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert_eq!(store.pending.as_ref().unwrap().snapshots_persisted, 2);
    store.fail_snapshot_at = None;
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);

    let mut store = MemoryStore {
        fail_snapshot_after_write_at: Some(2),
        ..MemoryStore::default()
    };
    let mut fs = MemoryFs::default();
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert_eq!(store.pending.as_ref().unwrap().snapshot_active, Some(2));
    assert_eq!(store.snapshots.len(), 3);
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
}

#[test]
fn pending_gate_runs_under_lock_before_preparation_closure() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    let prepared_called = std::cell::Cell::new(false);
    let result = Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .execute_locked(
        "stable-project-identity",
        "C:/source",
        || -> Result<PreparedTransaction, TransactionError> {
            prepared_called.set(true);
            Ok(export_prepared())
        },
    );
    assert!(result.is_err());
    assert!(!prepared_called.get());
}

#[test]
fn strict_journal_validation_rejects_corruption_before_recovery_mutation() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let mut fault = OneFault {
        target: Some(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".into(),
        }),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut fault,
    )
    .unwrap_err();
    let original = store.pending.clone().unwrap();
    let key = project_key("stable-project-identity");
    let mut corruptions = Vec::new();
    let mut state = original.clone();
    state.state = TransactionState::BeforePassed;
    corruptions.push(state);
    let mut counter = original.clone();
    counter.completed_steps = usize::MAX;
    corruptions.push(counter);
    let mut active = original.clone();
    active.active_step = Some(2);
    corruptions.push(active);
    let mut name = original.clone();
    name.candidate_name = Some("foreign".into());
    corruptions.push(name);
    let mut progress = original;
    progress.mutation_progress.pop();
    corruptions.push(progress);
    for journal in corruptions {
        assert!(super::validate::journal(&journal, &key, "C:/source").is_err());
    }

    let mut store = MemoryStore::default();
    let mut verified_fault = OneFault {
        target: Some(DurableBoundary::JournalPersisted(
            TransactionState::Verified,
        )),
        fired: false,
    };
    execute(
        export_prepared(),
        &mut store,
        &mut MemoryFs::default(),
        &mut AcceptingVerifier::default(),
        &mut verified_fault,
    )
    .unwrap_err();
    let verified = store.pending.unwrap();
    assert_eq!(
        verified.verification.last().map(|record| record.phase),
        Some(VerificationPhase::SourceUnchanged)
    );
    let mut missing = verified.clone();
    missing.verification.pop();
    assert!(super::validate::journal(&missing, &key, "C:/source").is_err());
    let mut false_gate = verified.clone();
    false_gate
        .verification
        .last_mut()
        .unwrap()
        .evidence
        .accepted = false;
    assert!(super::validate::journal(&false_gate, &key, "C:/source").is_err());
    let mut duplicate = verified;
    duplicate
        .verification
        .push(duplicate.verification[0].clone());
    assert!(super::validate::journal(&duplicate, &key, "C:/source").is_err());
}

#[test]
fn closed_mutation_grammar_executes_rewrite_relocate_remove_prune_and_external_preserve() {
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let report = execute(
        complex_in_place_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Verified);
    assert!(
        report
            .planned_mutations
            .iter()
            .any(|item| { item.kind == PlannedMutationKind::InPlace(MutationKind::AtomicRewrite) })
    );
    assert!(
        report
            .planned_mutations
            .iter()
            .any(|item| { item.kind == PlannedMutationKind::InPlace(MutationKind::Relocate) })
    );
    assert!(
        report
            .actual_mutations
            .iter()
            .any(|item| { item.id == "prune-empty" && item.direction == MutationDirection::Apply })
    );

    let mut invalid = complex_in_place_prepared();
    let PreparedMode::InPlace(plan) = &mut invalid.mode else {
        unreachable!()
    };
    plan.steps.swap(1, 4);
    let mut untouched = MemoryFs::default();
    assert!(
        execute(
            invalid,
            &mut MemoryStore::default(),
            &mut untouched,
            &mut AcceptingVerifier::default(),
            &mut NoFaults,
        )
        .is_err()
    );
    assert_eq!(untouched.source_mutations, 0);

    let mut external_fs = MemoryFs::default();
    let external = execute(
        external_preserve_prepared(),
        &mut MemoryStore::default(),
        &mut external_fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(external.outcome, Outcome::Verified);
    assert!(
        !external
            .actual_mutations
            .iter()
            .any(|item| item.id == "external-preserve")
    );
    assert_eq!(external_fs.source_mutations, 0);

    let mut collision = in_place_prepared();
    let PreparedMode::InPlace(plan) = &mut collision.mode else {
        unreachable!()
    };
    plan.contract_step.id = "in-place/quarantine".into();
    assert!(
        execute(
            collision,
            &mut MemoryStore::default(),
            &mut MemoryFs::default(),
            &mut AcceptingVerifier::default(),
            &mut NoFaults,
        )
        .is_err()
    );

    for malicious in ["other/file.toml", "vibevm/data"] {
        let mut prepared = in_place_prepared();
        let PreparedMode::InPlace(plan) = &mut prepared.mode else {
            unreachable!()
        };
        plan.contract = ContractCommit::DeleteLast {
            path: malicious.into(),
            empty_ancestors: vec!["other".into()],
        };
        assert!(
            execute(
                prepared,
                &mut MemoryStore::default(),
                &mut MemoryFs::default(),
                &mut AcceptingVerifier::default(),
                &mut NoFaults,
            )
            .is_err()
        );
    }
}

#[test]
fn phase_view_must_equal_expected_seal_before_child_execution() {
    let mut before_verifier = AcceptingVerifier {
        drift_view: Some(VerificationPhase::Before),
        ..AcceptingVerifier::default()
    };
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let report = execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut before_verifier,
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Refused);
    assert_eq!(before_verifier.execute_calls, 0);
    assert_eq!(fs.source_mutations, 0);

    let mut after_verifier = AcceptingVerifier {
        drift_view: Some(VerificationPhase::AfterHealth),
        ..AcceptingVerifier::default()
    };
    let mut store = MemoryStore::default();
    let mut fs = MemoryFs::default();
    let report = execute(
        complex_in_place_prepared(),
        &mut store,
        &mut fs,
        &mut after_verifier,
        &mut NoFaults,
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(fs.steps.values().all(|state| *state == StepWorld::Before));

    let mut source_drift = AcceptingVerifier {
        drift_real: Some(VerificationPhase::SourceUnchanged),
        ..AcceptingVerifier::default()
    };
    let mut fs = MemoryFs::default();
    let error = execute(
        export_prepared(),
        &mut MemoryStore::default(),
        &mut fs,
        &mut source_drift,
        &mut NoFaults,
    )
    .unwrap_err();
    assert!(matches!(error, TransactionError::ThirdState(_)));
    assert!(fs.output.is_none() && fs.candidate.is_none());
}

#[test]
fn report_and_retire_store_failures_recover_only_in_the_durable_direction() {
    for mut store in [
        MemoryStore {
            fail_report_once: true,
            ..MemoryStore::default()
        },
        MemoryStore {
            fail_retire_once: true,
            ..MemoryStore::default()
        },
    ] {
        let mut fs = MemoryFs::default();
        execute(
            export_prepared(),
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut NoFaults,
        )
        .unwrap_err();
        assert!(matches!(
            store.pending.as_ref().unwrap().state,
            TransactionState::Verified
                | TransactionState::CleanupPending
                | TransactionState::Complete
        ));
        let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
        assert_eq!(report.outcome, Outcome::Verified);
        assert_eq!(report.cleanup, Cleanup::Complete);
        assert!(fs.output.is_some());
    }
}

#[test]
fn terminal_journal_with_embedded_report_precedes_stable_copy_and_retire() {
    let mut store = MemoryStore::default();
    let report = execute(
        export_prepared(),
        &mut store,
        &mut MemoryFs::default(),
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap();
    let terminal = store
        .events
        .iter()
        .rposition(|event| event == "journal/Complete")
        .unwrap();
    let stable = store
        .events
        .iter()
        .rposition(|event| event == "report")
        .unwrap();
    let retire = store
        .events
        .iter()
        .rposition(|event| event == "retire")
        .unwrap();
    assert!(terminal < stable && stable < retire);
    assert_eq!(store.reports.last(), Some(&report));
}

#[test]
fn journal_store_failure_after_candidate_staging_recovers_from_prior_durable_state() {
    let mut store = MemoryStore {
        fail_journal_state: Some(TransactionState::Candidate),
        ..MemoryStore::default()
    };
    let mut fs = MemoryFs::default();
    execute(
        export_prepared(),
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut NoFaults,
    )
    .unwrap_err();
    assert_eq!(
        store.pending.as_ref().unwrap().state,
        TransactionState::Prepared
    );
    assert!(fs.candidate.is_some());
    let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert!(fs.candidate.is_none() && fs.output.is_none());
}

#[test]
fn every_recovery_boundary_restarts_to_the_same_exact_rollback_report() {
    fn interrupted() -> (MemoryStore, MemoryFs) {
        let mut store = MemoryStore::default();
        let mut fs = MemoryFs::default();
        let mut crash = OneFault {
            target: Some(DurableBoundary::MutationCompleted {
                label: "in-place-step-3".into(),
            }),
            fired: false,
        };
        execute(
            complex_in_place_prepared(),
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut crash,
        )
        .unwrap_err();
        (store, fs)
    }

    let (mut store, mut fs) = interrupted();
    let mut trace = TraceFaults::default();
    Engine::new(
        &mut store,
        &mut fs,
        &mut AcceptingVerifier::default(),
        &mut trace,
    )
    .recover("stable-project-identity", "C:/source")
    .unwrap();
    let mut boundaries = trace.0;
    boundaries.sort_by_key(|boundary| format!("{boundary:?}"));
    boundaries.dedup();
    assert!(boundaries.iter().any(|boundary| matches!(
        boundary,
        DurableBoundary::StepRollbackIntentPersisted { .. }
    )));

    for target in boundaries {
        let (mut store, mut fs) = interrupted();
        let mut fault = OneFault {
            target: Some(target.clone()),
            fired: false,
        };
        let first = Engine::new(
            &mut store,
            &mut fs,
            &mut AcceptingVerifier::default(),
            &mut fault,
        )
        .recover("stable-project-identity", "C:/source");
        assert!(matches!(first, Err(TransactionError::FaultInjected(_))));
        assert!(fault.fired, "unreached recovery boundary {target:?}");
        let report = recover(&mut store, &mut fs, &mut AcceptingVerifier::default()).unwrap();
        assert_eq!(report.outcome, Outcome::RolledBack, "{target:?}");
        assert_eq!(report.cleanup, Cleanup::Complete, "{target:?}");
        assert!(fs.steps.values().all(|state| *state == StepWorld::Before));
        assert!(!fs.quarantine);
        assert!(store.pending.is_none());
    }
}

#[cfg(windows)]
#[test]
fn production_store_and_safefs_restart_recover_a_real_applied_export_step() {
    use std::fs;

    let scope = tempfile::tempdir().unwrap();
    let source = scope.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("kept.txt"), b"kept").unwrap();
    let output = scope.path().join("output");
    let state_root = scope.path().join("external-state");

    let source_project = vibe_safefs::Project::open(&source).unwrap();
    let project_identity = source_project.identity_token().unwrap();
    let key = project_key(&project_identity);
    let source_tree = observe_real_tree(&source).unwrap();
    let file = source_tree
        .entries
        .iter()
        .find(|entry| entry.path == "kept.txt")
        .unwrap()
        .clone();
    let output_parent = vibe_safefs::Project::open(scope.path()).unwrap();
    let output_slot = vibe_safefs::Project::pin_absent_path(&output).unwrap();
    let mut wire_plan: vibe_wire::generated::scrape::e1::plan::Plan =
        serde_json::from_slice(&canonical_plan(TransactionMode::Export)).unwrap();
    wire_plan.project.display_root = source.display().to_string();
    wire_plan.project.tree_digest = source_tree.digest.0.clone();
    let canonical = serde_json::to_vec(&wire_plan).unwrap();
    let snapshots = vec![
        Snapshot {
            kind: SnapshotKind::Contract,
            name: "contract".into(),
            bytes: b"contract".to_vec(),
            mode: None,
        },
        Snapshot {
            kind: SnapshotKind::CanonicalContract,
            name: "canonical-contract".into(),
            bytes: b"canonical-contract".to_vec(),
            mode: None,
        },
        Snapshot {
            kind: SnapshotKind::CanonicalPlan,
            name: "plan".into(),
            bytes: canonical.clone(),
            mode: None,
        },
        Snapshot {
            kind: SnapshotKind::Verifier,
            name: "verifier".into(),
            bytes: b"verifier".to_vec(),
            mode: None,
        },
    ];
    let prepared = PreparedTransaction {
        project_identity_token: project_identity.clone(),
        project_display_root: source.display().to_string(),
        plan_id: hash("plan"),
        canonical_plan: canonical,
        snapshots,
        mode: PreparedMode::Export(Box::new(ExportPlan {
            output_identity: output_slot.identity_token(),
            output_parent_identity: output_parent.identity_token().unwrap(),
            output_display_path: output.display().to_string(),
            output_name: "output".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![ExportEntry {
                target_path: "kept.txt".into(),
                kind: TreeEntryKind::File,
                mode: file.mode,
                payload: Some(ExportPayload::Source {
                    source_path: "kept.txt".into(),
                    before: FileState {
                        sha256: file.sha256.clone().unwrap(),
                        bytes: file.bytes.unwrap(),
                        mode: file.mode,
                    },
                }),
            }],
            source_tree: source_tree.clone(),
            final_manifest: source_tree.clone(),
        })),
    };

    let mut store = SystemTransactionStore::new(&state_root).unwrap();
    let mut filesystem = SafefsTransactionFilesystem::for_prepared(&prepared).unwrap();
    let mut verifier = RealTreeVerifier;
    let mut fault = OneFault {
        target: Some(DurableBoundary::OwnedTreeMutationBeforeReseal {
            label: "export-entry-0".into(),
        }),
        fired: false,
    };
    let error = Engine::new(&mut store, &mut filesystem, &mut verifier, &mut fault)
        .execute_locked(&project_identity, &source.display().to_string(), || {
            Ok(prepared)
        })
        .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    let transaction = store.pending(&key).unwrap().unwrap().transaction_id;
    drop(filesystem);
    drop(store);

    let mut restarted_store = SystemTransactionStore::new(&state_root).unwrap();
    let mut restarted_filesystem =
        SafefsTransactionFilesystem::open(&source, &project_identity).unwrap();
    let report = Engine::new(
        &mut restarted_store,
        &mut restarted_filesystem,
        &mut RealTreeVerifier,
        &mut NoFaults,
    )
    .recover(&project_identity, &source.display().to_string())
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert_eq!(fs::read(source.join("kept.txt")).unwrap(), b"kept");
    assert!(!output.exists());
    assert!(
        !scope
            .path()
            .join(format!(".vibe-scrape-candidate-{}", transaction.0))
            .exists()
    );
    let stable_report = state_root
        .join("reports")
        .join(format!("{}.json", transaction.0));
    let wire: serde_json::Value =
        serde_json::from_slice(&fs::read(stable_report).unwrap()).unwrap();
    assert_eq!(wire["outcome"], "rolled-back");
    assert!(
        !state_root
            .join("t")
            .join(&key.0)
            .join(&transaction.0)
            .exists()
    );
}

#[cfg(windows)]
#[test]
fn production_engine_recovers_an_export_file_staged_before_publication() {
    use std::fs;

    let scope = tempfile::tempdir().unwrap();
    let source = scope.path().join("source-stage");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("kept.txt"), b"kept").unwrap();
    let output = scope.path().join("output-stage");
    let state_root = scope.path().join("external-stage-state");

    let source_project = vibe_safefs::Project::open(&source).unwrap();
    let project_identity = source_project.identity_token().unwrap();
    let key = project_key(&project_identity);
    let source_tree = observe_real_tree(&source).unwrap();
    let file = source_tree
        .entries
        .iter()
        .find(|entry| entry.path == "kept.txt")
        .unwrap()
        .clone();
    let output_parent = vibe_safefs::Project::open(scope.path()).unwrap();
    let output_slot = vibe_safefs::Project::pin_absent_path(&output).unwrap();
    let mut wire_plan: vibe_wire::generated::scrape::e1::plan::Plan =
        serde_json::from_slice(&canonical_plan(TransactionMode::Export)).unwrap();
    wire_plan.project.display_root = source.display().to_string();
    wire_plan.project.tree_digest = source_tree.digest.0.clone();
    let canonical = serde_json::to_vec(&wire_plan).unwrap();
    let prepared = PreparedTransaction {
        project_identity_token: project_identity.clone(),
        project_display_root: source.display().to_string(),
        plan_id: hash("plan"),
        canonical_plan: canonical.clone(),
        snapshots: vec![
            Snapshot {
                kind: SnapshotKind::Contract,
                name: "contract".into(),
                bytes: b"contract".to_vec(),
                mode: None,
            },
            Snapshot {
                kind: SnapshotKind::CanonicalContract,
                name: "canonical-contract".into(),
                bytes: b"canonical-contract".to_vec(),
                mode: None,
            },
            Snapshot {
                kind: SnapshotKind::CanonicalPlan,
                name: "plan".into(),
                bytes: canonical,
                mode: None,
            },
            Snapshot {
                kind: SnapshotKind::Verifier,
                name: "verifier".into(),
                bytes: b"verifier".to_vec(),
                mode: None,
            },
        ],
        mode: PreparedMode::Export(Box::new(ExportPlan {
            output_identity: output_slot.identity_token(),
            output_parent_identity: output_parent.identity_token().unwrap(),
            output_display_path: output.display().to_string(),
            output_name: "output-stage".into(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![ExportEntry {
                target_path: "kept.txt".into(),
                kind: TreeEntryKind::File,
                mode: file.mode,
                payload: Some(ExportPayload::Source {
                    source_path: "kept.txt".into(),
                    before: FileState {
                        sha256: file.sha256.clone().unwrap(),
                        bytes: file.bytes.unwrap(),
                        mode: file.mode,
                    },
                }),
            }],
            source_tree: source_tree.clone(),
            final_manifest: source_tree,
        })),
    };

    let mut store = SystemTransactionStore::new(&state_root).unwrap();
    let mut filesystem = SafefsTransactionFilesystem::for_prepared(&prepared).unwrap();
    let mut fault = OneFault {
        target: Some(DurableBoundary::StepIntentPersisted {
            index: 0,
            id: "export/entry/0/kept.txt".into(),
        }),
        fired: false,
    };
    let error = Engine::new(
        &mut store,
        &mut filesystem,
        &mut RealTreeVerifier,
        &mut fault,
    )
    .execute_locked(&project_identity, &source.display().to_string(), || {
        Ok(prepared)
    })
    .unwrap_err();
    assert!(matches!(error, TransactionError::FaultInjected(_)));
    assert!(fault.fired);
    let journal = store.pending(&key).unwrap().unwrap();
    let transaction = journal.transaction_id.clone();
    let candidate = journal.candidate_name.clone().unwrap();
    let owner = journal.owned_tree_token.clone().unwrap();
    assert_eq!(journal.active_step, Some(0));
    drop(filesystem);

    let stage_name = super::safefs::transaction_stage_name(&owner, "export:kept.txt", "kept.txt");
    fs::write(scope.path().join(&candidate).join(&stage_name), b"kept").unwrap();
    drop(store);

    let mut restarted_store = SystemTransactionStore::new(&state_root).unwrap();
    let mut restarted_filesystem =
        SafefsTransactionFilesystem::open(&source, &project_identity).unwrap();
    let report = Engine::new(
        &mut restarted_store,
        &mut restarted_filesystem,
        &mut RealTreeVerifier,
        &mut NoFaults,
    )
    .recover(&project_identity, &source.display().to_string())
    .unwrap();
    assert_eq!(report.outcome, Outcome::RolledBack);
    assert_eq!(report.cleanup, Cleanup::Complete);
    assert_eq!(fs::read(source.join("kept.txt")).unwrap(), b"kept");
    assert!(!output.exists());
    assert!(!scope.path().join(candidate).exists());
    assert!(
        !state_root
            .join("t")
            .join(&key.0)
            .join(&transaction.0)
            .exists()
    );
    let stable_report = state_root
        .join("reports")
        .join(format!("{}.json", transaction.0));
    let wire: serde_json::Value =
        serde_json::from_slice(&fs::read(stable_report).unwrap()).unwrap();
    assert_eq!(wire["outcome"], "rolled-back");
}
