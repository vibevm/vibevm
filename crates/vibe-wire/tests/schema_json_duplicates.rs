//! Raw JSON duplicate-key guard for authored native mechanism schemas.

use std::collections::BTreeSet;
use std::fmt;

use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};

#[derive(Clone, Copy)]
struct UniqueValue;

impl<'de> DeserializeSeed<'de> for UniqueValue {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for UniqueValue {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(A::Error::custom(format!("duplicate object key `{key}`")));
            }
            map.next_value_seed(self)?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element_seed(self)?.is_some() {}
        Ok(())
    }

    fn visit_bool<E>(self, _value: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }
}

fn reject_duplicate_keys(raw: &str) -> Result<(), serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(raw);
    UniqueValue.deserialize(&mut deserializer)?;
    deserializer.end()
}

#[test]
fn native_mechanism_schemas_have_no_duplicate_object_keys() {
    for (name, raw) in [
        (
            "mechanism_manifest",
            include_str!("../../../schemas/native/e1/mechanism_manifest.jtd.json"),
        ),
        (
            "deploy_request",
            include_str!("../../../schemas/native/e1/deploy_request.jtd.json"),
        ),
        (
            "deploy_reply",
            include_str!("../../../schemas/native/e1/deploy_reply.jtd.json"),
        ),
    ] {
        reject_duplicate_keys(raw).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn guard_observes_nested_duplicates_before_value_projection() {
    let raw = r#"{"outer":{"epoch":1,"epoch":2}}"#;
    assert!(
        reject_duplicate_keys(raw)
            .unwrap_err()
            .to_string()
            .contains("epoch")
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(raw).unwrap()["outer"]["epoch"],
        2
    );
}
