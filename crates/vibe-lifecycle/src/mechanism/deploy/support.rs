//! The hermetic deploy fixture — one in-process provider, and the world
//! every §7.2 law is proven against.
//!
//! §7.0.2 rules that "the only executing implementations this atom are
//! hermetic fixtures at the unit seam", and this is that seam. The
//! provider below is a real [`DeployProvider`]: it implements all six §3.2
//! verbs, keeps its whole destination inside a temp directory, reads no
//! environment and reaches no network. What makes it a *fixture* rather
//! than a shipped provider is only that it is compiled under `cfg(test)`
//! and that its failure points are declarable — a real provider fails when
//! the world fails, and a law about crash windows needs the world to fail
//! on cue.
//!
//! Every path this fixture writes is under a `TempDir` the test owns, and
//! the deployment state home is likewise a temp root handed in as data. The
//! operator's real settings directory is unreachable from this suite by
//! construction: nothing here resolves a home, and no test names one.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use vibe_core::manifest::{ArtifactKind, DeployTarget, MechanismRoutes};
use vibe_extension_registry::{MechanismRegistry, SelectionStep};
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource,
    PlannedDeployResource, RemoveReport, ResolvedDeployArtifact,
};
use super::state::CheckpointLedger;
use super::{DeployExecution, DeploySelection, Selected};
use crate::mechanism::package::support::{key, pin, registry as collect, temp, write};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};
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
    /// Whether the destination supports atomic replacement, and therefore
    /// whether the engine offers a staging directory.
    atomic: bool,
    /// Whether this provider can undo an apply.
    reversible: bool,
    faults: Faults,
    /// Every verb call, in order — the witness the ordering laws read.
    calls: RefCell<Vec<String>>,
}

