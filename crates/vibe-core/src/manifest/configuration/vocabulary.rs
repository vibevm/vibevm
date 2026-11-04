//! Authored spelling adapters; no serialized protocol is added to the kernel.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-DEFINITIONS");
use super::*;
impl From<&ConfigurationGuard> for crate::configuration::ChoiceGuard {
    fn from(g: &ConfigurationGuard) -> Self {
        match g {
            ConfigurationGuard::Group(id) => Self::Group(id.clone()),
            ConfigurationGuard::Member(id) => Self::Member(id.clone()),
        }
    }
}
pub(super) mod cardinality {
    use crate::configuration::Cardinality;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &Cardinality, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match value {
            Cardinality::ZeroOrMore => "zero-or-more",
            Cardinality::OneOrMore => "one-or-more",
            Cardinality::ZeroOrOne => "zero-or-one",
            Cardinality::ExactlyOne => "exactly-one",
        })
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cardinality, D::Error> {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "zero-or-more" => Ok(Cardinality::ZeroOrMore),
            "one-or-more" => Ok(Cardinality::OneOrMore),
            "zero-or-one" => Ok(Cardinality::ZeroOrOne),
            "exactly-one" => Ok(Cardinality::ExactlyOne),
            _ => Err(serde::de::Error::custom(
                "unknown configuration cardinality",
            )),
        }
    }
}

pub(super) mod intent {
    use crate::configuration::MergeIntent;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &MergeIntent, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match value {
            MergeIntent::Additive => "additive",
            MergeIntent::Exact => "exact",
        })
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<MergeIntent, D::Error> {
        match String::deserialize(d)?.as_str() {
            "additive" => Ok(MergeIntent::Additive),
            "exact" => Ok(MergeIntent::Exact),
            _ => Err(serde::de::Error::custom(
                "unknown configuration selection intent",
            )),
        }
    }
}
