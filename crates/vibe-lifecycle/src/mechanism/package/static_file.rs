//! The builtin `package:static-file` provider — §13.1's opaque file copier.
//!
//! The provider deliberately knows nothing about the file it packages. One
//! contained workspace path becomes one `file` artifact under the exact safe
//! filename declared as the output id. The engine still owns input resolution,
//! the output directory, fingerprint carriage, verification and the A2 record.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY");

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{
    ArtifactKind, ExtensionConfig, declarant_path_component, is_portable_token,
};
use vibe_safefs::Project;
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::error::preview;
use crate::mechanism::package::contained_identity;
use crate::mechanism::package::protocol::{
    InputOrigin, PackageConfig, PackageFingerprint, PackagePlan, PlannedPackageOutput,
    StagedArtifact, VerifiedPackageArtifact,
};
use crate::mechanism::skill::supported;
use crate::mechanism::{
    BUILTIN_STATIC_FILE_PIN, EffectClass, MechanismError, NetworkUse, PackageProvider,
    PackageTargetRequest, PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};

const PRODUCED_KINDS: [ArtifactKind; 1] = [ArtifactKind::File];

const PACKAGE_OPERATIONS: [ProviderOperation; 4] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
];

/// The validated empty config carried by this provider's plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StaticFileConfig;

impl StaticFileConfig {
    fn parse(target: &str, config: Option<&ExtensionConfig>) -> Result<Self, MechanismError> {
        if let Some((member, _)) = config.and_then(|value| value.as_table().iter().next()) {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: preview(member),
                reason: "the static-file config is empty; source and destination are the one path input and one output id"
                    .to_owned(),
            });
        }
        Ok(Self)
    }
}

/// The builtin `org.vibevm/vibe#static-file` provider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StaticFileProvider;

