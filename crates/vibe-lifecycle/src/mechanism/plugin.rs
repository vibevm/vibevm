//! The builtin `package:agent-plugin` provider — §6.2, in process.
//!
//! > "`package:agent-plugin` produces a directory, because Agent Plugins
//! > 1.0 defines a directory—not zip/tar—as the package unit."
//!
//! So the distributable IS the target's engine-owned package directory,
//! and its content witness is the canonical tree digest specified in
//! [`digest`] — never an implicit archive. The rest of §6.2 lands as:
//!
//! * the fixed shape and the containment law across links, junctions and
//!   reparse points — [`shape`];
//! * local validation of the published 1.0.0 manifests, member by member —
//!   [`manifest`];
//! * "no adapter silently drops an unsupported component" — every declared
//!   input must be PLACED, by name, in a valid reverse-domain
//!   client-extension directory, or the target refuses;
//! * the canonical directory digest, recorded in the A2 record with
//!   `kind = "agent-plugin"` and `shape = "directory"`.
//!
//! Client projections (§6.3) are deliberately absent: §6.0.5 puts them in
//! the deploy lane, and "the canonical Agent Plugin and a client-native
//! projection are distinct package artifacts".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

pub(crate) mod config;
mod manifest;
mod shape;

use crate::mechanism::contain::{read_file_bounded, tree_digest};
use crate::mechanism::error::preview;
use crate::mechanism::package::contained_identity;
use crate::mechanism::package::protocol::{
    PackageConfig, PackageFingerprint, PackagePlan, PlannedPackageOutput, StagedArtifact,
    VerifiedPackageArtifact,
};
use crate::mechanism::skill::{supported, write_distributable};
use crate::mechanism::{
    BUILTIN_AGENT_PLUGIN_PIN, EffectClass, MechanismError, NetworkUse, PackageProvider,
    PackageTargetRequest, PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};
use config::AgentPluginConfig;

/// The largest single file this provider will stage into a plugin.
///
/// A bound rather than a stream because the capability-relative writer
/// takes bytes; a file past it refuses BY NAME rather than being packaged
/// through a second, weaker write path.
const STAGE_CAP: u64 = 64 * 1024 * 1024;

/// The artifact kinds this provider produces — §6.2 produces a DIRECTORY.
const PRODUCED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Directory];

/// The §3.2 operations a package-role provider implements.
const PACKAGE_OPERATIONS: [ProviderOperation; 4] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
];

/// The builtin `org.vibevm/vibe#agent-plugin` provider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AgentPluginProvider;

