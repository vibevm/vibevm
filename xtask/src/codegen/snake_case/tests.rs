//! Unit tests for the snake_case field pass — one per behaviour the pass
//! owes its callers: the rewrite and its identity-rename removal, the
//! information-carrying rename that STAYS (twice — once on the general
//! shape, once on the live keyword-escape site the measurement behind
//! this pass had missed), the named skip rule for enum
//! variants, the pass-throughs (type attributes, enum declarations,
//! second attributes in a kept run), and the loud refusals that keep a
//! moved generator pin from being absorbed silently. The samples quote
//! the pinned emission shape of jtd-codegen 0.4.1 verbatim — the pass
//! exists to be exact about that shape, so the tests must be exact about
//! it too.

use super::snake_case_fields;
use anyhow::Result;

/// The sample the core test drives — the field form of the real `entry`
/// schema, quoted verbatim.
const ENTRY_SOURCE: &str = r#"#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "content_hash")]
    pub contentHash: String,
}
"#;

/// The core rewrite: a camelCase identifier takes its snake_case form,
/// and the rename that would then repeat the identifier is removed.
/// The assertion is on the exact output, so a partial or creative
/// rewrite cannot sneak through.
#[test]
fn snake_cases_a_field_and_drops_the_identity_rename() -> Result<()> {
    let out = snake_case_fields(ENTRY_SOURCE, "entry/mod.rs")?;
    assert_eq!(
        out,
        r#"#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub content_hash: String,
}
"#
    );
    Ok(())
}

/// The branch that carries information: when the wire string differs
/// from `snake_case(identifier)` the rename is NOT decoration — the
/// schema declared a camelCase property on the wire — and it stays while
/// the identifier still takes its snake_case form.
#[test]
fn a_rename_that_carries_information_stays() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "contentHash")]
    pub contentHash: String,
}
"#;
    let out = snake_case_fields(src, "entry/mod.rs")?;
    assert_eq!(
        out,
        r#"#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}
"#
    );
    Ok(())
}

/// The LIVE site of that same branch, pinned with the shape it actually
/// has in the tree: `registry_sync_report` declares a property named
/// `ref`, a Rust keyword, so the generator escapes the identifier to
/// `ref_`. `snake_case("ref_")` is `"ref_"`, which is not `"ref"`, so the
/// rename is the only carrier of the wire name and must survive
/// untouched — dropping it would publish `"ref_"`, and that format has no
/// oracle to notice. A keyword-named schema property is a permanent
/// class, which is why it is pinned by data and not only by rule.
#[test]
fn a_keyword_escaped_identifier_keeps_its_rename() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct RefreshedEntry {
    #[serde(rename = "ref")]
    pub ref_: String,
}
"#;
    let out = snake_case_fields(src, "registry_sync_report/mod.rs")?;
    assert_eq!(
        out, src,
        "a keyword escape leaves the whole field untouched"
    );
    Ok(())
}

/// A field whose attribute run holds no rename refuses loudly: every
/// field of the pinned emission carries exactly one, so absence means
/// the emission shape moved. The message names the file, the line, and
/// the recipe.
#[test]
fn a_field_without_a_rename_refuses() {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<String>,
}
"#;
    let err = snake_case_fields(src, "entry/mod.rs")
        .expect_err("a field without a rename is not the pinned shape");
    let msg = err.to_string();
    assert!(msg.contains("entry/mod.rs:4"), "names file and line: {msg}");
    assert!(msg.contains("`maybe`"), "names the field: {msg}");
    assert!(
        msg.contains("snake_case.rs") && msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
}

/// Two renames in one run refuse too: the pinned emission writes exactly
/// one per field, and two names cannot both be the wire.
#[test]
fn two_renames_in_one_run_refuse() {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "a")]
    #[serde(rename = "b")]
    pub contentHash: String,
}
"#;
    let err = snake_case_fields(src, "entry/mod.rs")
        .expect_err("two renames in one run is not the pinned shape");
    let msg = err.to_string();
    assert!(msg.contains("entry/mod.rs:5"), "names file and line: {msg}");
    assert!(msg.contains("2"), "names the count: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
}

/// The named skip rule for variants: a rename followed by a variant line
/// (no `pub ` prefix — PascalCase identifiers the lint does not police,
/// wire strings no case rule derives) passes through byte for byte,
/// while the field next to it is rewritten.
#[test]
fn variants_pass_through_byte_for_byte() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Package {
    #[serde(rename = "delivery")]
    pub delivery: DeliveryMode,
}

#[derive(Serialize, Deserialize)]
pub enum DeliveryMode {
    #[serde(rename = "eager")]
    Eager,

