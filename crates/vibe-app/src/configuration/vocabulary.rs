//! Typed adapters; no serializer roundtrip participates in normalization.
use vibe_core::{configuration as k, manifest as m};
use vibe_wire::generated::shared as w;

macro_rules! vocabulary {
    ($forward:ident, $back:ident, $source:path, $wire:path, [$($variant:ident),+]) => {
        pub(super) fn $forward(value: &$source) -> $wire {
            match value { $(<$source>::$variant => <$wire>::$variant,)+ }
        }
        pub(super) fn $back(value: &$wire) -> $source {
            match value { $(<$wire>::$variant => <$source>::$variant,)+ }
        }
    };
}
vocabulary!(
    policy,
    read_policy,
    k::ChoicePolicy,
    w::ConfigurationPolicy,
    [RequireSelection, UseDeclaredDefaults]
);
vocabulary!(
    intent,
    read_intent,
    k::MergeIntent,
    w::ConfigurationIntent,
    [Exact, Additive]
);
vocabulary!(
    cardinality,
    read_cardinality,
    k::Cardinality,
    w::ConfigurationCardinality,
    [ExactlyOne, ZeroOrOne, OneOrMore, ZeroOrMore]
);
vocabulary!(
    parameter_type,
    read_parameter_type,
    m::ParameterType,
    w::ConfigurationParameterType,
    [Bool, Integer, String, Choice, Path]
);
vocabulary!(
    visibility,
    read_visibility,
    m::ConfigurationFeatureVisibility,
    w::ConfigurationVisibility,
    [Public, Internal]
);
vocabulary!(
    strength,
    read_strength,
    m::ForwardStrength,
    w::ConfigurationEffectForwardStrength,
    [Strong, Weak]
);

pub(super) fn integer(raw: &str) -> Result<i64, String> {
    raw.parse::<i64>()
        .ok()
        .filter(|v| v.to_string() == raw)
        .ok_or_else(|| "expected a canonical signed 64-bit decimal integer".into())
}
pub(super) fn value(value: &m::ParameterValue) -> w::ConfigurationValue {
    match value {
        m::ParameterValue::Bool(value) => {
            w::ConfigurationValue::Bool(Box::new(w::ConfigurationValueBool { value: *value }))
        }
        m::ParameterValue::Integer(value) => {
            w::ConfigurationValue::Integer(Box::new(w::ConfigurationValueInteger {
                value: value.to_string(),
            }))
        }
        m::ParameterValue::Text(value) => {
            w::ConfigurationValue::Text(Box::new(w::ConfigurationValueText {
                value: value.clone(),
            }))
        }
    }
}
pub(super) fn read_value(value: &w::ConfigurationValue) -> Result<m::ParameterValue, String> {
    Ok(match value {
        w::ConfigurationValue::Bool(v) => m::ParameterValue::Bool(v.value),
        w::ConfigurationValue::Integer(v) => m::ParameterValue::Integer(integer(&v.value)?),
        w::ConfigurationValue::Text(v) => m::ParameterValue::Text(v.value.clone()),
    })
}
pub(super) fn selection(v: &m::ConfigurationSelection) -> w::ConfigurationSelection {
    w::ConfigurationSelection {
        intent: intent(&v.intent),
        members: v.members.clone(),
    }
}
pub(super) fn read_selection(v: &w::ConfigurationSelection) -> m::ConfigurationSelection {
    m::ConfigurationSelection {
        intent: read_intent(&v.intent),
        members: v.members.clone(),
    }
}
pub(super) fn source_input(v: &m::ConfigurationIntentDecl) -> w::ConfigurationSourceInput {
    w::ConfigurationSourceInput {
        policy: policy(&v.policy),
        profile: v.profile.clone(),
        features: v.features.clone(),
        default_features: v.default_features,
        selections: v
            .selections
            .iter()
            .map(|(k, v)| (k.clone(), selection(v)))
            .collect(),
        parameters: v
            .parameters
            .iter()
            .map(|(k, v)| (k.clone(), value(v)))
            .collect(),
    }
}
pub(super) fn read_input(
    v: &w::ConfigurationSourceInput,
) -> Result<m::ConfigurationIntentDecl, String> {
    Ok(m::ConfigurationIntentDecl {
        policy: read_policy(&v.policy),
        profile: v.profile.clone(),
        features: v.features.clone(),
        default_features: v.default_features,
        selections: v
            .selections
            .iter()
            .map(|(k, v)| (k.clone(), read_selection(v)))
            .collect(),
        parameters: v
            .parameters
            .iter()
            .map(|(k, v)| read_value(v).map(|v| (k.clone(), v)))
            .collect::<Result<_, _>>()?,
    })
}
pub(super) fn guard(v: &m::ConfigurationGuard) -> w::ConfigurationGuard {
    match v {
        m::ConfigurationGuard::Group(id) => {
            w::ConfigurationGuard::Group(Box::new(w::ConfigurationGuardGroup { id: id.clone() }))
        }
        m::ConfigurationGuard::Member(id) => {
            w::ConfigurationGuard::Member(Box::new(w::ConfigurationGuardMember { id: id.clone() }))
        }
    }
}
pub(super) fn read_guard(v: &w::ConfigurationGuard) -> m::ConfigurationGuard {
    match v {
        w::ConfigurationGuard::Group(v) => m::ConfigurationGuard::Group(v.id.clone()),
        w::ConfigurationGuard::Member(v) => m::ConfigurationGuard::Member(v.id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_integer_boundary_is_exact_and_does_not_echo_values() {
        for v in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert_eq!(integer(&v.to_string()), Ok(v));
        }
        for s in [
            "9223372036854775808",
            "-9223372036854775809",
            "-0",
            "+1",
            "01",
            " 1",
            "1 ",
            "CANARY",
        ] {
            let e = integer(s).unwrap_err();
            assert!(!e.contains(s));
        }
    }
    #[test]
    fn omitted_choice_and_explicit_empty_survive_wire() {
        let missing: w::ConfigurationSelection =
            serde_json::from_str(r#"{"intent":"exact","members":null}"#).unwrap();
        let empty: w::ConfigurationSelection =
            serde_json::from_str(r#"{"intent":"exact","members":[]}"#).unwrap();
        assert!(
            serde_json::from_str::<w::ConfigurationSelection>(r#"{"intent":"exact"}"#).is_err()
        );
        assert_eq!(missing.members, None);
        assert_eq!(empty.members, Some(vec![]));
        assert_eq!(
            serde_json::to_string(&missing).unwrap(),
            r#"{"intent":"exact","members":null}"#
        );
        assert_eq!(
            serde_json::to_string(&empty).unwrap(),
            r#"{"intent":"exact","members":[]}"#
        );
    }
}
