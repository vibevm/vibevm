//! `deploy:{claude,codex,opencode}-skill` — §6.3.0.5's three standalone
//! skill destinations, ONE closed provider parameterised by
//! [`SkillClient`].
//!
//! §6.3.0.5 in one paragraph: the three rows "accept only a file-shaped
//! `skill` artifact plus strict `config={name=…}` and own exactly one
//! entry file under `.claude/skills/<name>/SKILL.md`,
//! `.agents/skills/<name>/SKILL.md`, or
//! `.config/opencode/skills/<name>/SKILL.md`. They never own or remove an
//! unrecorded neighbour and never route a Codex/OpenCode selection through
//! the same shared physical path by convenience."
//!
//! The cell is split by responsibility, as the packet's own law asks:
//! [`client`] is the closed client vocabulary; [`config`] is the strict
//! one-member table; [`entry`] is the entry destination — identity,
//! observation, the occupant judgement, publication and pruning; THIS
//! file is the provider itself: admission, the six §3.2 verbs, and the
//! digests. The laws worth finding in code rather than prose:
//!
//! 1. **admission is provenance, not resemblance** — a recorded
//!    `kind=skill, shape=file` artifact only, its bytes read as bounded
//!    UTF-8 and parsed through the ONE existing Agent Skills frontmatter
//!    reader, with the frontmatter name required to equal the config's
//!    ([`SkillDeployProvider::admit`]);
//! 2. **desired bytes are the exact proven artifact bytes** — no rewrite,
//!    no generated header, no line-ending work;
//! 3. **`apply` repeats the occupant judgement under the engine's locks
//!    immediately before its first write** — plan evidence is not write
//!    authority (the judgement itself lives in [`entry`]). The plan-time
//!    judgement alone may also recognise §7.2's INTERRUPTED occupant
//!    through the injected durable intent — recovery occupancy, never
//!    ownership — so the next ordinary run reaches `recover` instead of
//!    refusing its own crash window shut;
//! 4. **`recover` is idempotent roll-forward** — the engine proved the
//!    three-digest law before calling it, so an already-desired entry is
//!    a no-op and anything else is reconciled; the first-apply occupant
//!    gate deliberately does not run there;
//! 5. **`remove` needs the injected current receipt to own every
//!    requested entry** and prunes only proven-empty directories, never
//!    past the named skill directory.
//!
//! Nothing here reads a token, opens a socket, spawns a client or touches
//! a settings-root marketplace: `ClientExecutables` may be all `Missing`
//! and these three providers still work, because a skill destination is a
//! documented filesystem projection.
//!
//! [`entry`]: self::entry

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use sha2::{Digest, Sha256};
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::contain::read_file_bounded;
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource,
    PlannedDeployResource, RemoveReport, ResolvedDeployArtifact,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::vibebin::store;
use crate::mechanism::{
    DeployProvider, DeployTargetRequest, EffectClass, MechanismError, NetworkUse, PrivilegeNeed,
    ProviderDescriptor, ProviderOperation, Reversibility,
};

pub(crate) mod client;
pub(crate) mod config;
mod entry;

pub(crate) use client::SkillClient;
use config::SkillDeployConfig;
use entry::{destination_of, frontmatter_of};

/// The version of THIS adapter family's translation — recorded in every
/// fingerprint, exactly as the projection family's §6.3.0.4 epoch is.
pub(crate) const ADAPTER_EPOCH: u32 = 1;

/// The fingerprint's domain separator: the family, then its epoch.
const FINGERPRINT_DOMAIN: &str = "skill-deploy/1";

/// The config digest's domain separator.
const CONFIG_DOMAIN: &str = "skill-deploy-config/1";

/// The resource-identity prefix that roots an owned member in the
/// INJECTED user home — the identity side of §6.3.0.9's owned/locked
/// resource vocabulary for a user-scope destination.
pub(super) const HOME_PREFIX: &str = "home:";

