//! Cooperative same-host current policy; adapters do not yet consume this owner.
//!
//! The embedding supplies scope, source authority and facts. Source identifiers
//! are diagnostic identities, never secrets or wire-body grants. Root tokens
//! detect ordinary same-host replacement only: identity reuse (ABA), cold
//! deletion and rollback cannot be ruled out by this record. Initialization is
//! an explicit trusted action, never an automatic question-preparation step.
//! Publication is atomic and checked against the current record. Unsupported
//! directory flushes and JournalRecoverable checkpoints are accepted with a
//! contextual warning and weaker power-loss guarantees; this policy file has
//! no durable intent recovery or antirollback proof. A read-only snapshot does
//! not establish that an earlier publication was fsynced.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");
// Owner-bound policy observations: req r1.

use super::HostPolicyFacts;
use std::sync::Arc;
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::{DirectoryDurability, DurableWrite, Project, PublishStage};

mod record;
use record::Record;
#[cfg(test)]
mod tests;

/// Bounded identity supplied explicitly by trusted host configuration.
/// Possession of a string confers no authority. Never construct from answers
/// or operation bodies; the embedding already holds the source-authority grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPolicySource(String);
impl TrustedPolicySource {
    pub fn from_host_identifier(identifier: String) -> Result<Self, PolicyError> {
        record::validate_text(&identifier)?;
        Ok(Self(identifier))
    }
    pub fn identifier(&self) -> &str {
        &self.0
    }
}

/// Immutable observation, constructible only by validated owner reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySnapshot {
    generation: String,
    facts: HostPolicyFacts,
    source: TrustedPolicySource,
}
impl PolicySnapshot {
    pub fn generation(&self) -> &str {
        &self.generation
    }
    pub fn facts(&self) -> &HostPolicyFacts {
        &self.facts
    }
    pub fn source(&self) -> &TrustedPolicySource {
        &self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    Unavailable,
    InvalidRecord(String),
    ForeignScope,
    StartupDisagreement,
    StaleGeneration,
    RevisionOverflow,
    Infrastructure(String),
    /// May be visible, but durability or post-publication validation failed.
    /// This outcome can never serve as a successful policy grant.
    UncertainPublication(String),
}
impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trusted policy owner: {self:?}")
    }
}
impl std::error::Error for PolicyError {}
fn infrastructure(error: impl std::fmt::Display) -> PolicyError {
    PolicyError::Infrastructure(format!("{error:#}"))
}

/// Assess every actual metadata checkpoint, including newly created parents.
/// Weak platform support is distinct from failed or absent flush evidence.
fn assess_checkpoints(
    write: &DurableWrite,
    record_parent: &std::path::Path,
) -> Result<Vec<String>, PolicyError> {
    let uncertain = |reason: &str| PolicyError::UncertainPublication(reason.into());
    if !write.file_synced {
        return Err(uncertain("file sync evidence is false"));
    }
    let mut expected = Vec::new();
    for directory in &write.published.created_directories {
        expected.push(
            directory
                .parent()
                .ok_or_else(|| uncertain("created directory has no parent"))?,
        );
    }
    expected.push(record_parent);
    if write.directory_syncs.len() != expected.len()
        || write
            .directory_syncs
            .iter()
            .zip(expected)
            .any(|(sync, path)| sync.directory != path)
        || write.directory_syncs.last().map(|sync| sync.durability) != Some(write.parent)
    {
        return Err(uncertain(
            "missing or inconsistent directory checkpoint evidence",
        ));
    }
    let mut warnings = Vec::new();
    for sync in &write.directory_syncs {
        match sync.durability {
            DirectoryDurability::Synced => {}
            DirectoryDurability::JournalRecoverable | DirectoryDurability::Unsupported(_) => {
                warnings.push(format!(
                    "directory={} kind={:?}",
                    sync.directory.display(),
                    sync.durability
                ));
            }
            DirectoryDurability::Failed(kind) => {
                return Err(PolicyError::UncertainPublication(format!(
                    "directory flush failed: directory={} kind={kind:?}",
                    sync.directory.display()
                )));
            }
        }
    }
    Ok(warnings)
}

