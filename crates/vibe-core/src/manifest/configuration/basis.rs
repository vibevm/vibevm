//! Pure automatic reliance and retained-basis review; no evaluation or domain I/O.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS");
use super::{
    ConfigurationDecl, ConfigurationGroupDecl, ConfigurationIntentDecl,
    NormalizedConfigurationInput,
};
use crate::configuration::{
    ChoicePolicy, FeatureBase, FeatureResolution, ImplicationReason, Selection,
};
use std::collections::BTreeSet;

/// Authority already established by the operation owner, never inferred from input edits.
/// ```
/// use vibe_core::manifest::configuration_basis::Authority;
/// let retained = Authority::Reuse;
/// assert_ne!(retained, Authority::Reviewed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    Reuse,
    Reviewed,
    Replacement,
}

/// Reliance attributable to one actual consumer's original/normalized input.
/// ```
/// use vibe_core::manifest::configuration_basis::ConsumerReliance;
/// let reason = ConsumerReliance::Choice("mode".into());
/// assert_ne!(reason, ConsumerReliance::Profile);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsumerReliance {
    Profile,
    FeatureDefaults,
    Choice(String),
}

/// Reliance of the merged feature closure; not a fabricated consumer answer.
/// ```
/// use vibe_core::manifest::configuration_basis::InstanceReliance;
/// let reason = InstanceReliance::Closure("backend".into());
/// assert_eq!(reason, InstanceReliance::Closure("backend".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstanceReliance {
    Closure(String),
}

/// One domain-owned input paired with its actual normalization result.
/// ```
/// use vibe_core::manifest::{ConfigurationDecl, ConfigurationIntentDecl, configuration_basis::ConsumerInput};
/// let definition = ConfigurationDecl::default();
/// let original: ConfigurationIntentDecl = toml::from_str("policy='require-selection'\ndefault_features=false")?;
/// let normalized = definition.normalize_input(&original, "consumer")?;
/// let input = ConsumerInput { original: &original, normalized: &normalized };
/// assert_eq!(input.normalized.choices.key, "consumer");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy)]
pub struct ConsumerInput<'a> {
    pub original: &'a ConfigurationIntentDecl,
    pub normalized: &'a NormalizedConfigurationInput,
}

/// Collect current and carried consumer reliance, including dormant/absent defaults.
/// ```
/// use vibe_core::manifest::{ConfigurationDecl, ConfigurationIntentDecl, configuration_basis::*};
/// let d = ConfigurationDecl::default();
/// let i: ConfigurationIntentDecl = toml::from_str("policy='require-selection'")?;
/// let n = d.normalize_input(&i, "c")?;
/// let reasons = consumer_reliance(&d, ConsumerInput { original: &i, normalized: &n }, &Default::default(), Authority::Reuse);
/// assert!(reasons.contains(&ConsumerReliance::FeatureDefaults));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
pub fn consumer_reliance(
    definition: &ConfigurationDecl,
    input: ConsumerInput<'_>,
    previous: &BTreeSet<ConsumerReliance>,
    authority: Authority,
) -> BTreeSet<ConsumerReliance> {
    let mut out = BTreeSet::new();
    if input.original.profile.is_some() {
        out.insert(ConsumerReliance::Profile);
    } else if input.original.default_features {
        out.insert(ConsumerReliance::FeatureDefaults);
    }
    let automatic = |id: &str| {
        input.original.policy == ChoicePolicy::UseDeclaredDefaults
            && input
                .normalized
                .choices
                .answers
                .get(id)
                .is_none_or(|a| matches!(a.selection, Selection::Omitted))
    };
    for group in &definition.groups {
        if let ConfigurationGroupDecl::Choice { id, .. } = group
            && automatic(id)
        {
            out.insert(ConsumerReliance::Choice(id.clone()));
        }
    }
    if authority != Authority::Replacement {
        for reason in previous {
            if let ConsumerReliance::Choice(id) = reason {
                let exists = definition.groups.iter().any(|g| matches!(g, ConfigurationGroupDecl::Choice { id: current, .. } if current == id));
                if automatic(id) && (exists || authority != Authority::Reviewed) {
                    out.insert(reason.clone());
                }
            }
        }
    }
    out
}

