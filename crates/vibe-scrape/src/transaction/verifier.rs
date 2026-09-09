//! Transaction adapter over one already-prepared health plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

use super::model as tx;
use super::traits::TransactionVerifier;
use crate::health::{self, CheckState, HealthPhase, HealthStatus, HealthVerdict};
use vibe_wire::generated::scrape::e2::verification_health_evidence::ScrapeHealthTerminalState;

pub(crate) mod evidence;
mod phase;

use evidence::FailureEvidence;
use phase::{
    before_accepted, create_phase_directory, materialize_exact_tree, observe, proof_evidence, seal,
};

pub struct PreparedHealthVerifier {
    prepared: health::PreparedHealth,
    backend: health::LocalProcessBackend,
    before: Option<health::PhaseHealthResult>,
    before_view: Option<PhaseDirectory>,
    after_view: Option<PhaseDirectory>,
}

struct PhaseDirectory {
    path: PathBuf,
    _capability: vibe_safefs::Pinned,
}

fn rejected_health_evidence(
    prepared: &health::PreparedHealth,
    failure: evidence::FailureEvidence,
) -> Result<tx::VerificationEvidence, tx::TransactionError> {
    let canonical_evidence = evidence::failure_bytes(prepared, &failure)?;
    Ok(tx::VerificationEvidence {
        accepted: false,
        assurance: tx::Assurance::Reduced,
        summary: failure.message,
        canonical_evidence,
    })
}

/// Recovery adapter for journals discovered before their verifier snapshots
/// became durable. `Preparing` recovery is defined to settle without invoking
/// health; every verifier call in the unavailable variant fails closed.
pub enum RecoveryHealthVerifier {
    Available(Box<PreparedHealthVerifier>),
    Unavailable { detail: String },
}

impl RecoveryHealthVerifier {
    #[must_use]
    pub fn available(verifier: PreparedHealthVerifier) -> Self {
        Self::Available(Box::new(verifier))
    }

    #[must_use]
    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self::Unavailable {
            detail: detail.into(),
        }
    }

    fn unavailable_error(detail: &str) -> tx::TransactionError {
        tx::TransactionError::Verification(format!(
            "recovery requires a durable prepared-health snapshot: {detail}"
        ))
    }
}

impl PreparedHealthVerifier {
    #[must_use]
    pub fn new(prepared: health::PreparedHealth) -> Self {
        Self {
            prepared,
            backend: health::LocalProcessBackend::new(),
            before: None,
            before_view: None,
            after_view: None,
        }
    }

    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, tx::TransactionError> {
        let prepared = health::snapshot_from_bytes(bytes).map_err(|error| {
            tx::TransactionError::Verification(format!(
                "decoding sealed prepared health snapshot: {error}"
            ))
        })?;
        Ok(Self::new(prepared))
    }

    pub fn from_journal_snapshots<Read>(
        health_bytes: &[u8],
        journal: &tx::Journal,
        read: Read,
    ) -> Result<Self, tx::TransactionError>
    where
        Read: FnMut(&str) -> Result<Vec<u8>, tx::TransactionError>,
    {
        Self::from_snapshot_records(health_bytes, &journal.snapshots, read)
    }

    fn from_snapshot_records<Read>(
        health_bytes: &[u8],
        snapshots: &[tx::SnapshotRecord],
        mut read: Read,
    ) -> Result<Self, tx::TransactionError>
    where
        Read: FnMut(&str) -> Result<Vec<u8>, tx::TransactionError>,
    {
        let mut verifier = Self::from_snapshot(health_bytes)?;
        for check in &mut verifier.prepared.checks {
            let Some(bundle) = &mut check.custom_bundle else {
                continue;
            };
            for entry in &mut bundle.entries {
                if entry.kind != health::BundleEntryKind::File {
                    continue;
                }
                let name = format!("verifier/{}/{}", check.id, entry.path);
                let record = snapshots
                    .iter()
                    .find(|record| record.name == name)
                    .ok_or_else(|| {
                        tx::TransactionError::Verification(format!(
                            "custom verifier snapshot `{name}` is not journaled"
                        ))
                    })?;
                if record.kind != tx::SnapshotKind::Verifier {
                    return Err(tx::TransactionError::Verification(format!(
                        "custom verifier snapshot `{name}` differs from sealed bundle"
                    )));
                }
                let bytes = read(&name)?;
                let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
                if entry.sha256.as_deref() != Some(digest.as_str())
                    || entry.bytes != Some(bytes.len() as u64)
                    || entry.mode != record.mode
                    || record.sha256.0 != digest
                    || record.bytes != bytes.len() as u64
                {
                    return Err(tx::TransactionError::Verification(format!(
                        "custom verifier snapshot `{name}` differs from sealed bundle"
                    )));
                }
                entry.content = Some(bytes);
            }
        }
        Ok(verifier)
    }

