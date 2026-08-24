//! Unit tests for the optional-shapes pass — one per behaviour the pass
//! owes its callers: the schema-side site read (the `x-default` a
//! scalar must carry, the literals the pass refuses to build, the
//! structure that reads no annotation at all), the three reshaped
//! emissions (`null` scalar, `false` boolean, required-nullable), the
//! class cross-check between the two sides, the conflict and the
//! diamond, and the site-count tripwire. The samples quote the pinned
//! emission shape of jtd-codegen as the earlier passes leave it
//! (snake_case fields, `BTreeMap` maps, collections already collapsed) —
//! the pass exists to be exact about that shape, so the tests must be
//! exact about it too.

use std::path::Path;

use super::{OptionalShapes, apply_with_shapes, decisions_from_doc};
use anyhow::Result;
use serde_json::{Value, json};

/// The optional string of the real `by_name` output, quoted verbatim.
const OPTIONAL_STRING: &str = r#"#[derive(Serialize, Deserialize)]
pub struct Tombstone {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<Box<String>>,
}
"#;

/// The optional boolean of the real `list_report` output — the twin
/// `commands/list.rs` carries collapsed.
const OPTIONAL_BOOL: &str = r#"#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overridden: Option<Box<bool>>,
}
"#;

/// The optional enum shape the slot-record schema introduces.
const OPTIONAL_ENUM: &str = r#"#[derive(Serialize, Deserialize)]
pub struct SlotFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<Box<Disposition>>,
}

#[derive(Serialize, Deserialize)]
pub enum Disposition {
    #[serde(rename = "converted")]
    Converted,

    #[serde(rename = "copied")]
    Copied,
}
"#;

/// The optional structure of the real `entry` output, payload included.
const OPTIONAL_STRUCT: &str = r#"#[derive(Serialize, Deserialize)]
pub struct VersionEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_origin: Option<Box<WorkspaceOriginEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceOriginEntry {
    pub upstream: String,
}
"#;

/// The ref-resolved scalar of the real `by_name` output — the payload is
/// the local alias the generator minted for the `version` vocabulary.
const ALIAS_SCALAR: &str = r#"#[derive(Serialize, Deserialize)]
pub struct PackageEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_stable: Option<Box<Version>>,
}

pub type Version = String;
"#;

/// The ref-resolved DATE of the real `hello` output — the payload is
/// the local alias the generator minted for a `timestamp` member. The
/// form the tree carried nowhere until the handshake needed a world's
/// sunset.
const ALIAS_DATE: &str = r#"#[derive(Serialize, Deserialize)]
pub struct World {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunset: Option<Box<WorldSunset>>,
}

pub type WorldSunset = DateTime<FixedOffset>;
"#;

/// The required-nullable member of the real `list_report` output — no
/// skip attribute, the generator's own wire for "always present,
/// sometimes null".
const REQUIRED_NULLABLE: &str = r#"#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    /// Filename of the package's boot snippet under `vibevm/vibespecs/boot/`, or null
    /// if absent.
    pub boot_snippet: Option<Box<String>>,
}
"#;

/// The decisions of a one-off schema document.
fn shapes(doc: Value) -> Result<OptionalShapes> {
    decisions_from_doc(&doc, Path::new("schema.jtd.json"))
}

/// The full stitch over a one-off document.
fn apply(src: &str, file: &str, doc: Value) -> Result<String> {
    apply_with_shapes(src, file, &shapes(doc)?)
}

/// A3's first form: `x-default: null` keeps the `Option`, lifts the
/// `Box`, and replaces the pinned skip with the `default`-carrying form
/// — not a duplicate beside it.
#[test]
fn a_null_defaulted_scalar_keeps_the_option_and_lifts_the_box() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "superseded_by": {
                "type": "string",
                "metadata": { "x-default": null }
            }
        }
    });
    assert_eq!(
        apply(OPTIONAL_STRING, "by_name/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct Tombstone {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
}
"#
    );
    Ok(())
}