/// The largest entry document this provider will read or place.
const ENTRY_CAP: u64 = 4 * 1024 * 1024;

/// The six §3.2 operations this provider implements — every one of them.
const DEPLOY_OPERATIONS: [ProviderOperation; 6] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
    ProviderOperation::Remove,
    ProviderOperation::Recover,
];

/// The one artifact kind §6.3.0.5 admits, with the one physical shape.
const SUPPORTED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Skill];

/// The same list, rendered for the refusal.
const SUPPORTED_KINDS_LIST: &str = "skill";

/// One builtin standalone-skill deploy provider, for one closed client.
///
/// The client is a construction parameter rather than a config member, so
/// the three registry rows dispatch to three values of one type and each
/// answers under its own pin — the projection family's landed shape, not
/// a second opinion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillDeployProvider {
    client: SkillClient,
}

/// One target's resolved destination — the owned resource identity, its
/// home-relative spelling, and the skill's identity.
///
/// Derived in one place ([`destination_of`]) because `plan`, `apply`,
/// `recover` and `remove` must agree about them byte-for-byte. The
/// ABSOLUTE entry path is deliberately not cached here: it is answered
/// per call by the PURE injected-home helper (§6.3.1.7).
pub(super) struct Destination {
    /// The skill's portable identity.
    pub(super) name: String,
    /// `home:.claude/skills/<name>/SKILL.md` — the owned and locked
    /// resource identity, forward-slashed and home-rooted.
    pub(super) resource: String,
    /// `.claude/skills/<name>/SKILL.md` — the same member as a
    /// home-relative path, for the audited publish/remove primitives.
    pub(super) relative: String,
}

/// What the occupant judgement concluded about the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Occupancy {
    /// Nothing is there — a first deployment may create.
    Absent,
    /// This deployment's prior receipt owns the entry at the digest it
    /// recorded — an update may run.
    Owned,
    /// §7.2's interrupted FIRST deployment: the entry holds this
    /// deployment's own desired bytes — proven by the injected durable
    /// intent naming the exact resource at the observed digest, with no
    /// receipt anywhere — because the run that wrote it died between
    /// publishing and finalising one.
    ///
    /// This is RECOVERY occupancy, never ownership: it admits a plan (so
    /// the next ordinary run reaches the settlement that completes the
    /// interrupted generation), and it authorizes no write. Settlement
    /// itself remains the transaction's, decided by plan hash; the
    /// apply-time recheck never sees an intent. Settling this window
    /// finalises the interrupted generation as a first deployment, whose
    /// inverse is removal — reversibility stays honest.
    Interrupted,
    /// §7.2's interrupted UPDATE: the same exact intent evidence, over a
    /// receipt that still owns the entry at the PRIOR generation's
    /// digest — the crashed run published the new bytes and died before
    /// the receipt moved to describe them.
    ///
    /// Recovery occupancy exactly as [`Occupancy::Interrupted`] — never
    /// ownership, never write authority — with one honest difference:
    /// settling it finalises an update whose prior bytes are already
    /// gone, so the generation it completes is as irreversible as the
    /// update it completes.
    InterruptedUpdate,
}

impl SkillDeployProvider {
    /// The provider for one client's skill row.
    pub(crate) const fn new(client: SkillClient) -> Self {
        Self { client }
    }

