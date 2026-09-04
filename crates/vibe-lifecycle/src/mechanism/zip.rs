//! The builtin `package:windows-zip` provider — §7.0.8, in process.
//!
//! §7.0.8 in full, as laws this cell implements:
//!
//! 1. "a byte-identical archive on re-run" — the acceptance, proven by an
//!    end-to-end that packages the same inputs twice and compares one
//!    digest. Everything else on this list exists to make it true;
//! 2. "entries sorted by archived name" — the census is sorted once, here,
//!    and [`archive::write_archive`] refuses one that is not;
//! 3. "forward-slash names" and "no platform extra fields" — the writer's,
//!    stated as constants beside the headers they fill;
//! 4. "one fixed timestamp constant" and "fixed compression parameters" —
//!    likewise, and the config refuses both by name so a manifest cannot
//!    reintroduce a knob;
//! 5. "a directory input enters by its canonical walk" — the SHARED walk
//!    (`contain::walk_tree`), the same one the canonical directory digest
//!    reads, so an archived tree and its recorded digest can never
//!    disagree about which files a tree holds;
//! 6. "strict snake_case config refusing unknown members, with `layout` as
//!    an OPTIONAL archive-internal entry prefix" — [`config`].
//!
//! It is engine-fresh (§4.1) and entitled to be: an archive's complete
//! input set is the declared inputs, and those are closed and hashable by
//! the time a provider sees them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

mod archive;
pub(crate) mod config;

use crate::mechanism::contain::{digest_file, read_file_bounded, walk_tree};
use crate::mechanism::error::preview;
use crate::mechanism::package::contained_identity;
use crate::mechanism::package::protocol::{
    PackageConfig, PackageFingerprint, PackagePlan, PlannedPackageOutput, ResolvedInput,
    StagedArtifact, VerifiedPackageArtifact,
};
use crate::mechanism::skill::{supported, write_distributable};
use crate::mechanism::{
    BUILTIN_WINDOWS_ZIP_PIN, EffectClass, MechanismError, NetworkUse, PackageProvider,
    PackageTargetRequest, PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};
use archive::{ArchiveEntry, write_archive};
use config::WindowsZipConfig;

/// The declared media type of the one distributable.
const OUTPUT_MEDIA_TYPE: &str = "application/zip";

/// The largest single member this provider will hold in memory while it
/// renders the archive. The writer is not streaming: determinism is a
/// property of the whole byte sequence, and a bound stated here is
/// preferable to one discovered by a machine running out of memory.
const MEMBER_CAP: u64 = 512 * 1024 * 1024;

/// The artifact kinds this provider produces — one ARCHIVE.
const PRODUCED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Archive];

/// The §3.2 operations a package-role provider implements.
const PACKAGE_OPERATIONS: [ProviderOperation; 4] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
];

/// The builtin `org.vibevm/vibe#windows-zip` provider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WindowsZipProvider;

