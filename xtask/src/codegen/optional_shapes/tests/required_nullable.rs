//! The required-nullable row — the one site where the schema says a
//! member is required AND nullable, so `null` and "missing" are two
//! different answers and the pass has to keep them apart.
//!
//! Sliced out of `tests.rs` along that seam when the parent crossed the
//! 600-line budget. Everything here turns on the same distinction: the
//! lifted `Box`, the shared deserializer that makes `None` serialise as
//! `null`, the collection alias where `null` is not the empty map, and
//! the refusal when a skip attribute would erase the difference.
//!
//! Helpers and sample emissions come from the parent module; this file
//! declares none of its own.

use super::*;

/// The tree's own third row: a REQUIRED `nullable: true` member arrives
/// as `Option<Box<…>>` with no skip attribute. The pass lifts the `Box`
/// and adds the shared deserializer: `None` still serialises as `null`,
/// while an absent key becomes a parse refusal.
#[test]
fn a_required_nullable_member_lifts_the_box_and_becomes_strict() -> Result<()> {
    let doc = json!({
        "properties": {
            "boot_snippet": {
                "type": "string",
                "nullable": true
            }
        }
    });
    assert_eq!(
        apply(REQUIRED_NULLABLE, "list_report/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    /// Filename of the package's boot snippet under `vibevm/vibespecs/boot/`, or null
    /// if absent.
    #[serde(deserialize_with = "crate::behaviour::required_nullable::deserialize")]
    pub boot_snippet: Option<String>,
}
"#
    );
    Ok(())
}

#[test]
fn flat_some_null_none_and_missing_refusal_are_serde_exact() {
    mod required_nullable {
        use serde::Deserialize;

        pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            D: serde::Deserializer<'de>,
            T: Deserialize<'de>,
        {
            Option::<T>::deserialize(deserializer)
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum Payload {
        Made { value: String },
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Holder {
        #[serde(deserialize_with = "required_nullable::deserialize")]
        payload: Option<Payload>,
    }

    let some = Holder {
        payload: Some(Payload::Made { value: "x".into() }),
    };
    assert_eq!(
        serde_json::to_string(&some).unwrap(),
        r#"{"payload":{"kind":"made","value":"x"}}"#
    );
    let none = Holder { payload: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"payload":null}"#);
    assert!(serde_json::from_str::<Holder>(r#"{"payload":null}"#).is_ok());
    assert!(serde_json::from_str::<Holder>("{}").is_err());
}

#[test]
fn a_required_nullable_collection_alias_keeps_null_distinct_from_empty() -> Result<()> {
    let doc = json!({
        "properties": {
            "authored_config": {
                "ref": "json_map",
                "nullable": true
            }
        },
        "definitions": {
            "json_map": { "values": {} }
        }
    });
    assert_eq!(
        apply(REQUIRED_NULLABLE_MAP_ALIAS, "extensions_report/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct ExtensionEntry {
    #[serde(deserialize_with = "crate::behaviour::required_nullable::deserialize")]
    pub authored_config: Option<JsonMap>,
}

pub type JsonMap = BTreeMap<String, Option<Value>>;
"#
    );
    Ok(())
}

/// A skip attribute over a required-nullable field refuses — the pinned
/// emission never writes one there, and a skip would turn the written
/// `null` into an absent key.
#[test]
fn a_required_nullable_field_with_a_skip_attribute_refuses() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_snippet: Option<Box<String>>,
}
"#;
    let doc = json!({
        "properties": {
            "boot_snippet": { "type": "string", "nullable": true }
        }
    });
    let err = apply(src, "list_report/mod.rs", doc)
        .expect_err("a skip over a required-nullable field is a moved pin");
    assert!(
        err.to_string()
            .contains("the pinned emission does not write"),
        "names the moved pin: {err}"
    );
    Ok(())
}
