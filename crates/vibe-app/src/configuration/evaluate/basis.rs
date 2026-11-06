//! Existing app wire spelling adapts the shared typed policy; parameters stay here.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS");
use super::*;
use vibe_core::manifest::configuration_basis as policy;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Reason {
    Consumer(policy::ConsumerReliance),
    Instance(policy::InstanceReliance),
    Parameter(String),
    Other(String),
}
fn reason(value: &str) -> Reason {
    match value {
        "profile" => Reason::Consumer(policy::ConsumerReliance::Profile),
        "feature-defaults" => Reason::Consumer(policy::ConsumerReliance::FeatureDefaults),
        _ => {
            if let Some(id) = value.strip_prefix("choice:") {
                Reason::Consumer(policy::ConsumerReliance::Choice(id.into()))
            } else if let Some(id) = value.strip_prefix("closure:") {
                Reason::Instance(policy::InstanceReliance::Closure(id.into()))
            } else if let Some(id) = value.strip_prefix("parameter:") {
                Reason::Parameter(id.into())
            } else {
                Reason::Other(value.into())
            }
        }
    }
}
pub(super) fn authority(replacement: bool, reviewed: bool) -> policy::Authority {
    if replacement {
        policy::Authority::Replacement
    } else if reviewed {
        policy::Authority::Reviewed
    } else {
        policy::Authority::Reuse
    }
}
pub(super) fn feature_reliance(
    definition: &m::ConfigurationDecl,
    input: &m::ConfigurationIntentDecl,
    normalized: &m::NormalizedConfigurationInput,
    result: Option<&k::FeatureResolution>,
    previous: &[String],
    authority: policy::Authority,
) -> Vec<String> {
    let mut consumers = BTreeSet::new();
    let mut instance = BTreeSet::new();
    for value in previous {
        match reason(value) {
            Reason::Consumer(value) => {
                consumers.insert(value);
            }
            Reason::Instance(value) => {
                instance.insert(value);
            }
            _ => {}
        }
    }
    let input = policy::ConsumerInput {
        original: input,
        normalized,
    };
    let consumers = policy::consumer_reliance(definition, input, &consumers, authority);
    let instance = policy::instance_reliance(definition, &[input], result, &instance, authority);
    consumers
        .into_iter()
        .map(|value| match value {
            policy::ConsumerReliance::Profile => "profile".into(),
            policy::ConsumerReliance::FeatureDefaults => "feature-defaults".into(),
            policy::ConsumerReliance::Choice(id) => format!("choice:{id}"),
        })
        .chain(instance.into_iter().map(|value| match value {
            policy::InstanceReliance::Closure(id) => format!("closure:{id}"),
        }))
        .collect()
}
pub(super) fn review_required(
    authority: policy::Authority,
    fingerprint: &str,
    reliance: &[String],
    previous: &crate::configuration::ConfigurationHistory<'_>,
) -> bool {
    let retained = previous.reliance.iter().map(|r| reason(r)).collect();
    let current = reliance.iter().map(|r| reason(r)).collect();
    matches!(
        policy::review(
            authority,
            Some(policy::Basis {
                fingerprint: previous.fingerprint,
                reliance: &retained
            }),
            policy::Basis {
                fingerprint,
                reliance: &current
            }
        ),
        policy::Review::Required(_)
    )
}
