//! Unit tests for the vocabulary-opening pass — one per behaviour the
//! pass owes its callers: the schema-side policy read (annotation
//! present, absent, stranger-valued; the diamond and its conflict), the
//! open/closed stitch with the union skip rule and the site-count
//! tripwire, and the loud refusals that keep a moved generator pin from
//! being absorbed silently. The samples quote the pinned emission shape
//! of jtd-codegen 0.4.1 verbatim — the pass exists to be exact about
//! that shape, so the tests must be exact about it too.

use std::path::Path;

use super::{open_with_policies, policies_from_doc};
use anyhow::Result;
use serde_json::{Value, json};

/// The vocabulary enum of the real `by_purl` output, quoted verbatim —
/// doc comment above the derive, per-variant renames, blank lines.
const VOCAB_ENUM: &str = r#"/// Installable package kind (VIBEVM-SPEC §4.1). Open vocabulary: the
/// register grows by owner amendment, so a reader must not hard-fail on
/// an unseen kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageKind {
    #[serde(rename = "feat")]
    Feat,

    #[serde(rename = "flow")]
    Flow,
}
"#;

/// The policies of a one-off schema document, as the pass would read them
/// off a resolved file.
fn policies(doc: Value) -> Result<super::VocabularyPolicies> {
    policies_from_doc(&doc, Path::new("schema.jtd.json"))
}

/// One open site over the given values, wrapped as a resolved schema.
fn open_site(values: &[&str]) -> Value {
    json!({
        "definitions": {
            "site": {
                "metadata": { "x-vocabulary": "open" },
                "enum": values
            }
        }
    })
}

/// An `open` policy takes the PROP-044 §4.2a form, exactly: the derive
/// comes off (it would collide with the manual impls), the doc comment
/// above it stays, variants keep their file order, and every known wire
/// string lands verbatim on both sides. This is the test that fails
/// without the transformation — it asserts the exact output, so a
/// partial or creative rewrite cannot sneak through.
#[test]
fn an_open_vocabulary_takes_the_open_form() -> Result<()> {
    let out = open_with_policies(
        VOCAB_ENUM,
        "by_purl/mod.rs",
        &policies(open_site(&["feat", "flow"]))?,
    )?;
    assert_eq!(
        out,
        r#"/// Installable package kind (VIBEVM-SPEC §4.1). Open vocabulary: the
/// register grows by owner amendment, so a reader must not hard-fail on
/// an unseen kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageKind {
    Feat,
    Flow,
    /// A value this build does not know. The string is preserved
    /// verbatim across a read/write cycle, so an older reader never
    /// silently drops or rewrites a newer writer's vocabulary
    /// (PROP-044 §4.2a).
    Unknown(String),
}

impl Serialize for PackageKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire: &str = match self {
            PackageKind::Feat => "feat",
            PackageKind::Flow => "flow",
            PackageKind::Unknown(value) => value.as_str(),
        };
        serializer.serialize_str(wire)
    }
}

impl<'de> Deserialize<'de> for PackageKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = String::deserialize(deserializer)?;
        Ok(match wire.as_str() {
            "feat" => PackageKind::Feat,
            "flow" => PackageKind::Flow,
            _ => PackageKind::Unknown(wire),
        })
    }
}
"#
    );
    Ok(())
}

/// A `closed` vocabulary is replayed byte for byte — derive, renames,
/// blank lines, everything. The stitch decides openness per vocabulary,
/// and a closed one must not move by one byte, or `check-codegen` would
/// report drift that is nobody's.
#[test]
fn a_closed_vocabulary_is_replayed_byte_for_byte() -> Result<()> {
    let doc = json!({
        "definitions": {
            "site": {
                "metadata": { "x-vocabulary": "closed" },
                "enum": ["feat", "flow"]
            }
        }
    });
    assert_eq!(
        open_with_policies(VOCAB_ENUM, "by_purl/mod.rs", &policies(doc)?)?,
        VOCAB_ENUM
    );
    Ok(())
}

