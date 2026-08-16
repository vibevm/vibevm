//! Unit tests for the empty-collection policy pass — one per behaviour
//! the pass owes its callers: the schema-side site read (annotation
//! present, absent, stranger-valued; rule R21's required-plus-omit
//! refusal; the diamond and its conflict), the collapse with its
//! container-chosen predicate, the required-plus-emit replay, the
//! pass-throughs, the inner-`elements`-under-`values` cut, and the
//! site-count tripwire. The samples quote the pinned emission shape of
//! jtd-codegen 0.4.1 as the earlier passes leave it (snake_case fields,
//! `BTreeMap` maps) — the pass exists to be exact about that shape, so
//! the tests must be exact about it too.

use std::path::Path;

use super::emit::apply_with_policies;
use super::{EmptyPolicies, policies_from_doc};
use anyhow::Result;
use serde_json::{Value, json};

/// The optional collection of the real `entry` output, quoted verbatim —
/// skip attribute, `Option<Box<…>>` type, four-space indent.
const OPTIONAL_VEC_FIELD: &str = r#"#[derive(Serialize, Deserialize)]
pub struct ConflictsEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Box<Vec<String>>>,
}
"#;

/// The optional map collection of the real `entry` output — the
/// `values` form, whose inner `elements` is the map's VALUE type.
const OPTIONAL_MAP_FIELD: &str = r#"#[derive(Serialize, Deserialize)]
pub struct FeaturesEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<Box<BTreeMap<String, Vec<String>>>>,
}
"#;

/// A required collection as the emission writes it — no attribute at
/// all, the bare `Vec<…>`.
const REQUIRED_VEC_FIELD: &str = r#"#[derive(Serialize, Deserialize)]
pub struct UninstallReport {
    pub paths: Vec<String>,
}
"#;

/// The policies of a one-off schema document, as the pass would read
/// them off a resolved file.
fn policies(doc: Value) -> Result<EmptyPolicies> {
    policies_from_doc(&doc, Path::new("schema.jtd.json"))
}

/// The full stitch over a one-off document.
fn apply(src: &str, file: &str, doc: Value) -> Result<String> {
    apply_with_policies(src, file, &policies(doc)?)
}

/// An optional `omit` collection collapses to the bare `Vec` with the
/// container's own emptiness predicate — the exact form the hand-written
/// twins carry. This is the test that fails without the transformation:
/// it asserts the exact output, so a partial or creative rewrite cannot
/// sneak through. The green `Ok` IS the site tally converging (one
/// schema site, one Rust field).
#[test]
fn an_omit_optional_vec_collapses_with_the_vec_predicate() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "packages": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    assert_eq!(
        apply(OPTIONAL_VEC_FIELD, "conflicts/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct ConflictsEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
}
"#
    );
    Ok(())
}

/// The map twin: the predicate is chosen by container —
/// `BTreeMap::is_empty`, never `Vec::is_empty` — and the collapsed type
/// keeps the inner `Vec<String>` untouched: only the `Option<Box<…>`
/// wrapper comes off.
#[test]
fn an_omit_optional_map_collapses_with_the_btreemap_predicate() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "exclusive": {
                "values": { "elements": { "type": "string" } },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    assert_eq!(
        apply(OPTIONAL_MAP_FIELD, "entry/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct FeaturesEntry {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub exclusive: BTreeMap<String, Vec<String>>,
}
"#
    );
    Ok(())
}

/// Rule R21's table, required + `emit` row: the emission is already the
/// bare collection with no skip, so the pass changes NOTHING — byte for
/// byte, layout, line endings and all — which is also what keeps
/// `check-codegen` from reporting drift nobody caused.
#[test]
fn a_required_emit_collection_is_replayed_byte_for_byte() -> Result<()> {
    let doc = json!({
        "properties": {
            "paths": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "emit" }
            }
        }
    });
    assert_eq!(
        apply(REQUIRED_VEC_FIELD, "uninstall_report/mod.rs", doc)?,
        REQUIRED_VEC_FIELD
    );
    Ok(())
}

/// A4.2's red-made-green: optional + `emit` collapses with
/// `#[serde(default)]` and NO skip — zero sites in today's tree, so this
/// test is the only proof the branch exists and behaves. The reader
/// still accepts both an absent key and `[]`; the writer always writes.
#[test]
fn an_optional_emit_collection_collapses_without_the_skip() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Report {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carriers: Option<Box<Vec<String>>>,
}
"#;
    let doc = json!({
        "optionalProperties": {
            "carriers": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "emit" }
            }
        }
    });
    let out = apply(src, "report/mod.rs", doc)?;
    assert_eq!(
        out,
        r#"#[derive(Serialize, Deserialize)]
pub struct Report {
    #[serde(default)]
    pub carriers: Vec<String>,
}
"#
    );
    assert!(
        !out.contains("skip_serializing_if"),
        "emit never skips: {out}"
    );
    Ok(())
}

