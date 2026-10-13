//! Ordinary cooperative exclusive directory creation, not native M13 sealing.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-NO-OVERWRITE");
use crate::{Pinned, project::ExclusiveChildError};

#[derive(Debug)]
pub enum ScaffoldDirectoryError {
    NotCreated(String),
    CreatedButUnsealed {
        path: std::path::PathBuf,
        reason: String,
    },
}
impl ScaffoldDirectoryError {
    pub fn created(&self) -> bool {
        matches!(self, Self::CreatedButUnsealed { .. })
    }
}
#[derive(Debug)]
pub struct ScaffoldDirectory {
    pub directory: Pinned,
    pub identity: String,
    pub durability: crate::DirectoryDurability,
}
impl Pinned {
    /// Exclusive mkdir followed by no-follow reopen and a second identity
    /// comparison. A noncooperating substitution before the first reopen is
    /// not excluded: this is finite same-host correlation, not authentication.
    pub fn create_scaffold_directory(
        &self,
        name: &str,
    ) -> Result<ScaffoldDirectory, ScaffoldDirectoryError> {
        let directory = self
            .create_child_exclusive(name)
            .map_err(|error| match error {
                ExclusiveChildError::NotCreated(reason) => {
                    ScaffoldDirectoryError::NotCreated(reason.to_string())
                }
                ExclusiveChildError::CreatedNotReopened { path, source } => {
                    ScaffoldDirectoryError::CreatedButUnsealed {
                        path,
                        reason: source.to_string(),
                    }
                }
            })?;
        let checked = (|| -> anyhow::Result<String> {
            let held = directory.identity()?;
            let reopened = self.open_child(name)?;
            if reopened.identity()? != held {
                anyhow::bail!("created directory changed during reopen");
            }
            Ok(held.observation_correlation_token())
        })();
        let identity = checked.map_err(|reason| ScaffoldDirectoryError::CreatedButUnsealed {
            path: directory.path().into(),
            reason: reason.to_string(),
        })?;
        Ok(ScaffoldDirectory {
            directory,
            identity,
            durability: self.sync_directory(),
        })
    }
}
