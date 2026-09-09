//! Required-null tagged-union coverage for the optional-shapes stitch.

use super::*;
use serde::{Deserialize, Serialize};

const REQUIRED_UNION: &str = r#"#[derive(Serialize, Deserialize)]
pub struct Holder {
    pub payload: Option<Box<Payload>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Payload {
    #[serde(rename = "made")]
    Made(Box<PayloadMade>),
}

#[derive(Serialize, Deserialize)]
pub struct PayloadMade {
    pub value: String,
}
"#;

fn union_doc(required: bool) -> Value {
    let member = json!({"ref": "payload", "nullable": true});
    let mut doc = json!({
        "definitions": {
            "payload": {
                "discriminator": "kind",
                "mapping": {"made": {"properties": {"value": {"type": "string"}}}}
            }
        }
    });
    let key = if required {
        "properties"
    } else {
        "optionalProperties"
    };
    doc[key] = json!({"payload": member});
    doc
}

#[test]
fn required_nullable_union_lifts_only_the_field_box_and_keeps_arms_boxed() -> Result<()> {
    let output = apply(REQUIRED_UNION, "journal/mod.rs", union_doc(true))?;
    assert!(output.contains(
        "#[serde(deserialize_with = \"crate::behaviour::required_nullable::deserialize\")]\n    pub payload: Option<Payload>,"
    ));
    assert!(output.contains("Made(Box<PayloadMade>),"));
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
fn optional_and_untagged_unions_remain_loudly_unsupported() {
    let optional = union_doc(false);
    let error = shapes(optional).expect_err("optional nullable union must refuse");
    assert!(
        error.to_string().contains("may be absent AND null"),
        "{error}"
    );

    let untagged = REQUIRED_UNION.replace("#[serde(tag = \"kind\")]", "#[serde(untagged)]");
    let error = apply(&untagged, "journal/mod.rs", union_doc(true))
        .expect_err("untagged union must refuse");
    assert!(error.to_string().contains("untagged enum"), "{error}");
}

#[test]
fn vocabulary_and_union_classes_cannot_be_confused() {
    let vocabulary = REQUIRED_UNION.replace("#[serde(tag = \"kind\")]\n", "");
    let error = apply(&vocabulary, "journal/mod.rs", union_doc(true))
        .expect_err("plain enum is a vocabulary, not a tagged union");
    let text = error.to_string();
    assert!(text.contains("class disagrees"), "{text}");
    assert!(text.contains("tagged union"), "{text}");
    assert!(text.contains("vocabulary"), "{text}");
}