/// Keep observed closure fallback history until explicit takeover or removal review.
/// Normalized inputs belong to the same merged instance, not ambient installations.
/// ```
/// use vibe_core::manifest::{ConfigurationDecl, configuration_basis::*};
/// use std::collections::BTreeSet;
/// let prior = BTreeSet::from([InstanceReliance::Closure("removed".into())]);
/// assert_eq!(instance_reliance(&ConfigurationDecl::default(), &[], None, &prior, Authority::Reuse), prior);
/// assert!(instance_reliance(&ConfigurationDecl::default(), &[], None, &prior, Authority::Reviewed).is_empty());
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
pub fn instance_reliance(
    definition: &ConfigurationDecl,
    consumers: &[ConsumerInput<'_>],
    result: Option<&FeatureResolution>,
    previous: &BTreeSet<InstanceReliance>,
    authority: Authority,
) -> BTreeSet<InstanceReliance> {
    let mut out = BTreeSet::new();
    if let Some(result) = result {
        for implied in &result.implied {
            if let ImplicationReason::GroupDefault { group } = &implied.reason {
                out.insert(InstanceReliance::Closure(group.clone()));
            }
        }
    }
    if authority != Authority::Replacement {
        for reason in previous {
            let InstanceReliance::Closure(id) = reason;
            let group = definition.groups.iter().find_map(|g| match g {
                ConfigurationGroupDecl::Closure {
                    id: current,
                    members,
                    ..
                } if current == id => Some(members),
                _ => None,
            });
            let explicit = group.is_some_and(|members| members.iter().any(|member| consumers.iter().any(|input| {
                input.original.features.contains(member)
                    || matches!(&input.normalized.features.base, FeatureBase::Selected(v) if v.contains(member))
                    || input.normalized.choices.answers.values().any(|a| matches!(&a.selection, Selection::Explicit(v) if v.contains(member)))
            })));
            if !explicit && (group.is_some() || authority != Authority::Reviewed) {
                out.insert(reason.clone());
            }
        }
    }
    out
}

/// Domain-supplied semantic fingerprint and typed reliance (including domain reasons).
/// ```
/// use vibe_core::manifest::configuration_basis::Basis;
/// use std::collections::BTreeSet;
/// let reasons = BTreeSet::from(["parameter:count"]);
/// let basis = Basis { fingerprint: "meaning-v1", reliance: &reasons };
/// assert_eq!(basis.fingerprint, "meaning-v1");
/// ```
pub struct Basis<'a, R> {
    pub fingerprint: &'a str,
    pub reliance: &'a BTreeSet<R>,
}

/// Why retained automatic meaning needs another explicit decision.
/// ```
/// use vibe_core::manifest::configuration_basis::ReviewCause;
/// assert_ne!(ReviewCause::ChangedMeaning, ReviewCause::AdditionalReliance);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewCause {
    ChangedMeaning,
    AdditionalReliance,
}

/// Pure review verdict; it grants no operation or persistence authority.
/// ```
/// use vibe_core::manifest::configuration_basis::Review;
/// assert_eq!(Review::NotRequired, Review::NotRequired);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Review {
    NotRequired,
    Required(BTreeSet<ReviewCause>),
}

/// Compare retained meaning without treating fresh input reads as consent renewal.
/// Domain-specific parameter reliance can participate without moving host policy here.
/// ```
/// use vibe_core::manifest::configuration_basis::*;
/// use std::collections::BTreeSet;
/// let reasons = BTreeSet::from([ConsumerReliance::FeatureDefaults]);
/// let old = Basis { fingerprint: "old", reliance: &reasons };
/// let now = Basis { fingerprint: "new", reliance: &reasons };
/// assert_eq!(review(Authority::Reuse, Some(old), now), Review::Required(BTreeSet::from([ReviewCause::ChangedMeaning])));
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
pub fn review<R: Ord>(
    authority: Authority,
    previous: Option<Basis<'_, R>>,
    current: Basis<'_, R>,
) -> Review {
    let Some(previous) = previous.filter(|_| authority == Authority::Reuse) else {
        return Review::NotRequired;
    };
    let mut causes = BTreeSet::new();
    if previous.fingerprint != current.fingerprint && !current.reliance.is_empty() {
        causes.insert(ReviewCause::ChangedMeaning);
    }
    if current
        .reliance
        .iter()
        .any(|reason| !previous.reliance.contains(reason))
    {
        causes.insert(ReviewCause::AdditionalReliance);
    }
    if causes.is_empty() {
        Review::NotRequired
    } else {
        Review::Required(causes)
    }
}

#[cfg(test)]
#[path = "basis/tests.rs"]
mod tests;
