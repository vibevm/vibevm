//! The three builtin client-plugin projection providers — §6.3, in
//! process.
//!
//! > "The canonical Agent Plugin and a client-native projection are
//! > distinct package artifacts. Client adaptation happens in `package`,
//! > where it is reproducible and verifiable; `deploy` only installs the
//! > selected projection."
//!
//! So a projection is ordinary PACKAGE work: it consumes one recorded
//! artifact, emits one recorded directory, touches no home, spawns no
//! client, reads no token and reaches no network. Everything the deploy
//! lane owns — marketplaces, CLI argv, config merges, receipts — is absent
//! here by construction, not by discipline.
//!
//! **One implementation, three pinned providers.** §6.3.0.2 ships three
//! registry rows (`package:claude-plugin`, `package:codex-plugin`,
//! `package:opencode-plugin`) whose provider ids differ from their logical
//! names. What differs BETWEEN them is data — a manifest directory, an MCP
//! shape — so it lives in [`client::ProjectionClient`] and the adapter is
//! written once. Three copies of one provider would be three places for
//! §6.3's frozen shapes to drift; a `client = "…"` config member would put
//! provider selection inside the table the routing law sits above.
//!
//! **The admission law is provenance, not resemblance.** §6.3.0.3 gives a
//! projection "exactly one recorded `agent-plugin` directory artifact".
//! A workspace path has no recorded kind, and a recorded plain `directory`
//! — which is exactly what a projection itself produces — is not a
//! canonical plugin. Both refuse before anything is prepared, so a
//! projection can never be fed a projection.
//!
//! **Adapter epoch 1.** §6.3.0.4 requires every projection to "record
//! adapter epoch 1 in its fingerprint/evidence". The epoch is the version
//! of THIS translation, not of any client: it is what makes a re-projection
//! by a later adapter a different artifact even when the source and the
//! config did not move.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

pub(crate) mod client;
pub(crate) mod config;
pub(crate) mod error;
mod opencode;
mod source;

pub use error::ClientProjectionError;

use crate::mechanism::contain::{read_file_bounded, tree_digest};
use crate::mechanism::error::preview;
use crate::mechanism::package::contained_identity;
use crate::mechanism::package::protocol::{
    PackageConfig, PackageFingerprint, PackagePlan, PlannedPackageOutput, ResolvedInput,
    StagedArtifact, VerifiedPackageArtifact,
};
use crate::mechanism::plugin::STAGE_CAP;
use crate::mechanism::skill::{supported, write_distributable};
use crate::mechanism::{
    EffectClass, MechanismError, NetworkUse, PackageProvider, PackageTargetRequest, PrivilegeNeed,
    ProviderDescriptor, ProviderOperation, Reversibility,
};
use client::ProjectionClient;
use config::ClientProjectionConfig;
use source::{EmittedBytes, Projection, read_projection};

/// The version of THIS adapter family's translation — §6.3.0.4's "adapter
/// epoch 1", recorded in every fingerprint and every evidence line.
pub(crate) const ADAPTER_EPOCH: u32 = 1;

/// The fingerprint's domain separator: the family, then its epoch.
const FINGERPRINT_DOMAIN: &str = "client-plugin-projection/1";

/// The artifact kinds a projection produces.
///
/// A plain `directory`, deliberately NOT `agent-plugin`: a projection is a
/// client-native tree, and §6.2 keeps it "distinct" from the canonical
/// plugin. The kinds are what §6.3.0.3's admission law reads, so recording
/// a projection as an Agent Plugin would let one be fed to another
/// projection — the exact confusion the typed provenance exists to end.
const PRODUCED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Directory];

/// The §3.2 operations a package-role provider implements.
const PACKAGE_OPERATIONS: [ProviderOperation; 4] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
];

/// One builtin client-plugin projection provider.
///
/// The client is a construction parameter rather than a config member, so
/// the three registry rows dispatch to three distinct values of one type
/// and each answers under its own pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientProjectionProvider {
    client: ProjectionClient,
}

impl ClientProjectionProvider {
    /// The provider that projects for one client.
    pub(crate) const fn new(client: ProjectionClient) -> Self {
        Self { client }
    }

    /// The one canonical Agent Plugin this target projects — §6.3.0.3's
    /// admission law, and the earliest refusal this adapter has.
    ///
    /// It reads nothing: the ENGINE already resolved the input, re-proved
    /// its bytes and attached the kind its record declares (§6.0.2), so the
    /// gate is a question about provenance and never about what a directory
    /// happens to look like.
    fn canonical<'a>(
        &self,
        request: &'a PackageTargetRequest<'_>,
    ) -> Result<&'a ResolvedInput, MechanismError> {
        let input = match request.inputs {
            [only] => only,
            other => {
                return Err(ClientProjectionError::InputCount {
                    target: request.target.id.clone(),
                    provider: self.client.pin(),
                    found: other.len(),
                }
                .into());
            }
        };
        let refuse = |found: String| {
            MechanismError::from(ClientProjectionError::InputNotAgentPlugin {
                target: request.target.id.clone(),
                client: self.client.as_str(),
                input: preview(&input.name),
                found,
            })
        };
        match input.origin.recorded_kind() {
            Some(ArtifactKind::AgentPlugin) => {}
            Some(kind) => {
                return Err(refuse(format!("a recorded `{}` artifact", kind.as_str())));
            }
            None => {
                return Err(refuse(
                    "a workspace source path, which carries no recorded kind at all".to_owned(),
                ));
            }
        }
        if input.shape != ArtifactShape::Directory {
            return Err(ClientProjectionError::InputNotDirectory {
                target: request.target.id.clone(),
                input: preview(&input.name),
                shape: "file",
            }
            .into());
        }
        Ok(input)
    }

    /// The resolved projection of this target's canonical input.
    fn projection(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Projection, MechanismError> {
        let config = projection_config(self.client, plan)?;
        let canonical = self.canonical(request)?;
        read_projection(
            &request.target.id,
            request.project_root,
            self.client,
            config,
            &canonical.relative,
        )
    }
}

