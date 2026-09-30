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
use vibe_safefs::Project;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactRecord, ArtifactShape, DigestAlgorithm, RelativeRoot,
};

use super::error::DeployError;
use super::protocol::ResolvedDeployArtifact;
use crate::mechanism::contain::{checked_relative, digest_file, join_relative, tree_digest};
use crate::mechanism::record::ARTIFACT_RECORD_DIR;
use crate::mechanism::record::read_record;
use crate::mechanism::vibebin::bound::BoundProjectArtifact;

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

/// Resolve the first bound builtin's source exclusively through the host's
/// held project root. This is a separate admission path because the legacy
/// resolver reopens its root and artifact by pathname. The returned bound
/// source keeps the record's exact project-relative coordinates for the
/// later held-handle copy, which rechecks bytes at the effect boundary.
pub(crate) fn resolve_bound_project_artifact(
    project: &Project,
    target: &DeployTarget,
) -> Result<(ResolvedDeployArtifact, BoundProjectArtifact), DeployError> {
    let unusable = |reason: String| DeployError::ArtifactRecordUnusable {
        target: target.id.clone(),
        artifact: target.artifact.clone(),
        reason,
    };
    if !vibe_core::manifest::is_portable_token(&target.artifact) {
        return Err(unusable(
            "artifact ID is outside the portable-token grammar".into(),
        ));
    }
    let record_relative = format!("{ARTIFACT_RECORD_DIR}/{}.json", target.artifact);
    let bytes = project
        .read_file_bounded(&record_relative, 1024 * 1024)
        .map_err(|error| unusable(format!("{error:#}")))?
        .ok_or_else(|| DeployError::ArtifactNotRecorded {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
        })?;
    let record: ArtifactRecord =
        serde_json::from_slice(&bytes).map_err(|error| unusable(error.to_string()))?;
    vibe_wire::behaviour::artifact_record::validate(&record)
        .map_err(|error| unusable(error.to_string()))?;
    if record.id != target.artifact {
        return Err(unusable(
            "record identity differs from requested artifact".into(),
        ));
    }
    if record.path_relative.root != RelativeRoot::Project
        || record.kind != RecordKind::Executable
        || record.shape != ArtifactShape::File
        || record.digest.algorithm != DigestAlgorithm::Sha256
    {
        return Err(unusable(
            "bound vibe-bin requires a project-root executable SHA-256 file record".into(),
        ));
    }
    let relative = checked_relative(&record.path_relative.path)
        .map_err(|fault| unusable(fault.reason().into()))?;
    let observed = project
        .stable_file_state(&relative)
        .map_err(|error| DeployError::ArtifactMissing {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
            path: relative.clone(),
            reason: format!("{error:#}"),
        })?
        .ok_or_else(|| DeployError::ArtifactMissing {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
            path: relative.clone(),
            reason: "recorded project file is absent".into(),
        })?;
    let bound = BoundProjectArtifact::from_record(&record, observed.bytes)
        .map_err(|error| unusable(error.to_string()))?;
    if observed.sha256 != record.digest.value {
        return Err(DeployError::ArtifactStale {
            target: target.id.clone(),
            artifact: target.artifact.clone(),
            path: relative,
            recorded: record.digest.value,
            found: observed.sha256,
        });
    }
    let resolved = ResolvedDeployArtifact {
        id: target.artifact.clone(),
        kind: kind_of(&record.kind),
        shape: record.shape,
        absolute: project.root_path().join(&relative),
        relative,
        digest: observed.sha256,
        bytes: observed.bytes,
    };
    Ok((resolved, bound))
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

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};
    use vibe_safefs::Project;

    use super::resolve_bound_project_artifact;
    use crate::mechanism::deploy::error::DeployError;
    use crate::mechanism::vibebin::support::target;

    #[test]
    fn pinned_resolver_proves_exact_project_file_and_refuses_slot_or_stale_bytes() {
        let root = tempfile::tempdir().unwrap();
        let artifact = root.path().join("helper.bin");
        let bytes = b"expected executable";
        std::fs::write(&artifact, bytes).unwrap();
        std::fs::create_dir_all(root.path().join(".vibe/state/artifacts")).unwrap();
        let record_path = root.path().join(".vibe/state/artifacts/helper.json");
        let mut record = serde_json::json!({
            "schema": 1,
            "id": "helper",
            "kind": "executable",
            "shape": "file",
            "path_absolute": artifact.to_string_lossy().replace('\\', "/"),
            "path_relative": {"root":"project", "path":"helper.bin"},
            "digest": {"algorithm":"sha256", "value":format!("{:x}", Sha256::digest(bytes))},
            "producer": {"target":"helper", "mechanism":"build:cargo",
                "provider":{"key":"org.vibevm/vibe#cargo"}},
            "freshness": {},
            "created_at":"2026-09-30T00:00:00Z",
            "verification":{"status":"verified", "evidence":"fixture source measured"}
        });
        std::fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
        let project = Project::open(root.path()).unwrap();
        let row = target("local-helper", "helper", Some("command = \"helper\""));
        let (resolved, bound) = resolve_bound_project_artifact(&project, &row).unwrap();
        assert!(bound.matches_resolved(&resolved.relative, &resolved.digest, resolved.bytes));
        assert_eq!(resolved.bytes, bytes.len() as u64);

        record["path_relative"]["root"] = "slot".into();
        std::fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(matches!(
            resolve_bound_project_artifact(&project, &row),
            Err(DeployError::ArtifactRecordUnusable { .. })
        ));

        record["path_relative"]["root"] = "project".into();
        std::fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
        std::fs::write(&artifact, b"changed executable").unwrap();
        assert!(matches!(
            resolve_bound_project_artifact(&project, &row),
            Err(DeployError::ArtifactStale { .. })
        ));
    }
}
