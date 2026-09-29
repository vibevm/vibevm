//! Declaration consistency cases, never actual product/source review.
use super::*;
use serde_json::{Value, json};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");

fn introduced() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-affected-source-review/e1/valid/introduced.json"
    ))
    .unwrap()
}

fn review(value: Value) -> AffectedSourceReview {
    serde_json::from_value(value).unwrap()
}

fn refused(value: Value, field: &str) {
    assert_eq!(
        review(value)
            .validate_affected_source_review()
            .unwrap_err()
            .field,
        field
    );
}

fn known_document(value: &mut Value) {
    let document = value["contract_sources"][0]["documents"][0].clone();
    value["paths"].as_array_mut().unwrap().push(json!({
        "path":document["path"], "interface_ids":[],
        "after":{"git_blob_sha1":document["git_blob_sha1"],"sha256":document["sha256"]},
        "user_no_impact_reason":"Permanent source accounting", "documentation_no_impact_reason":"Independent documentation disposition"
    }));
}

#[test]
fn generated_readers_own_all_eight_fixtures_and_existing_effect_kind_codec() {
    for raw in [
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/valid/introduced.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/valid/internal.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/valid/transient.json"
        ),
    ] {
        let declared: AffectedSourceReview = serde_json::from_str(raw).unwrap();
        declared.validate_affected_source_review().unwrap();
        assert_eq!(
            declared,
            serde_json::from_str(&serde_json::to_string(&declared).unwrap()).unwrap()
        );
    }
    for raw in [
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/invalid/unknown-field.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/invalid/wrong-epoch-type.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/invalid/wrong-endpoint-type.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/invalid/wrong-documents-type.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/invalid/unknown-effect-kind.json"
        ),
    ] {
        assert!(serde_json::from_str::<AffectedSourceReview>(raw).is_err());
    }
}

#[test]
fn opaque_ids_transient_endpoint_absence_and_empty_scope_remain_declarations() {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-affected-source-review/e1/valid/transient.json"
    ))
    .unwrap();
    for field in ["review_id", "before_capture_id", "after_capture_id"] {
        value[field] = json!(" ../opaque:/α\n spaced ID ");
    }
    let declared = review(value);
    let original = declared.clone();
    declared.validate_affected_source_review().unwrap();
    assert_eq!(declared, original);
    assert!(declared.paths[0].before.is_none());
    assert!(declared.paths[0].after.is_none());
    let mut empty = declared;
    empty.paths.clear();
    empty.validate_affected_source_review().unwrap();
}

#[test]
fn epoch_blank_identity_and_raw_source_binding_refuse() {
    let mut value = introduced();
    value["schema"] = json!(2);
    refused(value, "schema");
    for field in ["review_id", "before_capture_id", "after_capture_id"] {
        let mut value = introduced();
        value[field] = json!(" \n\t");
        refused(value, "identity");
    }
    for source in ["../outside.md", "/outside.md", "a\\b", "x\0y"] {
        let mut value = introduced();
        value["source_record"] = json!(source);
        refused(value, "source_record");
    }
    let mut value = introduced();
    value["source_record_sha256"] = json!("A".repeat(64));
    refused(value, "source_record");
}

#[test]
fn unsafe_repeated_paths_and_bad_endpoint_identities_refuse() {
    for path in ["../escape", "a//b", "a/./b", "a\0b", "C:/file", "a\\b"] {
        let mut value = introduced();
        value["paths"][0]["path"] = json!(path);
        refused(value, "paths");
    }
    let mut value = introduced();
    let row = value["paths"][0].clone();
    value["paths"].as_array_mut().unwrap().push(row);
    refused(value, "paths");
    for (endpoint, field, bad) in [
        ("after", "git_blob_sha1", "a".repeat(39)),
        ("after", "sha256", "F".repeat(64)),
        ("before", "sha256", "a".repeat(63)),
    ] {
        let mut value = introduced();
        value["paths"][0][endpoint] = value["paths"][0]["after"].clone();
        value["paths"][0][endpoint][field] = json!(bad);
        refused(value, "paths.identity");
    }
}

