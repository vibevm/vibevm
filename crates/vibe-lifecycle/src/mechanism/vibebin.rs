//! `deploy:vibe-bin` — the FIRST executing deploy provider (§7.1, §7.1.0).
//!
//! §7.1 in one paragraph: it "stores immutable payloads under a
//! content-addressed `~/.vibe` store and writes a version-free launcher in
//! `~/.vibe/bin`. The launcher resolves only its active deployment receipt;
//! it does not embed a package version or copy a mutable binary into PATH."
//!
//! Three decisions of §7.1.0 shape everything below and are implemented,
//! not re-decided:
//!
//! 1. **the launcher is version-free by construction** (ruling 3). Its body
//!    embeds the command name, the genre marker and the pointer
//!    indirection, and there is no parameter through which anything else
//!    could enter it — see [`launcher`]. An update rewrites the POINTER;
//!    the launcher's bytes are the same in every generation, which is why
//!    a rollback leaves it alone;
//! 2. **the owned resources are the launcher and the pointer, NOT the
//!    payload** (ruling 4). A CAS payload is write-once and may be named by
//!    an older generation's pointer, so a receipt that owned it would make
//!    `undeploy` delete state something else still resolves through. The
//!    payload write is CHECKPOINTED — it is a completed operation §7.2 asks
//!    apply to record — and it is never RECEIPTED;
//! 3. **the collision law is a hard refusal that names both origins**
//!    (ruling 5). A `bin/<command>` that does not carry this genre's marker
//!    is either the PROP-025 project-pinned shim or a file of the user's
//!    own, and either way this provider will not overwrite it.
//!
//! What this cell deliberately does NOT own: where the settings directory
//! is (§7.1.0 ruling 2 — "a provider never resolves a home"; it arrives on
//! [`DeployTargetRequest::settings_root`]), the intent journal, the
//! receipt, the locks, the staging directory and the ordering. Those are
//! the engine's, and nothing here reads an environment variable, opens a
//! socket or resolves a home.
//!
//! [`DeployTargetRequest::settings_root`]: crate::mechanism::DeployTargetRequest

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use sha2::{Digest, Sha256};
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::contain::{FileFault, checked_relative, digest_file};
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource,
    PlannedDeployResource, RemoveReport, ResolvedDeployArtifact,
};
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::{
    BUILTIN_VIBE_BIN_PIN, DeployTargetRequest, EffectClass, MechanismError, NetworkUse,
    PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};

pub(crate) mod config;
pub(crate) mod launcher;
pub(crate) mod store;

use config::VibeBinConfig;
use launcher::{LauncherFlavour, POINTER_SUFFIX};
use store::BIN_DIR;

/// The six §3.2 operations this provider implements — every one of them,
/// which is what makes it the role's first complete implementation.
const DEPLOY_OPERATIONS: [ProviderOperation; 6] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
    ProviderOperation::Remove,
    ProviderOperation::Recover,
];

/// The one artifact kind §7.1 admits: "Only an explicit executable
/// artifact and target may use this provider."
const SUPPORTED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Executable];

/// The same list, rendered — the refusal quotes a constant rather than
/// building one, so the shared error enum stays small enough for every
/// `Result` in the mechanism layer to carry cheaply. A unit test pins the
/// two spellings together so they cannot drift apart.
const SUPPORTED_KINDS_LIST: &str = "executable";

/// The `prior_state_handle` spelling this provider writes and reads.
///
/// Self-describing rather than a bare digest so that a later generation of
/// this provider can add a second handle kind without a receipt from this
/// one being misread as it.
const POINTER_HANDLE_PREFIX: &str = "pointer:";

/// The builtin `deploy:vibe-bin` provider.
///
/// A unit struct with no state at all: everything one operation needs
/// arrives on the request, which is what "a provider never resolves a
/// home" means in code rather than in prose.
pub(crate) struct VibeBinProvider;

/// One target's resolved destination — the two owned resource identities
/// and the exact bytes each wants.
///
/// Derived in one place because `plan`, `apply`, `recover` and `remove`
/// must agree byte-for-byte about them: a plan whose desired digest came
/// from a different rendering than apply's would fail independent verify
/// every time, and the failure would look like a filesystem fault.
struct Destination {
    command: String,
    flavour: LauncherFlavour,
    /// `bin/<command>[.cmd]`, settings-relative and forward-slashed.
    launcher: String,
    /// `bin/<command>.current`, beside it.
    pointer: String,
    /// The launcher's exact bytes.
    body: Vec<u8>,
}

