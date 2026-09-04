//! The hermetic deploy PROVIDER — one in-process implementation of all six
//! §3.2 verbs, with declarable failure points.
//!
//! §7.0.2 rules that "the only executing implementations this atom are
//! hermetic fixtures at the unit seam", and this is that seam. The provider
//! below is a real [`DeployProvider`]: it keeps its whole destination
//! inside a temp directory the test owns, reads no environment and reaches
//! no network. What makes it a *fixture* rather than a shipped provider is
//! only that it is compiled under `cfg(test)` and that its failure points
//! are declarable — a real provider fails when the world fails, and a law
//! about crash windows needs the world to fail on cue.
//!
//! The world it runs inside — the temp trees, the recorded artifact, the
//! execution — is [`support`](super::support) next door.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::model::ClientExecutables;
use super::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource,
    PlannedDeployResource, RemoveReport, ResolvedDeployArtifact,
};
use super::state::CheckpointLedger;
use crate::mechanism::{
    DeployProvider, DeployTargetRequest, EffectClass, MechanismError, NetworkUse, PrivilegeNeed,
    ProviderDescriptor, ProviderOperation, Reversibility,
};

/// The pin the fixture provider answers under. It is a plausible installed
/// identity, never the reserved engine owner: nothing in this suite
/// pretends to be a builtin.
pub(crate) const FIXTURE_PIN: &str = "org.example/deployers#fixture";

/// The six §3.2 operations a deploy provider implements.
const DEPLOY_OPERATIONS: [ProviderOperation; 6] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
    ProviderOperation::Remove,
    ProviderOperation::Recover,
];

/// How the fixture should misbehave, if at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Faults {
    /// Refuse `plan` — the pre-apply window, which §6.3.0.10 says must
    /// close before ANY target's apply begins.
    pub(crate) fail_plan: bool,
    /// Fail `apply` after this many resources have been checkpointed —
    /// the crash window BETWEEN the intent and the receipt, and between
    /// two checkpoints when the value is neither zero nor the full set.
    pub(crate) fail_apply_after: Option<usize>,
    /// Write the wrong bytes, so INDEPENDENT verify disagrees with what
    /// apply claimed.
    pub(crate) corrupt: bool,
    /// Refuse `remove`, so a saga rollback cannot complete.
    pub(crate) fail_remove: bool,
}

/// One hermetic destination provider.
pub(crate) struct FixtureProvider {
    /// The destination root — a directory the test owns.
    root: PathBuf,
    /// The resource identities this provider reconciles, in plan order.
    resources: Vec<String>,
    /// The PHYSICAL identities it locks. `None` is the ordinary provider:
    /// the lock set is the owned set, computed rather than declared, so a
    /// fixture cannot drift from the law it is proving.
    locks: Option<Vec<String>>,
    /// Whether the destination supports atomic replacement, and therefore
    /// whether the engine offers a staging directory.
    atomic: bool,
    /// Whether this provider owns a logical member of a shared physical
    /// destination — §6.3.0.9's declared capability.
    references: bool,
    /// Whether this provider can undo an apply.
    reversible: bool,
    faults: Faults,
    /// Every verb call, in order — the witness the ordering laws read.
    calls: RefCell<Vec<String>>,
    /// The injected authority this provider was HANDED, once per `plan`.
    ///
    /// §6.3.0.6's law is negative — no cell below the surface resolves a
    /// home or finds a client — and a negative is proven by showing the
    /// value arrived instead. Recorded rather than asserted inline so a
    /// test can compare it against the temp roots it created.
    authority: RefCell<Vec<(PathBuf, ClientExecutables)>>,
    /// The PRIOR RECEIPT this provider was handed, once per `plan`.
    ///
    /// §6.3.1.1's law is positive and the recording is the whole proof: an
    /// engine that read its state home and then dropped the answer on the
    /// floor is indistinguishable from one that never read it, unless the
    /// value is observed where a provider would use it.
    priors: RefCell<Vec<Option<DeployReceipt>>>,
}