#[test]
fn unmapped_paths_require_both_reasons_and_mapped_paths_cannot_smuggle_them() {
    for key in ["user_no_impact_reason", "documentation_no_impact_reason"] {
        let mut value = introduced();
        value["paths"][0][key] = json!("reason");
        refused(value, "paths.no_impact");
        let internal: Value = serde_json::from_str(include_str!(
            "../../../../../formats/corpora/preview-affected-source-review/e1/valid/internal.json"
        ))
        .unwrap();
        let mut absent = internal.clone();
        absent["paths"][0].as_object_mut().unwrap().remove(key);
        refused(absent, "paths.no_impact");
        let mut blank = internal;
        blank["paths"][0][key] = json!(" ");
        refused(blank, "paths.no_impact");
    }
}

#[test]
fn path_and_interface_membership_must_resolve_in_both_directions() {
    let mut value = introduced();
    value["paths"][0]["interface_ids"] = json!(["unknown"]);
    refused(value, "interfaces.source_paths");
    let mut value = introduced();
    value["interfaces"][0]["source_paths"] = json!(["missing.rs"]);
    refused(value, "interfaces.source_paths");
    let mut value = introduced();
    value["interfaces"][0]["source_paths"] = json!([]);
    refused(value, "interfaces.source_paths");
    let mut value = introduced();
    value["paths"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"src/extra.rs","interface_ids":["preview-tool"]}));
    refused(value, "paths.interface_ids");
    let mut value = introduced();
    value["paths"][0]["interface_ids"] = json!(["preview-tool", "unknown"]);
    refused(value, "paths.interface_ids");
}

#[test]
fn interface_scope_duplicate_identity_and_list_members_refuse() {
    for key in [
        "interface_id",
        "capability_id",
        "audience",
        "platform",
        "before",
        "after",
    ] {
        let mut value = introduced();
        value["interfaces"][0][key] = json!(" ");
        refused(value, "interfaces");
    }
    let mut value = introduced();
    let row = value["interfaces"][0].clone();
    value["interfaces"].as_array_mut().unwrap().push(row);
    refused(value, "interfaces");
    for (key, field) in [
        ("interface_ids", "paths.interface_ids"),
        ("source_paths", "interfaces.source_paths"),
        ("contract_anchors", "interfaces.contract_anchors"),
        ("topic_ids", "interfaces.topic_ids"),
    ] {
        let mut value = introduced();
        let row = if key == "interface_ids" {
            &mut value["paths"][0]
        } else {
            &mut value["interfaces"][0]
        };
        let first = row[key][0].clone();
        row[key].as_array_mut().unwrap().push(first);
        refused(value, field);
    }
}

#[test]
fn permanent_contract_references_and_exact_topic_obligations_are_required() {
    for anchors in [
        json!([]),
        json!(["https://example.invalid"]),
        json!(["spec://org.example/tool/next/temporary#x"]),
    ] {
        let mut value = introduced();
        value["interfaces"][0]["contract_anchors"] = anchors;
        refused(value, "interfaces.contract_anchors");
    }
    for topics in [json!([]), json!(["unknown"])] {
        let mut value = introduced();
        value["interfaces"][0]["topic_ids"] = topics;
        refused(value, "interfaces.topic_ids");
    }
    let mut value = introduced();
    let row = value["topics"][0].clone();
    value["topics"].as_array_mut().unwrap().push(row);
    refused(value, "topics");
    let mut value = introduced();
    value["topics"][0]["obligation"] = json!(" ");
    refused(value, "topics");
    let mut value = introduced();
    value["topics"]
        .as_array_mut()
        .unwrap()
        .push(json!({"topic_id":"stale","obligation":"Unreferenced padding"}));
    refused(value, "topics");
}