impl Destination {
    /// Resolve one target's destination from its config alone.
    ///
    /// It touches no filesystem: the launcher body and both identities are
    /// pure functions of the command name and the platform flavour, which
    /// is exactly what makes the launcher version-free.
    fn resolve(
        target: &str,
        config: Option<&vibe_core::manifest::ExtensionConfig>,
        flavour: LauncherFlavour,
    ) -> Result<Self, DeployProviderError> {
        let config = VibeBinConfig::parse(target, config)?;
        let command = config.command;
        Ok(Self {
            launcher: format!("{BIN_DIR}/{command}{}", flavour.launcher_suffix()),
            pointer: format!("{BIN_DIR}/{command}{POINTER_SUFFIX}"),
            body: launcher::render(flavour, &command),
            command,
            flavour,
        })
    }

    /// The digest of the desired CONFIG this deployment reconciles to.
    ///
    /// The template epoch is folded in so that changing the launcher body
    /// invalidates every deployed launcher through §4.1's ordinary
    /// staleness rather than through a special case.
    fn config_digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"vibe-bin-config/1\x00");
        hash.update(self.command.as_bytes());
        hash.update(b"\x00flavour\x00");
        hash.update(self.flavour.as_str().as_bytes());
        hash.update(b"\x00template\x00");
        hash.update(launcher::TEMPLATE_EPOCH.to_string().as_bytes());
        format!("{:x}", hash.finalize())
    }
}

impl VibeBinProvider {
    /// Resolve the destination and prove the artifact is one this provider
    /// may install — the shared preamble of `plan`, `apply` and `recover`.
    fn admit<'a>(
        &self,
        request: &DeployTargetRequest<'a>,
    ) -> Result<(Destination, &'a ResolvedDeployArtifact), DeployProviderError> {
        let target = &request.target.id;
        let destination = Destination::resolve(
            target,
            request.target.config.as_ref(),
            LauncherFlavour::NATIVE,
        )?;
        let artifact = request
            .artifact
            .ok_or_else(|| DeployProviderError::NoArtifact {
                target: target.clone(),
                provider: BUILTIN_VIBE_BIN_PIN,
            })?;
        // §7.1.0 ruling 7: "an `executable`-kind file artifact named by an
        // explicit `[[deploy.target]]`; every other kind refuses by name."
        if !SUPPORTED_KINDS.contains(&artifact.kind) {
            return Err(DeployProviderError::ArtifactKind {
                target: target.clone(),
                artifact: artifact.id.clone(),
                provider: BUILTIN_VIBE_BIN_PIN,
                kind: artifact.kind.as_str(),
                supported: SUPPORTED_KINDS_LIST,
            });
        }
        if artifact.shape != ArtifactShape::File {
            return Err(DeployProviderError::ArtifactShape {
                target: target.clone(),
                artifact: artifact.id.clone(),
                provider: BUILTIN_VIBE_BIN_PIN,
            });
        }
        Ok((destination, artifact))
    }

    /// §7.1's collision law, consulted before a launcher body is promised
    /// at `plan` and enforced again before one is written at `apply`.
    ///
    /// It is checked twice on purpose: a plan's verdict is not durable, and
    /// the two moments are separated by the intent journal, the destination
    /// lock and — on `--plan` — by an operator reading the report and
    /// deciding.
    fn admit_launcher(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
    ) -> Result<(), DeployProviderError> {
        let path = store::join(request.settings_root, &destination.launcher);
        let occupant = launcher::classify(&path).map_err(|fault| DeployProviderError::Observe {
            target: request.target.id.clone(),
            resource: destination.launcher.clone(),
            reason: fault.reason(),
        })?;
        launcher::refuse_collision(&request.target.id, &destination.launcher, occupant)
    }

    /// The payload digest the pointer names right now, when it names one
    /// that is not the digest this generation installs.
    ///
    /// That is exactly §7.2's "prior-state handle": what restoration needs,
    /// and nothing else. A pointer that already names this generation's
    /// payload displaced nothing, so there is no prior state to keep and
    /// the honest answer is `None` — which is also what keeps `remove`
    /// from "restoring" a state identical to the one it was asked to
    /// remove.
    fn prior_handle(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        installing: &str,
    ) -> Option<String> {
        let path = store::join(request.settings_root, &destination.pointer);
        let bytes = std::fs::read(path).ok()?;
        let named = launcher::pointer_digest(&bytes)?;
        (named != installing).then(|| format!("{POINTER_HANDLE_PREFIX}{named}"))
    }

    /// The shared body of `apply` and `recover` — §7.1.0 ruling 4's three
    /// writes, in the order the ruling states them.
    ///
    /// `settled` is what independent observation already found, so a
    /// resource already at its desired digest is skipped. `apply` passes
    /// an empty slice (it settles nothing in advance); `recover` passes
    /// what the engine observed, which is what makes the two operations
    /// the same operation and the CAS write idempotent.
    fn reconcile(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
        settled: &[ObservedResource],
    ) -> Result<ApplyReport, MechanismError> {
        let (destination, artifact) = self.admit(request)?;
        let target = &request.target.id;
        let handle = self.prior_handle(request, &destination, &artifact.digest);
        // 1 — the CAS payload. Write-once: an entry already holding these
        // bytes is a checkpointed no-op, which is what makes a re-apply and
        // a recovery the same idempotent operation.
        let placement = store::place_payload(
            target,
            request.settings_root,
            request.staging,
            destination.flavour,
            &artifact.absolute,
            &artifact.digest,
        )?;
        let payload = store::payload_relative(destination.flavour, &artifact.digest);
        checkpoint.completed(&payload)?;
        // 2 — the launcher, behind the collision law.
        self.admit_launcher(request, &destination)?;
        if !holds(
            settled,
            &destination.launcher,
            &desired(plan, &destination.launcher),
        ) {
            store::place_resource(
                target,
                request.settings_root,
                request.staging,
                &destination.launcher,
                &destination.body,
                destination.flavour.needs_executable_bit(),
            )?;
        }
        checkpoint.completed(&destination.launcher)?;
        // 3 — the pointer, LAST: until it moves, the command still runs the
        // generation that was there before, so an interrupted apply leaves
        // a working command rather than a broken one.
        let pointer = launcher::pointer_body(&artifact.digest);
        if !holds(
            settled,
            &destination.pointer,
            &desired(plan, &destination.pointer),
        ) {
            store::place_resource(
                target,
                request.settings_root,
                request.staging,
                &destination.pointer,
                &pointer,
                false,
            )?;
        }
        checkpoint.completed(&destination.pointer)?;
        Ok(ApplyReport {
            prior_state_handle: handle,
            evidence: format!(
                "vibe-bin installed `{}` ({} flavour): payload {} at {payload}, launcher \
                 {launcher}, pointer {pointer_resource}",
                destination.command,
                destination.flavour.as_str(),
                placement.as_str(),
                launcher = destination.launcher,
                pointer_resource = destination.pointer,
            ),
        })
    }
}

