//! Explicit null aliases use the pinned optional shape, preserving old policies.
#![cfg(test)]
use super::*;

fn alias(mut value: Value) -> Value {
    value["nullable"] = true.into();
    value["metadata"] = json!({"x-default":null,"x-null-alias":"absent"});
    value
}
#[test]
fn marker_requires_an_optional_nullable_none_default_member() {
    let legal = alias(json!({"type":"string"}));
    assert!(shapes(json!({"optionalProperties":{"label":legal.clone()}})).is_ok());
    let mut variants = Vec::new();
    for bad in [json!(false), json!("present"), json!(null)] {
        let mut node = legal.clone();
        node["metadata"]["x-null-alias"] = bad;
        variants.push(json!({"optionalProperties":{"label":node}}));
    }
    let mut bad = legal.clone();
    bad.as_object_mut().unwrap().remove("nullable");
    variants.push(json!({"optionalProperties":{"label":bad}}));
    let mut bad = legal.clone();
    bad["metadata"]["x-default"] = false.into();
    variants.push(json!({"optionalProperties":{"label":bad}}));
    let mut bad = legal.clone();
    bad["metadata"]["x-empty"] = "omit".into();
    variants.push(json!({"optionalProperties":{"label":bad}}));
    variants.push(json!({"properties":{"label":legal.clone()}}));
    variants.push(legal.clone());
    variants.push(json!({"elements":legal.clone()}));
    variants.push(json!({"optionalProperties":{"values":{"elements":legal.clone(),"metadata":{"x-empty":"preserve"}}}}));
    for bad in variants {
        let error =
            shapes(bad).expect_err("misplaced alias cannot bypass collection/optional scanning");
        assert!(error.to_string().contains("x-null-alias"), "{error}");
    }
    // Existing unopted nullable direct collections remain the empty-policy owner's concern.
    assert!(shapes(json!({"optionalProperties":{"items":{"elements":{"type":"string"},"nullable":true,"metadata":{"x-empty":"preserve"}}}})).is_ok());
}

#[test]
fn required_and_both_optional_scalar_forms_share_only_equal_shape_decisions() -> Result<()> {
    let schema = json!({"definitions":{
        "required":{"properties":{"label":{"type":"string","nullable":true}}},
        "legacy":{"optionalProperties":{"label":{"type":"string","metadata":{"x-default":null}}}},
        "portable":{"optionalProperties":{"label":alias(json!({"type":"string"}))}}
    }});
    let src = r#"pub struct Required {
    pub label: Option<Box<String>>,
}
pub struct Legacy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Box<String>>,
}
pub struct Portable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Box<String>>,
}
"#;
    let output = apply(src, "fixture.rs", schema)?;
    assert_eq!(output.matches("required_nullable::deserialize").count(), 1);
    assert_eq!(
        output.matches("serde(default, skip_serializing_if").count(),
        2
    );
    let conflict = json!({"definitions":{
        "legacy":{"optionalProperties":{"flag":{"type":"boolean","metadata":{"x-default":false}}}},
        "portable":{"optionalProperties":{"flag":alias(json!({"type":"boolean"}))}}
    }});
    assert!(
        shapes(conflict)
            .unwrap_err()
            .to_string()
            .contains("different shape policies")
    );
    Ok(())
}

#[path = "null_alias/compiled.rs"]
mod compiled;

#[test]
fn schema_dictionary_names_cannot_hide_an_illegal_alias() {
    let misplaced = alias(json!({"type":"string"}));
    for doc in [
        json!({"definitions":{"metadata":misplaced}}),
        json!({"discriminator":"kind","mapping":{"metadata":{"properties":{},"nullable":true,"metadata":{"x-null-alias":"absent","x-default":null}}}}),
    ] {
        let error = shapes(doc).expect_err("definition/variant names are not schema metadata keys");
        assert!(error.to_string().contains("x-null-alias"), "{error}");
    }
}
