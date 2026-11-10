//! The judgment the slot plan is decided by, and the consent an untrusted
//! dependency's contributions need before any of them runs.
//!
//! Whether a contribution runs and what it requires first is PROP-065
//! `##ordinary-plan`'s shared library (`vibe_core::judgment`); what this
//! module adds is the plane's own facts — a PATH probe, an environment
//! probe, attendedness, offline — plus the trust policy of PROP-065
//! `##ordinary-trust`. Execution stays the lifecycle handler engine, which
//! is the half the two planes deliberately do not share.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan");

use std::sync::Arc;

use specmark::spec;
use vibe_core::judgment::{self, Condition, Need};
use vibe_core::manifest::SlotJudgment;
use vibe_core::trust::TrustPolicy;

/// One package whose contributions a question is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotConsentRequest {
    /// `group/name@version` of the declaring dependency.
    pub package: String,
    /// The group the trust decision is about.
    pub group: String,
    /// One line per contribution that will run: `id (handler) title`.
    pub contributions: Vec<String>,
}

/// Asks the operator the one question of PROP-065 `##ordinary-trust`.
pub type SlotConsentAsk =
    Arc<dyn Fn(&SlotConsentRequest) -> std::result::Result<bool, String> + Send + Sync>;

/// The host facts and consent inputs one install pass judges its slot plan
/// against.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
#[derive(Clone)]
pub struct SlotJudgmentContext {
    /// Whether a bare command name resolves on this host.
    pub command_present: fn(&str) -> bool,
    /// Whether an environment variable is set.
    pub env_present: fn(&str) -> bool,
    /// Whether a terminal is attached.
    pub attended: bool,
    /// Whether an agent is configured for this run.
    pub agent_configured: bool,
    /// The effective trust of this run, resolved by the caller from the
    /// one mechanism of `vibe_core::trust`: the base list vibe ships
    /// (`org.vibevm.*`), the site profile's `[trust]` table and the user's
    /// `agents.toml`, with either file's `untrusted_groups` overriding all
    /// of it.
    pub trust: TrustPolicy,
    /// `--assume-yes`, which `--unattended` implies: the per-run escape
    /// agent steps already use. No new flag is invented.
    pub assume_yes: bool,
    /// The attended question; `None` when no terminal can answer one.
    pub confirm: Option<SlotConsentAsk>,
}

fn never(_: &str) -> bool {
    false
}

impl SlotJudgmentContext {
    /// A context that decides nothing and consents to everything — the
    /// caller states it, rather than defaulting into it, exactly as
    /// [`crate::SlotLifecycleSeams::refusing`] does for the agent backend.
    /// Correct for a fixture whose packages are its own, and for the legacy
    /// `apply` entry points, where consent is the caller's business.
    ///
    /// ```
    /// use vibe_install::SlotJudgmentContext;
    ///
    /// let context = SlotJudgmentContext::consenting();
    /// assert!(context.assume_yes);
    /// assert!(!context.trusts("org.vibevm"));
    /// ```
    #[must_use]
    pub fn consenting() -> Self {
        Self {
            command_present: never,
            env_present: never,
            attended: false,
            agent_configured: false,
            trust: TrustPolicy::EMPTY,
            assume_yes: true,
            confirm: None,
        }
    }

    /// Whether this group's contributions run without a question.
    ///
    /// The answer is the shared mechanism's and no second rule: an entry is
    /// an exact group or the subtree `prefix.*`, and a group the policy's
    /// `untrusted_groups` covers is never trusted.
    ///
    /// ```
    /// use vibe_core::trust::TrustPolicy;
    /// use vibe_install::SlotJudgmentContext;
    ///
    /// let context = SlotJudgmentContext {
    ///     trust: TrustPolicy::trusting(["org.example"]).unwrap(),
    ///     ..SlotJudgmentContext::consenting()
    /// };
    /// assert!(context.trusts("org.example"));
    /// assert!(!context.trusts("org.example.sub"));
    ///
    /// // What vibe ships: everything under `org.vibevm.*`, the group
    /// // itself included (owner decision 2026-11-10).
    /// let shipped = SlotJudgmentContext {
    ///     trust: TrustPolicy::base(),
    ///     ..SlotJudgmentContext::consenting()
    /// };
    /// assert!(shipped.trusts("org.vibevm.ai-native"));
    /// assert!(!shipped.trusts("org.example"));
    /// ```
    pub fn trusts(&self, group: &str) -> bool {
        self.trust.trusts(group)
    }

    /// Decide one contribution: whether it runs, and why not.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
    pub(super) fn decide(
        &self,
        declared: Option<&SlotJudgment>,
        offline: bool,
        slot: &std::path::Path,
    ) -> (bool, Option<String>) {
        let Some(declared) = declared else {
            return (true, None);
        };
        let facts = judgment::ConditionFacts {
            // `feature` and `param` read the package-scope configuration of
            // PROP-069 `#AUTHORED-DEFINITIONS`, which `#PACKAGE-ADMISSION`
            // still refuses to install at all — so the selected set is empty
            // by construction and such a condition is false. The seam is the
            // closure, not a branch, so the day that interim refusal is
            // lifted this plane reads the real snapshot unchanged.
            feature_selected: &never,
            param_truthy: &|_| Ok(false),
            param_equals: &|_, _| Ok(false),
            command_present: &self.command_present,
            // The materialised slot is the ordinary package's content, and
            // so its equivalent of an application's payload root.
            file_present: &|path| Ok(slot.join(path).exists()),
            env_present: &self.env_present,
            agent_configured: self.agent_configured,
            offline,
        };
        match judgment::verdict(&declared.when, &declared.unless, &facts) {
            Ok(judgment::Verdict::Run) => (true, None),
            Ok(judgment::Verdict::Skip { reason }) => (false, Some(reason)),
            Err(reason) => (false, Some(reason)),
        }
    }

    /// The needs of a contribution this host does not meet.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
    pub(super) fn unmet(&self, needs: &[Need], offline: bool) -> Vec<String> {
        let facts = judgment::NeedFacts {
            command_present: &self.command_present,
            // An ordinary package's contribution fetches nothing through
            // vibe, so `network` is simply whether this run may use it.
            network: !offline,
            agent: self.agent_configured,
            terminal: self.attended,
        };
        judgment::unmet_needs(needs, &facts)
            .into_iter()
            .map(ToString::to_string)
            .collect()
    }
}

/// The condition spellings a plan row prints under its label, so a reader
/// sees what the verdict was decided from.
pub(super) fn condition_lines(declared: Option<&SlotJudgment>) -> Vec<String> {
    let Some(declared) = declared else {
        return Vec::new();
    };
    let render = |field: &str, conditions: &[Condition]| -> Option<String> {
        (!conditions.is_empty()).then(|| {
            format!(
                "{field}: {}",
                conditions
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    };
    render("when", &declared.when)
        .into_iter()
        .chain(render("unless", &declared.unless))
        .collect()
}

#[cfg(test)]
mod tests;