    fn materialize_before(
        &mut self,
        workspace: &tx::VerificationWorkspace,
        source: &str,
        expected: &tx::TreeManifest,
    ) -> Result<PathBuf, tx::TransactionError> {
        if let Some(path) = &self.before_view {
            return Ok(path.path.clone());
        }
        let directory = create_phase_directory(workspace, "view", "before")?;
        materialize_exact_tree(&directory.path, source, expected)?;
        let root = directory.path.clone();
        self.before_view = Some(directory);
        Ok(root)
    }

    fn materialize_after(
        &mut self,
        workspace: &tx::VerificationWorkspace,
        source: &str,
        expected: &tx::TreeManifest,
    ) -> Result<PathBuf, tx::TransactionError> {
        if let Some(path) = &self.after_view {
            return Ok(path.path.clone());
        }
        let directory = create_phase_directory(workspace, "view", "after")?;
        materialize_exact_tree(&directory.path, source, expected)?;
        let root = directory.path.clone();
        self.after_view = Some(directory);
        Ok(root)
    }
}

impl TransactionVerifier for PreparedHealthVerifier {
    fn release_verification_workspace(&mut self) {
        self.before_view = None;
        self.after_view = None;
    }

    fn observe_phase_view(
        &mut self,
        _journal: &tx::Journal,
        context: &tx::VerificationContext<'_>,
    ) -> Result<tx::TreeManifest, tx::TransactionError> {
        let root = match context.phase {
            tx::VerificationPhase::Before => self.materialize_before(
                context.workspace,
                context.root_display,
                context.expected_tree,
            )?,
            tx::VerificationPhase::AfterHealth => self.materialize_after(
                context.workspace,
                context.root_display,
                context.expected_tree,
            )?,
            _ => PathBuf::from(context.root_display),
        };
        observe(&root)
    }