/// THE UNION SKIP RULE, proven on a file that has both (A5.4): a
/// discriminator union next to a vocabulary enum is returned byte for
/// byte — derive, tag attribute, boxed arms, exactly as the boxing pass
/// left them — the vocabulary opens, and the site tally converges (one
/// schema site, one enum found; the pass returning `Ok` here IS the
/// counter check passing).
#[test]
fn a_discriminator_union_next_to_a_vocabulary_is_skipped_verbatim() -> Result<()> {
    let union = r#"/// The union the scanner must not touch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RepomdFileEntry {
    #[serde(rename = "directory")]
    Directory(Box<RepomdFileEntryDirectory>),

    #[serde(rename = "file")]
    File(Box<RepomdFileEntryFile>),
}
"#;
    let src = format!("{VOCAB_ENUM}\n{union}");
    let out = open_with_policies(
        &src,
        "repomd/mod.rs",
        &policies(open_site(&["feat", "flow"]))?,
    )?;
    assert!(
        out.contains(union),
        "the union rides along byte for byte: {out}"
    );
    assert!(
        out.contains("impl Serialize for PackageKind"),
        "the vocabulary opened: {out}"
    );
    assert!(
        !out.contains(
            "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub enum PackageKind"
        ),
        "the derive came off the opened enum: {out}"
    );
    Ok(())
}

/// A union BETWEEN two vocabularies: the scanner must re-sync on the
/// second derive after the union's body — its arm lines look like
/// variant lines but belong to no vocabulary — and both tallies
/// converge.
#[test]
fn the_scanner_resyncs_after_a_union_between_two_vocabularies() -> Result<()> {
    let union = "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n#[serde(tag = \"kind\")]\npub enum Event {\n    #[serde(rename = \"frozen\")]\n    Frozen(Box<EventFrozen>),\n}\n";
    let delivery = r#"/// Subskill materialisation mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryMode {
    #[serde(rename = "eager")]
    Eager,

    #[serde(rename = "lazy-pull")]
    LazyPull,

    #[serde(rename = "lazy-push")]
    LazyPush,
}
"#;
    let src = format!("{VOCAB_ENUM}\n{union}\n{delivery}");
    let doc = json!({
        "definitions": {
            "package_kind": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["feat", "flow"]
            },
            "delivery_mode": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["eager", "lazy-pull", "lazy-push"]
            }
        }
    });
    let out = open_with_policies(&src, "journal/mod.rs", &policies(doc)?)?;
    assert!(
        out.contains(union),
        "the union rides along byte for byte: {out}"
    );
    assert!(
        out.contains("impl Serialize for PackageKind")
            && out.contains("impl Serialize for DeliveryMode"),
        "both vocabularies opened: {out}"
    );
    Ok(())
}

/// A5.5's red: the schema describes two vocabularies, the Rust carries
/// one — the site tally refuses, naming both counts. This is the
/// tripwire that makes the union skip rule safe: a vocabulary that
/// slipped past the scanner fails the run instead of passing for
/// processed.
#[test]
fn a_vocabulary_count_mismatch_refuses_with_both_counts() -> Result<()> {
    let doc = json!({
        "definitions": {
            "package_kind": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["feat", "flow"]
            },
            "delivery_mode": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["eager", "lazy-pull", "lazy-push"]
            }
        }
    });
    let err = open_with_policies(VOCAB_ENUM, "entry/mod.rs", &policies(doc)?)
        .expect_err("one enum in Rust against two sites in the schema");
    let msg = err.to_string();
    assert!(
        msg.contains(
            "describes 2 enum definitions but the generated file carries 1 vocabulary enum"
        ),
        "names both counts: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
    Ok(())
}

/// A5.3's red: an enum that already spells a variant `Unknown` cannot
/// take the open form — the pass would mint a colliding second one — so
/// it refuses instead of corrupting the type.
#[test]
fn an_existing_unknown_variant_refuses() -> Result<()> {
    let src = r#"#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Odd {
    #[serde(rename = "a")]
    Unknown,
}
"#;
    let err = open_with_policies(src, "odd/mod.rs", &policies(open_site(&["a"]))?)
        .expect_err("an `Unknown` variant collides with the open form's own");
    let msg = err.to_string();
    assert!(
        msg.contains("already has a variant named `Unknown`"),
        "names the collision: {msg}"
    );
    Ok(())
}

