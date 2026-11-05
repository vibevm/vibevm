//! Explicit typed conversion between the two field-presence grammars.
use vibe_wire::generated::shared as w;

pub(super) fn to_portable(v: &w::ConfigurationDefinition) -> w::PortableConfigurationDefinition {
    w::PortableConfigurationDefinition {
        features: v.features.iter().map(feature_to).collect(),
        groups: v.groups.iter().map(group_to).collect(),
        parameters: v.parameters.iter().map(parameter_to).collect(),
        profiles: v.profiles.iter().map(profile_to).collect(),
        config_values: v.config_values.clone(),
    }
}
pub(super) fn from_portable(v: &w::PortableConfigurationDefinition) -> w::ConfigurationDefinition {
    w::ConfigurationDefinition {
        features: v.features.iter().map(feature_from).collect(),
        groups: v.groups.iter().map(group_from).collect(),
        parameters: v.parameters.iter().map(parameter_from).collect(),
        profiles: v.profiles.iter().map(profile_from).collect(),
        config_values: v.config_values.clone(),
    }
}
fn feature_to(v: &w::ConfigurationFeature) -> w::PortableConfigurationFeature {
    w::PortableConfigurationFeature {
        id: v.id.clone(),
        name: v.name.clone(),
        visibility: v.visibility.clone(),
        required: v.required,
        default: v.default,
        requires: v.requires.clone(),
        guards: v.guards.clone(),
        effects: v.effects.clone(),
        description: v.description.clone(),
        parent: v.parent.clone(),
    }
}
fn feature_from(v: &w::PortableConfigurationFeature) -> w::ConfigurationFeature {
    w::ConfigurationFeature {
        id: v.id.clone(),
        name: v.name.clone(),
        visibility: v.visibility.clone(),
        required: v.required,
        default: v.default,
        requires: v.requires.clone(),
        guards: v.guards.clone(),
        effects: v.effects.clone(),
        description: v.description.clone(),
        parent: v.parent.clone(),
    }
}
fn parameter_to(v: &w::ConfigurationParameter) -> w::PortableConfigurationParameter {
    w::PortableConfigurationParameter {
        id: v.id.clone(),
        name: v.name.clone(),
        type_: v.type_.clone(),
        required: v.required,
        choices: v.choices.clone(),
        must_exist: v.must_exist,
        secret: v.secret,
        description: v.description.clone(),
        default: v.default.clone(),
        min: v.min.clone(),
        max: v.max.clone(),
    }
}
fn parameter_from(v: &w::PortableConfigurationParameter) -> w::ConfigurationParameter {
    w::ConfigurationParameter {
        id: v.id.clone(),
        name: v.name.clone(),
        type_: v.type_.clone(),
        required: v.required,
        choices: v.choices.clone(),
        must_exist: v.must_exist,
        secret: v.secret,
        description: v.description.clone(),
        default: v.default.clone(),
        min: v.min.clone(),
        max: v.max.clone(),
    }
}
fn profile_to(v: &w::ConfigurationProfile) -> w::PortableConfigurationProfile {
    w::PortableConfigurationProfile {
        id: v.id.clone(),
        name: v.name.clone(),
        default: v.default,
        features: v.features.clone(),
        parameters: v.parameters.clone(),
        description: v.description.clone(),
        selections: v
            .selections
            .iter()
            .map(|(k, v)| (k.clone(), super::input::selection_to(v)))
            .collect(),
    }
}
fn profile_from(v: &w::PortableConfigurationProfile) -> w::ConfigurationProfile {
    w::ConfigurationProfile {
        id: v.id.clone(),
        name: v.name.clone(),
        default: v.default,
        features: v.features.clone(),
        parameters: v.parameters.clone(),
        description: v.description.clone(),
        selections: v
            .selections
            .iter()
            .map(|(k, v)| (k.clone(), super::input::selection_from(v)))
            .collect(),
    }
}
fn group_to(v: &w::ConfigurationGroup) -> w::PortableConfigurationGroup {
    match v {
        w::ConfigurationGroup::Choice(v) => {
            w::PortableConfigurationGroup::Choice(Box::new(w::PortableConfigurationGroupChoice {
                id: v.id.clone(),
                name: v.name.clone(),
                cardinality: v.cardinality.clone(),
                members: v.members.clone(),
                guards: v.guards.clone(),
                defaults: v.defaults.clone(),
                description: v.description.clone(),
            }))
        }
        w::ConfigurationGroup::Closure(v) => {
            w::PortableConfigurationGroup::Closure(Box::new(w::PortableConfigurationGroupClosure {
                id: v.id.clone(),
                name: v.name.clone(),
                cardinality: v.cardinality.clone(),
                members: v.members.clone(),
                fallback: v.fallback.clone(),
                description: v.description.clone(),
            }))
        }
    }
}
fn group_from(v: &w::PortableConfigurationGroup) -> w::ConfigurationGroup {
    match v {
        w::PortableConfigurationGroup::Choice(v) => {
            w::ConfigurationGroup::Choice(Box::new(w::ConfigurationGroupChoice {
                id: v.id.clone(),
                name: v.name.clone(),
                cardinality: v.cardinality.clone(),
                members: v.members.clone(),
                guards: v.guards.clone(),
                defaults: v.defaults.clone(),
                description: v.description.clone(),
            }))
        }
        w::PortableConfigurationGroup::Closure(v) => {
            w::ConfigurationGroup::Closure(Box::new(w::ConfigurationGroupClosure {
                id: v.id.clone(),
                name: v.name.clone(),
                cardinality: v.cardinality.clone(),
                members: v.members.clone(),
                fallback: v.fallback.clone(),
                description: v.description.clone(),
            }))
        }
    }
}
