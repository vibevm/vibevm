//! Projecting the selected ritual into its machine plan values.
//!
//! Selection only — nothing here runs a contribution, and nothing here renders.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::Result;
use specmark::spec;
use vibe_lifecycle::{ContributionTier, ExtensionProvider, RunMetadata};
use vibe_wire::generated::lifecycle_plan::PlannedContribution;

use crate::PlannedExecution;
use crate::RitualPlan;
use crate::ports::RunObserver;

pub(crate) fn surface_plan(
    observer: &dyn RunObserver,
    plan: &RitualPlan,
    metadata: &RunMetadata,
    emit_empty: bool,
) -> Result<()> {
    observer.observe_plan(plan, metadata, emit_empty)
}

/// Project one planned contribution into its machine plan row.
///
/// ```
/// use vibe_orchestrator::{PlannedExecution, planned_contribution};
/// fn project(execution: &PlannedExecution) -> String {
///     planned_contribution(execution).key
/// }
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn planned_contribution(execution: &PlannedExecution) -> PlannedContribution {
    let row = &execution.row;
    let (provider, version) = provider_and_version(row.provider());
    PlannedContribution {
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        reference: None,
        slot_target: None,
        tier: tier_name(row.effective_tier()).to_string(),
        version,
    }
}

/// The rendered provider identity and, when a version is known, its version.
///
/// ```
/// use vibe_orchestrator::provider_and_version;
/// fn describe(provider: &vibe_lifecycle::ExtensionProvider) -> String {
///     provider_and_version(provider).0
/// }
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn provider_and_version(provider: &ExtensionProvider) -> (String, Option<String>) {
    match provider {
        ExtensionProvider::Dependency(provider) => {
            (provider.id.to_string(), Some(provider.version.clone()))
        }
        ExtensionProvider::Host(provider) => (
            provider.identity.to_string(),
            (!provider.version.is_empty()).then(|| provider.version.clone()),
        ),
    }
}

/// The exact wire spelling of one contribution tier.
///
/// ```
/// use vibe_lifecycle::ContributionTier;
/// use vibe_orchestrator::tier_name;
/// assert_eq!(tier_name(ContributionTier::Preset), "preset");
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub const fn tier_name(tier: ContributionTier) -> &'static str {
    match tier {
        ContributionTier::Preset => "preset",
        ContributionTier::Dependency => "dependency",
        ContributionTier::HostDeclaration => "host-declaration",
        ContributionTier::HostActivation => "host-activation",
    }
}
