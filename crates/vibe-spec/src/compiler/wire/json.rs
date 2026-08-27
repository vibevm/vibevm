//! The strict byte reader: JSON whose objects repeat a key never reaches the
//! generated types, because direct deserialization would silently last-wins
//! every `BTreeMap`/`values` field.
//!
//! The walker is a raw `DeserializeSeed`/`Visitor` over the token stream: it
//! retains only each current object's key set, never builds a JSON value and
//! never a second DTO. serde_json's own recursion limit still applies — the
//! seed recursion runs inside the same deserializer.

use std::collections::HashSet;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use super::{IrWireError, bounded};

/// Parse wire bytes into the generated strict type, refusing a repeated key
/// in ANY object on the way. Reader strictness, not a semantic gate.
pub(super) fn from_strict_slice(bytes: &[u8]) -> Result<super::wire::Ir, IrWireError> {
    // Pass 1 walks the token stream for repeated keys only (no value is
    // built); pass 2 is the generated strict parse of the same bytes. Two
    // passes over one buffer, never a materialized JSON value.
    let mut walker = serde_json::Deserializer::from_slice(bytes);
    NoDuplicateKeys
        .deserialize(&mut walker)
        .map_err(|source| IrWireError::StrictReader {
            detail: bounded::display(source),
        })?;
    walker.end().map_err(|source| IrWireError::StrictReader {
        detail: bounded::display(source),
    })?;
    // serde builds its own `unknown field …` message eagerly; the bounded
    // sink is where that text stops, and the unbounded source is dropped
    // rather than carried into every rendering of our refusal.
    serde_json::from_slice::<super::wire::Ir>(bytes).map_err(|source| IrWireError::Reader {
        detail: bounded::display(source),
    })
}

struct NoDuplicateKeys;

impl<'de> DeserializeSeed<'de> for NoDuplicateKeys {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(TokenWalker)
    }
}

struct TokenWalker;

impl<'de> Visitor<'de> for TokenWalker {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
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

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element_seed(NoDuplicateKeys)?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        // Owned keys: serde_json unescapes `\u`-style spellings into the same
        // String, so a literal key and its escaped twin compare as the SAME
        // object key — and legal escaped keys parse without a borrowed
        // scratch-buffer rejection.
        //
        // A UNIQUE key is MOVED into the set, never cloned: cloning would
        // double an attacker-sized key's footprint on the ordinary path,
        // which is exactly the amplification this reader exists to refuse.
        // The duplicate branch borrows the key it is about to reject.
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if seen.contains(&key) {
                return Err(de::Error::custom(duplicate_key_detail(&key)));
            }
            seen.insert(key);
            map.next_value_seed(NoDuplicateKeys)?;
        }
        Ok(())
    }
}

/// A bounded diagnostic on the one shared discipline: a capped prefix and the
/// true byte length, never an echo of arbitrarily large input.
fn duplicate_key_detail(key: &str) -> String {
    format!("duplicate object key ({})", super::bounded::preview(key))
}
