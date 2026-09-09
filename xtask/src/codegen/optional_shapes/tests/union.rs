//! Required-null tagged-union coverage for the optional-shapes stitch.

use super::*;

const DERIVE_LINE: &str = concat!("#[", "derive(Serialize, Deserialize)]");
const REQUIRED_UNION_TEMPLATE: &str = r#"$DERIVE
pub struct Holder {
    pub payload: Option<Box<Payload>>,
}

$DERIVE
#[serde(tag = "kind")]
pub enum Payload {
    #[serde(rename = "made")]
    Made(Box<PayloadMade>),
}

$DERIVE
pub struct PayloadMade {
    pub value: String,
}
"#;

fn required_union() -> String {
    REQUIRED_UNION_TEMPLATE.replace("$DERIVE", DERIVE_LINE)
}

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
    let output = apply(&required_union(), "journal/mod.rs", union_doc(true))?;
    assert!(output.contains(
        "#[serde(deserialize_with = \"crate::behaviour::required_nullable::deserialize\")]\n    pub payload: Option<Payload>,"
    ));
    assert!(output.contains("Made(Box<PayloadMade>),"));
    Ok(())
}

#[test]
fn optional_and_untagged_unions_remain_loudly_unsupported() {
    let optional = union_doc(false);
    let error = shapes(optional).expect_err("optional nullable union must refuse");
    assert!(
        error.to_string().contains("may be absent AND null"),
        "{error}"
    );

    let untagged = required_union().replace("#[serde(tag = \"kind\")]", "#[serde(untagged)]");
    let error = apply(&untagged, "journal/mod.rs", union_doc(true))
        .expect_err("untagged union must refuse");
    assert!(error.to_string().contains("untagged enum"), "{error}");
}

#[test]
fn vocabulary_and_union_classes_cannot_be_confused() {
    let vocabulary = required_union().replace("#[serde(tag = \"kind\")]\n", "");
    let error = apply(&vocabulary, "journal/mod.rs", union_doc(true))
        .expect_err("plain enum is a vocabulary, not a tagged union");
    let text = error.to_string();
    assert!(text.contains("class disagrees"), "{text}");
    assert!(text.contains("tagged union"), "{text}");
    assert!(text.contains("vocabulary"), "{text}");
}