#[test]
fn contract_source_bindings_roots_and_documents_require_safe_declared_coordinates() {
    for key in ["group", "name", "source_profile"] {
        let mut value = introduced();
        value["contract_sources"][0][key] = json!(" ");
        refused(value, "contract_sources");
    }
    let mut value = introduced();
    let row = value["contract_sources"][0].clone();
    value["contract_sources"].as_array_mut().unwrap().push(row);
    refused(value, "contract_sources");
    for root in [json!([".."]), json!(["a/b"]), json!([""]), json!(["x\0y"])] {
        let mut value = introduced();
        value["contract_sources"][0]["spec_root_components"] = root;
        refused(value, "contract_sources.spec_root_components");
    }
    for path in [
        "../outside.xml",
        "vibevm/vibespecs-sibling/a.xml",
        "vibevm/vibespecs/x\0y",
    ] {
        let mut value = introduced();
        value["contract_sources"][0]["documents"][0]["path"] = json!(path);
        refused(value, "contract_sources.documents");
    }
    let mut value = introduced();
    let row = value["contract_sources"][0]["documents"][0].clone();
    value["contract_sources"][0]["documents"]
        .as_array_mut()
        .unwrap()
        .push(row);
    refused(value, "contract_sources.documents");
    let mut value = introduced();
    value["contract_sources"][0]["manifest_path"] = json!("../vibe.toml");
    refused(value, "contract_sources.manifest");
    let mut value = introduced();
    value["contract_sources"][0]["manifest_sha256"] = json!("a".repeat(63));
    refused(value, "contract_sources.manifest");
    let mut value = introduced();
    value["contract_sources"][0]["documents"][0]["git_blob_sha1"] = json!("A".repeat(40));
    refused(value, "contract_sources.documents");
}

#[test]
fn known_current_documents_and_manifest_cannot_conflict_with_endpoints() {
    for field in ["sha256", "git_blob_sha1"] {
        let mut value = introduced();
        known_document(&mut value);
        value["contract_sources"][0]["documents"][0][field] = json!(if field == "sha256" {
            "a".repeat(64)
        } else {
            "a".repeat(40)
        });
        refused(value, "current_identity");
    }
    let mut value = introduced();
    known_document(&mut value);
    value["paths"][1].as_object_mut().unwrap().remove("after");
    refused(value, "current_identity");
    let mut value = introduced();
    let sha = value["contract_sources"][0]["manifest_sha256"].clone();
    value["paths"].as_array_mut().unwrap().push(json!({"path":"vibe.toml","interface_ids":[],"after":{"sha256":sha,"git_blob_sha1":"a".repeat(40)},"user_no_impact_reason":"one","documentation_no_impact_reason":"two"}));
    review(value.clone())
        .validate_affected_source_review()
        .unwrap();
    value["contract_sources"][0]["manifest_sha256"] = json!("a".repeat(64));
    refused(value, "current_identity");
}

#[test]
fn equivalent_cross_source_document_reuse_is_valid_but_conflicting_bytes_refuse() {
    let mut value = introduced();
    let mut second = value["contract_sources"][0].clone();
    second["name"] = json!("other reviewed authority");
    value["contract_sources"]
        .as_array_mut()
        .unwrap()
        .push(second);
    review(value.clone())
        .validate_affected_source_review()
        .unwrap();
    for field in ["git_blob_sha1", "sha256"] {
        let mut changed = value.clone();
        changed["contract_sources"][1]["documents"][0][field] = json!(if field == "sha256" {
            "a".repeat(64)
        } else {
            "a".repeat(40)
        });
        refused(changed, "current_identity");
    }
}

#[test]
fn foreign_profile_and_explicit_repository_root_are_valid_uninterpreted_data() {
    let mut value = introduced();
    value["contract_sources"][0]["source_profile"] = json!("urn:foreign:unavailable");
    value["contract_sources"][0]["spec_root_components"] = json!([]);
    let declared = review(value);
    let original = declared.clone();
    declared.validate_affected_source_review().unwrap();
    assert_eq!(declared, original);
}