impl PackageProvider for StaticFileProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            key: BUILTIN_STATIC_FILE_PIN,
            kinds: &PRODUCED_KINDS,
            effect: EffectClass::Workspace,
            network: NetworkUse::Never,
            privilege: PrivilegeNeed::None,
            reversibility: Reversibility::NotApplicable,
            operations: &PACKAGE_OPERATIONS,
        }
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
    fn plan(&self, request: &PackageTargetRequest<'_>) -> Result<PackagePlan, MechanismError> {
        let target = request.target;
        let descriptor = self.descriptor();
        let config = StaticFileConfig::parse(&target.id, target.config.as_ref())?;
        let input = match request.inputs {
            [only] => only,
            other => {
                return Err(MechanismError::Config {
                    target: target.id.clone(),
                    member: "inputs".to_owned(),
                    reason: format!(
                        "expected exactly one contained workspace `{{ path }}` file, found {} input(s)",
                        other.len(),
                    ),
                });
            }
        };
        if !matches!(input.origin, InputOrigin::WorkspacePath) {
            return Err(MechanismError::ArtifactInputRejected {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                input: input.name.clone(),
            });
        }
        if input.shape != ArtifactShape::File {
            return Err(MechanismError::Config {
                target: target.id.clone(),
                member: "inputs".to_owned(),
                reason: "the one static-file input must be a regular workspace file".to_owned(),
            });
        }
        if target.outputs.len() != 1 {
            return Err(MechanismError::OutputCount {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                expected: "exactly one `file` output whose id is its destination filename"
                    .to_owned(),
                found: target.outputs.len(),
            });
        }
        let output = &target.outputs[0];
        if !descriptor.supports(output.kind) {
            return Err(MechanismError::UnsupportedKind {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                output: output.id.clone(),
                kind: output.kind.to_string(),
                supported: supported(&PRODUCED_KINDS),
            });
        }
        if output.select.is_some() {
            return Err(MechanismError::Config {
                target: target.id.clone(),
                member: "outputs.select".to_owned(),
                reason: "static-file emits the whole opaque file; select metadata is not admitted"
                    .to_owned(),
            });
        }
        if !portable_filename(&output.id) {
            return Err(MechanismError::Config {
                target: target.id.clone(),
                member: "outputs.id".to_owned(),
                reason: format!(
                    "`{}` is not one safe portable filename",
                    preview(&output.id),
                ),
            });
        }
        Ok(PackagePlan {
            config: PackageConfig::StaticFile(config),
            output_dir: request.output_dir(),
            inputs: vec![input.reference.clone()],
            outputs: vec![PlannedPackageOutput {
                id: output.id.clone(),
                kind: output.kind,
                shape: ArtifactShape::File,
                relative: output.id.clone(),
                media_type: None,
            }],
            summary: format!(
                "static-file copies `{}` byte-for-byte as `{}`",
                input.relative, output.id,
            ),
        })
    }

    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<PackageFingerprint, MechanismError> {
        static_file_plan(plan)?;
        let input = one_input(request)?;
        let output = one_output(plan)?;
        let mut hash = Sha256::new();
        hash.update(b"static-file/1\x00input\x00");
        hash.update(input.reference.as_bytes());
        hash.update(b"\x00digest\x00");
        hash.update(input.digest.as_bytes());
        hash.update(b"\x00output\x00");
        hash.update(output.id.as_bytes());
        Ok(PackageFingerprint {
            digest: format!("{:x}", hash.finalize()),
            counted: 1,
        })
    }

    fn apply(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Vec<StagedArtifact>, MechanismError> {
        static_file_plan(plan)?;
        let input = one_input(request)?;
        let output = one_output(plan)?;
        let absolute = request.output_dir().join(&output.relative);
        let destination = format!("{}/{}", request.output_dir_relative(), output.relative);
        let project =
            Project::open(request.project_root).map_err(|error| MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: preview(&destination),
                reason: format!("{error:#}"),
            })?;
        let (source, _) = project
            .copy_stable_file_to_fresh_dir_expected(
                &input.relative,
                &project,
                &request.output_dir_relative(),
                &output.relative,
                None,
                &input.digest,
                input.bytes,
            )
            .map_err(|error| MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: preview(&destination),
                reason: format!("{:#}", error.into_report()),
            })?;
        if source.sha256 != input.digest || source.bytes != input.bytes {
            return Err(MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: preview(&destination),
                reason: format!(
                    "the held source changed since input resolution (resolved {} bytes at {}, copied {} bytes at {})",
                    input.bytes, input.digest, source.bytes, source.sha256,
                ),
            });
        }
        Ok(vec![StagedArtifact {
            output_id: output.id.clone(),
            kind: output.kind,
            shape: ArtifactShape::File,
            absolute,
            media_type: None,
        }])
    }

    fn verify(
        &self,
        request: &PackageTargetRequest<'_>,
        staged: &StagedArtifact,
    ) -> Result<VerifiedPackageArtifact, MechanismError> {
        let input = one_input(request)?;
        let (path_absolute, path_relative) =
            contained_identity(request, &staged.output_id, &staged.absolute)?;
        let project = Project::open(request.project_root).map_err(|error| {
            MechanismError::PackageOutputMissing {
                target: request.target.id.clone(),
                output: staged.output_id.clone(),
                path: path_relative.clone(),
                reason: format!("{error:#}"),
            }
        })?;
        let state = project
            .stable_file_state(&path_relative)
            .map_err(|error| MechanismError::PackageOutputMissing {
                target: request.target.id.clone(),
                output: staged.output_id.clone(),
                path: path_relative.clone(),
                reason: format!("{error:#}"),
            })?
            .ok_or_else(|| MechanismError::PackageOutputMissing {
                target: request.target.id.clone(),
                output: staged.output_id.clone(),
                path: path_relative.clone(),
                reason: "the output is absent".to_owned(),
            })?;
        if state.sha256 != input.digest || state.bytes != input.bytes {
            return Err(MechanismError::PackageOutputMissing {
                target: request.target.id.clone(),
                output: staged.output_id.clone(),
                path: path_relative,
                reason: format!(
                    "the copied file is not byte-identical to `{}` (input {} bytes at {}, output {} bytes at {})",
                    input.relative, input.bytes, input.digest, state.bytes, state.sha256,
                ),
            });
        }
        Ok(VerifiedPackageArtifact {
            output_id: staged.output_id.clone(),
            path_absolute,
            path_relative,
            digest: state.sha256,
            bytes: state.bytes,
            files: 1,
        })
    }
}

fn static_file_plan(plan: &PackagePlan) -> Result<(), MechanismError> {
    match &plan.config {
        PackageConfig::StaticFile(_) => Ok(()),
        PackageConfig::StaticSkill(_)
        | PackageConfig::AgentPlugin(_)
        | PackageConfig::WindowsZip(_)
        | PackageConfig::ClientProjection(_) => Err(MechanismError::PlanRoleMismatch {
            provider: BUILTIN_STATIC_FILE_PIN.to_owned(),
        }),
    }
}

fn one_input<'a>(
    request: &PackageTargetRequest<'a>,
) -> Result<&'a crate::mechanism::package::protocol::ResolvedInput, MechanismError> {
    request
        .inputs
        .first()
        .ok_or_else(|| MechanismError::Config {
            target: request.target.id.clone(),
            member: "inputs".to_owned(),
            reason: "the validated static-file plan lost its one input".to_owned(),
        })
}

fn one_output(plan: &PackagePlan) -> Result<&PlannedPackageOutput, MechanismError> {
    plan.outputs
        .first()
        .ok_or_else(|| MechanismError::PlanRoleMismatch {
            provider: BUILTIN_STATIC_FILE_PIN.to_owned(),
        })
}

/// One conservative cross-platform filename grammar shared with the matching
/// deploy provider. Artifact ids already use this token vocabulary; the extra
/// length and Windows-device checks make the id safe as a physical segment.
pub(crate) fn portable_filename(value: &str) -> bool {
    value.len() <= 255 && is_portable_token(value) && declarant_path_component(value).is_ok()
}

#[cfg(test)]
#[path = "static_file/tests.rs"]
mod tests;