/// Host-only retained root and immutable exact scope. No method acquires a
/// lease, mutex or retry loop. Mutations use the caller's incumbent outer lease.
pub struct PolicyOwner {
    project: Arc<Project>,
    scope: SubmissionScope,
    root_token: String,
    path: String,
}
impl PolicyOwner {
    /// Immutable trusted scope; comparison data does not grant admission.
    pub fn scope(&self) -> &SubmissionScope {
        &self.scope
    }

    fn pin(project: Arc<Project>, scope: SubmissionScope) -> Result<Self, PolicyError> {
        record::validate_scope(&scope)?;
        let root_token = project.root_correlation_token().map_err(infrastructure)?;
        let path = record::path(&scope);
        let owner = Self {
            project,
            scope,
            root_token,
            path,
        };
        owner.check_root(None)?;
        Ok(owner)
    }

    /// Validate existing state without creating infrastructure or minting a lease.
    pub fn attach_readonly(
        project: Arc<Project>,
        scope: SubmissionScope,
    ) -> Result<Self, PolicyError> {
        let owner = Self::pin(project, scope)?;
        let observed = owner.read(&owner.project);
        owner.check_root(None)?;
        observed?.ok_or(PolicyError::Unavailable)?;
        Ok(owner)
    }

    /// Observe the bounded current record without acquiring a mutation lease or
    /// creating infrastructure. This snapshot is comparison data, never an
    /// admission grant: admission must reread under its incumbent writer lease.
    pub fn observe_readonly(&self) -> Result<PolicySnapshot, PolicyError> {
        self.check_root(None)?;
        let observed = self.read(&self.project);
        self.check_root(None)?;
        Ok(observed?.ok_or(PolicyError::Unavailable)?.snapshot())
    }

    /// Explicit trusted startup. Existing identical source/facts is idempotent;
    /// disagreement or invalid state refuses without overwriting. No scratch
    /// allocation and no nested lease acquisition occur.
    pub fn initialize_under_lease(
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        source: TrustedPolicySource,
        facts: HostPolicyFacts,
    ) -> Result<PolicySnapshot, PolicyError> {
        let owner = Self::pin(
            Arc::new(Project::open(lease.root()).map_err(infrastructure)?),
            scope.clone(),
        )?;
        owner.check_root(Some(lease))?;
        let project = lease.publication_journal_project();
        let observed = owner.read(project);
        owner.check_root(Some(lease))?;
        if let Some(record) = observed? {
            if record.source != source.0 || record.facts.host() != facts {
                return Err(PolicyError::StartupDisagreement);
            }
            return Ok(record.snapshot());
        }
        owner.check_root(Some(lease))?;
        let candidate = Record::new(&owner.scope, &owner.root_token, source, facts, lease.root())?;
        owner.publish(lease, &candidate)
    }

    pub fn current_under_lease(
        &self,
        lease: &LifecycleLease,
    ) -> Result<PolicySnapshot, PolicyError> {
        self.check_root(Some(lease))?;
        let observed = self.read(lease.publication_journal_project());
        self.check_root(Some(lease))?;
        let record = observed?.ok_or(PolicyError::Unavailable)?;
        Ok(record.snapshot())
    }

    /// Compare the freshly read generation under the incumbent lease. Explicit
    /// updates advance revision even when source and facts remain identical.
    pub fn update_under_lease(
        &self,
        lease: &LifecycleLease,
        expected_generation: &str,
        source: TrustedPolicySource,
        facts: HostPolicyFacts,
    ) -> Result<PolicySnapshot, PolicyError> {
        self.check_root(Some(lease))?;
        let observed = self.read(lease.publication_journal_project());
        self.check_root(Some(lease))?;
        let mut candidate = observed?.ok_or(PolicyError::Unavailable)?;
        if candidate.snapshot().generation() != expected_generation {
            return Err(PolicyError::StaleGeneration);
        }
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or(PolicyError::RevisionOverflow)?;
        candidate.source = source.0;
        candidate.facts = record::Facts::from_host(facts);
        self.publish(lease, &candidate)
    }