impl PackageProvider for AgentPluginProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            key: BUILTIN_AGENT_PLUGIN_PIN,
            kinds: &PRODUCED_KINDS,
            effect: EffectClass::Workspace,
            network: NetworkUse::Never,
            privilege: PrivilegeNeed::None,
            reversibility: Reversibility::NotApplicable,
            operations: &PACKAGE_OPERATIONS,
        }
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn plan(&self, request: &PackageTargetRequest<'_>) -> Result<PackagePlan, MechanismError> {
        let target = request.target;
        let descriptor = self.descriptor();
        let config = AgentPluginConfig::parse(&target.id, target.config.as_ref())?;
        if target.outputs.len() != 1 {
            return Err(MechanismError::OutputCount {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                expected: "exactly one `directory` output — §6.2's package unit is the directory"
                    .to_owned(),
                found: target.outputs.len(),
            });
        }
        let mut outputs = Vec::with_capacity(1);
        for output in &target.outputs {
            if !descriptor.supports(output.kind) {
                return Err(MechanismError::UnsupportedKind {
                    target: target.id.clone(),
                    provider: descriptor.key.to_owned(),
                    output: output.id.clone(),
                    kind: output.kind.to_string(),
                    supported: supported(&PRODUCED_KINDS),
                });
            }
            outputs.push(PlannedPackageOutput {
                id: output.id.clone(),
                kind: output.kind,
                shape: ArtifactShape::Directory,
                relative: ".".to_owned(),
                media_type: None,
            });
        }
        check_placements(&target.id, request, &config)?;
        Ok(PackagePlan {
            summary: format!(
                "agent-plugin from `{}` with {} placed input(s)",
                config.source,
                config.place.len()
            ),
            output_dir: request.output_dir(),
            inputs: request
                .inputs
                .iter()
                .map(|input| input.reference.clone())
                .collect(),
            outputs,
            config: PackageConfig::AgentPlugin(config),
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<PackageFingerprint, MechanismError> {
        let config = plugin_config(plan)?;
        let source = shape::read_source(&request.target.id, request.project_root, &config.source)?;
        // The COMPLETE closed input set: every source-tree file and every
        // consumed artifact, each under the identity it is named by, in a
        // canonical order.
        let mut census: Vec<(String, String)> = Vec::new();
        for file in &source.files {
            let (digest, _) =
                crate::mechanism::contain::digest_file(&file.absolute).map_err(|fault| {
                    MechanismError::PluginShape {
                        target: request.target.id.clone(),
                        entry: preview(&file.relative),
                        reason: fault.reason(),
                    }
                })?;
            census.push((format!("source/{}", file.relative), digest));
        }
        for input in request.inputs {
            census.push((format!("input/{}", input.name), input.digest.clone()));
        }
        census.sort_unstable();
        let mut hash = Sha256::new();
        hash.update(b"agent-plugin/1\x00");
        for (name, digest) in &census {
            hash.update(name.as_bytes());
            hash.update(b"\x00");
            hash.update(digest.as_bytes());
            hash.update(b"\x00");
        }
        Ok(PackageFingerprint {
            digest: format!("{:x}", hash.finalize()),
            counted: census.len(),
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn apply(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Vec<StagedArtifact>, MechanismError> {
        let target = &request.target.id;
        let config = plugin_config(plan)?;
        let source = shape::read_source(target, request.project_root, &config.source)?;
        let root = request.output_dir_relative();
        let occupied: BTreeSet<&str> = source
            .files
            .iter()
            .map(|file| file.relative.as_str())
            .collect();
        for file in &source.files {
            let bytes = read_file_bounded(&file.absolute, STAGE_CAP).map_err(|fault| {
                MechanismError::PluginShape {
                    target: target.clone(),
                    entry: preview(&file.relative),
                    reason: fault.reason(),
                }
            })?;
            write_distributable(request, &format!("{root}/{}", file.relative), &bytes)?;
        }
        for (name, destination) in &config.place {
            if occupied.contains(destination.as_str()) {
                return Err(MechanismError::Config {
                    target: target.clone(),
                    member: format!("place.{}", preview(name)),
                    reason: format!(
                        "`{}` is already a file of the plugin source tree; a placed input never \
                         overwrites the authored plugin",
                        preview(destination)
                    ),
                });
            }
            let Some(input) = request.inputs.iter().find(|input| &input.name == name) else {
                // `check_placements` proved every key names a declared
                // input; this arm keeps the law a refusal rather than an
                // index.
                return Err(MechanismError::Config {
                    target: target.clone(),
                    member: format!("place.{}", preview(name)),
                    reason: "names no declared input of this target".to_owned(),
                });
            };
            let bytes = read_file_bounded(&input.absolute, STAGE_CAP).map_err(|fault| {
                MechanismError::SourceMissing {
                    target: target.clone(),
                    provider: BUILTIN_AGENT_PLUGIN_PIN.to_owned(),
                    path: preview(&input.relative),
                    reason: fault.reason(),
                }
            })?;
            write_distributable(request, &format!("{root}/{destination}"), &bytes)?;
        }
        let mut staged = Vec::with_capacity(plan.outputs.len());
        for output in &plan.outputs {
            staged.push(StagedArtifact {
                output_id: output.id.clone(),
                kind: output.kind,
                shape: ArtifactShape::Directory,
                absolute: request.output_dir(),
                media_type: None,
            });
        }
        Ok(staged)
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn verify(
        &self,
        request: &PackageTargetRequest<'_>,
        staged: &StagedArtifact,
    ) -> Result<VerifiedPackageArtifact, MechanismError> {
        let (path_absolute, path_relative) =
            contained_identity(request, &staged.output_id, &staged.absolute)?;
        let tree = tree_digest(&staged.absolute).map_err(|fault| MechanismError::PackageTree {
            target: request.target.id.clone(),
            output: staged.output_id.clone(),
            entry: preview(&fault.path),
            reason: fault.reason,
        })?;
        Ok(VerifiedPackageArtifact {
            output_id: staged.output_id.clone(),
            path_absolute,
            path_relative,
            digest: tree.digest,
            bytes: tree.bytes,
            files: tree.files,
        })
    }
}

/// The validated agent-plugin config carried on the plan.
fn plugin_config(plan: &PackagePlan) -> Result<&AgentPluginConfig, MechanismError> {
    match &plan.config {
        PackageConfig::AgentPlugin(config) => Ok(config),
        PackageConfig::StaticSkill(_) | PackageConfig::WindowsZip(_) => {
            Err(MechanismError::PlanRoleMismatch {
                provider: BUILTIN_AGENT_PLUGIN_PIN.to_owned(),
            })
        }
    }
}

/// §6.2's "no adapter silently drops an unsupported component", as the
/// exactly-once law over declared inputs and their placements.
fn check_placements(
    target: &str,
    request: &PackageTargetRequest<'_>,
    config: &AgentPluginConfig,
) -> Result<(), MechanismError> {
    let mut destinations: BTreeSet<&str> = BTreeSet::new();
    for (name, destination) in &config.place {
        if !request.inputs.iter().any(|input| &input.name == name) {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: format!("place.{}", preview(name)),
                reason: format!(
                    "names no declared input of this target; declared: {}",
                    declared(request)
                ),
            });
        }
        if !destinations.insert(destination.as_str()) {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: format!("place.{}", preview(name)),
                reason: format!(
                    "`{}` is already the destination of another placed input",
                    preview(destination)
                ),
            });
        }
    }
    for input in request.inputs {
        if !config.place.iter().any(|(name, _)| name == &input.name) {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: "place".to_owned(),
                reason: format!(
                    "declared input `{}` has no placement; §6.2 forbids silently dropping a \
                     component, so every declared input names its destination inside a \
                     reverse-domain client-extension directory",
                    preview(&input.name)
                ),
            });
        }
    }
    Ok(())
}

/// The declared input identities a refusal lists.
fn declared(request: &PackageTargetRequest<'_>) -> String {
    if request.inputs.is_empty() {
        return "none declared".to_owned();
    }
    request
        .inputs
        .iter()
        .map(|input| input.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "plugin/tests.rs"]
mod tests;
