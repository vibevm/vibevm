//! The builtin `package:static-skill` provider — §6.1, in process.
//!
//! §6.1 in full, as laws this cell implements:
//!
//! 1. it "produces exactly one UTF-8 `SKILL.md` file" — one declared
//!    output, `kind = "file"`, always at `SKILL.md` inside the target's own
//!    engine-owned package directory;
//! 2. it "validates Agent Skills frontmatter" — locally and structurally;
//!    the exact member list is on [`frontmatter::parse`];
//! 3. it "aligns directory/name identity" — the `name` member must equal
//!    the source directory's own name;
//! 4. "a multi-file source is static-buildable only through explicit
//!    `vibe:include` directives … every directive names one declared
//!    textual resource and is replaced deterministically with visible
//!    origin/hash framing"; "every declared extra resource must be
//!    consumed exactly once or the build refuses" — both in [`include`];
//! 5. "static mode rejects executable scripts, shebang-bearing program
//!    files, binary assets, unsafe traversal and unresolved sibling
//!    references" — the first three in [`textual`], traversal in the
//!    engine's own path law plus the source-containment check below, the
//!    fourth in [`include`];
//! 6. "exact input/output digests … are required" — the input census is
//!    the engine-fresh fingerprint, the output digest is `verify`'s;
//! 7. "a decompiler is not" — nothing here reads a compiled artifact, and
//!    a consumed `{ artifact }` input refuses by name.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::ArtifactKind;
use vibe_safefs::Project;
use vibe_wire::generated::artifact_record::ArtifactShape;

pub(crate) mod config;
mod frontmatter;
mod include;

use crate::mechanism::contain::{digest_file, join_relative, read_file_bounded};
use crate::mechanism::error::preview;
use crate::mechanism::package::contained_identity;
use crate::mechanism::package::protocol::{
    PackageConfig, PackageFingerprint, PackagePlan, PlannedPackageOutput, StagedArtifact,
    VerifiedPackageArtifact,
};
use crate::mechanism::{
    BUILTIN_STATIC_SKILL_PIN, EffectClass, MechanismError, NetworkUse, PackageProvider,
    PackageTargetRequest, PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};
use config::{ENTRY_DOCUMENT, StaticSkillConfig};

/// The one distributable file this provider produces, inside the target's
/// engine-owned package directory.
const OUTPUT_FILE: &str = "SKILL.md";

/// The declared media type of that file.
const OUTPUT_MEDIA_TYPE: &str = "text/markdown";

/// The largest document or resource this provider will inline.
const TEXT_CAP: u64 = 4 * 1024 * 1024;

/// The artifact kinds this provider produces — §6.1 produces one FILE.
const PRODUCED_KINDS: [ArtifactKind; 1] = [ArtifactKind::File];

/// The §3.2 operations a package-role provider implements.
const PACKAGE_OPERATIONS: [ProviderOperation; 4] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
];

/// File extensions that name a program on a platform with no execute bit.
/// The shebang and execute-bit tests below catch the rest.
const PROGRAM_EXTENSIONS: [&str; 8] = ["exe", "dll", "com", "bat", "cmd", "ps1", "msi", "scr"];

/// The builtin `org.vibevm/vibe#static-skill` provider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StaticSkillProvider;

