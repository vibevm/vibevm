//! The deploy-phase engine's one error enum — the layer above the
//! provider's, and the third sibling of [`BuildError`] and [`PackageError`].
//!
//! The split is the one R8-CARGO drew and R8-PACKAGE kept, by *who is
//! wrong*: a [`MechanismError`] says the target, its config or the
//! destination the provider reached is wrong; a [`DeployError`] says the
//! routing, the consumed artifact's record, the engine's own state home or
//! the §7.2 transaction is.
//!
//! The transaction family is the part with no build or package twin, and it
//! is deliberately one variant per LAW rather than one per failure: "a
//! third digest was observed", "a receipt was finalised but verify never
//! ran", "a path changed after deployment" and "a partial saga survives"
//! are four different §7.2 sentences and four different repairs, so each
//! refuses by name and names the exact resources.
//!
//! [`BuildError`]: crate::BuildError
//! [`PackageError`]: crate::PackageError

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use specmark::spec;
use thiserror::Error;
use vibe_extension_registry::MechanismResolutionError;

use crate::mechanism::MechanismError;

/// Why the deploy phase could not plan, apply, verify, recover or reverse
/// one selected target.
///
/// ```
/// use vibe_lifecycle::DeployError;
///
/// let refusal = DeployError::RecoverDivergence {
///     target: "local-helper".into(),
///     resources: "bin/helper".into(),
/// };
/// assert!(refusal.to_string().contains("concurrent or user mutation"));
/// assert!(refusal.to_string().contains("bin/helper"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Error)]
pub enum DeployError {
    /// §3.1's four-step law refused to select a provider.
    #[error(transparent)]
    Resolution(#[from] MechanismResolutionError),

    /// The selected provider ran and refused.
    #[error(transparent)]
    Provider(#[from] MechanismError),

    /// Selection landed on a provider whose handler needs the
    /// out-of-process transport, which is a later atom. §7.0.2: "a
    /// non-builtin selection refuses by the unlanded transport's name".
    ///
    /// This is what proves routing is real: the builtin was NOT run.
    #[error(
        "`{key}` selected provider `{pin}`, whose handler kind `{kind}` needs the out-of-process \
         mechanism transport, which is not yet landed; the target was NOT deployed by a builtin \
         instead \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: remove the \
         `[mechanisms]` route or the target's `provider` pin, or wait for the provider-transport \
         atom)"
    )]
    TransportNotLanded {
        key: String,
        pin: String,
        kind: String,
    },

