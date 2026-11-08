//! Canonical decimal u64 serialization shared by report and journal owners.
//! The grammar is the existing index counter grammar, promoted without coercion.
//!
//! ```
//! use vibe_core::wire_decimal::DecimalU64;
//! let count = DecimalU64::new(u64::MAX);
//! assert_eq!(serde_json::to_string(&count).unwrap(), r#""18446744073709551615""#);
//! assert_eq!(serde_json::from_str::<DecimalU64>(r#""0""#).unwrap().get(), 0);
//! assert!(serde_json::from_str::<DecimalU64>(r#""00""#).is_err());
//! ```
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#M-WIDE-INTEGERS-AS-STRINGS");

use serde::{Deserialize, Deserializer, Serializer};

/// Serialize an unsigned count as its canonical decimal string.
/// Used through `#[serde(with = "vibe_core::wire_decimal")]`.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#M-WIDE-INTEGERS-AS-STRINGS"
)]
pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

/// Admit only canonical unsigned decimal strings within u64.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#M-WIDE-INTEGERS-AS-STRINGS"
)]
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let raw = String::deserialize(deserializer)?;
    parse_canonical(&raw).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "`{raw}` is not a canonical decimal u64 (ASCII digits only, no sign, \
             no leading zeros except the bare `0`, value within u64)"
        ))
    })
}

/// The canonical form accepted on the wire — exactly what
/// `u64::from_str` accepts for digits, tightened to reject leading
/// zeros and the empty string so one number has one spelling.
fn parse_canonical(raw: &str) -> Option<u64> {
    // The explicit digit check closes `from_str`'s one leniency: it
    // accepts a leading `+`, which would give one number a second
    // spelling.
    if raw.is_empty()
        || (raw.len() > 1 && raw.starts_with('0'))
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse().ok()
}

/// A typed wire count: construction from u64 is exact; deserialization checks
/// the shared canonical grammar and range.
///
/// ```
/// use vibe_core::wire_decimal::DecimalU64;
/// let count = serde_json::from_str::<DecimalU64>(r#""9007199254740993""#).unwrap();
/// assert_eq!(count.get(), 9_007_199_254_740_993);
/// assert_eq!(serde_json::to_string(&count).unwrap(), r#""9007199254740993""#);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecimalU64(u64);
impl DecimalU64 {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}
impl serde::Serialize for DecimalU64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize(&self.0, serializer)
    }
}
impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize(deserializer).map(Self)
    }
}

/// Optional domain counts retain u64 arithmetic and the same wire admission.
pub mod optional {
    use super::*;
    use serde::Serialize;
    pub fn serialize<S: Serializer>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error> {
        value.map(DecimalU64::new).serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u64>, D::Error> {
        Option::<DecimalU64>::deserialize(deserializer).map(|value| value.map(DecimalU64::get))
    }
}
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
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#M-WIDE-INTEGERS-AS-STRINGS"
    )]
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
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#M-WIDE-INTEGERS-AS-STRINGS"
    )]
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
