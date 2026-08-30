//! Input resolution — §6.0.2's load-bearing sentence, mechanised.
//!
//! > "A `[[artifacts.package]]` input names a build output's id; the
//! > package executor reads the A2 record the build executor wrote
//! > (engine-owned state, never a guessed path) and refuses a missing or
//! > stale-digest input by name."
//!
//! Two things make that sentence real rather than decorative, and both are
//! here:
//!
//! 1. the ONLY way a consumed artifact's path is obtained is
//!    `.vibe/state/artifacts/<id>.json`. There is no `target/` scan, no
//!    filename convention, no fallback — an unrecorded artifact refuses;
//! 2. the record is not believed. Its path is re-proven against the
//!    filesystem and its digest is recomputed from the bytes that are
//!    there NOW, so an artifact that changed behind its own record refuses
//!    instead of being packaged.
//!
//! Resolution is the ENGINE's, not the provider's (§3.2: the engine owns
//! state persistence and artifact identities), which is why a provider
//! receives inputs that are already proven and never learns where the
//! state home is.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{ArtifactInput, ArtifactPackageTarget};

use super::error::PackageError;
use super::protocol::{InputOrigin, ResolvedInput};
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::contain::{
    checked_relative, digest_file, forward_slashed, join_relative, tree_digest,
};
use crate::mechanism::record::{manifest_kind, read_record};

/// Resolve and prove every declared input of one package target, in
/// declaration order.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub(crate) fn resolve_inputs(
    project_root: &Path,
    target: &ArtifactPackageTarget,
) -> Result<Vec<ResolvedInput>, PackageError> {
    let declared = target.inputs.as_deref().unwrap_or(&[]);
    let mut resolved = Vec::with_capacity(declared.len());
    for input in declared {
        resolved.push(match input {
            ArtifactInput::Artifact { artifact } => {
                from_record(project_root, &target.id, artifact)?
            }
            ArtifactInput::Path { path } => {
                from_workspace(project_root, &target.id, &forward_slashed(path))?
            }
        });
    }
    Ok(resolved)
}

/// One consumed artifact, through the engine's own record — path AND
/// digest re-proven before the bytes are used for anything.
fn from_record(
    project_root: &Path,
    target: &str,
    artifact: &str,
) -> Result<ResolvedInput, PackageError> {
    let record = read_record(project_root, artifact)
        .map_err(|reason| PackageError::InputRecordUnusable {
            target: target.to_owned(),
            input: artifact.to_owned(),
            reason,
        })?
        .ok_or_else(|| PackageError::InputNotRecorded {
            target: target.to_owned(),
            input: artifact.to_owned(),
        })?;
    // The record's own project-relative identity is the path, joined
    // component by component to THIS project's root. The absolute member
    // is deliberately not used: it names the machine the artifact was
    // produced on, and a record that travelled would then point outside.
    let relative = checked_relative(&record.path_relative.path).map_err(|fault| {
        PackageError::InputRecordUnusable {
            target: target.to_owned(),
            input: artifact.to_owned(),
            reason: format!(
                "its recorded relative path `{}` is unusable: {}",
                record.path_relative.path,
                fault.reason()
            ),
        }
    })?;
    let absolute = join_relative(project_root, &relative);
    // The recorded SHAPE decides how the bytes are witnessed: §4 gives a
    // file its SHA-256 and a directory its canonical tree digest, and
    // reading a directory through the file primitive would refuse every
    // directory artifact this plane can legitimately consume.
    let (digest, bytes) = match record.shape {
        ArtifactShape::File => {
            digest_file(&absolute).map_err(|fault| PackageError::InputArtifactMissing {
                target: target.to_owned(),
                input: artifact.to_owned(),
                path: relative.clone(),
                reason: fault.reason(),
            })?
        }
        ArtifactShape::Directory => {
            let witness =
                tree_digest(&absolute).map_err(|fault| PackageError::InputArtifactMissing {
                    target: target.to_owned(),
                    input: artifact.to_owned(),
                    path: relative.clone(),
                    reason: format!("{}: {}", fault.path, fault.reason),
                })?;
            (witness.digest, witness.bytes)
        }
    };
    if digest != record.digest.value {
        return Err(PackageError::InputStale {
            target: target.to_owned(),
            input: artifact.to_owned(),
            path: relative,
            recorded: record.digest.value,
            found: digest,
        });
    }
    Ok(ResolvedInput {
        name: artifact.to_owned(),
        reference: format!("artifact:{artifact}"),
        absolute,
        relative,
        digest,
        bytes,
        shape: record.shape.clone(),
        // The record's own declared kind travels with the resolved input.
        // A provider whose law admits only one kind of artifact (§6.3.0.3's
        // "exactly one recorded `agent-plugin` directory artifact") can then
        // ask the ENGINE what this is, instead of inferring it from a shape
        // every directory on disk shares.
        origin: InputOrigin::ArtifactRecord {
            kind: manifest_kind(&record.kind),
        },
    })
}

/// One workspace source path — "a plain contained read", and nothing more.
fn from_workspace(
    project_root: &Path,
    target: &str,
    declared: &str,
) -> Result<ResolvedInput, PackageError> {
    let relative = checked_relative(declared).map_err(|fault| PackageError::InputPathUnsafe {
        target: target.to_owned(),
        input: declared.to_owned(),
        reason: fault.reason().to_owned(),
    })?;
    let absolute = join_relative(project_root, &relative);
    let (digest, bytes) =
        digest_file(&absolute).map_err(|fault| PackageError::InputSourceMissing {
            target: target.to_owned(),
            input: relative.clone(),
            reason: fault.reason(),
        })?;
    Ok(ResolvedInput {
        name: relative.clone(),
        reference: format!("path:{relative}"),
        absolute,
        relative,
        digest,
        bytes,
        // A `{ path }` input stays a single file: the workspace half of
        // this resolver reads a declared resource, and a directory named
        // there would be a second, unstated way to enumerate a tree.
        shape: ArtifactShape::File,
        origin: InputOrigin::WorkspacePath,
    })
}
