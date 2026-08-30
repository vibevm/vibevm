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

/// One client CLI, as the command SURFACE resolved it — §6.3.0.6's
/// "explicit Claude/Codex/OpenCode executable paths".
///
/// The two variants are the total answer to one question, and neither is
/// an absence: [`Resolved`](Self::Resolved) carries an ABSOLUTE path the
/// surface really found, and [`Missing`](Self::Missing) carries the command
/// word the surface looked for and did not find. It is deliberately not
/// `Option<PathBuf>`: an option's `None` says only "no value", and a lower
/// cell reading it would have to invent what to do — which is the ambient
/// lookup §6.3.0.6 exists to abolish. `Missing` says *what was looked for*,
/// so a selected client provider can refuse with remediation naming the
/// exact command an operator must install.
///
/// A bare command word never appears in `Resolved`. That is the whole
/// point: `Command::new("claude")` searches `PATH` at spawn time, in the
/// provider, which is precisely the resolution the surface was supposed to
/// have already done. Resolution happens once, above; below, there is
/// either a path or an honest refusal.
///
/// ```
/// use std::path::{Path, PathBuf};
/// use vibe_lifecycle::ClientExecutable;
///
/// let found = ClientExecutable::Resolved {
///     command: "claude".into(),
///     path: PathBuf::from("/opt/bin/claude"),
/// };
/// assert_eq!(found.command(), "claude");
/// // The path the surface found, never the word it searched for.
/// assert_ne!(found.resolved_path(), Some(Path::new("claude")));
///
/// let absent = ClientExecutable::Missing { command: "codex".into() };
/// assert!(absent.resolved_path().is_none());
/// assert_eq!(absent.command(), "codex");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientExecutable {
    /// The surface found this client. `path` is absolute.
    Resolved {
        /// The command word the surface searched for, kept for refusals.
        command: String,
        /// The absolute executable the provider will spawn.
        path: PathBuf,
    },
    /// The surface searched and did not find this client. A run that never
    /// selects it stays perfectly legal; a target that selects it refuses
    /// by name.
    Missing {
        /// The command word an operator must make reachable.
        command: String,
    },
}

impl ClientExecutable {
    /// The command word this member is about, resolved or not.
    #[must_use]
    pub fn command(&self) -> &str {
        match self {
            Self::Resolved { command, .. } | Self::Missing { command } => command,
        }
    }

    /// The absolute executable, when the surface found one.
    ///
    /// Returning an option here is not the ambiguity the type rejects: the
    /// VALUE still says which case it is and why, and this accessor is the
    /// narrow read a spawn site wants. A provider that gets `None` has the
    /// `Missing` variant beside it and must refuse, never search.
    #[must_use]
    pub fn resolved_path(&self) -> Option<&Path> {
        match self {
            Self::Resolved { path, .. } => Some(path),
            Self::Missing { .. } => None,
        }
    }
}

/// The client executables one deploy run may invoke, injected whole by the
/// command surface — §6.3.0.6's "Home and executable authority are
/// injected".
///
/// §6.3.0.6 in one type: "The CLI surface resolves them once; every lower
/// cell and provider is forbidden from calling `dirs::home_dir`, reading
/// `HOME`/`USERPROFILE`/`CODEX_HOME`/`CLAUDE_CONFIG_DIR`, searching `PATH`,
/// or finding a real client."
///
/// Three NAMED members rather than a map keyed by a client id, and each a
/// TOTAL [`ClientExecutable`] rather than an option: the surface answers
/// for all three exactly once, and a client it could not find is a recorded
/// fact rather than a hole a lower cell fills in. One missing client does
/// not fail the run — an ordinary `deploy:vibe-bin` profile never looks at
/// any of them.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_lifecycle::{ClientExecutable, ClientExecutables};
///
/// let fake = ClientExecutables {
///     claude: ClientExecutable::Resolved {
///         command: "claude".into(),
///         path: PathBuf::from("/tmp/fake/claude"),
///     },
///     codex: ClientExecutable::Missing { command: "codex".into() },
///     opencode: ClientExecutable::Missing { command: "opencode".into() },
/// };
/// assert!(fake.claude.resolved_path().is_some());
/// assert_eq!(fake.codex.command(), "codex");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientExecutables {
    /// The Claude Code CLI this run invokes.
    pub claude: ClientExecutable,
    /// The Codex CLI this run invokes.
    pub codex: ClientExecutable,
    /// The OpenCode CLI this run invokes.
    pub opencode: ClientExecutable,
}

impl ClientExecutables {
    /// The three members, in one order, for a caller that judges all of
    /// them — the arrival fence reads this rather than naming each field.
    #[must_use]
    pub fn all(&self) -> [&ClientExecutable; 3] {
        [&self.claude, &self.codex, &self.opencode]
    }
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
    /// The absolute vibevm settings directory the state home hangs off,
    /// carried beside it — §7.1.0 ruling 2's "`DeployExecution` carries the
    /// settings root beside the state home; a provider never resolves a
    /// home".
    ///
    /// A user-scope deploy provider reconciles a destination *inside* this
    /// root, so it has to be told where the root is. It is the same
    /// parameter the state home already is, and for the same reason: the
    /// command layer resolves the settings directory ONCE, and no cell
    /// below a surface calls `settings_dir()`.
    pub settings_root: &'a Path,
    /// The invoking user's home directory, resolved ONCE at the command
    /// surface — §6.3.0.6's "`DeployExecution` carries the exact user home
    /// beside `settings_root`".
    ///
    /// It is NOT `settings_root`, and the two are not derivable from each
    /// other: `$VIBE_SETTINGS` relocates the settings directory anywhere,
    /// while a client destination (`~/.claude/…`, `~/.agents/…`,
    /// `~/.config/opencode/…`) hangs off the home itself. A cell that took
    /// the settings root for the home would write a user's skills inside
    /// vibevm's own state directory, or — with the override unset — the
    /// reverse.
    pub user_home: &'a Path,
    /// The client executables this run may invoke, injected whole.
    pub clients: &'a ClientExecutables,
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
