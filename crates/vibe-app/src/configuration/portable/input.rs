//! Original omitted/empty/exact/additive input is transferred without normalization.
use vibe_wire::generated::shared as w;
pub(super) fn to_portable(v: &w::ConfigurationInput) -> w::PortableConfigurationInput {
    w::PortableConfigurationInput {
        intent: w::PortableConfigurationSourceInput {
            policy: v.intent.policy.clone(),
            features: v.intent.features.clone(),
            default_features: v.intent.default_features,
            selections: v
                .intent
                .selections
                .iter()
                .map(|(k, v)| (k.clone(), selection_to(v)))
                .collect(),
            parameters: v.intent.parameters.clone(),
            profile: v.intent.profile.clone(),
        },
        disabled_features: v.disabled_features.clone(),
    }
}
pub(super) fn selection_to(v: &w::ConfigurationSelection) -> w::PortableConfigurationSelection {
    w::PortableConfigurationSelection {
        intent: v.intent.clone(),
        members: v.members.clone(),
    }
}
pub(super) fn from_portable(v: &w::PortableConfigurationInput) -> w::ConfigurationInput {
    w::ConfigurationInput {
        intent: w::ConfigurationSourceInput {
            policy: v.intent.policy.clone(),
            features: v.intent.features.clone(),
            default_features: v.intent.default_features,
            selections: v
                .intent
                .selections
                .iter()
                .map(|(k, v)| (k.clone(), selection_from(v)))
                .collect(),
            parameters: v.intent.parameters.clone(),
            profile: v.intent.profile.clone(),
        },
        disabled_features: v.disabled_features.clone(),
    }
}
pub(super) fn selection_from(v: &w::PortableConfigurationSelection) -> w::ConfigurationSelection {
    w::ConfigurationSelection {
        intent: v.intent.clone(),
        members: v.members.clone(),
    }
}
