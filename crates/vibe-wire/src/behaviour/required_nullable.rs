//! Strict reading for a required-nullable generated field.
//!
//! The wire law has three states, not two: a present value is
//! `Some(value)`, a present `null` is `None`, and an absent key is a
//! parse error because the schema placed the member in `properties`.
//! Serde normally folds the last two states together for `Option<T>`.
//! Generated required-nullable fields name [`deserialize`] without a
//! `default` attribute, so serde keeps the missing-field refusal while
//! this helper handles the two present states.

use serde::{Deserialize, Deserializer};

/// Deserialize a present required-nullable value without providing an
/// absent-field default to the containing struct.
pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Subject {
        #[serde(deserialize_with = "deserialize")]
        value: Option<String>,
    }

    #[test]
    fn a_present_value_is_some() {
        let parsed: Subject = serde_json::from_str(r#"{"value":"kept"}"#).unwrap();
        assert_eq!(
            parsed,
            Subject {
                value: Some("kept".to_string())
            }
        );
    }

    #[test]
    fn a_present_null_is_none() {
        let parsed: Subject = serde_json::from_str(r#"{"value":null}"#).unwrap();
        assert_eq!(parsed, Subject { value: None });
    }

    #[test]
    fn an_absent_key_is_a_parse_error() {
        let error = serde_json::from_str::<Subject>("{}")
            .expect_err("required-nullable absence must not become None");
        assert!(
            error.to_string().contains("missing field `value`"),
            "absence names the missing field: {error}"
        );
    }
}
