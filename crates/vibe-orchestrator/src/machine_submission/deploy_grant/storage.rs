//! SafeFS publication and held-root checks for the durable Deploy pointer.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH-DECISION"
);

use super::*;
use vibe_safefs::{DirectoryDurability, DurableWrite, PublishStage};

impl DeployGrantService {
    pub(super) fn lease(&self) -> Result<LifecycleLease, DeployGrantError> {
        self.check_roots(None)?;
        let lease =
            LifecycleLease::acquire(self.project.root_path()).map_err(DeployGrantError::Lease)?;
        self.check_roots(Some(&lease))?;
        Ok(lease)
    }
    pub(super) fn check_roots(
        &self,
        lease: Option<&LifecycleLease>,
    ) -> Result<(), DeployGrantError> {
        for (root, token) in [
            (&self.project, &self.root_tokens[0]),
            (&self.settings, &self.root_tokens[1]),
            (&self.home, &self.root_tokens[2]),
        ] {
            if root.root_correlation_token().map_err(infrastructure)? != *token
                || Project::open(root.root_path())
                    .map_err(infrastructure)?
                    .root_correlation_token()
                    .map_err(infrastructure)?
                    != *token
            {
                return Err(DeployGrantError::ForeignBinding);
            }
        }
        if let Some(lease) = lease {
            if lease.root() != self.project.root_path()
                || lease
                    .publication_journal_project()
                    .root_correlation_token()
                    .map_err(infrastructure)?
                    != self.root_tokens[0]
            {
                return Err(DeployGrantError::ForeignBinding);
            }
        }
        Ok(())
    }
    pub(super) fn read(&self, lease: &LifecycleLease) -> Result<Option<Pointer>, DeployGrantError> {
        self.check_roots(Some(lease))?;
        let bytes = lease
            .publication_journal_project()
            .read_file_bounded(&self.path, CAP)
            .map_err(infrastructure)?;
        self.check_roots(Some(lease))?;
        bytes
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|error| DeployGrantError::InvalidPointer(error.to_string()))
            })
            .transpose()
    }
    pub(super) fn publish(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
    ) -> Result<(), DeployGrantError> {
        self.validate_base(pointer)?;
        let bytes = serde_json::to_vec(pointer).map_err(infrastructure)?;
        if bytes.len() > CAP {
            return Err(DeployGrantError::InvalidPointer(
                "candidate exceeds bound".into(),
            ));
        }
        self.check_roots(Some(lease))?;
        let project = lease.publication_journal_project();
        let pinned = project.root_dir().map_err(infrastructure)?;
        let write = project
            .write_atomic_durable_in(&pinned, &self.path, &bytes)
            .map_err(|error| {
                let possibly = error.stage == PublishStage::PossiblyPublished;
                let reason = format!("{:#}", error.into_report());
                if possibly {
                    DeployGrantError::UncertainPublication(reason)
                } else {
                    DeployGrantError::Infrastructure(reason)
                }
            })?;
        let parent = self
            .project
            .root_path()
            .join(&self.path)
            .parent()
            .ok_or_else(|| DeployGrantError::InvalidPointer("missing parent".into()))?
            .to_path_buf();
        require_durable(&write, &parent)?;
        let observed = project
            .read_file_bounded(&self.path, CAP)
            .map_err(|error| DeployGrantError::UncertainPublication(error.to_string()))?;
        if observed.as_deref() != Some(bytes.as_slice()) {
            return Err(DeployGrantError::UncertainPublication(
                "published readback differs".into(),
            ));
        }
        self.check_roots(Some(lease))
            .map_err(|error| DeployGrantError::UncertainPublication(error.to_string()))
    }
}

fn require_durable(write: &DurableWrite, parent: &std::path::Path) -> Result<(), DeployGrantError> {
    let uncertain = || {
        DeployGrantError::UncertainPublication(
            "incomplete or unsupported durable directory checkpoints".into(),
        )
    };
    let mut expected = Vec::new();
    for directory in &write.published.created_directories {
        expected.push(directory.parent().ok_or_else(uncertain)?);
    }
    expected.push(parent);
    if !write.file_synced
        || write.parent != DirectoryDurability::Synced
        || write.directory_syncs.len() != expected.len()
        || write
            .directory_syncs
            .iter()
            .zip(expected)
            .any(|(sync, path)| {
                sync.directory != path || sync.durability != DirectoryDurability::Synced
            })
    {
        return Err(uncertain());
    }
    Ok(())
}
