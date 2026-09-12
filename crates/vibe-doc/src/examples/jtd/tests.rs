//! The JTD validator: the eight forms, and the one behaviour that is not
//! about JSON at all — an unchecked document says so.

use serde_json::json;

use super::{split_documents, validate};

#[test]
fn an_unknown_member_is_a_violation_because_a_schema_is_a_contract() {
    let schema = json!({ "properties": { "ok": { "type": "boolean" } } });
    let found = validate(&schema, &json!({ "ok": true, "invoked_by": "claude-code" }));
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("unexpected member `invoked_by`"));
}

#[test]
fn a_missing_required_member_is_named() {
    let schema = json!({ "properties": { "plans": { "elements": { "type": "string" } } } });
    let found = validate(&schema, &json!({ "packages": [] }));
    let text: Vec<String> = found.iter().map(|v| v.message.clone()).collect();
    assert!(
        text.iter()
            .any(|m| m.contains("missing required member `plans`"))
    );
    assert!(
        text.iter()
            .any(|m| m.contains("unexpected member `packages`"))
    );
}

#[test]
fn the_empty_form_accepts_anything() {
    assert!(validate(&json!({}), &json!({ "whatever": [1, 2] })).is_empty());
}

#[test]
fn nullable_admits_null_and_nothing_else_changes() {
    let schema = json!({ "type": "string", "nullable": true });
    assert!(validate(&schema, &json!(null)).is_empty());
    assert!(validate(&schema, &json!("text")).is_empty());
    assert_eq!(validate(&schema, &json!(7)).len(), 1);
}

#[test]
fn enum_values_and_integer_ranges_are_checked() {
    let schema = json!({ "enum": ["flow", "feat"] });
    assert!(validate(&schema, &json!("flow")).is_empty());
    assert_eq!(validate(&schema, &json!("doc")).len(), 1);
    assert_eq!(validate(&json!({ "type": "uint8" }), &json!(300)).len(), 1);
    assert!(validate(&json!({ "type": "uint8" }), &json!(255)).is_empty());
}

#[test]
fn values_elements_and_ref_all_resolve() {
    let schema = json!({
        "definitions": { "name": { "type": "string" } },
        "properties": {
            "list": { "elements": { "ref": "name" } },
            "map": { "values": { "type": "uint32" } }
        }
    });
    let ok = json!({ "list": ["a"], "map": { "k": 1 } });
    assert!(validate(&schema, &ok).is_empty());
    let bad = json!({ "list": [1], "map": { "k": "x" } });
    assert_eq!(validate(&schema, &bad).len(), 2);
}

#[test]
fn a_discriminator_picks_its_mapped_form_and_hides_its_own_tag() {
    let schema = json!({
        "discriminator": "kind",
        "mapping": {
            "file": { "properties": { "path": { "type": "string" } } },
            "git": { "properties": { "url": { "type": "string" } } }
        }
    });
    assert!(validate(&schema, &json!({ "kind": "file", "path": "p" })).is_empty());
    let wrong = validate(&schema, &json!({ "kind": "file", "url": "u" }));
    assert_eq!(wrong.len(), 2);
    let missing = validate(&schema, &json!({ "kind": "socket" }));
    assert!(missing[0].message.contains("is in no mapping"));
}

#[test]
fn additional_properties_opens_the_object() {
    let schema = json!({
        "properties": { "ok": { "type": "boolean" } },
        "additionalProperties": true
    });
    assert!(validate(&schema, &json!({ "ok": true, "extra": 1 })).is_empty());
}

#[test]
fn a_stream_of_three_documents_is_three_documents() {
    // `vibe install --json` prints a plan, a closure diff and a report;
    // one `from_str` over the whole stream fails at the second.
    let stream = "{\"command\":\"install:plan\"}\n{\"command\":\"install:closure-diff\"}\n{\"command\":\"install\"}\n";
    let docs = split_documents(stream);
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[2]["command"], "install");
}
