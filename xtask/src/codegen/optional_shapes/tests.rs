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
use serde::{Deserialize, Serialize};
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

const REQUIRED_NULLABLE_MAP_ALIAS: &str = r#"#[derive(Serialize, Deserialize)]
pub struct ExtensionEntry {
    pub authored_config: Option<Box<JsonMap>>,
}

pub type JsonMap = BTreeMap<String, Option<Value>>;
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

/// An OPTIONAL member whose wire name is a Rust keyword is keyed by the
/// rename, never by the escaped identifier. The rename is the only thing
/// carrying `abstract` to the wire — the generator escapes the field to
/// `abstract_` and the snake_case pass keeps the rename precisely because
/// it no longer repeats the identifier — so a pass that read the
/// identifier instead would look up a member (`abstract_`) the schema
/// cannot describe and refuse a correct schema. The required half of this
/// class (`ref_`) is covered above; this is the optional half, which is
/// the one that reaches this pass at all.
#[test]
fn a_keyword_renamed_optional_is_keyed_by_its_wire_name() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct VersionEntry {
    #[serde(rename = "abstract")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_: Option<Box<String>>,
}
"#;
    let doc = json!({
        "optionalProperties": {
            "abstract": {
                "type": "string",
                "metadata": { "x-default": null }
            }
        }
    });
    let out = apply(src, "shared/mod.rs", doc)?;
    assert!(
        out.contains(r#"#[serde(rename = "abstract")]"#),
        "the rename rides through — it is the wire name: {out}"
    );
    assert!(
        out.contains("pub abstract_: Option<String>,"),
        "the box is lifted and the escaped identifier is untouched: {out}"
    );
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

#[path = "tests/required_nullable.rs"]
mod required_nullable;

#[path = "tests/refusals.rs"]
mod refusals;

#[path = "tests/stitch.rs"]
mod stitch;

#[path = "tests/union.rs"]
mod union;
