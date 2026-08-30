//! The deploy engine's PUBLIC value model — what a surface passes in and
//! what it reads back.
//!
//! Its own cell because it is its own responsibility: the executor next
//! door owns selection, the transaction and the saga, while these types
//! are the vocabulary every surface speaks. Splitting them keeps a
//! reader who only wants to know *what a deploy reports* from having to
//! read *how it is applied*.
//!
//! Two members carry the atom's load-bearing decisions and are worth
//! finding here rather than in prose: [`DeploySelection`] is §7.0.5's
//! "travels as data", and [`DeployExecution::state_home`] is §7.0.3's
//! state home arriving as a PARAMETER, which is what makes the
//! operator's real settings directory unreachable from a test.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{DeployTarget, MechanismRoutes};
use vibe_extension_registry::MechanismRegistry;
use vibe_wire::generated::deploy_receipt::{DeployReceipt, ReceiptStatus};

/// The deployment state home, relative to the vibevm settings directory —
/// §7.0.3's `state/deployments/`.
///
/// ```
/// assert_eq!(vibe_lifecycle::DEPLOY_STATE_DIR, "state/deployments");
/// ```
pub const DEPLOY_STATE_DIR: &str = "state/deployments";

/// The absolute deployment state home under one settings directory.
///
/// A pure join, and the ONE place the two components are spelled together.
/// It takes the settings directory rather than resolving it, because a
/// deployment's state home is user state and a test must be able to name
/// it without the operator's real home ever being reachable.
///
/// ```
/// use std::path::Path;
///
/// let home = vibe_lifecycle::deploy_state_home(Path::new("/tmp/settings"));
/// assert!(home.ends_with("deployments"));
/// ```
#[must_use]
pub fn deploy_state_home(settings_dir: &Path) -> PathBuf {
    let mut home = settings_dir.to_path_buf();
    for segment in DEPLOY_STATE_DIR.split('/') {
        home.push(segment);
    }
    home
}

/// The resolved deploy-profile selection, as DATA.
///
/// §7.0.5: "Profile resolution happens ONCE, in the command layer that
/// owns flags, and travels as data: explicit `--profile`, else the
/// manifest's `default_profile`, else the exactly-one rule, else a typed
/// refusal naming the defined profiles. Environment and secrets never
/// choose."
///
/// This type is the "travels as data" half. It carries a NAME and an
/// ordered list of target ids and deliberately nothing that would let a
/// reader re-derive it — no manifest, no flag, no environment.
///
/// ```
/// use vibe_lifecycle::DeploySelection;
///
/// let selection = DeploySelection {
///     profile: "local".into(),
///     targets: vec!["local-helper".into()],
/// };
/// assert_eq!(selection.targets.len(), 1);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploySelection {
    /// The profile name that was resolved.
    pub profile: String,
    /// Its target ids, in authored order.
    pub targets: Vec<String>,
}

/// Everything one deploy-phase execution needs, and nothing more.
///
/// The state home is a PARAMETER for the reason [`state`] states: it makes
/// the operator's real settings directory unreachable from a test by
/// construction rather than by an environment variable a test could forget.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, Copy)]
pub struct DeployExecution<'a> {
    /// The selected project's absolute root.
    pub project_root: &'a Path,
    /// The landed `[[deploy.target]]` rows, in declaration order.
    pub targets: &'a [DeployTarget],
    /// The profile selection the command layer resolved.
    pub selection: &'a DeploySelection,
    /// The collected mechanism plane of this world.
    pub registry: &'a MechanismRegistry,
    /// The host's `[mechanisms]` routes.
    pub routes: &'a MechanismRoutes,
    /// The absolute deployment state home — [`deploy_state_home`].
    pub state_home: &'a Path,
    /// The project identity every intent and receipt is keyed under.
    pub project: &'a str,
    /// The package identity, when the deploy comes from one package
    /// rather than the host project.
    pub package: Option<&'a str>,
    /// The run's RFC 3339 clock value, stamped into every record.
    pub created_at: &'a str,
}

/// One resource a deployment owns, as the engine recorded it.
///
/// ```
/// use vibe_lifecycle::DeployedResource;
///
/// let owned = DeployedResource {
///     resource: "bin/vibe-helper".into(),
///     post_digest: "0".repeat(64),
/// };
/// assert_eq!(owned.post_digest.len(), 64);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployedResource {
    pub resource: String,
    /// 64 lowercase hex observed by INDEPENDENT verify after apply.
    pub post_digest: String,
}