impl FixtureProvider {
    /// A reversible, non-staging provider over one resource.
    pub(crate) fn new(root: &Path, resources: &[&str]) -> Self {
        Self {
            root: root.to_path_buf(),
            resources: resources.iter().map(|name| (*name).to_owned()).collect(),
            locks: None,
            atomic: false,
            references: false,
            reversible: true,
            faults: Faults::default(),
            calls: RefCell::new(Vec::new()),
            authority: RefCell::new(Vec::new()),
            priors: RefCell::new(Vec::new()),
        }
    }

    /// The same provider, declaring that its destination supports atomic
    /// replacement — so the engine prepares staging for it.
    pub(crate) fn staging(mut self) -> Self {
        self.atomic = true;
        self
    }

    /// The same provider, declaring reference ownership over a shared
    /// physical destination it locks but does not own outright — the
    /// §6.3.0.9 shape a client config-entry provider will have.
    pub(crate) fn referencing(mut self, locks: &[&str]) -> Self {
        self.references = true;
        self.locks = Some(locks.iter().map(|name| (*name).to_owned()).collect());
        self
    }

    /// The same provider, locking something other than what it owns while
    /// declaring NO reference ownership — the shape the engine must refuse.
    pub(crate) fn mislocking(mut self, locks: &[&str]) -> Self {
        self.locks = Some(locks.iter().map(|name| (*name).to_owned()).collect());
        self
    }

    /// The same provider, declaring itself irreversible.
    pub(crate) fn irreversible(mut self) -> Self {
        self.reversible = false;
        self
    }

    /// The same provider, with a declared fault.
    pub(crate) fn faulty(mut self, faults: Faults) -> Self {
        self.faults = faults;
        self
    }

    /// The verb call log.
    pub(crate) fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    /// The injected home and client executables this provider was handed.
    pub(crate) fn authority(&self) -> Vec<(PathBuf, ClientExecutables)> {
        self.authority.borrow().clone()
    }

    /// The prior receipts this provider was handed, one per `plan`.
    pub(crate) fn priors(&self) -> Vec<Option<DeployReceipt>> {
        self.priors.borrow().clone()
    }

    /// The absolute path one resource identity names.
    pub(crate) fn destination(&self, resource: &str) -> PathBuf {
        self.root.join(resource)
    }

    /// The bytes this provider would put at one resource, given an
    /// artifact. Deterministic, so a plan and an apply agree.
    fn desired_bytes(artifact: Option<&ResolvedDeployArtifact>, resource: &str) -> Vec<u8> {
        let digest = artifact.map_or_else(|| "no-artifact".to_owned(), |a| a.digest.clone());
        format!("{digest}\n{resource}\n").into_bytes()
    }

    fn digest(bytes: &[u8]) -> String {
        let mut hash = Sha256::new();
        hash.update(bytes);
        format!("{:x}", hash.finalize())
    }

    fn note(&self, verb: &str) {
        self.calls.borrow_mut().push(verb.to_owned());
    }

    /// Write one resource, honouring the declared staging posture.
    fn place(
        &self,
        request: &DeployTargetRequest<'_>,
        resource: &str,
        bytes: &[u8],
    ) -> Result<(), MechanismError> {
        let destination = self.destination(resource);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: resource.to_owned(),
                reason: error.to_string(),
            })?;
        }
        // Staging where the destination supports atomic replacement
        // (§7.2). The engine handed the directory down; the provider does
        // not choose it and cannot invent one.
        if let Some(staging) = request.staging {
            let staged = staging.join(resource.replace('/', "_"));
            std::fs::write(&staged, bytes).map_err(|error| MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: resource.to_owned(),
                reason: error.to_string(),
            })?;
            std::fs::rename(&staged, &destination).map_err(|error| {
                MechanismError::PackageWrite {
                    target: request.target.id.clone(),
                    path: resource.to_owned(),
                    reason: error.to_string(),
                }
            })?;
            return Ok(());
        }
        std::fs::write(&destination, bytes).map_err(|error| MechanismError::PackageWrite {
            target: request.target.id.clone(),
            path: resource.to_owned(),
            reason: error.to_string(),
        })
    }

    /// The shared body of `apply` and `recover`.
    fn reconcile(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
        skip: &[String],
    ) -> Result<ApplyReport, MechanismError> {
        let mut done = 0_usize;
        for planned in &plan.resources {
            if skip.contains(&planned.resource) {
                continue;
            }
            if self.faults.fail_apply_after == Some(done) {
                return Err(MechanismError::PackageWrite {
                    target: request.target.id.clone(),
                    path: planned.resource.clone(),
                    reason: "the fixture was told to fail here".to_owned(),
                });
            }
            let mut bytes = Self::desired_bytes(request.artifact, &planned.resource);
            if self.faults.corrupt {
                bytes.extend_from_slice(b"corrupted\n");
            }
            self.place(request, &planned.resource, &bytes)?;
            checkpoint.completed(&planned.resource)?;
            done += 1;
        }
        Ok(ApplyReport {
            prior_state_handle: None,
            evidence: format!("fixture reconciled {done} resource(s)"),
        })
    }
}