/// A3's second form: `x-default: false` collapses to the bare `bool`
/// with the hand-written twin's exact predicate — an absent key already
/// means `false`, so `false` is the value never written.
#[test]
fn a_false_defaulted_bool_collapses_to_the_bare_bool() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "overridden": {
                "type": "boolean",
                "metadata": { "x-default": false }
            }
        }
    });
    assert_eq!(
        apply(OPTIONAL_BOOL, "list_report/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overridden: bool,
}
"#
    );
    Ok(())
}

#[test]
fn a_null_defaulted_vocabulary_keeps_the_option_and_lifts_the_box() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "disposition": {
                "enum": ["converted", "copied"],
                "metadata": { "x-default": null }
            }
        }
    });
    assert_eq!(
        apply(OPTIONAL_ENUM, "slot_record/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct SlotFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<Disposition>,
}

#[derive(Serialize, Deserialize)]
pub enum Disposition {
    #[serde(rename = "converted")]
    Converted,

    #[serde(rename = "copied")]
    Copied,
}
"#
    );
    Ok(())
}

/// A4.5's green proof: an optional STRUCTURE carries no `x-default` and
/// the run is green anyway — and a stray annotation on one is not read,
/// because the decision for a structure is already made.
#[test]
fn an_optional_structure_needs_no_annotation() -> Result<()> {
    let definitions = json!({
        "workspace_origin_entry": {
            "properties": { "upstream": { "type": "string" } }
        }
    });
    let bare = json!({
        "optionalProperties": { "workspace_origin": { "ref": "workspace_origin_entry" } },
        "definitions": definitions
    });
    let annotated = json!({
        "optionalProperties": {
            "workspace_origin": {
                "ref": "workspace_origin_entry",
                "metadata": { "x-default": null }
            }
        },
        "definitions": definitions
    });
    let expected = r#"#[derive(Serialize, Deserialize)]
pub struct VersionEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_origin: Option<WorkspaceOriginEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceOriginEntry {
    pub upstream: String,
}
"#;
    assert_eq!(apply(OPTIONAL_STRUCT, "entry/mod.rs", bare)?, expected);
    assert_eq!(apply(OPTIONAL_STRUCT, "entry/mod.rs", annotated)?, expected);
    Ok(())
}

/// The schema classifies by the RESOLVED form (`{"ref": "version"}` →
/// `{"type": "string"}` → scalar) and the Rust side matches it through
/// the alias — `Version` is a `pub type` alias to `String`, not a
/// structure, and the reshaped field keeps the alias name verbatim.
#[test]
fn a_ref_resolved_scalar_and_its_alias_payload_agree() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "latest_stable": {
                "ref": "version",
                "metadata": { "x-default": null }
            }
        },
        "definitions": {
            "version": {
                "metadata": { "x-rust-type": "semver::Version" },
                "type": "string"
            }
        }
    });
    assert_eq!(
        apply(ALIAS_SCALAR, "by_name/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct PackageEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_stable: Option<Version>,
}

pub type Version = String;
"#
    );
    Ok(())
}

/// An OPTIONAL date — the form that had never reached this pass, and
/// the one hole in its primitive list. The schema side always called a
/// `timestamp` member a scalar; the Rust side knew every other `type`
/// form's spelling and not this one, so the stitch refused rather than
/// guessed and no schema in the tree could describe an optional date at
/// all. A REQUIRED date never got here, because only an optional
/// payload is reshaped — which is why the hole survived until the
/// eternal handshake needed `worlds[].sunset`.
#[test]
fn an_optional_date_collapses_like_any_other_scalar() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "sunset": {
                "ref": "world_sunset",
                "metadata": { "x-default": null }
            }
        },
        "definitions": {
            "world_sunset": {
                "metadata": { "x-rust-type": "chrono::DateTime<chrono::Utc>" },
                "type": "timestamp"
            }
        }
    });
    assert_eq!(
        apply(ALIAS_DATE, "hello/e1/hello/mod.rs", doc)?,
        r#"#[derive(Serialize, Deserialize)]
pub struct World {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunset: Option<WorldSunset>,
}

pub type WorldSunset = DateTime<FixedOffset>;
"#
    );
    Ok(())
}

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