    #[serde(rename = "lazy-pull")]
    LazyPull,
}
"#;
    let out = snake_case_fields(src, "entry/mod.rs")?;
    assert_eq!(
        out,
        r#"#[derive(Serialize, Deserialize)]
pub struct Package {
    pub delivery: DeliveryMode,
}

#[derive(Serialize, Deserialize)]
pub enum DeliveryMode {
    #[serde(rename = "eager")]
    Eager,

    #[serde(rename = "lazy-pull")]
    LazyPull,
}
"#
    );
    Ok(())
}

/// A field already in snake_case whose rename repeats it: the rename is
/// removed, the identifier does not move.
#[test]
fn an_identity_rename_on_an_already_snake_field_just_drops() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Boot {
    #[serde(rename = "path")]
    pub path: String,
}
"#;
    let out = snake_case_fields(src, "entry/mod.rs")?;
    assert_eq!(
        out,
        r#"#[derive(Serialize, Deserialize)]
pub struct Boot {
    pub path: String,
}
"#
    );
    Ok(())
}

/// A run of rename plus `skip_serializing_if`: the rename goes, the
/// second attribute survives on its own place in the run — order inside
/// a kept run is the generator's, not ours to rearrange.
#[test]
fn a_run_with_rename_and_skip_keeps_the_second_attribute_in_place() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
pub struct Conflicts {
    #[serde(rename = "packages")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Box<Vec<String>>>,
}
"#;
    let out = snake_case_fields(src, "entry/mod.rs")?;
    assert_eq!(
        out,
        r#"#[derive(Serialize, Deserialize)]
pub struct Conflicts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Box<Vec<String>>>,
}
"#
    );
    Ok(())
}

/// Type-level attributes resolve against `pub enum` / `pub struct`, not
/// a field, so the whole group passes through untouched — including the
/// tag attribute the boxing pass keys on (this pass runs after it, and
/// must not so much as breathe on its trigger).
#[test]
fn type_attributes_and_enum_declarations_pass_through() -> Result<()> {
    let src = r#"#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RepomdFileEntry {
    #[serde(rename = "directory")]
    Directory(Box<RepomdFileEntryDirectory>),
}
"#;
    assert_eq!(snake_case_fields(src, "repomd/mod.rs")?, src);
    Ok(())
}

/// A field identifier that is not an ASCII identifier refuses: the
/// pinned emission only emits ASCII field names, so this is a moved
/// pin, not a pass-through.
#[test]
fn a_non_ascii_field_identifier_refuses() {
    let src = "#[serde(rename = \"caf\u{e9}\")]\npub caf\u{e9}: String,\n";
    let err = snake_case_fields(src, "entry/mod.rs")
        .expect_err("a non-ASCII identifier is not the pinned shape");
    let msg = err.to_string();
    assert!(msg.contains("entry/mod.rs:2"), "names file and line: {msg}");
    assert!(msg.contains("ASCII"), "names the violation: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "says what to do: {msg}"
    );
}

/// A whole generated file in the shape of the real one — header, `use`,
/// a doc comment QUOTING attribute text (must not open a run decision),
/// structs with camelCase, optional, and boxed fields, a vocabulary
/// enum — changes in exactly one place per field. Everything around the
/// fields is the generator's layout, and the pass must not so much as
/// breathe on it, or `check-codegen` would report drift that isn't ours.
#[test]
fn a_whole_generated_file_changes_only_in_its_fields() -> Result<()> {
    let src = r#"// Code generated by jtd-codegen for Rust v0.2.1

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tags are lowercase, per the writer's
/// `#[serde(tag = "kind", rename_all = "lowercase")]`.
#[derive(Serialize, Deserialize)]
pub struct Compatibility {
    #[serde(rename = "min_vibe_version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minVibeVersion: Option<Box<String>>,

    #[serde(rename = "one_of")]
    pub oneOf: Vec<String>,

    #[serde(rename = "files")]
    pub files: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub enum NamingConvention {
    #[serde(rename = "fqdn")]
    Fqdn,

    #[serde(rename = "kind-name")]
    KindName,
}
"#;
    let out = snake_case_fields(src, "by_name/mod.rs")?;
    let expected = src
        .replace("    #[serde(rename = \"min_vibe_version\")]\n", "")
        .replace("    #[serde(rename = \"one_of\")]\n", "")
        .replace("    #[serde(rename = \"files\")]\n", "")
        .replace("pub minVibeVersion", "pub min_vibe_version")
        .replace("pub oneOf", "pub one_of");
    assert_eq!(out, expected);
    Ok(())
}