impl crate::mechanism::DeployProvider for VibeBinProvider {
    fn descriptor(&self) -> DeployDescriptor {
        DeployDescriptor {
            provider: ProviderDescriptor {
                key: BUILTIN_VIBE_BIN_PIN,
                kinds: &SUPPORTED_KINDS,
                // §7.1's destination is the invoking user's own `~/.vibe`,
                // never a machine-wide prefix: an ordinary application that
                // wants one names a different mechanism.
                effect: EffectClass::User,
                network: NetworkUse::Never,
                privilege: PrivilegeNeed::None,
                reversibility: Reversibility::Reversible,
                operations: &DEPLOY_OPERATIONS,
            },
            // Every owned file is published by rename, so the destination
            // really does support atomic replacement and §7.2's staging
            // sentence applies.
            atomic_replacement: true,
        }
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        let (destination, artifact) = self.admit(request)?;
        // The collision law is consulted HERE too, so `--plan` reports the
        // refusal an apply would raise instead of promising a write that
        // cannot happen. This reads the occupying file and writes nothing.
        self.admit_launcher(request, &destination)?;
        let resources = vec![
            PlannedDeployResource {
                desired_digest: digest_of(&destination.body),
                resource: destination.launcher.clone(),
            },
            PlannedDeployResource {
                desired_digest: digest_of(&launcher::pointer_body(&artifact.digest)),
                resource: destination.pointer.clone(),
            },
        ];
        Ok(DeployPlan {
            summary: format!(
                "vibe-bin would install `{}` as {} with the active-payload pointer {}, resolving \
                 the payload {}",
                destination.command,
                destination.launcher,
                destination.pointer,
                store::payload_relative(destination.flavour, &artifact.digest),
            ),
            resources,
            config_digest: destination.config_digest(),
            // §7.1.0 ruling 6: update and rollback are the same saga the
            // engine already owns, and this provider keeps what restoring
            // needs, so it says so before apply.
            reversible: true,
        })
    }

