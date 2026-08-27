//! The stitch's refusals — the half of the optional-shapes tests that
//! is about the two sides DISAGREEING rather than about what the pass
//! emits when they agree. Sliced out of `tests.rs` along that seam when
//! the parent crossed the 600-line budget: the emissions and the
//! site reads stay there, the cross-check between the schema side and
//! the Rust side lives here — the count tally, the class mismatch, the
//! undeclared payload, the optional vocabulary, the nullable member,
//! the literal that fits no payload, the conflicting decisions, and the
//! walk over the real vocabulary home.
//!
//! Helpers and sample emissions come from the parent module; this file
//! declares none of its own, so a sample used on both sides is defined
//! once.

use super::*;

/// A4.3's red: the schema describes two fields, the Rust carries one —
/// the tally refuses, naming both counts.
#[test]
fn a_count_mismatch_refuses_with_both_numbers() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "superseded_by": {
                "type": "string",
                "metadata": { "x-default": null }
            },
            "license": {
                "type": "string",
                "metadata": { "x-default": null }
            }
        }
    });
    let err = apply(OPTIONAL_STRING, "by_name/mod.rs", doc)
        .expect_err("one field in Rust against two sites in the schema");
    let msg = err.to_string();
    assert!(
        msg.contains(
            "describes 2 optional scalar / structure fields but the generated \
             file carries 1"
        ),
        "names both counts: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
    Ok(())
}

/// A4.4's red: the schema keys `tombstone` as a structure, the field
/// carries a scalar payload — the class disagrees between the sides.
#[test]
fn a_class_mismatch_between_the_sides_refuses() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct ByName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tombstone: Option<Box<String>>,
}
"#;
    let doc = json!({
        "optionalProperties": {
            "tombstone": { "ref": "tombstone" }
        },
        "definitions": {
            "tombstone": {
                "properties": { "reason": { "type": "string" } }
            }
        }
    });
    let err = apply(src, "by_name/mod.rs", doc).expect_err("the class must agree from both sides");
    let msg = err.to_string();
    assert!(
        msg.contains("the class disagrees between the sides"),
        "names the disagreement: {msg}"
    );
    assert!(
        msg.contains("keys `tombstone` as a structure"),
        "names the schema's class: {msg}"
    );
    Ok(())
}

/// A payload the file declares as neither a `pub type` alias nor a
/// `pub struct` — an optional vocabulary — has no shape rule; the schema
/// here keys nothing, so the refusal must name the payload itself.
#[test]
fn an_undeclared_payload_type_refuses() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Report {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<Box<PackageKind>>,
}
"#;
    let err = apply(src, "report/mod.rs", json!({}))
        .expect_err("an optional vocabulary has no shape rule");
    assert!(
        err.to_string().contains("`PackageKind`"),
        "names the payload type: {err}"
    );
    Ok(())
}

/// A vocabulary is now a supported optional shape, but its absent-key policy
/// remains explicit: without `x-default: null` the schema side refuses before
/// generated Rust is touched.
#[test]
fn an_optional_vocabulary_without_a_default_policy_refuses() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "kind": { "ref": "package_kind" }
        },
        "definitions": {
            "package_kind": { "enum": ["feat", "flow"] }
        }
    });
    let err = shapes(doc).expect_err("an optional vocabulary must declare its absent policy");
    let msg = err.to_string();
    assert!(
        msg.contains("optional vocabulary field"),
        "names the form: {msg}"
    );
    assert!(
        msg.contains("optionalProperties.kind"),
        "names the site: {msg}"
    );
    assert!(msg.contains("x-default"), "names the missing policy: {msg}");
    Ok(())
}

/// An optional member that is also `nullable` refuses — no rule may be
/// absent AND null at once, and no site of today's tree carries it.
#[test]
fn an_optional_member_with_nullable_refuses() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "label": { "type": "string", "nullable": true }
        }
    });
    let err = shapes(doc).expect_err("absent-and-null is not a rule");
    assert!(
        err.to_string().contains("may be absent AND null at once"),
        "names the combination: {err}"
    );
    Ok(())
}

/// A schema ruling a boolean site false-defaulted against a non-bool
/// payload refuses: collapsing `Option<Box<String>>` to `bool` would
/// hide a type disagreement behind a green run.
#[test]
fn a_false_default_against_a_non_bool_payload_refuses() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overridden: Option<Box<String>>,
}
"#;
    let doc = json!({
        "optionalProperties": {
            "overridden": {
                "type": "boolean",
                "metadata": { "x-default": false }
            }
        }
    });
    let err = apply(src, "list_report/mod.rs", doc)
        .expect_err("a boolean ruling against a string payload");
    assert!(
        err.to_string().contains("collapsing a non-bool to `bool`"),
        "names the disagreement: {err}"
    );
    Ok(())
}

/// A1's conflict: two sites sharing the (wire, class) key with
/// DIFFERENT decisions refuse, the refusal naming the key and both.
#[test]
fn the_same_key_with_different_decisions_refuses() -> Result<()> {
    let doc = json!({
        "definitions": {
            "left": {
                "optionalProperties": {
                    "flag": {
                        "type": "boolean",
                        "metadata": { "x-default": false }
                    }
                }
            },
            "right": {
                "optionalProperties": {
                    "flag": {
                        "type": "string",
                        "metadata": { "x-default": null }
                    }
                }
            }
        }
    });
    let err = shapes(doc).expect_err("one key cannot carry two shapes");
    let msg = err.to_string();
    assert!(msg.contains("`flag`"), "names the shared key: {msg}");
    assert!(msg.contains("an Option value"), "names one decision: {msg}");
    assert!(
        msg.contains("a false-defaulted bool"),
        "names the other decision: {msg}"
    );
    Ok(())
}

/// The real data, walked: the vocabulary home — wrapped as
/// `definitions`, exactly where `Vocabularies::resolve` places every
/// fragment — carries 23 sites this pass rules on (14 optional scalars,
/// including `compile_trace_report.run_path`,
/// and 9 optional structures) and two legal diamonds (`describes` and
/// `description` each live in both `subskill_entry` and `version_entry`
/// with the same decision), so the map holds one key per pair while the
/// tally counts all four.
#[test]
fn the_real_vocabulary_home_walks_to_its_sites() -> Result<()> {
    // One `..` deeper than the parent module's own paths: `include_str!`
    // resolves against THIS file, and the slice moved it a level down.
    let home: Value =
        serde_json::from_str(include_str!("../../../../../formats/vocabularies.json"))
            .expect("the vocabulary home parses");
    let doc_shapes = shapes(json!({ "definitions": home }))?;
    assert_eq!(
        doc_shapes.sites, 23,
        "14 optional scalars + 9 optional structures"
    );
    Ok(())
}