impl PackageProvider for WindowsZipProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            key: BUILTIN_WINDOWS_ZIP_PIN,
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
        let config = WindowsZipConfig::parse(&target.id, target.config.as_ref())?;
        if target.outputs.len() != 1 {
            return Err(MechanismError::OutputCount {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                expected: "exactly one `archive` output — §7.0.8 produces one zip".to_owned(),
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
                relative: archive_name(&target.id),
                media_type: Some(OUTPUT_MEDIA_TYPE.to_owned()),
            });
        }
        // The census is computed at PLAN time, so the archive's shape is
        // knowable before anything is written — which is what makes a dry
        // report of this provider honest.
        let census = census(request, &config)?;
        Ok(PackagePlan {
            config: PackageConfig::WindowsZip(config),
            output_dir: request.output_dir(),
            inputs: request
                .inputs
                .iter()
                .map(|input| input.reference.clone())
                .collect(),
            outputs,
            summary: format!(
                "windows-zip: {} entry(ies) STORED at the fixed 1980-01-01 timestamp into `{}`",
                census.len(),
                archive_name(&target.id),
            ),
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<PackageFingerprint, MechanismError> {
        let config = zip_config(plan)?;
        // The complete closed input set: every ARCHIVED name and the
        // digest of the bytes that will sit under it. Naming the archived
        // name rather than the input's own is what makes a `layout` change
        // a freshness change — the archive really is different.
        let census = census(request, config)?;
        let mut hash = Sha256::new();
        hash.update(b"windows-zip/1\x00");
        for entry in &census {
            hash.update(entry.name.as_bytes());
            hash.update(b"\x00");
            hash.update(entry.digest.as_bytes());
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
        let config = zip_config(plan)?;
        let census = census(request, config)?;
        let mut bodies: Vec<Vec<u8>> = Vec::with_capacity(census.len());
        for entry in &census {
            bodies.push(
                read_file_bounded(&entry.absolute, MEMBER_CAP).map_err(|fault| {
                    MechanismError::SourceMissing {
                        target: target.clone(),
                        provider: BUILTIN_WINDOWS_ZIP_PIN.to_owned(),
                        path: preview(&entry.name),
                        reason: fault.reason(),
                    }
                })?,
            );
        }
        let entries: Vec<ArchiveEntry<'_>> = census
            .iter()
            .zip(bodies.iter())
            .map(|(entry, bytes)| ArchiveEntry {
                name: &entry.name,
                bytes,
            })
            .collect();
        let bytes = write_archive(&entries).map_err(|fault| MechanismError::PackageWrite {
            target: target.clone(),
            path: archive_name(target),
            reason: fault.reason(),
        })?;
        let relative = format!("{}/{}", request.output_dir_relative(), archive_name(target));
        write_distributable(request, &relative, &bytes)?;
        let absolute = request.output_dir().join(archive_name(target));
        Ok(plan
            .outputs
            .iter()
            .map(|output| StagedArtifact {
                output_id: output.id.clone(),
                kind: output.kind,
                shape: ArtifactShape::File,
                absolute: absolute.clone(),
                media_type: output.media_type.clone(),
            })
            .collect())
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

/// One entry of the archive census: what it will be archived as, where its
/// bytes are, and the digest that enters the freshness fingerprint.
struct CensusEntry {
    name: String,
    absolute: std::path::PathBuf,
    digest: String,
}

/// The complete, sorted archive census of one target.
///
/// A FILE input contributes one entry under its own declared name; a
/// DIRECTORY input contributes its canonical walk, nested under the input's
/// name, so two directory inputs can never collide and each archived tree
/// keeps the identity it was consumed by. Two inputs that would archive to
/// one name refuse: an archive with two claimants for one entry is exactly
/// the silent-overwrite this plane refuses everywhere else.
fn census(
    request: &PackageTargetRequest<'_>,
    config: &WindowsZipConfig,
) -> Result<Vec<CensusEntry>, MechanismError> {
    let target = &request.target.id;
    let mut census: Vec<CensusEntry> = Vec::new();
    for input in request.inputs {
        match input.shape {
            ArtifactShape::File => census.push(CensusEntry {
                name: config.placed(&archived_name(input)),
                absolute: input.absolute.clone(),
                digest: input.digest.clone(),
            }),
            ArtifactShape::Directory => {
                let walked =
                    walk_tree(&input.absolute).map_err(|fault| MechanismError::SourceMissing {
                        target: target.clone(),
                        provider: BUILTIN_WINDOWS_ZIP_PIN.to_owned(),
                        path: preview(&input.relative),
                        reason: format!("{}: {}", fault.path, fault.reason),
                    })?;
                for (relative, absolute) in walked {
                    let (digest, _) =
                        digest_file(&absolute).map_err(|fault| MechanismError::SourceMissing {
                            target: target.clone(),
                            provider: BUILTIN_WINDOWS_ZIP_PIN.to_owned(),
                            path: preview(&relative),
                            reason: fault.reason(),
                        })?;
                    census.push(CensusEntry {
                        name: config.placed(&format!("{}/{relative}", archived_name(input))),
                        absolute,
                        digest,
                    });
                }
            }
        }
    }
    census.sort_by(|left, right| left.name.cmp(&right.name));
    for pair in census.windows(2) {
        let [left, right] = pair else { continue };
        if left.name == right.name {
            return Err(MechanismError::Config {
                target: target.clone(),
                member: "inputs".to_owned(),
                reason: format!(
                    "two declared inputs both archive as `{}`; an archive never has two claimants \
                     for one entry name",
                    preview(&left.name)
                ),
            });
        }
    }
    Ok(census)
}

/// The archived name one input contributes under.
///
/// The input's own identity, forward-slashed: an artifact id for a
/// consumed artifact, the canonical relative spelling for a workspace
/// path. Deriving it rather than accepting a per-input rename keeps the
/// archive's contents a function of the declaration alone.
fn archived_name(input: &ResolvedInput) -> String {
    input.name.replace('\\', "/")
}

/// The one distributable file name, inside the target's own engine-owned
/// package directory.
fn archive_name(target: &str) -> String {
    format!("{target}.zip")
}

/// The validated windows-zip config carried on the plan.
fn zip_config(plan: &PackagePlan) -> Result<&WindowsZipConfig, MechanismError> {
    match &plan.config {
        PackageConfig::WindowsZip(config) => Ok(config),
        PackageConfig::StaticFile(_)
        | PackageConfig::StaticSkill(_)
        | PackageConfig::AgentPlugin(_)
        | PackageConfig::ClientProjection(_) => Err(MechanismError::PlanRoleMismatch {
            provider: BUILTIN_WINDOWS_ZIP_PIN.to_owned(),
        }),
    }
}

#[cfg(test)]
#[path = "zip/tests.rs"]
mod tests;