/// What one executed deploy target did, including the routing decision.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployOutcome {
    pub target: String,
    pub mechanism: String,
    pub provider: String,
    /// Which of §3.1's steps selected the provider.
    pub via: String,
    /// The builtin default this selection displaced, if any.
    pub displaced_default: Option<String>,
    /// The generation this run finalised.
    pub generation: u32,
    pub reversible: bool,
    pub resources: Vec<DeployedResource>,
    /// What settling a previous run's intent journal did, if anything.
    pub settlement: String,
}

/// What one target's `--plan` would do.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployPlanReport {
    pub target: String,
    pub mechanism: String,
    pub provider: String,
    pub via: String,
    pub displaced_default: Option<String>,
    /// Whether this target is work the run would really do — either
    /// because the target itself is stale, or because a target it depends
    /// on is (§7: a plan "reports preceding stale targets as planned
    /// work").
    pub planned: bool,
    /// Why it is planned, in one clause.
    pub reason: String,
    /// The resources the plan would touch. Empty when the artifact this
    /// target deploys has not been produced yet — a read-only planner does
    /// not build it to find out.
    pub resources: Vec<DeployResourcePlan>,
    /// A control-free one-line summary from the provider's own `plan`.
    pub summary: String,
}

/// One planned resource, with what the receipt currently records for it.
///
/// ```
/// use vibe_lifecycle::DeployResourcePlan;
///
/// // A resource no receipt owns yet is a `create`.
/// let planned = DeployResourcePlan {
///     resource: "bin/vibe-helper".into(),
///     desired_digest: "0".repeat(64),
///     recorded_digest: None,
///     change: "create".into(),
/// };
/// assert!(planned.recorded_digest.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployResourcePlan {
    pub resource: String,
    /// 64 lowercase hex the plan wants the resource at.
    pub desired_digest: String,
    /// What the last receipt recorded, when it owned this resource.
    pub recorded_digest: Option<String>,
    /// `create` | `update` | `unchanged`.
    pub change: String,
}

/// What one inverse deployment removed.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalOutcome {
    pub target: String,
    pub provider: String,
    pub removed: Vec<String>,
}

/// One row of `vibe deployments` — receipt facts only, and never a secret.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentRow {
    /// The engine-owned deployment id — this deployment's directory name.
    pub deployment: String,
    pub project: String,
    pub package: Option<String>,
    pub profile: String,
    pub target: String,
    pub generation: u32,
    pub status: DeployStatus,
    /// `workspace` | `user` | `remote` | `system`.
    pub scope: String,
    /// The exact provider identity that applied it.
    pub provider: String,
    pub reversible: bool,
    /// How many resources the deployment still owns.
    pub resources: usize,
    pub applied_at: String,
    pub finalized_at: Option<String>,
}

/// A receipt's final status, in the vocabulary the wire freezes.
///
/// ```
/// use vibe_lifecycle::DeployStatus;
///
/// assert_eq!(DeployStatus::Verified.as_str(), "verified");
/// assert_eq!(DeployStatus::RolledBack.as_str(), "rolled-back");
/// assert_ne!(DeployStatus::Applied, DeployStatus::Failed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployStatus {
    Applied,
    Verified,
    Failed,
    RolledBack,
}

impl DeployStatus {
    /// The word a narration prints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::RolledBack => "rolled-back",
        }
    }

    const fn of(status: &ReceiptStatus) -> Self {
        match status {
            ReceiptStatus::Applied => Self::Applied,
            ReceiptStatus::Verified => Self::Verified,
            ReceiptStatus::Failed => Self::Failed,
            ReceiptStatus::RolledBack => Self::RolledBack,
        }
    }
}

/// One receipt projected into a listing row.
pub(crate) fn row(deployment: String, receipt: &DeployReceipt) -> DeploymentRow {
    DeploymentRow {
        deployment,
        project: receipt.identity.project.clone(),
        package: receipt.identity.package.clone(),
        profile: receipt.profile.clone(),
        target: receipt.target.clone(),
        generation: receipt.generation,
        status: DeployStatus::of(&receipt.status),
        scope: scope_word(&receipt.scope).to_owned(),
        provider: receipt.provider.key.clone(),
        reversible: receipt.reversible,
        resources: receipt.resources.len(),
        applied_at: receipt.applied_at.to_rfc3339(),
        finalized_at: receipt
            .finalized_at
            .as_ref()
            .map(|stamped| stamped.to_rfc3339()),
    }
}

/// The word one destination scope prints as.
const fn scope_word(
    scope: &vibe_wire::generated::deploy_receipt::DestinationScope,
) -> &'static str {
    use vibe_wire::generated::deploy_receipt::DestinationScope as Scope;
    match scope {
        Scope::Workspace => "workspace",
        Scope::User => "user",
        Scope::Remote => "remote",
        Scope::System => "system",
    }
}
