//! Lossless typed authored snapshot for descriptions and review history.
use super::vocabulary::*;
use vibe_core::manifest as m;
use vibe_wire::generated::shared as w;

pub(super) fn definition(v: &m::ConfigurationDecl) -> w::ConfigurationDefinition {
    w::ConfigurationDefinition {
        features: v.features.iter().map(feature).collect(),
        groups: v.groups.iter().map(group).collect(),
        parameters: v.parameters.iter().map(parameter).collect(),
        profiles: v
            .profiles
            .iter()
            .map(|p| w::ConfigurationProfile {
                id: p.id.clone(),
                name: p.name.clone(),
                description: p.description.clone(),
                default: p.default,
                features: p.features.clone(),
                selections: p
                    .selections
                    .iter()
                    .map(|(k, v)| (k.clone(), selection(v)))
                    .collect(),
                parameters: p
                    .parameters
                    .iter()
                    .map(|(k, v)| (k.clone(), value(v)))
                    .collect(),
            })
            .collect(),
        config_values: v
            .config_values
            .iter()
            .map(|(k, v)| (k.clone(), value(v)))
            .collect(),
    }
}
pub(super) fn read_definition(
    v: &w::ConfigurationDefinition,
) -> Result<m::ConfigurationDecl, String> {
    let result = m::ConfigurationDecl {
        features: v
            .features
            .iter()
            .map(|f| m::ConfigurationFeatureDecl {
                id: f.id.clone(),
                name: f.name.clone(),
                description: f.description.clone(),
                parent: f.parent.clone(),
                visibility: read_visibility(&f.visibility),
                required: f.required,
                default: f.default,
                requires: f.requires.clone(),
                guards: f.guards.iter().map(read_guard).collect(),
                effects: f.effects.iter().map(read_effect).collect(),
            })
            .collect(),
        groups: v.groups.iter().map(read_group).collect(),
        parameters: v
            .parameters
            .iter()
            .map(read_parameter)
            .collect::<Result<_, _>>()?,
        profiles: v
            .profiles
            .iter()
            .map(|p| {
                Ok(m::ConfigurationProfileDecl {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    description: p.description.clone(),
                    default: p.default,
                    features: p.features.clone(),
                    selections: p
                        .selections
                        .iter()
                        .map(|(k, v)| (k.clone(), read_selection(v)))
                        .collect(),
                    parameters: p
                        .parameters
                        .iter()
                        .map(|(k, v)| read_value(v).map(|v| (k.clone(), v)))
                        .collect::<Result<_, String>>()?,
                })
            })
            .collect::<Result<_, String>>()?,
        config_values: v
            .config_values
            .iter()
            .map(|(k, v)| read_value(v).map(|v| (k.clone(), v)))
            .collect::<Result<_, _>>()?,
    };
    result
        .compile_choices()
        .map_err(|e| format!("{}: {e}", e.field))?;
    Ok(result)
}
fn feature(f: &m::ConfigurationFeatureDecl) -> w::ConfigurationFeature {
    w::ConfigurationFeature {
        id: f.id.clone(),
        name: f.name.clone(),
        description: f.description.clone(),
        parent: f.parent.clone(),
        visibility: visibility(&f.visibility),
        required: f.required,
        default: f.default,
        requires: f.requires.clone(),
        guards: f.guards.iter().map(guard).collect(),
        effects: f.effects.iter().map(effect).collect(),
    }
}
fn effect(e: &m::ConfigurationEffect) -> w::ConfigurationEffect {
    match e {
        m::ConfigurationEffect::ConfigValue { name } => {
            w::ConfigurationEffect::ConfigValue(Box::new(w::ConfigurationEffectConfigValue {
                name: name.clone(),
            }))
        }
        m::ConfigurationEffect::Dependency { target } => {
            w::ConfigurationEffect::Dependency(Box::new(w::ConfigurationEffectDependency {
                target: target.clone(),
            }))
        }
        m::ConfigurationEffect::Subskill { path } => {
            w::ConfigurationEffect::Subskill(Box::new(w::ConfigurationEffectSubskill {
                path: path.clone(),
            }))
        }
        m::ConfigurationEffect::Forward {
            target,
            feature,
            strength: s,
        } => w::ConfigurationEffect::Forward(Box::new(w::ConfigurationEffectForward {
            target: target.clone(),
            feature: feature.clone(),
            strength: strength(s),
        })),
    }
}
fn read_effect(e: &w::ConfigurationEffect) -> m::ConfigurationEffect {
    match e {
        w::ConfigurationEffect::ConfigValue(e) => m::ConfigurationEffect::ConfigValue {
            name: e.name.clone(),
        },
        w::ConfigurationEffect::Dependency(e) => m::ConfigurationEffect::Dependency {
            target: e.target.clone(),
        },
        w::ConfigurationEffect::Subskill(e) => m::ConfigurationEffect::Subskill {
            path: e.path.clone(),
        },
        w::ConfigurationEffect::Forward(e) => m::ConfigurationEffect::Forward {
            target: e.target.clone(),
            feature: e.feature.clone(),
            strength: read_strength(&e.strength),
        },
    }
}
fn group(g: &m::ConfigurationGroupDecl) -> w::ConfigurationGroup {
    match g {
        m::ConfigurationGroupDecl::Choice {
            id,
            name,
            description,
            cardinality: c,
            members,
            defaults,
            guards,
        } => w::ConfigurationGroup::Choice(Box::new(w::ConfigurationGroupChoice {
            id: id.clone(),
            name: name.clone(),
            description: description.clone(),
            cardinality: cardinality(c),
            members: members.clone(),
            defaults: defaults.clone(),
            guards: guards.iter().map(guard).collect(),
        })),
        m::ConfigurationGroupDecl::Closure {
            id,
            name,
            description,
            cardinality: c,
            members,
            fallback,
        } => w::ConfigurationGroup::Closure(Box::new(w::ConfigurationGroupClosure {
            id: id.clone(),
            name: name.clone(),
            description: description.clone(),
            cardinality: cardinality(c),
            members: members.clone(),
            fallback: fallback.clone(),
        })),
    }
}
fn read_group(g: &w::ConfigurationGroup) -> m::ConfigurationGroupDecl {
    match g {
        w::ConfigurationGroup::Choice(g) => m::ConfigurationGroupDecl::Choice {
            id: g.id.clone(),
            name: g.name.clone(),
            description: g.description.clone(),
            cardinality: read_cardinality(&g.cardinality),
            members: g.members.clone(),
            defaults: g.defaults.clone(),
            guards: g.guards.iter().map(read_guard).collect(),
        },
        w::ConfigurationGroup::Closure(g) => m::ConfigurationGroupDecl::Closure {
            id: g.id.clone(),
            name: g.name.clone(),
            description: g.description.clone(),
            cardinality: read_cardinality(&g.cardinality),
            members: g.members.clone(),
            fallback: g.fallback.clone(),
        },
    }
}
fn parameter(p: &m::ParameterDecl) -> w::ConfigurationParameter {
    w::ConfigurationParameter {
        id: p.id.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        type_: parameter_type(&p.kind),
        required: p.required,
        secret: p.secret,
        must_exist: p.must_exist,
        default: p.default.as_ref().map(value),
        min: p.min.map(|v| v.to_string()),
        max: p.max.map(|v| v.to_string()),
        choices: p
            .choices
            .iter()
            .map(|c| w::ConfigurationParameterChoice {
                value: c.value.clone(),
                name: c.name.clone(),
            })
            .collect(),
    }
}
fn read_parameter(p: &w::ConfigurationParameter) -> Result<m::ParameterDecl, String> {
    Ok(m::ParameterDecl {
        id: p.id.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        kind: read_parameter_type(&p.type_),
        required: p.required,
        secret: p.secret,
        must_exist: p.must_exist,
        default: p.default.as_ref().map(read_value).transpose()?,
        min: p.min.as_deref().map(integer).transpose()?,
        max: p.max.as_deref().map(integer).transpose()?,
        choices: p
            .choices
            .iter()
            .map(|c| m::ChoiceDecl {
                value: c.value.clone(),
                name: c.name.clone(),
            })
            .collect(),
    })
}