    fn execute_verification(
        &mut self,
        _journal: &tx::Journal,
        context: tx::VerificationContext<'_>,
    ) -> Result<tx::VerificationEvidence, tx::TransactionError> {
        if !matches!(
            context.phase,
            tx::VerificationPhase::Before | tx::VerificationPhase::AfterHealth
        ) {
            return Ok(proof_evidence(context.phase, context.expected_tree));
        }
        let before = context.phase == tx::VerificationPhase::Before;
        let phase_directory = if before {
            self.before_view.take().ok_or_else(|| {
                tx::TransactionError::Verification("before view was not observed".into())
            })?
        } else {
            self.after_view.take().ok_or_else(|| {
                tx::TransactionError::Verification("after view was not observed".into())
            })?
        };
        let root = phase_directory.path.clone();
        let scratch_directory = create_phase_directory(
            context.workspace,
            "scratch",
            if before { "before" } else { "after" },
        )?;
        let scratch = scratch_directory.path.clone();
        let phase = if before {
            HealthPhase::Before
        } else {
            HealthPhase::After
        };
        let mut result = match health::run_phase(
            &mut self.backend,
            &self.prepared,
            &health::PhaseContext {
                phase,
                root: root.display().to_string(),
                protected_root: context.root_display.to_owned(),
                scratch: scratch.display().to_string(),
                result: scratch.join("result").display().to_string(),
                same_display_path_required: context.same_display_path_required,
                transactional_tree_reproof: false,
                expected_tree: seal(context.expected_tree),
                cancellation: health::CancellationToken::new(),
            },
        ) {
            Ok(result) => result,
            Err(health::HealthError::CommandFailed {
                check_id,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                return rejected_health_evidence(
                    &self.prepared,
                    FailureEvidence {
                        phase,
                        check_id,
                        terminal: ScrapeHealthTerminalState::ExecutionFailed,
                        execution: Some(*execution),
                        prior_executions,
                        prior_checks,
                        message: "health command returned an unaccepted exit code".to_owned(),
                    },
                );
            }
            Err(health::HealthError::CommandChangedTree {
                check_id,
                detail,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                return rejected_health_evidence(
                    &self.prepared,
                    FailureEvidence {
                        phase,
                        check_id,
                        terminal: ScrapeHealthTerminalState::ExecutionFailed,
                        execution: Some(*execution),
                        prior_executions,
                        prior_checks,
                        message: detail,
                    },
                );
            }
            Err(health::HealthError::TimedOut {
                check_id,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                return rejected_health_evidence(
                    &self.prepared,
                    FailureEvidence {
                        phase,
                        check_id,
                        terminal: ScrapeHealthTerminalState::TimedOut,
                        execution: Some(*execution),
                        prior_executions,
                        prior_checks,
                        message: "health command exceeded its sealed timeout".to_owned(),
                    },
                );
            }
            Err(health::HealthError::Cancelled {
                check_id,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                return rejected_health_evidence(
                    &self.prepared,
                    FailureEvidence {
                        phase,
                        check_id,
                        terminal: ScrapeHealthTerminalState::Cancelled,
                        execution: Some(*execution),
                        prior_executions,
                        prior_checks,
                        message: "health command was cancelled and its process tree terminated"
                            .to_owned(),
                    },
                );
            }
            Err(health::HealthError::CheckProtocolFailed {
                check_id,
                detail,
                prior_checks,
                mut executions,
            }) => {
                let execution = executions.pop();
                return rejected_health_evidence(
                    &self.prepared,
                    FailureEvidence {
                        phase,
                        check_id,
                        terminal: ScrapeHealthTerminalState::ExecutionFailed,
                        execution,
                        prior_executions: executions,
                        prior_checks,
                        message: detail,
                    },
                );
            }
            Err(error) => {
                return rejected_health_evidence(
                    &self.prepared,
                    FailureEvidence {
                        phase,
                        check_id: "health-panel".to_owned(),
                        terminal: ScrapeHealthTerminalState::ExecutionFailed,
                        execution: None,
                        prior_executions: Vec::new(),
                        prior_checks: Vec::new(),
                        message: error.to_string(),
                    },
                );
            }
        };
        if !before {
            let after_view = observe(&root)?;
            if after_view != *context.expected_tree {
                return Err(tx::TransactionError::Verification(
                    "after-health command changed its isolated final-tree copy".to_owned(),
                ));
            }
        }
        // Both phases execute in a different-path exact copy. This protects
        // the delivered/project tree but cannot claim same-path/full COW
        // assurance, even when every health verdict passes.
        result.assurance_reduced = true;
        let (accepted, assurance) = if before {
            let accepted = before_accepted(self.prepared.baseline, &result);
            self.before = Some(result.clone());
            (accepted, result.assurance_reduced)
        } else {
            let before_result = self.before.as_ref().ok_or_else(|| {
                tx::TransactionError::Verification(
                    "after health has no sealed before result".into(),
                )
            })?;
            match health::judge(self.prepared.baseline, before_result, &result) {
                health::BaselineDecision::AcceptFull => (true, false),
                health::BaselineDecision::AcceptReduced => (true, true),
                health::BaselineDecision::RefuseBefore
                | health::BaselineDecision::RollbackAfter => (false, true),
            }
        };
        let canonical_evidence = evidence::phase_bytes(&self.prepared, &result)?;
        Ok(tx::VerificationEvidence {
            accepted,
            assurance: if assurance {
                tx::Assurance::Reduced
            } else {
                tx::Assurance::Full
            },
            summary: format!("{} healthcheck(s) completed", result.checks.len()),
            canonical_evidence,
        })
    }

    fn reprove_real_tree(
        &mut self,
        _journal: &tx::Journal,
        _root_kind: tx::VerificationRootKind,
        root_display: &str,
    ) -> Result<tx::TreeManifest, tx::TransactionError> {
        observe(Path::new(root_display))
    }
}

impl TransactionVerifier for RecoveryHealthVerifier {
    fn release_verification_workspace(&mut self) {
        if let Self::Available(verifier) = self {
            verifier.release_verification_workspace();
        }
    }

    fn observe_phase_view(
        &mut self,
        journal: &tx::Journal,
        context: &tx::VerificationContext<'_>,
    ) -> Result<tx::TreeManifest, tx::TransactionError> {
        match self {
            Self::Available(verifier) => verifier.observe_phase_view(journal, context),
            Self::Unavailable { detail } => Err(Self::unavailable_error(detail)),
        }
    }

    fn execute_verification(
        &mut self,
        journal: &tx::Journal,
        context: tx::VerificationContext<'_>,
    ) -> Result<tx::VerificationEvidence, tx::TransactionError> {
        match self {
            Self::Available(verifier) => verifier.execute_verification(journal, context),
            Self::Unavailable { detail } => Err(Self::unavailable_error(detail)),
        }
    }

    fn reprove_real_tree(
        &mut self,
        journal: &tx::Journal,
        root_kind: tx::VerificationRootKind,
        root_display: &str,
    ) -> Result<tx::TreeManifest, tx::TransactionError> {
        match self {
            Self::Available(verifier) => {
                verifier.reprove_real_tree(journal, root_kind, root_display)
            }
            Self::Unavailable { detail } => Err(Self::unavailable_error(detail)),
        }
    }
}

#[cfg(test)]
#[path = "verifier/tests.rs"]
mod tests;
