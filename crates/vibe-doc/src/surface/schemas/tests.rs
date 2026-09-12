//! The registry and the members it points at.

use std::path::{Path, PathBuf};

use super::*;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}

#[test]
fn the_registry_comes_back_record_by_record() {
    let (schemas, formats) = read(&repo_root()).expect("the registry");
    let manifest = formats
        .iter()
        .find(|f| f.id == "doc-manifest")
        .expect("the page manifest is inventoried");
    assert_eq!(manifest.epoch, 1);
    assert!(manifest.recoverable);
    assert_eq!(manifest.foreign_parsers, "many");
    assert!(
        schemas.iter().any(|s| s.id == "doc-manifest"),
        "a record with a schema brings its members with it"
    );
}

#[test]
fn a_record_without_a_schema_is_inventoried_without_members() {
    let (schemas, formats) = read(&repo_root()).expect("the registry");
    assert!(
        formats.iter().any(|f| f.id == "llms"),
        "`llms` is a markdown convention and is inventoried all the same"
    );
    assert!(
        !schemas.iter().any(|s| s.id == "llms"),
        "there is nothing for JTD to describe, so there are no members to record"
    );
}

#[test]
fn the_members_of_a_schema_carry_their_form_and_their_requiredness() {
    let (schemas, _) = read(&repo_root()).expect("the registry");
    let manifest = schemas
        .iter()
        .find(|s| s.id == "doc-manifest")
        .expect("the page manifest");
    let root = manifest
        .fields
        .iter()
        .find(|f| f.form.is_empty() && f.name == "pages")
        .expect("the root's `pages` member");
    assert!(root.required);
    assert_eq!(root.shape, "elements of ref doc_page");
    assert!(
        manifest
            .fields
            .iter()
            .any(|f| f.form == "audience" && f.name == "user"),
        "an enum's values are a contract of the same kind as a struct's members"
    );
}

#[test]
fn a_reading_is_the_same_reading_twice() {
    let first = read(&repo_root()).expect("the registry");
    let second = read(&repo_root()).expect("the registry");
    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);
}

#[test]
fn a_tree_with_no_registry_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let error = read(tmp.path()).expect_err("a refusal").to_string();
    assert!(error.contains("cannot read"), "{error}");
    assert!(error.contains("OBS-SURFACE-SNAPSHOTS"), "{error}");
}

#[test]
fn a_nullable_member_says_so_in_its_shape() {
    let definition = serde_json::json!({ "type": "uint32", "nullable": true });
    assert_eq!(shape(&definition), "uint32, nullable");
}