    /// Resolve the destination, prove the artifact, and validate identity
    /// — the shared preamble of `plan`, `apply` and `recover`.
    ///
    /// The desired bytes are read HERE, once, as bounded UTF-8 and parsed
    /// through the ONE existing Agent Skills frontmatter reader (the
    /// producer's own parser, narrowly re-homed for exactly this reuse),
    /// because §6.3.1.6's "require frontmatter identity to match" is a
    /// judgement about the artifact's own document, made before any
    /// destination is touched.
    fn admit<'a>(
        &self,
        request: &'a DeployTargetRequest<'_>,
    ) -> Result<(Destination, &'a ResolvedDeployArtifact, Vec<u8>), DeployProviderError> {
        let target = &request.target.id;
        let config = SkillDeployConfig::parse(target, request.target.config.as_ref())?;
        let destination = destination_of(self.client, &config);
        // The pure Agent helper is the filesystem authority; prove its
        // home-relative answer is exactly the resource identity this plan will
        // lock and record before reading an artifact or touching a destination.
        self.exact_entry_relative(request, &destination)?;
        let artifact = request
            .artifact
            .ok_or_else(|| DeployProviderError::NoArtifact {
                target: target.clone(),
                provider: self.client.pin(),
            })?;
        // §6.3.0.5's admission law: "a file-shaped `skill` artifact". A
        // recorded plain `file` is not a skill by resemblance, and a
        // directory is §6.1's separate package kind — both refuse by name
        // before a destination write.
        if !SUPPORTED_KINDS.contains(&artifact.kind) {
            return Err(DeployProviderError::ArtifactKind {
                target: target.clone(),
                artifact: artifact.id.clone(),
                provider: self.client.pin(),
                kind: artifact.kind.as_str(),
                supported: SUPPORTED_KINDS_LIST,
            });
        }
        if artifact.shape != ArtifactShape::File {
            return Err(DeployProviderError::SkillShape {
                target: target.clone(),
                artifact: artifact.id.clone(),
                provider: self.client.pin(),
            });
        }
        let bytes = read_file_bounded(&artifact.absolute, ENTRY_CAP).map_err(|fault| {
            DeployProviderError::SkillUnreadable {
                target: target.clone(),
                artifact: artifact.id.clone(),
                reason: fault.reason(),
            }
        })?;
        let document =
            String::from_utf8(bytes).map_err(|_| DeployProviderError::SkillUnreadable {
                target: target.clone(),
                artifact: artifact.id.clone(),
                reason: "the bytes are not valid UTF-8, so they are not an Agent Skills entry \
                         document"
                    .to_owned(),
            })?;
        let parsed = frontmatter_of(target, &artifact.id, &document)?;
        if parsed != config.name {
            return Err(DeployProviderError::SkillName {
                target: target.clone(),
                artifact: artifact.id.clone(),
                declared: parsed,
                config: config.name,
            });
        }
        Ok((destination, artifact, document.into_bytes()))
    }

    /// The digest of the desired CONFIG — binds the client and the name.
    ///
    /// The client is folded in because the same name under two clients is
    /// two destinations and therefore two configs; the engine already
    /// binds the artifact digest and the provider pin.
    fn config_digest(&self, request: &DeployTargetRequest<'_>) -> String {
        let config = SkillDeployConfig::parse(&request.target.id, request.target.config.as_ref());
        let name = config.map_or_else(|_| "<invalid>".to_owned(), |config| config.name);
        let mut hash = Sha256::new();
        hash.update(CONFIG_DOMAIN.as_bytes());
        hash.update(b"\x00client\x00");
        hash.update(self.client.as_str().as_bytes());
        hash.update(b"\x00name\x00");
        hash.update(name.as_bytes());
        format!("{:x}", hash.finalize())
    }
}