impl PackageProvider for StaticSkillProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            key: BUILTIN_STATIC_SKILL_PIN,
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
        let config = StaticSkillConfig::parse(&target.id, target.config.as_ref())?;
        if target.outputs.len() != 1 {
            return Err(MechanismError::OutputCount {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                expected: "exactly one `file` output — §6.1 produces one `SKILL.md`".to_owned(),
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
                shape: ArtifactShape::File,
                relative: OUTPUT_FILE.to_owned(),
                media_type: Some(OUTPUT_MEDIA_TYPE.to_owned()),
            });
        }
        // Every declared resource must be a workspace file of THIS skill's
        // own source directory. A consumed build artifact refuses by name
        // rather than being read: §6.1 says a decompiler is not required,
        // and inlining a compiled artifact into a Markdown document is the
        // thing that would require one.
        let prefix = format!("{}/", config.source);
        for input in request.inputs {
            if input.origin.recorded_kind().is_some() {
                return Err(MechanismError::ArtifactInputRejected {
                    target: target.id.clone(),
                    provider: descriptor.key.to_owned(),
                    input: input.name.clone(),
                });
            }
            if !input.relative.starts_with(&prefix) || input.relative == config.entry_relative() {
                return Err(MechanismError::ResourceOutsideSource {
                    target: target.id.clone(),
                    name: preview(&input.relative),
                    source_dir: config.source.clone(),
                });
            }
        }
        Ok(PackagePlan {
            summary: format!(
                "static-skill from `{}` with {} declared resource(s)",
                config.source,
                request.inputs.len()
            ),
            output_dir: request.output_dir(),
            inputs: request
                .inputs
                .iter()
                .map(|input| input.reference.clone())
                .collect(),
            outputs,
            config: PackageConfig::StaticSkill(config),
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<PackageFingerprint, MechanismError> {
        let config = skill_config(plan)?;
        let entry = join_relative(request.project_root, &config.entry_relative());
        let (entry_digest, _) =
            digest_file(&entry).map_err(|fault| MechanismError::SourceMissing {
                target: request.target.id.clone(),
                provider: BUILTIN_STATIC_SKILL_PIN.to_owned(),
                path: config.entry_relative(),
                reason: fault.reason(),
            })?;
        // The COMPLETE closed input set: the entry document plus every
        // declared resource, each named and digested, in a canonical order
        // so the value does not depend on declaration order.
        let mut census: Vec<(&str, &str)> = request
            .inputs
            .iter()
            .map(|input| (input.name.as_str(), input.digest.as_str()))
            .collect();
        census.sort_unstable();
        let mut hash = Sha256::new();
        hash.update(b"static-skill/1\x00");
        hash.update(ENTRY_DOCUMENT.as_bytes());
        hash.update(b"\x00");
        hash.update(entry_digest.as_bytes());
        for (name, digest) in &census {
            hash.update(b"\x00resource\x00");
            hash.update(name.as_bytes());
            hash.update(b"\x00");
            hash.update(digest.as_bytes());
        }
        Ok(PackageFingerprint {
            digest: format!("{:x}", hash.finalize()),
            counted: census.len() + 1,
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn apply(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Vec<StagedArtifact>, MechanismError> {
        let target = &request.target.id;
        let config = skill_config(plan)?;
        let entry = read_text(
            target,
            &join_relative(request.project_root, &config.entry_relative()),
            &config.entry_relative(),
        )?;
        let parsed = frontmatter::parse(target, &entry)?;
        if parsed.name != config.directory_name() {
            return Err(MechanismError::SkillIdentity {
                target: target.clone(),
                declared: preview(&parsed.name),
                directory: config.directory_name().to_owned(),
            });
        }
        let mut resources = Vec::with_capacity(request.inputs.len());
        for input in request.inputs {
            let text = read_text(target, &input.absolute, &input.relative)?;
            textual(target, &input.relative, &input.absolute, &text)?;
            resources.push((input, text));
        }
        // A directive names a resource the way an AUTHOR sees it: relative
        // to the skill's own source directory. The framing still carries
        // the project-relative origin, so the document says where the
        // bytes really came from without making every directive repeat the
        // source path.
        let prefix = format!("{}/", config.source);
        let inlinable: Vec<include::Inlinable<'_>> = resources
            .iter()
            .map(|(input, text)| include::Inlinable {
                name: input
                    .relative
                    .strip_prefix(&prefix)
                    .unwrap_or(input.relative.as_str()),
                origin: &input.relative,
                digest: &input.digest,
                text,
            })
            .collect();
        let body = include::render(target, parsed.body, &inlinable)?;
        let document = format!("---\n{}---\n{body}", parsed.block);
        let relative = format!("{}/{OUTPUT_FILE}", request.output_dir_relative());
        write_distributable(request, &relative, document.as_bytes())?;
        let mut staged = Vec::with_capacity(plan.outputs.len());
        for output in &plan.outputs {
            staged.push(StagedArtifact {
                output_id: output.id.clone(),
                kind: output.kind,
                shape: ArtifactShape::File,
                absolute: request.output_dir().join(OUTPUT_FILE),
                media_type: output.media_type.clone(),
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
        let (digest, bytes) = digest_file(&staged.absolute).map_err(|fault| {
            MechanismError::PackageOutputMissing {
                target: request.target.id.clone(),
                output: staged.output_id.clone(),
                path: path_relative.clone(),
                reason: fault.reason(),
            }
        })?;
        Ok(VerifiedPackageArtifact {
            output_id: staged.output_id.clone(),
            path_absolute,
            path_relative,
            digest,
            bytes,
            files: 1,
        })
    }
}

/// The validated static-skill config carried on the plan.
fn skill_config(plan: &PackagePlan) -> Result<&StaticSkillConfig, MechanismError> {
    match &plan.config {
        PackageConfig::StaticSkill(config) => Ok(config),
        PackageConfig::AgentPlugin(_)
        | PackageConfig::WindowsZip(_)
        | PackageConfig::ClientProjection(_) => Err(MechanismError::PlanRoleMismatch {
            provider: BUILTIN_STATIC_SKILL_PIN.to_owned(),
        }),
    }
}

/// Read one bounded file and prove it is UTF-8.
fn read_text(
    target: &str,
    absolute: &std::path::Path,
    relative: &str,
) -> Result<String, MechanismError> {
    let bytes =
        read_file_bounded(absolute, TEXT_CAP).map_err(|fault| MechanismError::SourceMissing {
            target: target.to_owned(),
            provider: BUILTIN_STATIC_SKILL_PIN.to_owned(),
            path: preview(relative),
            reason: fault.reason(),
        })?;
    String::from_utf8(bytes).map_err(|_| MechanismError::ResourceRejected {
        target: target.to_owned(),
        name: preview(relative),
        reason: "not valid UTF-8, so it is a binary asset rather than a textual resource"
            .to_owned(),
    })
}

/// §6.1's three content refusals: an executable script, a shebang-bearing
/// program file, a binary asset.
fn textual(
    target: &str,
    relative: &str,
    absolute: &std::path::Path,
    text: &str,
) -> Result<(), MechanismError> {
    let refuse = |reason: &str| MechanismError::ResourceRejected {
        target: target.to_owned(),
        name: preview(relative),
        reason: reason.to_owned(),
    };
    if text.contains('\0') {
        return Err(refuse(
            "it carries NUL bytes, so it is a binary asset rather than a textual resource",
        ));
    }
    if text.starts_with("#!") {
        return Err(refuse(
            "it opens with a shebang, so it is a program file rather than a textual resource",
        ));
    }
    if is_executable(absolute) {
        return Err(refuse(
            "it is marked executable, and a static skill never inlines an executable script",
        ));
    }
    let extension = relative
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if PROGRAM_EXTENSIONS.contains(&extension.as_str()) {
        return Err(refuse(
            "its extension names a program file, which a static skill never inlines",
        ));
    }
    Ok(())
}

/// Whether the platform marks this file executable.
///
/// Windows has no execute bit, which is exactly why the extension list
/// above exists beside this: the two together cover both platforms'
/// spelling of "this file is a program".
#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool {
    false
}

/// Publish one distributable file through the capability-relative,
/// no-follow writer every durable byte of this crate goes through.
pub(crate) fn write_distributable(
    request: &PackageTargetRequest<'_>,
    relative: &str,
    bytes: &[u8],
) -> Result<(), MechanismError> {
    let refuse = |reason: String| MechanismError::PackageWrite {
        target: request.target.id.clone(),
        path: preview(relative),
        reason,
    };
    let project =
        Project::open(request.project_root).map_err(|error| refuse(format!("{error:#}")))?;
    project
        .write_atomic(relative, bytes)
        .map_err(|error| refuse(format!("{:#}", error.into_report())))?;
    Ok(())
}

/// The supported-kind list a refusal spells.
pub(crate) fn supported(kinds: &[ArtifactKind]) -> String {
    kinds
        .iter()
        .map(ArtifactKind::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "skill/tests.rs"]
mod tests;