impl DeployProvider for FixtureProvider {
    fn descriptor(&self) -> DeployDescriptor {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: FIXTURE_PIN,
                kinds: &[ArtifactKind::Executable, ArtifactKind::File],
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                reversibility: if self.reversible {
                    Reversibility::Reversible
                } else {
                    Reversibility::Irreversible
                },
                operations: &DEPLOY_OPERATIONS,
            },
            atomic_replacement: self.atomic,
            reference_ownership: self.references,
        }
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        self.note("plan");
        self.authority
            .borrow_mut()
            .push((request.user_home.to_path_buf(), request.clients.clone()));
        self.priors
            .borrow_mut()
            .push(request.prior_receipt.cloned());
        if self.faults.fail_plan {
            return Err(MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: "<plan>".to_owned(),
                reason: "the fixture was told to refuse planning".to_owned(),
            });
        }
        let resources = self
            .resources
            .iter()
            .map(|resource| PlannedDeployResource {
                resource: resource.clone(),
                desired_digest: Self::digest(&Self::desired_bytes(request.artifact, resource)),
            })
            .collect::<Vec<_>>();
        let lock_resources = self.locks.clone().unwrap_or_else(|| {
            resources
                .iter()
                .map(|planned| planned.resource.clone())
                .collect()
        });
        let mut hash = Sha256::new();
        hash.update(b"fixture-config/1\x00");
        hash.update(request.target.id.as_bytes());
        Ok(DeployPlan {
            summary: format!("fixture would reconcile {} resource(s)", resources.len()),
            resources,
            lock_resources,
            config_digest: format!("{:x}", hash.finalize()),
            reversible: self.reversible,
        })
    }

    fn fingerprint(
        &self,
        _request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        self.note("fingerprint");
        Ok(DeployFingerprint {
            digest: plan.config_digest.clone(),
            summary: "fixture toolchain".to_owned(),
        })
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.note("apply");
        self.reconcile(request, plan, checkpoint, &[])
    }

    fn verify(
        &self,
        _request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        self.note("verify");
        Ok(resources
            .iter()
            .map(|resource| ObservedResource {
                resource: resource.clone(),
                digest: std::fs::read(self.destination(resource))
                    .ok()
                    .map(|bytes| Self::digest(&bytes)),
            })
            .collect())
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        _prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        self.note("remove");
        if self.faults.fail_remove {
            return Err(MechanismError::PackageWrite {
                target: request.target.id.clone(),
                path: "<remove>".to_owned(),
                reason: "the fixture was told to refuse removal".to_owned(),
            });
        }
        let mut removed = Vec::with_capacity(resources.len());
        for resource in resources {
            let destination = self.destination(resource);
            if destination.exists() {
                std::fs::remove_file(&destination).map_err(|error| {
                    MechanismError::PackageWrite {
                        target: request.target.id.clone(),
                        path: resource.clone(),
                        reason: error.to_string(),
                    }
                })?;
            }
            removed.push(resource.clone());
        }
        Ok(RemoveReport {
            expected_remaining: Vec::new(),
            evidence: format!("fixture removed {} resource(s)", removed.len()),
            removed,
        })
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.note("recover");
        // Idempotent roll-forward: whatever already holds the desired
        // digest is left alone, and the rest is reconciled.
        let done: Vec<String> = plan
            .resources
            .iter()
            .filter(|planned| {
                observed.iter().any(|seen| {
                    seen.resource == planned.resource
                        && seen.digest.as_deref() == Some(planned.desired_digest.as_str())
                })
            })
            .map(|planned| planned.resource.clone())
            .collect();
        self.reconcile(request, plan, checkpoint, &done)
    }
}