    fn fingerprint(
        &self,
        _request: &DeployTargetRequest<'_>,
        _plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        // §4.1's "provider portion": what THIS provider would produce for
        // an unchanged target — the launcher template's epoch and the
        // platform flavour, and deliberately nothing about the artifact,
        // which the engine already folds in.
        let flavour = LauncherFlavour::NATIVE;
        let mut hash = Sha256::new();
        hash.update(b"vibe-bin-fingerprint/1\x00");
        hash.update(launcher::TEMPLATE_EPOCH.to_string().as_bytes());
        hash.update(b"\x00flavour\x00");
        hash.update(flavour.as_str().as_bytes());
        Ok(DeployFingerprint {
            digest: format!("{:x}", hash.finalize()),
            summary: format!(
                "the vibe-bin launcher template epoch {} on the {} flavour",
                launcher::TEMPLATE_EPOCH,
                flavour.as_str(),
            ),
        })
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.reconcile(request, plan, checkpoint, &[])
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
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        let target = &request.target.id;
        let destination = Destination::resolve(
            target,
            request.target.config.as_ref(),
            LauncherFlavour::NATIVE,
        )?;
        // §7.1.0 ruling 6: "rollback is the landed saga/remove path
        // restoring the prior pointer through that handle", and the ruling
        // proves it by RUNNING the rolled-back launcher. So a handle means
        // restore-the-pointer and leave the launcher: its bytes are the
        // prior generation's too — the launcher is version-free — and
        // deleting it would break the very command the restoration exists
        // to bring back.
        if let Some(digest) = restored_digest(prior_state_handle) {
            store::place_resource(
                target,
                request.settings_root,
                request.staging,
                &destination.pointer,
                &launcher::pointer_body(&digest),
                false,
            )?;
            return Ok(RemoveReport {
                removed: Vec::new(),
                evidence: format!(
                    "vibe-bin restored the prior active-payload pointer {} to {digest}; the \
                     version-free launcher {} and every stored payload were left in place",
                    destination.pointer, destination.launcher,
                ),
            });
        }
        // With no prior state there is nothing to restore: both owned
        // files go, and §7.1.0 ruling 4's payload stays as disclosed store
        // garbage that a later GC atom collects.
        let mut removed = Vec::with_capacity(resources.len());
        for resource in resources {
            let relative = self.contained(request, resource)?;
            if store::remove_resource(target, request.settings_root, &relative)? {
                removed.push(resource.clone());
            }
        }
        Ok(RemoveReport {
            evidence: format!(
                "vibe-bin removed {} owned resource(s) of `{}`; the content-addressed payloads \
                 were not touched",
                removed.len(),
                destination.command,
            ),
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
        // The same three writes, idempotently: the engine has already
        // proven every observed resource is at the prior or the desired
        // digest, so completing the interrupted apply is a re-derivation
        // rather than a decision.
        self.reconcile(request, plan, checkpoint, observed)
    }
}

impl VibeBinProvider {
    /// Observe one owned resource's digest, or `None` when nothing is
    /// there.
    ///
    /// Absence is a value (§7.2's recover reasons about it); anything that
    /// is present and is NOT a readable regular file is a refusal, because
    /// reporting a link as "absent" would let an apply write through it.
    fn observe(
        &self,
        request: &DeployTargetRequest<'_>,
        resource: &str,
    ) -> Result<Option<String>, DeployProviderError> {
        let relative = self.contained(request, resource)?;
        let path = store::join(request.settings_root, &relative);
        match digest_file(&path) {
            Ok((digest, _)) => Ok(Some(digest)),
            Err(FileFault::Missing(_)) => Ok(None),
            Err(fault) => Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: resource.to_owned(),
                reason: fault.reason(),
            }),
        }
    }

    /// One recorded resource identity, proven to name a place inside the
    /// settings root before it is joined to it.
    fn contained(
        &self,
        request: &DeployTargetRequest<'_>,
        resource: &str,
    ) -> Result<String, DeployProviderError> {
        checked_relative(resource).map_err(|fault| DeployProviderError::Observe {
            target: request.target.id.clone(),
            resource: resource.to_owned(),
            reason: fault.reason().to_owned(),
        })
    }
}

/// The digest one plan wants at a named resource, or the empty string when
/// the plan does not name it at all.
fn desired(plan: &DeployPlan, resource: &str) -> String {
    plan.resources
        .iter()
        .find(|planned| planned.resource == resource)
        .map_or_else(String::new, |planned| planned.desired_digest.clone())
}

/// Whether observation already found one resource at the digest a plan
/// wants it at.
fn holds(settled: &[ObservedResource], resource: &str, digest: &str) -> bool {
    !digest.is_empty()
        && settled
            .iter()
            .any(|seen| seen.resource == resource && seen.digest.as_deref() == Some(digest))
}

/// The payload digest one prior-state handle restores, when it is this
/// provider's own pointer handle.
fn restored_digest(handle: Option<&str>) -> Option<String> {
    let named = handle?.strip_prefix(POINTER_HANDLE_PREFIX)?;
    (named.len() == 64 && named.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| named.to_ascii_lowercase())
}

/// The SHA-256 of one resource's exact bytes, in the 64-hex spelling every
/// record and every plan uses.
fn digest_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// The world the two law cells share — a `pub(crate)` fixture home under
// the test cfg, exactly as the deploy engine's own fixture is.
#[cfg(test)]
#[path = "vibebin/support.rs"]
pub(crate) mod support;

#[cfg(test)]
#[path = "vibebin/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "vibebin/apply_tests.rs"]
mod apply_tests;

#[cfg(test)]
#[path = "vibebin/e2e_tests.rs"]
mod e2e_tests;
