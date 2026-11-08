//! Compatibility reexport of the canonical counter serializer now owned by core.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout");
pub(crate) use vibe_core::wire_decimal::{deserialize, serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct WireValue {
        #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
        value: u64,
    }

    #[test]
    fn writes_canonical_decimal_strings_for_the_full_u64_domain() {
        assert_eq!(
            serde_json::to_string(&WireValue { value: 0 }).unwrap(),
            r#"{"value":"0"}"#
        );
        assert_eq!(
            serde_json::to_string(&WireValue { value: u64::MAX }).unwrap(),
            format!(r#"{{"value":"{}"}}"#, u64::MAX)
        );
    }

    #[test]
    fn reads_only_canonical_decimal_strings() {
        let max = format!(r#"{{"value":"{}"}}"#, u64::MAX);
        assert_eq!(
            serde_json::from_str::<WireValue>(&max).unwrap(),
            WireValue { value: u64::MAX }
        );
        for refused in [
            r#"{"value":1}"#,
            r#"{"value":""}"#,
            r#"{"value":"007"}"#,
            r#"{"value":"+1"}"#,
            r#"{"value":"-1"}"#,
            r#"{"value":"18446744073709551616"}"#,
        ] {
            assert!(
                serde_json::from_str::<WireValue>(refused).is_err(),
                "accepted non-canonical value: {refused}"
            );
        }
    }
}