/// A5.2's red: two enum sites with the same wire values and different
/// policies — one set of values is one vocabulary, and it cannot be open
/// and closed at once. The refusal names the set and both policies.
#[test]
fn the_same_values_with_different_policies_refuse() -> Result<()> {
    let doc = json!({
        "definitions": {
            "left": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["feat", "flow"]
            },
            "right": {
                "metadata": { "x-vocabulary": "closed" },
                "enum": ["flow", "feat"]
            }
        }
    });
    let err = policies(doc).expect_err("one vocabulary cannot carry two policies");
    let msg = err.to_string();
    assert!(
        msg.contains("\"feat\", \"flow\""),
        "names the shared set: {msg}"
    );
    assert!(
        msg.contains("`open` and `closed`"),
        "names both policies: {msg}"
    );
    Ok(())
}

/// The legal diamond: two definitions carrying the same values with the
/// SAME policy — the vocabulary substitution can deliver one name by
/// several routes — is accepted, and both count as sites.
#[test]
fn a_diamond_of_identical_policies_is_legal() -> Result<()> {
    let doc = json!({
        "definitions": {
            "left": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["feat", "flow"]
            },
            "right": {
                "metadata": { "x-vocabulary": "open" },
                "enum": ["flow", "feat"]
            }
        }
    });
    let doc_policies = policies(doc)?;
    assert_eq!(
        doc_policies.sites, 2,
        "sites count definitions, not distinct sets"
    );
    Ok(())
}

/// A missing `metadata."x-vocabulary"` on an enum site is a generation
/// error, not a default: the refusal names the schema, the vocabulary's
/// values, and the recipe.
#[test]
fn a_site_without_an_annotation_refuses() -> Result<()> {
    let doc = json!({"definitions": {"site": {"enum": ["feat"]}}});
    let err = policies(doc).expect_err("a site must carry its policy");
    let msg = err.to_string();
    assert!(msg.contains("schema.jtd.json"), "names the schema: {msg}");
    assert!(msg.contains("\"feat\""), "names the values: {msg}");
    assert!(msg.contains("x-vocabulary"), "names the missing key: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// The annotation must be exactly `"open"` or `"closed"` — anything else
/// refuses, naming what was found.
#[test]
fn a_stranger_annotation_refuses_naming_it() -> Result<()> {
    let doc = json!({
        "definitions": {
            "site": {
                "metadata": { "x-vocabulary": "sometimes" },
                "enum": ["feat"]
            }
        }
    });
    let err = policies(doc).expect_err("only open and closed are policies");
    let msg = err.to_string();
    assert!(msg.contains("sometimes"), "names what was found: {msg}");
    Ok(())
}

/// An `"enum"`-shaped key inside a `metadata` block is data, not a site —
/// the walk skips metadata on the way down, so this document describes
/// zero vocabularies and an enum-free Rust file passes clean.
#[test]
fn an_enum_key_inside_metadata_is_data_not_a_site() -> Result<()> {
    let doc = json!({
        "properties": {
            "note": {
                "type": "string",
                "metadata": { "enum": ["not", "a", "vocabulary"] }
            }
        }
    });
    let doc_policies = policies(doc)?;
    assert_eq!(doc_policies.sites, 0, "metadata is data, not form");
    let out = open_with_policies("// nothing generated\n", "empty/mod.rs", &doc_policies)?;
    assert_eq!(out, "// nothing generated\n");
    Ok(())
}

/// A vocabulary enum holding a line that is none of the three legal
/// shapes refuses, naming the file, the line and the offending text —
/// the same contract the boxing pass keeps: the emission shape is
/// pinned, and an unfamiliar line means the pin moved.
#[test]
fn an_unfamiliar_line_inside_a_vocabulary_refuses() {
    let src = r#"#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Odd {
    #[serde(rename = "feat")]
    Feat,

    pub entries: u32,
}
"#;
    let err = open_with_policies(
        src,
        "odd/mod.rs",
        &policies(open_site(&["feat"])).expect("the document is well formed"),
    )
    .expect_err("a field line inside a vocabulary enum is not the pinned shape");
    let msg = err.to_string();
    assert!(msg.contains("odd/mod.rs:6"), "names file and line: {msg}");
    assert!(msg.contains("pub entries: u32"), "quotes the line: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
}
