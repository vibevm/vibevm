//! Resolve portable body descriptions from supplied lane bytes and locked slots.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES");

use std::path::PathBuf;

use super::AgentTypeBody;
use crate::vibedeps;

/// One package identity and version already resolved by the workspace lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
pub struct LockedSlot {
    /// Package group identity.
    pub group: vibe_core::Group,
    /// Package name within the group.
    pub name: String,
    /// Exact locked package version.
    pub version: semver::Version,
}

/// Inputs supplied by the caller without parsing a lockfile or installing packages.
#[derive(Debug, Clone)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
pub struct AgentTypeSources {
    /// Composed project lane bytes, or the reason the lane is unavailable.
    pub lane: Result<Vec<u8>, String>,
    /// WORKSPACE root where vibe.lock and vibevm/vibedeps live, never a member node's directory.
    pub workspace_root: PathBuf,
    /// Resolved package slots, or the reason the workspace lock is unavailable.
    pub locked: Result<Vec<LockedSlot>, String>,
}

impl AgentTypeSources {
    /// Resolve the supplied lane or a file from the exact locked workspace slot.
    ///
    /// ```
    /// use vibe_workspace::system_prompt::agent_types::{AgentTypeBody, AgentTypeSources};
    ///
    /// let sources = AgentTypeSources {
    ///     lane: Ok(b"LANE\r\n\xff".to_vec()),
    ///     workspace_root: std::path::PathBuf::new(),
    ///     locked: Err("no lock".into()),
    /// };
    /// assert_eq!(sources.resolve(&AgentTypeBody::ProjectLane), Ok(b"LANE\r\n\xff".to_vec()));
    /// ```
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
    pub fn resolve(&self, body: &AgentTypeBody) -> Result<Vec<u8>, String> {
        let AgentTypeBody::PackageFile { group, name, path } = body else {
            return self.lane.clone();
        };
        let locked = self.locked.as_ref().map_err(Clone::clone)?;
        let slot = locked
            .iter()
            .find(|slot| slot.group.as_str() == group && slot.name == *name)
            .ok_or_else(|| {
                format!(
                    "`{group}/{name}` is not installed in this project, so this type has no body; installing it provides one"
                )
            })?;
        let root = &self.workspace_root;
        let mut file = if vibedeps::is_in_place_slot(root, &slot.group, name) {
            vibedeps::in_place_slot_abs_path(root, &slot.group, name)
        } else {
            vibedeps::slot_abs_path(root, &slot.group, name, &slot.version)
        };
        for segment in path.split('/') {
            file.push(segment);
        }
        std::fs::read(&file).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "`{}` is missing; run `vibe install`",
                    crate::path_to_slash(file.strip_prefix(root).unwrap_or(&file))
                )
            } else {
                error.to_string()
            }
        })
    }
}