    fn check_root(&self, lease: Option<&LifecycleLease>) -> Result<(), PolicyError> {
        if self
            .project
            .root_correlation_token()
            .map_err(infrastructure)?
            != self.root_token
            || Project::open(self.project.root_path())
                .map_err(infrastructure)?
                .root_correlation_token()
                .map_err(infrastructure)?
                != self.root_token
        {
            return Err(PolicyError::ForeignScope);
        }
        if let Some(lease) = lease {
            if lease.root() != self.project.root_path()
                || lease
                    .publication_journal_project()
                    .root_correlation_token()
                    .map_err(infrastructure)?
                    != self.root_token
            {
                return Err(PolicyError::ForeignScope);
            }
        }
        Ok(())
    }

    fn read(&self, project: &Project) -> Result<Option<Record>, PolicyError> {
        Ok(self.read_encoded(project)?.map(|(record, _)| record))
    }

    fn read_encoded(&self, project: &Project) -> Result<Option<(Record, Vec<u8>)>, PolicyError> {
        let Some(bytes) = project
            .read_file_bounded(&self.path, record::CAP)
            .map_err(infrastructure)?
        else {
            return Ok(None);
        };
        let record: Record = serde_json::from_slice(&bytes)
            .map_err(|e| PolicyError::InvalidRecord(e.to_string()))?;
        record.validate(&self.scope, &self.root_token)?;
        Ok(Some((record, bytes)))
    }

    fn publish(
        &self,
        lease: &LifecycleLease,
        candidate: &Record,
    ) -> Result<PolicySnapshot, PolicyError> {
        candidate.validate(&self.scope, &self.root_token)?;
        let bytes = serde_json::to_vec(candidate).map_err(infrastructure)?;
        if bytes.len() > record::CAP {
            return Err(PolicyError::InvalidRecord(
                "candidate exceeds record cap".into(),
            ));
        }
        self.check_root(Some(lease))?;
        let project = lease.publication_journal_project();
        let pinned = project.root_dir().map_err(infrastructure)?;
        let durable = match project.write_atomic_durable_in(&pinned, &self.path, &bytes) {
            Ok(durable) => durable,
            Err(error) => {
                let possibly = error.stage == PublishStage::PossiblyPublished;
                let reason = format!("{:#}", error.into_report());
                // A visible readback must never erase a durability failure.
                return Err(if possibly {
                    PolicyError::UncertainPublication(reason)
                } else {
                    PolicyError::Infrastructure(reason)
                });
            }
        };
        let absolute_record = self.project.root_path().join(&self.path);
        let warnings =
            assess_checkpoints(&durable, absolute_record.parent().expect("record parent"))
                .map_err(|error| {
                    PolicyError::UncertainPublication(format!(
                        "record={} root={}: {error}",
                        self.path,
                        self.project.root_path().display()
                    ))
                })?;
        for warning in warnings {
            eprintln!(
                "trusted policy owner: record={} root={} {warning}; atomic current record accepted with weaker power-loss/rollback guarantees; no policy intent recovery",
                self.path,
                self.project.root_path().display()
            );
        }
        let verified = (|| {
            self.check_root(Some(lease))?;
            let readback = self.read_encoded(project);
            self.check_root(Some(lease))?;
            let (observed, observed_bytes) = readback?.ok_or(PolicyError::Unavailable)?;
            if observed != *candidate || observed_bytes != bytes {
                return Err(PolicyError::InvalidRecord(
                    "published readback differs".into(),
                ));
            }
            Ok(observed.snapshot())
        })();
        verified.map_err(|e: PolicyError| PolicyError::UncertainPublication(e.to_string()))
    }
}
