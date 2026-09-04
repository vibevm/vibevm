//! Transaction adapter over one already-prepared health plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

use super::model as tx;
use super::traits::TransactionVerifier;
use crate::health::{self, CheckState, HealthPhase, HealthStatus, HealthVerdict};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HealthFailureEvidence {
    pub phase: HealthPhase,
    pub check_id: String,
    pub terminal: String,
    pub execution: Option<health::CommandExecution>,
    #[serde(default)]
    pub prior_executions: Vec<health::CommandExecution>,
    #[serde(default)]
    pub prior_checks: Vec<health::CheckResult>,
    pub message: String,
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
        let prepared = serde_json::from_slice(bytes).map_err(|error| {
            tx::TransactionError::Verification(format!(
                "decoding sealed prepared health snapshot: {error}"
            ))
        })?;
        Ok(Self::new(prepared))
    }

    pub fn from_journal_snapshots<Read>(
        health_bytes: &[u8],
        journal: &tx::Journal,
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
                let record = journal
                    .snapshots
                    .iter()
                    .find(|record| record.name == name)
                    .ok_or_else(|| {
                        tx::TransactionError::Verification(format!(
                            "custom verifier snapshot `{name}` is not journaled"
                        ))
                    })?;
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
                let failure = HealthFailureEvidence {
                    phase,
                    check_id,
                    terminal: "execution-failed".to_owned(),
                    execution: Some(*execution),
                    prior_executions,
                    prior_checks,
                    message: "health command returned an unaccepted exit code".to_owned(),
                };
                return Ok(tx::VerificationEvidence {
                    accepted: false,
                    assurance: tx::Assurance::Reduced,
                    summary: failure.message.clone(),
                    canonical_evidence: serde_json::to_vec(&failure)
                        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?,
                });
            }
            Err(health::HealthError::CommandChangedTree {
                check_id,
                detail,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                let failure = HealthFailureEvidence {
                    phase,
                    check_id,
                    terminal: "execution-failed".to_owned(),
                    execution: Some(*execution),
                    prior_executions,
                    prior_checks,
                    message: detail,
                };
                return Ok(tx::VerificationEvidence {
                    accepted: false,
                    assurance: tx::Assurance::Reduced,
                    summary: failure.message.clone(),
                    canonical_evidence: serde_json::to_vec(&failure)
                        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?,
                });
            }
            Err(health::HealthError::TimedOut {
                check_id,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                let failure = HealthFailureEvidence {
                    phase,
                    check_id,
                    terminal: "timed-out".to_owned(),
                    execution: Some(*execution),
                    prior_executions,
                    prior_checks,
                    message: "health command exceeded its sealed timeout".to_owned(),
                };
                return Ok(tx::VerificationEvidence {
                    accepted: false,
                    assurance: tx::Assurance::Reduced,
                    summary: failure.message.clone(),
                    canonical_evidence: serde_json::to_vec(&failure)
                        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?,
                });
            }
            Err(health::HealthError::Cancelled {
                check_id,
                prior_checks,
                prior_executions,
                execution,
                ..
            }) => {
                let failure = HealthFailureEvidence {
                    phase,
                    check_id,
                    terminal: "cancelled".to_owned(),
                    execution: Some(*execution),
                    prior_executions,
                    prior_checks,
                    message: "health command was cancelled and its process tree terminated"
                        .to_owned(),
                };
                return Ok(tx::VerificationEvidence {
                    accepted: false,
                    assurance: tx::Assurance::Reduced,
                    summary: failure.message.clone(),
                    canonical_evidence: serde_json::to_vec(&failure)
                        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?,
                });
            }
            Err(health::HealthError::CheckProtocolFailed {
                check_id,
                detail,
                prior_checks,
                mut executions,
            }) => {
                let execution = executions.pop();
                let failure = HealthFailureEvidence {
                    phase,
                    check_id,
                    terminal: "execution-failed".to_owned(),
                    execution,
                    prior_executions: executions,
                    prior_checks,
                    message: detail,
                };
                return Ok(tx::VerificationEvidence {
                    accepted: false,
                    assurance: tx::Assurance::Reduced,
                    summary: failure.message.clone(),
                    canonical_evidence: serde_json::to_vec(&failure)
                        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?,
                });
            }
            Err(error) => {
                let failure = HealthFailureEvidence {
                    phase,
                    check_id: "health-panel".to_owned(),
                    terminal: "execution-failed".to_owned(),
                    execution: None,
                    prior_executions: Vec::new(),
                    prior_checks: Vec::new(),
                    message: error.to_string(),
                };
                return Ok(tx::VerificationEvidence {
                    accepted: false,
                    assurance: tx::Assurance::Reduced,
                    summary: failure.message.clone(),
                    canonical_evidence: serde_json::to_vec(&failure)
                        .map_err(|encode| tx::TransactionError::Verification(encode.to_string()))?,
                });
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
        let canonical_evidence = serde_json::to_vec(&result)
            .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
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

fn create_phase_directory(
    workspace: &tx::VerificationWorkspace,
    parent: &str,
    phase: &str,
) -> Result<PhaseDirectory, tx::TransactionError> {
    let project =
        vibe_safefs::Project::open(Path::new(&workspace.intent.display_root)).map_err(|error| {
            tx::TransactionError::Verification(format!(
                "opening journaled verification workspace: {error:#}"
            ))
        })?;
    let actual = project.identity_token().map_err(|error| {
        tx::TransactionError::Verification(format!(
            "identifying journaled verification workspace: {error:#}"
        ))
    })?;
    if actual != workspace.project_identity_token {
        return Err(tx::TransactionError::ThirdState(
            "verification workspace identity differs from the journal".to_owned(),
        ));
    }
    let root = project.root_dir().map_err(|error| {
        tx::TransactionError::Verification(format!("pinning verification workspace: {error:#}"))
    })?;
    let name = match (parent, phase) {
        ("view", "before") => "vb",
        ("view", "after") => "va",
        ("scratch", "before") => "sb",
        ("scratch", "after") => "sa",
        _ => {
            return Err(tx::TransactionError::Verification(
                "unknown verification workspace phase".to_owned(),
            ));
        }
    };
    let (directory, durability) = root
        .create_child_exclusive_journaled(name)
        .map_err(|error| {
            tx::TransactionError::Verification(format!(
                "exclusively creating verification phase directory: {error}"
            ))
        })?;
    if !matches!(
        durability,
        vibe_safefs::DirectoryDurability::Synced
            | vibe_safefs::DirectoryDurability::JournalRecoverable
    ) {
        return Err(tx::TransactionError::Verification(format!(
            "verification phase directory lacks namespace recovery evidence: {durability:?}"
        )));
    }
    Ok(PhaseDirectory {
        path: directory.path().to_path_buf(),
        _capability: directory,
    })
}

fn materialize_exact_tree(
    destination_root: &Path,
    source_root: &str,
    expected: &tx::TreeManifest,
) -> Result<(), tx::TransactionError> {
    let source = vibe_safefs::Project::open(Path::new(source_root))
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    let destination = vibe_safefs::Project::open(destination_root)
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    for entry in &expected.entries {
        match entry.kind {
            tx::TreeEntryKind::Directory => {
                let components = entry.path.split('/').collect::<Vec<_>>();
                destination
                    .dir(&components, true)
                    .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
            }
            tx::TreeEntryKind::File => {
                let digest = entry.sha256.as_ref().ok_or_else(|| {
                    tx::TransactionError::Verification("sealed file has no digest".into())
                })?;
                source
                    .copy_stable_file_to_expected(
                        &entry.path,
                        &destination,
                        &entry.path,
                        entry.mode,
                        digest.0.strip_prefix("sha256:").unwrap_or(&digest.0),
                        entry.bytes.unwrap_or(0),
                    )
                    .map_err(|error| {
                        tx::TransactionError::Verification(format!(
                            "materializing `{}`: {error}",
                            entry.path
                        ))
                    })?;
            }
        }
    }
    Ok(())
}

fn before_accepted(policy: health::BaselinePolicy, result: &health::PhaseHealthResult) -> bool {
    match policy {
        health::BaselinePolicy::Strict => result.checks.iter().all(|check| match &check.state {
            CheckState::Skipped { .. } | CheckState::Completed(HealthVerdict::Pass) => true,
            CheckState::Completed(HealthVerdict::Structured(value)) => {
                value.status == HealthStatus::Pass
            }
        }),
        health::BaselinePolicy::NoRegression => result.checks.iter().any(|check| {
            matches!(
                check.state,
                CheckState::Completed(HealthVerdict::Structured(_))
            )
        }),
    }
}

fn proof_evidence(
    phase: tx::VerificationPhase,
    tree: &tx::TreeManifest,
) -> tx::VerificationEvidence {
    let canonical_evidence =
        format!("tree-proof/e1\nphase={phase:?}\ntree={}\n", tree.digest.0).into_bytes();
    tx::VerificationEvidence {
        accepted: true,
        assurance: tx::Assurance::Full,
        summary: format!("sealed tree proof accepted for {phase:?}"),
        canonical_evidence,
    }
}

fn seal(tree: &tx::TreeManifest) -> health::tree::TreeSeal {
    health::tree::TreeSeal {
        tree_digest: tree.digest.0.clone(),
        entries: tree
            .entries
            .iter()
            .map(|entry| health::tree::TreeSealEntry {
                path: entry.path.clone(),
                kind: match entry.kind {
                    tx::TreeEntryKind::File => health::tree::TreeEntryKind::File,
                    tx::TreeEntryKind::Directory => health::tree::TreeEntryKind::Directory,
                },
                sha256: entry.sha256.as_ref().map(|digest| digest.0.clone()),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    }
}

fn observe(root: &Path) -> Result<tx::TreeManifest, tx::TransactionError> {
    let seal = health::tree::observe(root)
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    Ok(tx::TreeManifest {
        digest: tx::Digest(seal.tree_digest),
        entries: seal
            .entries
            .into_iter()
            .map(|entry| tx::TreeEntry {
                path: entry.path,
                kind: match entry.kind {
                    health::tree::TreeEntryKind::File => tx::TreeEntryKind::File,
                    health::tree::TreeEntryKind::Directory => tx::TreeEntryKind::Directory,
                },
                sha256: entry.sha256.map(tx::Digest),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    })
}
