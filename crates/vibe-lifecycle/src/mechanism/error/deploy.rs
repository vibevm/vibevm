//! The DEPLOY role's provider refusals — §7.1's destination laws, and the
//! deploy-shaped section of [`MechanismError`].
//!
//! Split out of the shared enum's own file along the one seam the three
//! roles really have: a build or package provider produces an artifact
//! inside an engine-owned output root, while a deploy provider reconciles
//! a DESTINATION it does not own — so its refusals are about collisions,
//! ownership, restoration and observation, none of which a producing
//! provider can raise. The layer still has ONE error type; this is a
//! section of it, carried transparently.
//!
//! Every value a variant holds that is a compile-time constant of this
//! engine is a `&'static str` rather than an owned `String`. That is not
//! micro-optimisation: [`MechanismError`] is returned by every provider
//! operation of every role, and an enum whose largest variant outgrows the
//! `Result` size budget makes every one of those call sites pay for it.
//!
//! [`MechanismError`]: crate::mechanism::MechanismError

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use specmark::spec;
use thiserror::Error;

/// Why a builtin deploy provider could not plan, apply, verify, remove or
/// recover one target.
///
/// ```
/// use vibe_lifecycle::DeployProviderError;
///
/// let refusal = DeployProviderError::Staging {
///     target: "local-helper".into(),
///     path: "bin/vibe-helper".into(),
/// };
/// assert!(refusal.to_string().contains("staging directory"));
/// assert!(refusal.to_string().contains("PROP-054#OPEN-DEPLOY-TARGETS"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DeployProviderError {
    /// One `config` member of a `[[deploy.target]]` row is missing,
    /// mistyped, unknown or engine-owned.
    ///
    /// A sibling of [`MechanismError::Config`] rather than a reuse of it:
    /// the two quote different manifest tables, and a refusal that told an
    /// operator to fix `[[artifacts.build]]` when the fault is in a
    /// `[[deploy.target]]` would send them to the wrong row.
    ///
    /// [`MechanismError::Config`]: crate::mechanism::MechanismError::Config
    #[error(
        "[[deploy.target]] `{target}` config member `{member}` is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: correct \
         the member in the target's `config` table)"
    )]
    Config {
        target: String,
        member: String,
        reason: String,
    },

    /// §7.1's "Only an explicit executable artifact and target may use this
    /// provider. … Merely producing an executable does not grant
    /// installation into `~/.vibe/bin`" — read in the other direction: an
    /// artifact that is not an executable is refused by its own kind.
    #[error(
        "[[deploy.target]] `{target}` deploys artifact `{artifact}` of kind `{kind}` through the \
         builtin provider `{provider}`, which installs {supported} artifacts only \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: an \
         application that wants MSI, dpkg, Homebrew or a custom prefix names a different deploy \
         mechanism — this one writes a launcher for an explicit executable and nothing else)"
    )]
    ArtifactKind {
        target: String,
        artifact: String,
        /// The reserved builtin identity — a compile-time constant of this
        /// engine, so it is borrowed rather than owned.
        provider: &'static str,
        kind: &'static str,
        supported: &'static str,
    },

    /// The artifact is an executable by kind and a DIRECTORY by shape. A
    /// launcher resolves one payload file; a tree has no single one.
    #[error(
        "[[deploy.target]] `{target}` deploys artifact `{artifact}`, which is a directory, through \
         the builtin provider `{provider}`, which installs a single executable FILE \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: name \
         the one executable file this command should run — a version-free launcher resolves \
         exactly one content-addressed payload)"
    )]
    ArtifactShape {
        target: String,
        artifact: String,
        provider: &'static str,
    },

    /// A deploy provider that installs an artifact was invoked without one.
    /// Unreachable through the executor, which resolves and proves the
    /// artifact before it plans, and a refusal rather than a panic for the
    /// same reason `PlanRoleMismatch` is one.
    #[error(
        "the builtin provider `{provider}` was asked to plan or apply [[deploy.target]] \
         `{target}` with no resolved artifact \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: this is \
         a defect in the deploy executor — an installing provider is handed the artifact its \
         record proved)"
    )]
    NoArtifact {
        target: String,
        provider: &'static str,
    },

    /// §7.1's collision law: "A name already owned by the other genre—or by
    /// an unmarked user file—is a hard collision that names both origins
    /// and asks the target to choose another command alias."
    #[error(
        "[[deploy.target]] `{target}` would install the launcher `{resource}`, but a file is \
         already there and it is not this genre's: {observed}. The two VibeVM launcher genres are \
         `deploy:vibe-bin`, whose bodies carry `{ours}`, and the PROP-025 project-pinned `vibe \
         bin` shim, whose bodies carry `{shim}`; anything carrying neither belongs to you \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: give \
         this target another `command` alias — this provider never overwrites a name it does not \
         already own)"
    )]
    LauncherCollision {
        target: String,
        resource: String,
        /// Which origin the occupying file turned out to have, naming the
        /// other genre's marker when that is what it carries.
        observed: String,
        /// This genre's exact marker spelling.
        ours: &'static str,
        /// The PROP-025 shim genre's exact marker spelling.
        shim: &'static str,
    },

    /// A destination file could not be written, renamed or removed.
    #[error(
        "[[deploy.target]] `{target}` could not write `{path}` under the vibevm settings \
         directory: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: make \
         the vibevm settings directory writable, then rerun — the intent journal is already \
         durable, so the rerun recovers rather than restarts)"
    )]
    Write {
        target: String,
        path: String,
        reason: String,
    },

    /// A staging-requiring write was reached with no staging directory.
    /// Always a defect in this engine: the descriptor declares
    /// `atomic_replacement`, so apply and recover are always offered one.
    #[error(
        "[[deploy.target]] `{target}` reached the staged write of `{path}` with no engine-owned \
         staging directory \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: this is \
         a defect in the deploy executor — §7.2 offers staging to every provider that declares \
         atomic replacement, and a provider never mints a scratch path of its own)"
    )]
    Staging { target: String, path: String },

    /// An owned resource is present and is not a readable regular file, so
    /// independent verify cannot say what is there. Absence would be a
    /// value; a link is not.
    #[error(
        "[[deploy.target]] `{target}` cannot observe its owned resource `{resource}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: inspect \
         the named path and remove it deliberately — a deployment never writes through a link, \
         and never reports one as absent)"
    )]
    Observe {
        target: String,
        resource: String,
        reason: String,
    },

    /// A content-addressed payload holds bytes that are not the bytes its
    /// own address names. §7.1.0 ruling 4 makes the store write-once, so
    /// this is damage rather than drift, and repairing it silently would
    /// erase what a prior generation's pointer still resolves to.
    #[error(
        "[[deploy.target]] `{target}` found the content-addressed payload `{path}` holding \
         `{found}` where its address names `{recorded}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: inspect \
         the named store entry and remove it deliberately — a payload store is write-once, and \
         this provider never overwrites an entry another generation may still resolve through)"
    )]
    PayloadCorrupt {
        target: String,
        path: String,
        recorded: String,
        found: String,
    },
}
