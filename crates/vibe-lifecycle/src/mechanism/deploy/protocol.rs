//! The deploy role's provider-protocol value types — what the six §3.2
//! operations of a [`DeployProvider`] take and hand back, plus the two
//! engine-owned values the command layer hands DOWN (the resolved profile
//! selection and the proven artifact).
//!
//! Shared by every deploy provider on purpose, exactly as the package
//! role's vocabulary is: one trait, one protocol. What is deliberately NOT
//! here is anything a provider could use to mint its own identity — no
//! output path, no state path, no generation counter. Those are the
//! engine's (§3.2), and a provider that cannot name them cannot invent a
//! second lifecycle.
//!
//! [`DeployProvider`]: crate::mechanism::DeployProvider

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{ArtifactKind, DeployTarget};
use vibe_wire::generated::artifact_record::ArtifactShape;
use vibe_wire::generated::deploy_receipt::DestinationScope;

use super::state::CheckpointLedger;
use crate::mechanism::{EffectClass, MechanismError, ProviderDescriptor, ProviderOperation};

/// One produced artifact a deploy target reconciles, resolved and proven by
/// the ENGINE before any provider sees it.
///
/// §6.0.2's law, reused rather than restated: the record under
/// `.vibe/state/artifacts/<id>.json` is the ONE place a consumed artifact
/// may be found, and its bytes are re-proven against the recorded digest
/// before a destination is touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedDeployArtifact {
    /// The declared artifact id the target named.
    pub(crate) id: String,
    pub(crate) kind: ArtifactKind,
    pub(crate) shape: ArtifactShape,
    pub(crate) absolute: PathBuf,
    /// Project-relative, forward-slashed.
    pub(crate) relative: String,
    /// 64 lowercase hex over the bytes that are really there NOW.
    pub(crate) digest: String,
    pub(crate) bytes: u64,
}

/// One resource a plan intends to touch, at the digest it wants it at.
///
/// The engine turns these into the intent journal's planned resources; the
/// prior digest is added there, from the prior receipt, because a provider
/// has no business reading the engine's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedDeployResource {
    /// The resource identity — an owned path or a named destination
    /// resource. Non-blank and control-free; the wire cell holds it to
    /// that law and the engine refuses before writing.
    pub(crate) resource: String,
    /// 64 lowercase hex the resource should hold after apply.
    pub(crate) desired_digest: String,
}

/// What `plan` reports. Producing it opens no destination and writes
/// nothing — §7's "A plan does not read tokens, call an LLM, download,
/// build or mutate any destination".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeployPlan {
    /// Every resource this deployment would own, in a stable order.
    pub(crate) resources: Vec<PlannedDeployResource>,
    /// The digest of the desired CONFIG this deployment reconciles to —
    /// the receipt member of the same name.
    pub(crate) config_digest: String,
    /// Whether the provider can undo what this plan would apply. Declared
    /// per plan rather than per provider because §3.2 requires the answer
    /// "before apply", and a provider may know it only once it has seen
    /// the target: "An installer that cannot roll back says so before
    /// apply; VibeVM never implies a transaction it cannot provide."
    pub(crate) reversible: bool,
    /// A control-free one-line summary for the receipt's evidence.
    pub(crate) summary: String,
}

/// The provider portion of one deployment's freshness digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeployFingerprint {
    /// 64 lowercase hex.
    pub(crate) digest: String,
    /// What the digest was taken over, for the evidence line.
    pub(crate) summary: String,
}

/// What `apply` (and `recover`'s roll-forward) hands back.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ApplyReport {
    /// The handle under which the prior state survives for rollback, when
    /// the provider kept one. Absent for a first deployment, and for an
    /// irreversible one.
    pub(crate) prior_state_handle: Option<String>,
    /// Typed evidence the provider adds (§3.2: "A provider may add typed
    /// evidence"). Free text, sanitised by the engine before it is
    /// recorded.
    pub(crate) evidence: String,
}

/// One resource as `verify` OBSERVED it.
///
/// `digest` is `None` when nothing is there. Absence is a value, not a
/// fault: `recover` distinguishes "never applied" from "applied and then
/// changed", and an error would collapse the two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservedResource {
    pub(crate) resource: String,
    /// 64 lowercase hex, or `None` when the resource is absent.
    pub(crate) digest: Option<String>,
}

/// What `remove` hands back.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RemoveReport {
    /// The resource identities the provider really removed.
    pub(crate) removed: Vec<String>,
    /// Typed evidence, sanitised by the engine before it is reported.
    pub(crate) evidence: String,
}

/// The receipt vocabulary's spelling of one effect class.
///
/// The two sets are the same closed §3.2/§9 vocabulary, so the mapping is
/// total — which is why the scope is derived from the descriptor rather
/// than declared beside it.
pub(crate) const fn destination_scope(effect: EffectClass) -> DestinationScope {
    match effect {
        EffectClass::Workspace => DestinationScope::Workspace,
        EffectClass::User => DestinationScope::User,
        EffectClass::Remote => DestinationScope::Remote,
        EffectClass::System => DestinationScope::System,
    }
}

// The deploy role's PROTOCOL — §3.2's six operations, the descriptor they
// are declared under, and the request one target arrives as.
//
// It lives beside the value types it speaks rather than in `mechanism.rs`
// with its two siblings, because the deploy role is the one that needed a
// descriptor of its own (§7.2's staging sentence) and a request shape of
// its own (a proven artifact and an engine-owned staging directory).
// `mechanism.rs` re-exports all three, so every use site still spells them
// `crate::mechanism::…` — one seam, one name.