    /// Selection landed on an engine-owned row this deploy phase does not
    /// know at all. Reachable only if the builtin table grows a deploy-role
    /// row before this phase learns it.
    ///
    /// R8-DEPLOY's sibling `ProviderNotLanded` — "the one deploy builtin
    /// row (`#vibe-bin`) refuses as provider-not-landed" — is gone with
    /// R8-VIBE-BIN: that row's provider now runs, so the variant had no
    /// construction site left, and a public refusal nothing can raise
    /// tells a reader this engine can produce a state it cannot. A future
    /// deploy builtin that is collected before its adapter lands refuses
    /// through THIS variant, which is the one that already means exactly
    /// that.
    #[error(
        "`{key}` selected builtin provider `{pin}` (`{name}`), which this deploy phase does not \
         implement \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: route `{key}` \
         to a provider that implements it, or land the builtin's adapter)"
    )]
    UnknownBuiltinProvider {
        key: String,
        pin: String,
        name: String,
    },

    /// §3.2: "`plan` is mandatory for deploy providers." A provider that
    /// does not declare it cannot be dry-run, and §7's `--plan` is a law.
    #[error(
        "[[deploy.target]] `{target}` selected provider `{pin}`, whose descriptor does not \
         declare the mandatory `plan` operation \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: this is \
         a defect in the provider — §3.2 makes `plan` mandatory for every deploy provider, so a \
         deployment can always be reported before it is applied)"
    )]
    PlanNotSupported { target: String, pin: String },

    /// The resolved profile named a target the manifest does not declare.
    #[error(
        "the resolved deploy profile `{profile}` selects target `{target}`, which this project \
         does not declare; declared: {declared} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: correct \
         the profile's `targets` list, or declare the [[deploy.target]] row)"
    )]
    UnknownTarget {
        profile: String,
        target: String,
        declared: String,
    },

    /// The selected targets do not form a DAG. A validated manifest cannot
    /// reach this; a programmatically built selection can.
    #[error(
        "the selected [[deploy.target]] rows form a `depends_on` cycle: {cycle} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: break \
         the cycle — a deploy selection is applied in dependency order, which a cycle has none of)"
    )]
    Cycle { cycle: String },

    /// A deployed artifact has no record in the engine's own state, so
    /// there is no proven path to reconcile from.
    #[error(
        "[[deploy.target]] `{target}` deploys artifact `{artifact}`, which this project has no \
         artifact record for \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: run the \
         phase that produces `{artifact}` first — a deployed artifact is read from its record \
         under `.vibe/state/artifacts/`, and its path is never guessed)"
    )]
    ArtifactNotRecorded { target: String, artifact: String },

    /// The record exists and is unusable — refused, never partly believed.
    #[error(
        "[[deploy.target]] `{target}` deploys artifact `{artifact}`, whose artifact record is \
         unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         producing phase so the record is rewritten)"
    )]
    ArtifactRecordUnusable {
        target: String,
        artifact: String,
        reason: String,
    },

    /// The record names something that is not there any more.
    #[error(
        "[[deploy.target]] `{target}` deploys artifact `{artifact}`, recorded at `{path}`, where \
         no readable artifact is: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         producing phase; a destination is never reconciled from a stale record)"
    )]
    ArtifactMissing {
        target: String,
        artifact: String,
        path: String,
        reason: String,
    },

    /// The artifact is there and is not the artifact the record describes.
    #[error(
        "[[deploy.target]] `{target}` deploys artifact `{artifact}` at `{path}`, which digests to \
         `{found}` but whose record says `{recorded}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun the \
         producing phase; a destination is never reconciled from an artifact that changed behind \
         its own record)"
    )]
    ArtifactStale {
        target: String,
        artifact: String,
        path: String,
        recorded: String,
        found: String,
    },

    /// The engine could not create, pin or walk its own state home.
    #[error(
        "the engine-owned deployment state home `{path}` is unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: make \
         the vibevm settings directory writable, then rerun — deployment intents and receipts are \
         user state and live nowhere else)"
    )]
    StateHome { path: String, reason: String },

    /// A durable state file could not be published.
    #[error(
        "the deployment state file `{path}` could not be written: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: make \
         the vibevm settings directory writable, then rerun; nothing was applied, because the \
         intent is written before the first external write)"
    )]
    StateWrite { path: String, reason: String },

    /// A durable state file could not be read or decoded.
    #[error(
        "the deployment state file `{path}` could not be read: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: repair \
         or remove the named file — a deployment record is never partly believed)"
    )]
    StateRead { path: String, reason: String },

    /// The engine built a §7.2 record its own A2 cell refuses. Always a bug
    /// in this engine, and it stops here rather than reaching a reader.
    #[error(
        "the deployment `{record}` does not satisfy the record laws: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: this is \
         a defect in the producing engine — a record that does not validate is never written)"
    )]
    RecordInvalid {
        record: &'static str,
        reason: String,
    },

    /// The per-destination lock could not be taken.
    #[error(
        "the per-destination deployment lock `{path}` could not be taken: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: make \
         the vibevm settings directory writable, then rerun — apply holds one lock per \
         destination, and it will not reconcile a destination it cannot own)"
    )]
    DestinationLock { path: String, reason: String },

    /// The injected clock value is not an RFC 3339 timestamp.
    #[error(
        "the deployment of `{target}` cannot be stamped: `{value}` is not an RFC 3339 timestamp \
         ({reason}) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: pass \
         the run's own RFC 3339 clock value)"
    )]
    Clock {
        target: String,
        value: String,
        reason: String,
    },

    /// §7.2's third-digest law: "a third digest means concurrent/user
    /// mutation, so recovery refuses and names the exact resources".
    #[error(
        "recovery of [[deploy.target]] `{target}` refuses: {resources} hold neither the prior nor \
         the desired digest, which means a concurrent or user mutation happened under the \
         interrupted deployment \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: inspect \
         the named resources and decide explicitly — VibeVM never rolls forward over a third \
         digest)"
    )]
    RecoverDivergence { target: String, resources: String },

    /// Independent verify found the destination is not what the plan
    /// wanted. The receipt is NOT finalised as verified: §7.2 puts verify
    /// before finalisation exactly so this cannot be reported as success.
    #[error(
        "[[deploy.target]] `{target}` applied, but independent verify found {resources} not at \
         the planned digest \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: inspect \
         the named resources; the receipt records this deployment as failed rather than claiming \
         a state it never reached)"
    )]
    VerifyMismatch { target: String, resources: String },

    /// §7.2's undeploy law: "refuses to erase a path changed after
    /// deployment without an explicit force/recovery decision".
    #[error(
        "`undeploy` of [[deploy.target]] `{target}` refuses: {resources} changed after the \
         deployment recorded them \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: inspect \
         the named resources and remove them deliberately — an inverse deployment erases only \
         what its receipt still owns)"
    )]
    UndeployDrift { target: String, resources: String },

    /// `undeploy` was asked for a target this state home has no receipt
    /// for. Not a silent success: removal of nothing is a different fact
    /// from removal of something.
    #[error(
        "`undeploy` of [[deploy.target]] `{target}` found no deployment receipt in the vibevm \
         state home \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: run \
         `vibe deployments` to see what this machine has deployed — an inverse deployment removes \
         only receipt-owned state, so with no receipt there is nothing it may touch)"
    )]
    NoReceipt { target: String },

    /// §7.2's saga: a multi-target deploy failed and what had already been
    /// applied was reversed as far as it could be.
    #[error(
        "[[deploy.target]] `{target}` failed: {reason}; rolled back in reverse order: \
         {rolled_back}; still applied and NOT reversible: {retained} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: repair \
         the failure, then rerun — the retained targets remain visible as a partial deployment \
         and are never reported as success)"
    )]
    Saga {
        target: String,
        reason: String,
        rolled_back: String,
        retained: String,
    },

    /// §7.2's ownership collision: "A collision with state owned by another
    /// deployment is an error".
    #[error(
        "[[deploy.target]] `{target}` plans to own {resources}, which deployment `{owner}` \
         already owns \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: give \
         one of the two targets a different destination — two deployments never own one resource)"
    )]
    OwnershipCollision {
        target: String,
        owner: String,
        resources: String,
    },
}
