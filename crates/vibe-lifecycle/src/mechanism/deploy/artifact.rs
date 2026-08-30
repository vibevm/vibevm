//! The deployed artifact's resolution — §6.0.2's law, in the deploy role.
//!
//! > "the package executor reads the A2 record the build executor wrote
//! > (engine-owned state, never a guessed path) and refuses a missing or
//! > stale-digest input by name."
//!
//! Unchanged for a destination: the engine's own record under
//! `.vibe/state/artifacts/<id>.json` is the ONE place a deployed
//! artifact's path may come from, and the bytes that are there NOW are
//! re-digested before any destination is touched. A deployment built from
//! an artifact that changed behind its own record is the failure this
//! cell exists to make impossible.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::path::Path;

use vibe_core::manifest::DeployTarget;

use super::error::DeployError;
use super::protocol::ResolvedDeployArtifact;
use crate::mechanism::contain::{checked_relative, digest_file, join_relative, tree_digest};
use crate::mechanism::record::read_record;

/// Resolve and prove the artifact one deploy target reconciles.
///
/// §6.0.2's law, unchanged: the engine's own record is the ONE place the
/// path may come from, and the bytes there NOW are re-digested before a
/// destination is touched.
pub(crate) fn resolve_artifact(
    project_root: &Path,
    target: &DeployTarget,
) -> Result<ResolvedDeployArtifact, DeployError> {
    let record = read_record(project_root, &target.artifact)
        .map_err(|reason| DeployError::ArtifactRecordUnusable {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
            reason,
        })?
        .ok_or_else(|| DeployError::ArtifactNotRecorded {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
        })?;
    let relative = checked_relative(&record.path_relative.path).map_err(|fault| {
        DeployError::ArtifactRecordUnusable {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
            reason: format!(
                "its recorded relative path `{}` is unusable: {}",
                record.path_relative.path,
                fault.reason()
            ),
        }
    })?;
    let absolute = join_relative(project_root, &relative);
    let (digest, bytes) =
        if record.shape == vibe_wire::generated::artifact_record::ArtifactShape::Directory {
            let tree = tree_digest(&absolute).map_err(|fault| DeployError::ArtifactMissing {
                target: target.id.clone(),
                artifact: target.artifact.clone(),
                path: format!("{relative}/{}", fault.path),
                reason: fault.reason,
            })?;
            (tree.digest, tree.bytes)
        } else {
            digest_file(&absolute).map_err(|fault| DeployError::ArtifactMissing {
                target: target.id.clone(),
                artifact: target.artifact.clone(),
                path: relative.clone(),
                reason: fault.reason(),
            })?
        };
    if digest != record.digest.value {
        return Err(DeployError::ArtifactStale {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
            path: relative,
            recorded: record.digest.value,
            found: digest,
        });
    }
    Ok(ResolvedDeployArtifact {
        id: target.artifact.clone(),
        kind: kind_of(&record.kind),
        shape: record.shape.clone(),
        absolute,
        relative,
        digest,
        bytes,
    })
}

/// The manifest vocabulary's spelling of one recorded artifact kind.
const fn kind_of(
    kind: &vibe_wire::generated::artifact_record::ArtifactKind,
) -> vibe_core::manifest::ArtifactKind {
    use vibe_core::manifest::ArtifactKind as Manifest;
    use vibe_wire::generated::artifact_record::ArtifactKind as Record;
    match kind {
        Record::Executable => Manifest::Executable,
        Record::Archive => Manifest::Archive,
        Record::File => Manifest::File,
        Record::Directory => Manifest::Directory,
        Record::Skill => Manifest::Skill,
        Record::AgentPlugin => Manifest::AgentPlugin,
    }
}
