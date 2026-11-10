//! The judgment half of structured installation — one implementation, two
//! planes (PROP-065 `##ordinary-plan`).
//!
//! An installation's declarations answer two questions before anything runs:
//! *will this row run* (its `when` / `unless` conditions over the closed
//! vocabulary of PROP-065 `##conditions`) and *what does it require first*
//! (its `needs`, PROP-065 `##needs`). Both answers are pure: they consume
//! declared rows plus supplied host facts and return a verdict. Nothing here
//! touches the filesystem, starts a process, opens a connection or asks a
//! question — the shape PROP-069 `##PURE-CORE` requires of shared
//! configuration logic, and the reason the application step planner
//! (`vibe-app`) and the ordinary package's slot lifecycle (`vibe-install`)
//! can share it while keeping their own execution engines.
//!
//! Every host-dependent fact arrives as a caller-supplied closure, so the
//! file-presence root of an application's payload and the materialised slot
//! of an ordinary package are the same parameter rather than two evaluators.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan");

use specmark::spec;

pub use crate::manifest::{Condition, Need, ParamEquals};

/// Whether a declared row runs, and the condition that decided against it
/// when it does not.
///
/// A skip carries the deciding condition's own spelling, because a plan that
/// says `skip` without naming the reason is unreadable — PROP-065
/// `##conditions` requires the condition that decided it.
///
/// ```
/// use vibe_core::judgment::Verdict;
///
/// let skipped = Verdict::Skip {
///     reason: "when `feature = \"tools\"` does not hold".into(),
/// };
/// assert!(!skipped.runs());
/// assert_eq!(
///     skipped.reason(),
///     Some("when `feature = \"tools\"` does not hold"),
/// );
/// assert!(Verdict::Run.runs());
/// assert_eq!(Verdict::Run.reason(), None);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every `when` held and no `unless` did.
    Run,
    /// One condition decided against the row; `reason` is its spelling.
    Skip { reason: String },
}

impl Verdict {
    /// Whether the row runs.
    ///
    /// ```
    /// use vibe_core::judgment::Verdict;
    ///
    /// assert!(Verdict::Run.runs());
    /// ```
    pub fn runs(&self) -> bool {
        matches!(self, Self::Run)
    }

    /// The deciding condition's spelling, when the row was skipped.
    ///
    /// ```
    /// use vibe_core::judgment::Verdict;
    ///
    /// assert_eq!(Verdict::Run.reason(), None);
    /// ```
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Run => None,
            Self::Skip { reason } => Some(reason.as_str()),
        }
    }
}

/// The host facts one condition of PROP-065 `##conditions` is decided
/// against.
///
/// Each member answers exactly one key of the closed vocabulary. The three
/// fallible ones are fallible for one reason only: a condition may name a
/// parameter or a template path the declaration does not admit, and a
/// caller's refusal must reach the planner rather than be swallowed as
/// `false`.
///
/// ```
/// use vibe_core::judgment::{Condition, ConditionFacts, condition_holds};
///
/// let facts = ConditionFacts {
///     feature_selected: &|id| id == "tools",
///     param_truthy: &|_| Ok(false),
///     param_equals: &|_, _| Ok(false),
///     command_present: &|name| name == "cargo",
///     file_present: &|_| Ok(false),
///     env_present: &|_| false,
///     agent_configured: false,
///     offline: true,
/// };
/// assert_eq!(
///     condition_holds(&Condition::Feature("tools".into()), &facts),
///     Ok(true),
/// );
/// assert_eq!(
///     condition_holds(&Condition::CommandPresent("gcc".into()), &facts),
///     Ok(false),
/// );
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
#[derive(Clone, Copy)]
pub struct ConditionFacts<'a> {
    /// Whether a feature id is in the chosen set.
    pub feature_selected: &'a dyn Fn(&str) -> bool,
    /// Whether a parameter is a true `bool` or carries a non-empty value.
    pub param_truthy: &'a dyn Fn(&str) -> Result<bool, String>,
    /// Whether a parameter's value equals the expected one.
    pub param_equals: &'a dyn Fn(&str, &str) -> Result<bool, String>,
    /// Whether a bare command name resolves on this host.
    pub command_present: &'a dyn Fn(&str) -> bool,
    /// Whether a declared path exists, after the caller's own expansion.
    pub file_present: &'a dyn Fn(&str) -> Result<bool, String>,
    /// Whether an environment variable is set.
    pub env_present: &'a dyn Fn(&str) -> bool,
    /// Whether an agent is configured and reachable for this run.
    pub agent_configured: bool,
    /// Whether this run is offline.
    pub offline: bool,
}