impl PackageProvider for ClientProjectionProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            key: self.client.pin(),
            kinds: &PRODUCED_KINDS,
            // A projection writes inside the engine-owned package root and
            // nowhere else: no home, no client state, no network, no
            // privilege, and nothing to reverse because it reconciles no
            // destination. §6.3.0.1 puts every one of those in the DEPLOY
            // lane, and this descriptor is that boundary, declared.
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
        let config = ClientProjectionConfig::parse(&target.id, target.config.as_ref())?;
        // Provenance BEFORE the source is opened: an input this adapter may
        // not project is refused without reading a byte of it.
        let canonical = self.canonical(request)?;
        if target.outputs.len() != 1 {
            return Err(MechanismError::OutputCount {
                target: target.id.clone(),
                provider: descriptor.key.to_owned(),
                expected: "exactly one `directory` output — a client projection is one \
                           client-native tree"
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
        // The capability report is part of PLANNING: §6.3.0.11 keeps plan
        // read-only, and a requested component the plugin cannot supply —
        // or a member this client cannot express — refuses here, before the
        // engine prepares an output directory for a projection that will
        // never exist.
        let projection = read_projection(
            &target.id,
            request.project_root,
            self.client,
            &config,
            &canonical.relative,
        )?;
        Ok(PackagePlan {
            summary: format!(
                "{} projection (adapter epoch {ADAPTER_EPOCH}) of `{}` {} with components [{}]; \
                 emits {}",
                self.client.as_str(),
                preview(&projection.name),
                preview(&projection.version),
                config.rendered(),
                projection.census(),
            ),
            output_dir: request.output_dir(),
            inputs: request
                .inputs
                .iter()
                .map(|input| input.reference.clone())
                .collect(),
            outputs,
            config: PackageConfig::ClientProjection(config),
        })
    }

    /// The engine-fresh fingerprint over the projection's COMPLETE closed
    /// input set.
    ///
    /// The set really is closed, which is why §6.3.0.2 rules the projection
    /// rows engine-fresh: the canonical plugin enters as ONE value — the
    /// tree digest the engine re-proved against the bytes on disk before
    /// this provider saw it — and the only other inputs are the adapter's
    /// own identity and the requested component set.
    ///
    /// Client and epoch are hashed as named fields, so two clients cannot
    /// produce one value for one source (§6.3.0.4: "The three fingerprints
    /// must differ for identical source/config") and a later adapter epoch
    /// cannot reuse this one's.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<PackageFingerprint, MechanismError> {
        let config = projection_config(self.client, plan)?;
        let canonical = self.canonical(request)?;
        let projection = self.projection(request, plan)?;
        let mut hash = Sha256::new();
        for (field, value) in [
            ("client", self.client.as_str().to_owned()),
            ("adapter-epoch", ADAPTER_EPOCH.to_string()),
            ("plugin", canonical.digest.clone()),
            ("name", projection.name.clone()),
            ("version", projection.version.clone()),
        ] {
            hash.update(FINGERPRINT_DOMAIN.as_bytes());
            hash.update(b"\x00");
            hash.update(field.as_bytes());
            hash.update(b"\x00");
            hash.update(value.as_bytes());
            hash.update(b"\x00");
        }
        for component in config.components() {
            hash.update(FINGERPRINT_DOMAIN.as_bytes());
            hash.update(b"\x00component\x00");
            hash.update(component.as_str().as_bytes());
            hash.update(b"\x00");
        }
        Ok(PackageFingerprint {
            digest: format!("{:x}", hash.finalize()),
            counted: request.inputs.len(),
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn apply(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Vec<StagedArtifact>, MechanismError> {
        let target = &request.target.id;
        let projection = self.projection(request, plan)?;
        let root = request.output_dir_relative();
        for file in &projection.emitted {
            let bytes = match &file.bytes {
                EmittedBytes::Canonical {
                    absolute,
                    canonical,
                } => read_file_bounded(absolute, STAGE_CAP).map_err(|fault| {
                    MechanismError::SourceMissing {
                        target: target.clone(),
                        provider: self.client.pin().to_owned(),
                        path: preview(canonical),
                        reason: fault.reason(),
                    }
                })?,
                EmittedBytes::Rendered(rendered) => rendered.clone(),
            };
            write_distributable(request, &format!("{root}/{}", file.relative), &bytes)?;
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

/// The validated projection config carried on the plan.
fn projection_config(
    client: ProjectionClient,
    plan: &PackagePlan,
) -> Result<&ClientProjectionConfig, MechanismError> {
    match &plan.config {
        PackageConfig::ClientProjection(config) => Ok(config),
        PackageConfig::StaticSkill(_)
        | PackageConfig::AgentPlugin(_)
        | PackageConfig::WindowsZip(_) => Err(MechanismError::PlanRoleMismatch {
            provider: client.pin().to_owned(),
        }),
    }
}

#[cfg(test)]
#[path = "client_projection/support.rs"]
pub(crate) mod support;

#[cfg(test)]
#[path = "client_projection/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "client_projection/record_tests.rs"]
mod record_tests;

#[cfg(test)]
#[path = "client_projection/shape_tests.rs"]
mod shape_tests;

#[cfg(test)]
#[path = "client_projection/mcp_tests.rs"]
mod mcp_tests;