impl DeployProvider for SkillDeployProvider {
    fn descriptor(&self) -> DeployDescriptor {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: self.client.pin(),
                kinds: &SUPPORTED_KINDS,
                // §6.3's commissioning matrix: the user-scope skill roots
                // live in the invoking user's home, never a workspace and
                // never a machine-wide prefix.
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                // The capability exists (removal undoes a first
                // deployment); the PER-PLAN answer below is the honest
                // one, and §3.2 asks for it before apply.
                reversibility: Reversibility::Reversible,
                operations: &DEPLOY_OPERATIONS,
            },
            // The entry is published by staged rename, so the destination
            // supports atomic replacement and §7.2's staging sentence
            // applies.
            atomic_replacement: true,
            // §6.3.0.9: "A normal provider's lock resources equal its
            // owned resources." This provider owns exactly one whole file
            // under the injected home and shares no document, so it locks
            // exactly what it owns and a second claimant is §7.2's flat
            // collision.
            reference_ownership: false,
        }
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        let (destination, artifact, _) = self.admit(request)?;
        // The occupant is read HERE too, read-only, so `--plan` reports
        // the refusal an apply would raise instead of promising a write
        // that cannot happen. It creates nothing — not the home, not the
        // skills root, not the entry. The plan-time judgement is the one
        // seat that may also recognise §7.2's interrupted occupant through
        // the injected durable intent; `apply` below deliberately runs the
        // strict, receipt-only twin.
        let occupancy = self.plan_occupancy(request, &destination)?;
        let resources = vec![PlannedDeployResource {
            desired_digest: artifact.digest.clone(),
            resource: destination.resource.clone(),
        }];
        // The lock set IS the owned set: no reference ownership declared.
        let lock_resources = vec![destination.resource.clone()];
        Ok(DeployPlan {
            summary: format!(
                "{} skill `{}` as {} from the proven {} bytes",
                self.client.as_str(),
                destination.name,
                destination.resource,
                artifact.id,
            ),
            resources,
            lock_resources,
            config_digest: self.config_digest(request),
            // §3.2's declaration, per plan: creating the entry is undone
            // by removing it, an update over a receipt-owned entry holds
            // no prior bytes to restore and says so BEFORE apply, and an
            // interrupted occupant settles as the generation it actually
            // completes — a stranded FIRST deployment as its own
            // generation 0 (removal reverses it exactly as it reverses a
            // first deployment), a stranded UPDATE over a prior receipt
            // exactly as irreversibly as the update it finishes.
            reversible: matches!(occupancy, Occupancy::Absent | Occupancy::Interrupted),
        })
    }

    fn fingerprint(
        &self,
        _request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        // §4.1's "provider portion": THIS adapter's identity — the client
        // and the translation epoch — and deliberately nothing about the
        // artifact or the name, which the engine already folds in (the
        // artifact digest through the plan hash, the name through the
        // config digest).
        let mut hash = Sha256::new();
        hash.update(FINGERPRINT_DOMAIN.as_bytes());
        hash.update(b"\x00client\x00");
        hash.update(self.client.as_str().as_bytes());
        hash.update(b"\x00adapter-epoch\x00");
        hash.update(ADAPTER_EPOCH.to_string().as_bytes());
        Ok(DeployFingerprint {
            digest: format!("{:x}", hash.finalize()),
            summary: format!(
                "the {} skill destination, adapter epoch {ADAPTER_EPOCH}",
                self.client.as_str(),
            ),
        })
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let (destination, artifact, bytes) = self.admit(request)?;
        // §6.3.1's recheck: the occupant judgement runs AGAIN here, inside
        // the deployment and destination locks the engine holds around
        // this call, immediately before the first write. Plan evidence
        // alone is never write authority — an occupant that appeared (or
        // drifted) between plan and apply refuses now, and the recovery
        // intent that may have admitted the plan is deliberately absent
        // from this request, so the recheck is receipt-only.
        self.admit_occupant(request, &destination)?;
        self.publish(request, &destination, &bytes, checkpoint)?;
        Ok(ApplyReport {
            prior_state_handle: None,
            evidence: format!(
                "{} skill `{}`: published the exact {} bytes of `{}` at {}; the destination was \
                 judged under the deployment locks immediately before the write",
                self.client.as_str(),
                destination.name,
                artifact.digest,
                artifact.id,
                destination.resource,
            ),
        })
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        let mut observed = Vec::with_capacity(resources.len());
        for resource in resources {
            observed.push(ObservedResource {
                digest: self.observe(request, resource)?,
                resource: resource.clone(),
            });
        }
        Ok(observed)
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        _prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        let target = &request.target.id;
        let config = SkillDeployConfig::parse(target, request.target.config.as_ref())?;
        let destination = destination_of(self.client, &config);
        // §6.3.1's remove law: the injected CURRENT receipt must own every
        // requested entry. The engine has already proven the drift law;
        // this is the provider's own half of the ownership sentence, and
        // an entry the receipt does not name is never this provider's to
        // delete.
        let receipt = request.prior_receipt.ok_or_else(|| {
            MechanismError::Deploy(DeployProviderError::RemoveNotOwned {
                target: target.clone(),
                resource: destination.resource.clone(),
            })
        })?;
        // This provider owns exactly the configured entry. A receipt and a
        // caller agreeing on some other string is not authority to broaden
        // that perimeter — especially because removal is a mutation and the
        // lower store primitive intentionally assumes its caller supplied a
        // contained relative path.
        if resources != std::slice::from_ref(&destination.resource) {
            let refused = resources
                .iter()
                .find(|resource| *resource != &destination.resource)
                .cloned()
                .unwrap_or_else(|| destination.resource.clone());
            return Err(MechanismError::Deploy(
                DeployProviderError::RemoveNotOwned {
                    target: target.clone(),
                    resource: refused,
                },
            ));
        }
        if !receipt
            .resources
            .iter()
            .any(|owned| owned.resource == destination.resource)
        {
            return Err(MechanismError::Deploy(
                DeployProviderError::RemoveNotOwned {
                    target: target.clone(),
                    resource: destination.resource.clone(),
                },
            ));
        }
        // Only the configured entry document — never a caller-derived path,
        // directory or neighbour. Absence is success.
        let mut removed = Vec::with_capacity(1);
        if store::remove_resource(target, request.user_home, &destination.relative)? {
            removed.push(destination.resource.clone());
        }
        self.prune(request, &destination)?;
        Ok(RemoveReport {
            expected_remaining: Vec::new(),
            evidence: format!(
                "{} skill `{}`: removed the receipt-owned entry {}; only proven-empty \
                 directories under the named skill directory were pruned and every foreign \
                 neighbour stayed byte-identical",
                self.client.as_str(),
                destination.name,
                destination.resource,
            ),
            removed,
        })
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        let (destination, artifact, bytes) = self.admit(request)?;
        // Idempotent roll-forward. The engine already proved every
        // observed resource holds the prior or the desired digest, so an
        // entry already at the desired digest is a completed write this
        // recovery only checkpoints; anything else is the interrupted
        // write completed. The first-apply occupant gate deliberately does
        // NOT run here: its question is "may a FIRST write take this
        // name", and the intent journal this recovery completes already
        // answered it when it was opened before that write began.
        let desired = &artifact.digest;
        let already = observed.iter().any(|seen| {
            seen.resource == destination.resource && seen.digest.as_ref() == Some(desired)
        });
        if !already {
            self.publish(request, &destination, &bytes, checkpoint)?;
        } else {
            checkpoint.completed(&destination.resource)?;
        }
        Ok(ApplyReport {
            prior_state_handle: None,
            evidence: format!(
                "{} skill `{}`: recovered {} — the entry {} the interrupted apply left it at",
                self.client.as_str(),
                destination.name,
                destination.resource,
                if already {
                    "was already desired and stayed"
                } else {
                    "now holds the desired bytes"
                },
            ),
        })
    }
}

// The suite's shared world, the provider-law cell, the interrupted-window
// cells and the engine-driven lifecycle cells — five questions, five
// files.
#[cfg(test)]
#[path = "skill/support.rs"]
pub(crate) mod support;

#[cfg(test)]
#[path = "skill/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "skill/intent_tests.rs"]
mod intent_tests;

#[cfg(test)]
#[path = "skill/lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "skill/update_recovery_tests.rs"]
mod update_recovery_tests;

#[cfg(test)]
#[path = "skill/containment_tests.rs"]
mod containment_tests;