/// One deploy target as the engine hands it to a provider.
///
/// The same shape of contract as its two siblings, with the two facts a
/// destination-reconciling role needs and they do not:
///
/// - the artifact is **already resolved and proven** from the engine's own
///   `.vibe/state/artifacts/` record (§6.0.2's law, reused verbatim — a
///   deploy provider never guesses a produced path either). It is
///   `Option` because `remove` runs from a receipt, long after the
///   artifact may have been cleaned away;
/// - `staging` is an engine-owned scratch directory, offered exactly when
///   the provider declared that its destination supports atomic
///   replacement (§7.2's "staging where the destination supports atomic
///   replacement"). A provider that declared otherwise is handed `None`
///   rather than a directory it would have to promise not to use;
/// - `settings_root` is §7.1.0 ruling 2's "`DeployExecution` carries the
///   settings root beside the state home; a provider never resolves a
///   home". A user-scope destination lives under it, and handing it down
///   is what keeps `settings_dir()` out of every cell below a surface —
///   the same law the state home already holds to, for the same reason: a
///   test can then name the whole destination, and the operator's real
///   `~/.vibe` is unreachable by construction rather than by an
///   environment variable a test could forget.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeployTargetRequest<'a> {
    pub(crate) target: &'a DeployTarget,
    /// The profile that selected this target — data, resolved once in the
    /// command layer (§7.0.5) and never re-derived here.
    pub(crate) profile: &'a str,
    /// The selected project's absolute root.
    #[allow(
        dead_code,
        reason = "the provider-facing request; a project-scope deploy provider is its first reader"
    )]
    pub(crate) project_root: &'a Path,
    /// The absolute vibevm settings directory this deployment's
    /// destination lives under.
    pub(crate) settings_root: &'a Path,
    /// The artifact this target reconciles, proven from its record.
    pub(crate) artifact: Option<&'a ResolvedDeployArtifact>,
    /// The engine-owned staging directory, when the provider takes one.
    pub(crate) staging: Option<&'a Path>,
}

/// What a deploy-role provider declares about itself.
///
/// It WRAPS [`ProviderDescriptor`] rather than restating it: §3.2's
/// descriptor list is one list for all three roles, and a second copy of
/// effect class, network use, privilege need and reversibility is a second
/// thing to drift. The one member added here is not in §3.2's list at all
/// — it answers §7.2's staging sentence, which only a destination has.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeployDescriptor {
    /// The shared §3.2 descriptor, unchanged.
    pub(crate) provider: ProviderDescriptor,
    /// Whether the destination supports atomic replacement, and therefore
    /// whether the engine stages into a scratch directory first.
    pub(crate) atomic_replacement: bool,
}

impl DeployDescriptor {
    /// The destination scope this provider reconciles, derived from the
    /// ONE effect class it already declares.
    ///
    /// Derived rather than declared a second time: §9's invariant 4 wants
    /// the scope explicit, and two spellings of one fact are how "explicit"
    /// turns into "ambiguous".
    pub(crate) const fn scope(&self) -> EffectClass {
        self.provider.effect
    }

    /// Whether the provider implements one §3.2 operation.
    pub(crate) fn implements(&self, operation: ProviderOperation) -> bool {
        self.provider.operations.contains(&operation)
    }
}

/// The in-process builtin provider protocol for the deploy role — §3.2's
/// SIX operations, the two extra ones being the pair the architecture
/// reserves for exactly this role.
///
/// Crate-internal for the same reason as its two siblings: the
/// out-of-process transport is a later atom, and publishing an in-process
/// shape now would freeze it as the provider contract.
///
/// The engine's half of the split is unchanged from §3.2 and is worth
/// restating because a destination makes it tempting to blur: the ENGINE
/// owns the intent journal, the checkpoints, the receipt, the locks, the
/// staging directory, the ordering and the redaction; the provider owns
/// only what a destination looks like and how bytes get there.
pub(crate) trait DeployProvider {
    /// What this provider declares about itself. `plan` support is
    /// mandatory (§3.2) and the executor refuses a descriptor that omits
    /// it — a deploy that cannot be planned cannot be dry-run, and §7's
    /// `--plan` is a law, not a convenience.
    fn descriptor(&self) -> DeployDescriptor;

    /// Validate the target's config, resolve the destination, and report
    /// every resource this deployment would touch with the digest it
    /// wants each at. Pure: it reads its own config and the resolved
    /// artifact, and it mutates no destination — `--plan` calls exactly
    /// this and nothing else.
    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError>;

    /// The provider/toolchain portion of the freshness digest.
    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError>;

    /// Reconcile the destination, checkpointing each completed operation
    /// through the engine's ledger (§7.2: "Apply checkpoints completed
    /// operations without storing secrets").
    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError>;

    /// Independently OBSERVE the named resources — the one verb three laws
    /// read: post-apply verification, `undeploy`'s drift refusal and
    /// `recover`'s three-digest comparison. An absent resource is a
    /// `None` digest, never an error: absence is a state to reason about.
    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError>;

    /// Remove state this deployment owns — and only that. The engine has
    /// already proven the drift law before this is called; a provider
    /// never decides whether removal is allowed.
    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError>;

    /// Roll an interrupted operation forward, given what the engine
    /// observed. Called only after the engine proved every observed
    /// resource is at the prior or the desired digest (§7.2), so this is
    /// an idempotent completion, never a decision.
    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError>;
}