/// Fields that are not `Option<Box<…>>` pass through on the strength of
/// their TYPE alone — required scalars, collapsed collections, a keyword
/// rename — byte for byte, attributes included.
#[test]
fn non_shape_fields_pass_through_byte_for_byte() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct RefreshedEntry {
    pub kind: PackageKind,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_written: Vec<String>,

    #[serde(rename = "ref")]
    pub ref_: String,
}
"#;
    let doc = json!({
        "properties": {
            "kind": { "ref": "package_kind" }
        }
    });
    assert_eq!(apply(src, "list_report/mod.rs", doc)?, src);
    Ok(())
}

/// An optional collection is not this pass's site — `empty_policy`
/// collapsed it one pass earlier — so the document describes zero sites
/// and the file rides through untouched.
#[test]
fn an_optional_collection_is_not_this_passs_site() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct ConflictsEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
}
"#;
    let doc = json!({
        "optionalProperties": {
            "packages": {
                "elements": { "type": "string" },
                "metadata": { "x-empty": "omit" }
            }
        }
    });
    let doc_shapes = shapes(doc)?;
    assert_eq!(doc_shapes.sites, 0, "collections are empty_policy's half");
    assert_eq!(apply_with_shapes(src, "entry/mod.rs", &doc_shapes)?, src);
    Ok(())
}

/// A4.1's red: an optional scalar without `x-default` is a generation
/// error, not a default — the policy is not derivable from the Rust.
#[test]
fn a_scalar_without_an_annotation_refuses() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "superseded_by": { "type": "string" }
        }
    });
    let err = shapes(doc).expect_err("an optional scalar must carry its policy");
    let msg = err.to_string();
    assert!(msg.contains("schema.jtd.json"), "names the schema: {msg}");
    assert!(
        msg.contains("optionalProperties.superseded_by"),
        "names the site's path: {msg}"
    );
    assert!(msg.contains("x-default"), "names the missing key: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// A4.2's red: a non-boolean literal — `"stable"` — refuses loudly,
/// naming the site, the value, and exactly what building it would take:
/// a NAMED default function and a NAMED skip predicate.
#[test]
fn a_non_boolean_literal_refuses_naming_what_it_would_take() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "latest_stable": {
                "ref": "version",
                "metadata": { "x-default": "stable" }
            }
        },
        "definitions": {
            "version": { "type": "string" }
        }
    });
    let err = shapes(doc).expect_err("the pass builds no string literals");
    let msg = err.to_string();
    assert!(
        msg.contains("optionalProperties.latest_stable"),
        "names the site: {msg}"
    );
    assert!(msg.contains("\"stable\""), "names the value: {msg}");
    assert!(
        msg.contains("#[serde(default = \"…\")]"),
        "names what it would take to build: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// The boolean twin of the same limit: `true` needs a named default
/// function (`serde(default)` spells `false`), so it refuses exactly
/// like `"stable"`.
#[test]
fn a_true_literal_refuses_like_any_other_named_default() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "yanked": {
                "type": "boolean",
                "metadata": { "x-default": true }
            }
        }
    });
    let err = shapes(doc).expect_err("serde(default) cannot spell true");
    let msg = err.to_string();
    assert!(msg.contains("true"), "names the value: {msg}");
    assert!(
        msg.contains("#[serde(default = \"…\")]"),
        "names what it would take to build: {msg}"
    );
    Ok(())
}

#[path = "tests/stitch.rs"]
mod stitch;