/// Whether one condition of the closed vocabulary holds.
///
/// The match is exhaustive by construction: the vocabulary is closed
/// (PROP-065 `##conditions`), so a new key fails compilation here rather
/// than defaulting to `false` in one of the two planes.
///
/// ```
/// use vibe_core::judgment::{Condition, ConditionFacts, condition_holds};
///
/// let facts = ConditionFacts {
///     feature_selected: &|_| false,
///     param_truthy: &|_| Ok(false),
///     param_equals: &|_, _| Ok(false),
///     command_present: &|_| false,
///     file_present: &|_| Ok(false),
///     env_present: &|name| name == "CI",
///     agent_configured: false,
///     offline: false,
/// };
/// assert_eq!(
///     condition_holds(&Condition::EnvPresent("CI".into()), &facts),
///     Ok(true),
/// );
/// assert_eq!(
///     condition_holds(&Condition::Offline(false), &facts),
///     Ok(true),
/// );
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
pub fn condition_holds(condition: &Condition, facts: &ConditionFacts<'_>) -> Result<bool, String> {
    Ok(match condition {
        Condition::Feature(id) => (facts.feature_selected)(id),
        Condition::Param(id) => (facts.param_truthy)(id)?,
        Condition::ParamEquals(test) => (facts.param_equals)(&test.id, &test.value)?,
        Condition::CommandPresent(name) => (facts.command_present)(name),
        Condition::FilePresent(path) => (facts.file_present)(path)?,
        Condition::EnvPresent(name) => (facts.env_present)(name),
        Condition::AgentConfigured(expected) => facts.agent_configured == *expected,
        Condition::Offline(expected) => facts.offline == *expected,
    })
}

/// The verdict of a row's `when` (all must hold) and `unless` (none may
/// hold) lists.
///
/// The first condition that decides against the row is the reason, and
/// `when` is evaluated before `unless` so a plan's reason is stable whatever
/// the host happens to be.
///
/// ```
/// use vibe_core::judgment::{Condition, ConditionFacts, verdict};
///
/// let facts = ConditionFacts {
///     feature_selected: &|_| false,
///     param_truthy: &|_| Ok(false),
///     param_equals: &|_, _| Ok(false),
///     command_present: &|name| name == "cargo",
///     file_present: &|_| Ok(false),
///     env_present: &|_| false,
///     agent_configured: false,
///     offline: false,
/// };
/// let decided = verdict(
///     &[Condition::CommandPresent("cargo".into())],
///     &[Condition::Offline(true)],
///     &facts,
/// )
/// .unwrap();
/// assert!(decided.runs());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
pub fn verdict(
    when: &[Condition],
    unless: &[Condition],
    facts: &ConditionFacts<'_>,
) -> Result<Verdict, String> {
    for condition in when {
        if !condition_holds(condition, facts)? {
            return Ok(Verdict::Skip {
                reason: format!("when `{condition}` does not hold"),
            });
        }
    }
    for condition in unless {
        if condition_holds(condition, facts)? {
            return Ok(Verdict::Skip {
                reason: format!("unless `{condition}` holds"),
            });
        }
    }
    Ok(Verdict::Run)
}

/// The host facts one `need` of PROP-065 `##needs` is decided against.
///
/// `network` is the base answer only: a plane whose step kinds can be served
/// from a cache layers that on top of this, which is why the member is a
/// plain `bool` and not a closure over a step.
///
/// ```
/// use vibe_core::judgment::{Need, NeedFacts, need_met};
///
/// let facts = NeedFacts {
///     command_present: &|name| name == "cargo",
///     network: false,
///     agent: false,
///     terminal: true,
/// };
/// assert!(need_met(&Need::Command("cargo".into()), &facts));
/// assert!(!need_met(&Need::Command("gcc".into()), &facts));
/// assert!(need_met(&Need::Terminal, &facts));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
#[derive(Clone, Copy)]
pub struct NeedFacts<'a> {
    /// Whether a bare command name resolves on this host, or is provided by
    /// something the planner has already placed on the child's PATH.
    pub command_present: &'a dyn Fn(&str) -> bool,
    /// Whether the network is usable for this run.
    pub network: bool,
    /// Whether an agent is configured and reachable.
    pub agent: bool,
    /// Whether a terminal is attached.
    pub terminal: bool,
}

/// Whether one need is met.
///
/// ```
/// use vibe_core::judgment::{Need, NeedFacts, need_met};
///
/// let facts = NeedFacts {
///     command_present: &|_| false,
///     network: true,
///     agent: false,
///     terminal: false,
/// };
/// assert!(need_met(&Need::Network, &facts));
/// assert!(!need_met(&Need::Agent, &facts));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
pub fn need_met(need: &Need, facts: &NeedFacts<'_>) -> bool {
    match need {
        Need::Command(name) => (facts.command_present)(name),
        Need::Network => facts.network,
        Need::Agent => facts.agent,
        Need::Terminal => facts.terminal,
    }
}

/// The needs of a row that this host does not meet, in declaration order.
///
/// The planner checks this for every row that will run **before anything
/// starts** and refuses the whole installation up front, naming the row and
/// the missing need (PROP-065 `##needs`). An empty answer is the only one
/// that lets the installation begin.
///
/// ```
/// use vibe_core::judgment::{Need, NeedFacts, unmet_needs};
///
/// let facts = NeedFacts {
///     command_present: &|name| name == "cargo",
///     network: true,
///     agent: false,
///     terminal: true,
/// };
/// let needs = vec![
///     Need::Command("cargo".into()),
///     Need::Command("rustfmt".into()),
///     Need::Agent,
/// ];
/// let missing: Vec<String> = unmet_needs(&needs, &facts)
///     .into_iter()
///     .map(ToString::to_string)
///     .collect();
/// assert_eq!(missing, vec!["command:rustfmt".to_string(), "agent".into()]);
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
pub fn unmet_needs<'n>(needs: &'n [Need], facts: &NeedFacts<'_>) -> Vec<&'n Need> {
    needs.iter().filter(|need| !need_met(need, facts)).collect()
}

#[cfg(test)]
mod tests;