impl FixtureProvider {
    /// A reversible, non-staging provider over one resource.
    pub(crate) fn new(root: &Path, resources: &[&str]) -> Self {
        Self {
            root: root.to_path_buf(),
            resources: resources.iter().map(|name| (*name).to_owned()).collect(),
            atomic: false,
            reversible: true,
            faults: Faults::default(),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// The same provider, declaring that its destination supports atomic
    /// replacement — so the engine prepares staging for it.
    pub(crate) fn staging(mut self) -> Self {
        self.atomic = true;
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
        }
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        self.note("plan");
        let resources = self
            .resources
            .iter()
            .map(|resource| PlannedDeployResource {
                resource: resource.clone(),
                desired_digest: Self::digest(&Self::desired_bytes(request.artifact, resource)),
            })
            .collect::<Vec<_>>();
        let mut hash = Sha256::new();
        hash.update(b"fixture-config/1\x00");
        hash.update(request.target.id.as_bytes());
        Ok(DeployPlan {
            summary: format!("fixture would reconcile {} resource(s)", resources.len()),
            resources,
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

/// One `[[deploy.target]]` row over an artifact id.
pub(crate) fn target(id: &str, artifact: &str, depends_on: &[&str]) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key("deploy:vibe-bin"),
        provider: Some(pin(FIXTURE_PIN)),
        depends_on: Some(depends_on.iter().map(|name| (*name).to_owned()).collect()),
        config: None,
    }
}

/// A prepared project with one produced artifact and its A2 record.
pub(crate) struct Fixture {
    pub(crate) project: TempDir,
    pub(crate) settings: TempDir,
    pub(crate) destination: TempDir,
    pub(crate) registry: MechanismRegistry,
    pub(crate) routes: MechanismRoutes,
}

impl Fixture {
    /// One project holding a produced `helper.exe` artifact, an empty
    /// deployment state home and an empty destination — three separate
    /// temp trees, so nothing in this suite can reach a real home.
    pub(crate) fn new(body: &str) -> Self {
        let project = temp();
        write(project.path(), "target/debug/helper.exe", body);
        let mut hash = Sha256::new();
        hash.update(body.as_bytes());
        let digest = format!("{:x}", hash.finalize());
        let absolute = crate::mechanism::contain::forward_slashed(
            &project.path().join("target/debug/helper.exe"),
        );
        let record = build_record(&RecordInputs {
            target: "helper",
            mechanism: &key("build:cargo"),
            provider_key: "org.vibevm/vibe#cargo",
            provider_version: None,
            provider_hash: None,
            output_id: "helper.exe",
            kind: ArtifactKind::Executable,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &absolute,
            path_relative: "target/debug/helper.exe",
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: None,
            created_at: "2026-08-30T00:00:00Z",
            evidence: "fixture artifact".to_owned(),
        })
        .expect("the fixture record builds");
        write_record(project.path(), &record).expect("the fixture record writes");
        let world = crate::mechanism::package::support::empty_world();
        Self {
            registry: collect(&world),
            routes: MechanismRoutes::default(),
            project,
            settings: temp(),
            destination: temp(),
        }
    }

    /// Rebuild the fixture's `helper.exe` with new bytes and re-record it —
    /// the manifest shape of "the artifact this target deploys was rebuilt",
    /// which is what makes a second `execute_deploy_targets` a new
    /// GENERATION of the same deployment rather than a no-op.
    pub(crate) fn rebuild(&self, body: &str) {
        write(self.project.path(), "target/debug/helper.exe", body);
        let mut hash = Sha256::new();
        hash.update(body.as_bytes());
        let digest = format!("{:x}", hash.finalize());
        let absolute = crate::mechanism::contain::forward_slashed(
            &self.project.path().join("target/debug/helper.exe"),
        );
        let record = build_record(&RecordInputs {
            target: "helper",
            mechanism: &key("build:cargo"),
            provider_key: "org.vibevm/vibe#cargo",
            provider_version: None,
            provider_hash: None,
            output_id: "helper.exe",
            kind: ArtifactKind::Executable,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &absolute,
            path_relative: "target/debug/helper.exe",
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: None,
            created_at: "2026-08-30T00:00:00Z",
            evidence: "fixture artifact, rebuilt".to_owned(),
        })
        .expect("the rebuilt record builds");
        write_record(self.project.path(), &record).expect("the rebuilt record writes");
    }

    /// The deployment state home of this fixture — a temp root, named as
    /// data exactly as the command layer would name the settings dir.
    pub(crate) fn state_home(&self) -> PathBuf {
        super::deploy_state_home(self.settings.path())
    }

    /// An execution over this fixture's project and state home.
    pub(crate) fn execution<'a>(
        &'a self,
        targets: &'a [DeployTarget],
        selection: &'a DeploySelection,
        state_home: &'a Path,
    ) -> DeployExecution<'a> {
        DeployExecution {
            project_root: self.project.path(),
            targets,
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home,
            settings_root: self.settings.path(),
            project: "org.example/demo",
            package: None,
            created_at: "2026-08-30T12:00:00Z",
        }
    }
}

/// One already-resolved target, as the executor's own selection step
/// would produce it — the seam the saga's laws are driven through.
pub(crate) fn selected<'a>(
    target: &'a DeployTarget,
    provider: Box<dyn DeployProvider>,
) -> Selected<'a> {
    Selected {
        target,
        provider,
        pin: FIXTURE_PIN.to_owned(),
        via: SelectionStep::TargetPin,
        displaced: None,
    }
}

/// One profile selection over the given target ids.
pub(crate) fn selection(profile: &str, targets: &[&str]) -> DeploySelection {
    DeploySelection {
        profile: profile.to_owned(),
        targets: targets.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// A borrowing shim so a test can keep the fixture and still hand the
/// executor an owned provider.
pub(crate) struct Witness(pub(crate) std::rc::Rc<FixtureProvider>);

impl DeployProvider for Witness {
    fn descriptor(&self) -> DeployDescriptor {
        self.0.descriptor()
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        self.0.plan(request)
    }

    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        self.0.fingerprint(request, plan)
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.0.apply(request, plan, checkpoint)
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        self.0.verify(request, resources)
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        self.0.remove(request, resources, prior_state_handle)
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.0.recover(request, plan, observed, checkpoint)
    }
}