/// A whole file in the shape of the real ones — header, `use` lines, a
/// doc comment, a required collection, an optional one, a non-collection
/// optional, a rename-carrying keyword field — changes in exactly two
/// places: the two optional collections' attribute and type. Everything
/// else is the generator's layout and must not so much as breathe.
#[test]
fn a_whole_file_changes_only_in_the_optional_collections() -> Result<()> {
    let src = r#"// Code generated by jtd-codegen for Rust v0.2.1

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub struct RegistrySyncReport {
    pub ok: bool,

    pub refreshed: Vec<RefreshedEntry>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<Box<Vec<SkippedEntry>>>,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshedEntry {
    #[serde(rename = "ref")]
    pub ref_: String,
}
"#;
    let doc = json!({
        "properties": {
            "ok": { "type": "boolean" },
            "refreshed": {
                "elements": { "ref": "refreshed_entry" },
                "metadata": { "x-empty": "emit" }
            }
        },
        "optionalProperties": {
            "skipped": {
                "elements": { "ref": "skipped_entry" },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    let expected = src.replace(
        "#[serde(skip_serializing_if = \"Option::is_none\")]\n    pub skipped: \
             Option<Box<Vec<SkippedEntry>>>,",
        "#[serde(default, skip_serializing_if = \"Vec::is_empty\")]\n    pub skipped: \
             Vec<SkippedEntry>,",
    );
    assert_eq!(apply(src, "registry_sync_report/mod.rs", doc)?, expected);
    Ok(())
}

/// Non-collection fields pass through on the strength of their TYPE
/// alone — a scalar, an `Option<Box<String>>`, a keyword field whose
/// rename survives the snake_case pass — byte for byte, attributes
/// included. Their policies are other passes' business.
#[test]
fn non_collection_fields_pass_through_byte_for_byte() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct RefreshedEntry {
    pub kind: PackageKind,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Box<String>>,

    #[serde(rename = "ref")]
    pub ref_: String,
}
"#;
    let doc = json!({
        "properties": {
            "kind": { "ref": "package_kind" }
        }
    });
    assert_eq!(apply(src, "entry/mod.rs", doc)?, src);
    Ok(())
}

/// A4.1's red: a REQUIRED collection carrying `omit` refuses per rule
/// R21 — a `properties` member is required; a writer omitting an empty
/// one would produce a document invalid by this same schema. The
/// refusal is its own (not the generic annotation complaint) and names
/// the schema, the site's path and the recipe.
#[test]
fn a_required_omit_collection_refuses_per_rule_r21() -> Result<()> {
    let doc = json!({
        "properties": {
            "outcomes": {
                "elements": { "ref": "outcome" },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    let err = policies(doc).expect_err("required + omit is a generation error");
    let msg = err.to_string();
    assert!(msg.contains("rule R21"), "names the rule: {msg}");
    assert!(
        msg.contains("properties.outcomes"),
        "names the site's path: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// A4.3's red: a collection member without `metadata."x-empty"` is a
/// generation error, not a default — the policy is decided on the
/// schema side and is not derivable from the generated Rust.
#[test]
fn a_collection_without_an_annotation_refuses() -> Result<()> {
    let doc = json!({
        "properties": {
            "outcomes": { "elements": { "ref": "outcome" } }
        }
    });
    let err = policies(doc).expect_err("a site must carry its policy");
    let msg = err.to_string();
    assert!(msg.contains("schema.jtd.json"), "names the schema: {msg}");
    assert!(
        msg.contains("properties.outcomes"),
        "names the site's path: {msg}"
    );
    assert!(msg.contains("x-empty"), "names the missing key: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// The annotation must be exactly `"omit"` or `"emit"` — anything else
/// refuses, naming what was found.
#[test]
fn a_stranger_annotation_refuses_naming_it() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "tags": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "sometimes" }
            }
        }
    });
    let err = policies(doc).expect_err("only omit and emit are policies");
    assert!(
        err.to_string().contains("sometimes"),
        "names what was found: {err}"
    );
    Ok(())
}

/// §2.6's conflict: two sites sharing the (wire, requiredness) key with
/// DIFFERENT policies — one field cannot omit and emit at once. The
/// refusal names the key and both policies.
#[test]
fn the_same_key_with_different_policies_refuses() -> Result<()> {
    let doc = json!({
        "definitions": {
            "left": {
                "optionalProperties": {
                    "packages": {
                        "elements": { "type": "string" },
                        "metadata": { "x-empty": "omit" }
                    }
                }
            },
            "right": {
                "optionalProperties": {
                    "packages": {
                        "elements": { "type": "string" },
                        "metadata": { "x-empty": "emit" }
                    }
                }
            }
        }
    });
    let err = policies(doc).expect_err("one key cannot carry two policies");
    let msg = err.to_string();
    assert!(msg.contains("`packages`"), "names the shared key: {msg}");
    assert!(
        msg.contains("`omit` and `emit`"),
        "names both policies: {msg}"
    );
    Ok(())
}

/// The legal diamond: two sites sharing a key with the SAME policy —
/// the vocabulary substitution can deliver one name by several routes —
/// is accepted, and both count as sites (the tally counts sites, not
/// distinct keys).
#[test]
fn a_diamond_of_identical_policies_is_legal() -> Result<()> {
    let doc = json!({
        "definitions": {
            "left": {
                "optionalProperties": {
                    "packages": {
                        "elements": { "type": "string" },
                        "metadata": { "x-empty": "omit" }
                    }
                }
            },
            "right": {
                "optionalProperties": {
                    "packages": {
                        "elements": { "type": "string" },
                        "metadata": { "x-empty": "omit" }
                    }
                }
            }
        }
    });
    assert_eq!(policies(doc)?.sites, 2);
    Ok(())
}

/// A4.4's red: the schema describes two collection fields, the Rust
/// carries one — the tally refuses, naming both counts. This is the
/// tripwire that keeps the site definition honest: a collection that
/// slipped past the scanner fails the run instead of passing for
/// processed.
#[test]
fn a_count_mismatch_refuses_with_both_numbers() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "packages": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "omit" }
            },
            "keywords": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    let err = apply(OPTIONAL_VEC_FIELD, "conflicts/mod.rs", doc)
        .expect_err("one collection in Rust against two sites in the schema");
    let msg = err.to_string();
    assert!(
        msg.contains("describes 2 collection fields but the generated file carries 1"),
        "names both counts: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
    Ok(())
}

/// Requiredness is read from BOTH sides of the stitch: the schema keys
/// `packages` optional, the Rust field is the bare (required) form — the
/// key (`packages`, required) describes nothing, and the pass refuses
/// rather than guess which side is right. One wire name alone would
/// have stitched these two different fields together.
#[test]
fn a_requiredness_mismatch_between_the_sides_refuses() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "packages": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    let err = apply(REQUIRED_VEC_FIELD, "uninstall_report/mod.rs", doc)
        .expect_err("a bare field against an optional-only schema key");
    let msg = err.to_string();
    assert!(msg.contains("`paths`"), "names the field: {msg}");
    assert!(
        msg.contains("required"),
        "names the requiredness it stitched with: {msg}"
    );
    Ok(())
}

/// A4.5's proof: the `elements` INSIDE a `values` node is the map's
/// value type — not a member of any `properties` block, so no policy is
/// expressible for it and it is NOT a site. The input here carries no
/// annotation on that inner node, the run is green, and the tally
/// converges (one site: the `values` field itself); had the pass
/// counted the inner node, it would be demanding an annotation no
/// schema position can carry.
#[test]
fn an_inner_elements_under_values_is_not_a_site() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "exclusive": {
                "values": { "elements": { "type": "string" } },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    let doc_policies = policies(doc)?;
    assert_eq!(
        doc_policies.sites, 1,
        "the inner `elements` under `values` is not a field"
    );
    let out = apply_with_policies(OPTIONAL_MAP_FIELD, "entry/mod.rs", &doc_policies)?;
    assert!(
        out.contains("skip_serializing_if = \"BTreeMap::is_empty\""),
        "the map site itself was stitched: {out}"
    );
    Ok(())
}

/// An `elements`-shaped key inside a `metadata` block is data, not a
/// site — the walk skips metadata on the way down, so this document
/// describes zero collection fields and a collection-free Rust file
/// passes clean.
#[test]
fn an_elements_key_inside_metadata_is_data_not_a_site() -> Result<()> {
    let doc = json!({
        "properties": {
            "note": {
                "type": "string",
                "metadata": { "elements": ["not", "a", "collection"] }
            }
        }
    });
    let doc_policies = policies(doc)?;
    assert_eq!(doc_policies.sites, 0, "metadata is data, not form");
    let out = apply_with_policies("// nothing generated\n", "empty/mod.rs", &doc_policies)?;
    assert_eq!(out, "// nothing generated\n");
    Ok(())
}
