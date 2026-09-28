use super::*;
use crate::generated::format_id::{ForeignParsers, FormatId, UnknownFields};
use crate::generated::preview::e1::documentation_capture::{BuildInputPurpose, Exclusion};
use crate::generated::preview::e1::documentation_map::TopicSourceKind;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-CAPTURE-MODEL");

fn git() -> DocumentationCapture {
    serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-documentation-capture/e1/valid/git.json"
    ))
    .unwrap()
}

fn content() -> DocumentationCapture {
    serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-documentation-capture/e1/valid/content.json"
    ))
    .unwrap()
}

fn refuses(capture: &DocumentationCapture, field: &str) {
    assert_eq!(
        capture.validate_documentation_capture().unwrap_err().field,
        field
    );
}

#[test]
fn registered_git_and_content_shapes_roundtrip_without_inferred_support() {
    for capture in [git(), content()] {
        capture.validate_documentation_capture().unwrap();
        let roundtrip: DocumentationCapture =
            serde_json::from_str(&serde_json::to_string(&capture).unwrap()).unwrap();
        assert_eq!(roundtrip, capture);
    }
    assert!(matches!(git().source_subject, SourceSubject::Git(_)));
    let capture = content();
    assert!(matches!(capture.source_subject, SourceSubject::Content(_)));
    assert!(capture.source_root_components.is_empty());
    assert!(capture.files[0].git_blob_sha1.is_none());
    assert!(capture.build_inputs[0].git_blob_sha1.is_none());
    let id = FormatId::PreviewDocumentationCapture;
    assert_eq!(id.epoch(), 1);
    assert!(!id.recoverable());
    assert_eq!(id.foreign_parsers(), ForeignParsers::Ours);
    assert_eq!(id.unknown_fields(), UnknownFields::Deny);
}

#[test]
fn generated_reader_refuses_invalid_subject_page_shape_and_shared_unknown_kind() {
    for invalid in [
        include_str!(
            "../../../../../formats/corpora/preview-documentation-capture/e1/invalid/unknown-source-kind.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-documentation-capture/e1/invalid/wrong-pages-type.json"
        ),
    ] {
        assert!(serde_json::from_str::<DocumentationCapture>(invalid).is_err());
    }
    let mut value = serde_json::to_value(git()).unwrap();
    value["pages"][0]["source_kind"] = serde_json::json!("invented-kind");
    assert!(serde_json::from_value::<DocumentationCapture>(value).is_err());
    let mut capture = content();
    capture.pages[0].source_kind = TopicSourceKind::Unbound;
    capture.validate_documentation_capture().unwrap();
}

#[test]
fn opaque_declarations_and_unknown_profile_recipe_are_retained_exactly() {
    let mut capture = content();
    capture.capture_id = " opaque:\n# α\0 ".into();
    capture.product_baseline_capture_id = "../opaque baseline".into();
    capture.source_profile = " foreign profile:\nα ".into();
    if let SourceSubject::Content(subject) = &mut capture.source_subject {
        subject.repository_key = " ../repository key ".into();
        subject.repository_identity = " opaque repository:\nα ".into();
        subject.recipe_id = "unknown/recipe:no-implementation".into();
    }
    let before = capture.clone();
    capture.validate_documentation_capture().unwrap();
    assert_eq!(capture, before);
    capture.pages[0].source_kind = TopicSourceKind::Generated;
    capture.pages[0].generated_from.clear();
    capture.validate_documentation_capture().unwrap();
    capture.pages.clear();
    capture.files.clear();
    capture.build_inputs.clear();
    capture.validate_documentation_capture().unwrap();
}

#[test]
fn blank_metadata_unknown_epoch_and_bad_source_identities_refuse() {
    for field in [
        "capture",
        "baseline",
        "profile",
        "repository-key",
        "repository-identity",
        "recipe",
    ] {
        let mut capture = content();
        match field {
            "capture" => capture.capture_id = " \n".into(),
            "baseline" => capture.product_baseline_capture_id.clear(),
            "profile" => capture.source_profile.clear(),
            _ => {
                if let SourceSubject::Content(subject) = &mut capture.source_subject {
                    match field {
                        "repository-key" => subject.repository_key.clear(),
                        "repository-identity" => subject.repository_identity.clear(),
                        _ => subject.recipe_id.clear(),
                    }
                }
            }
        }
        refuses(
            &capture,
            if matches!(field, "capture" | "baseline" | "profile") {
                "identity"
            } else {
                "source_subject"
            },
        );
    }
    let mut capture = git();
    capture.schema = 2;
    refuses(&capture, "schema");
    for tree in [false, true] {
        let mut capture = git();
        if let SourceSubject::Git(subject) = &mut capture.source_subject {
            if tree {
                subject.tree_sha1 = "A".repeat(40);
            } else {
                subject.commit_sha1 = "bad".into();
            }
        }
        refuses(&capture, "source_subject");
    }
    let mut capture = content();
    if let SourceSubject::Content(subject) = &mut capture.source_subject {
        subject.tree_sha256 = "bad".into();
    }
    refuses(&capture, "source_subject");
    let mut capture = content();
    capture.source_record_sha256 = "A".repeat(64);
    refuses(&capture, "source_record");
}

#[test]
fn git_requires_all_blob_ids_and_every_supplied_digest_is_checked() {
    let mut capture = git();
    capture.files[0].git_blob_sha1 = None;
    refuses(&capture, "files");
    let mut capture = git();
    capture.build_inputs[0].git_blob_sha1 = None;
    refuses(&capture, "build_inputs");
    let mut capture = content();
    capture.files[0].git_blob_sha1 = Some("bad".into());
    refuses(&capture, "files");
    let mut capture = content();
    capture.files[0].sha256 = "A".repeat(64);
    refuses(&capture, "files");
    let mut capture = content();
    capture.build_inputs[0].sha256 = "bad".into();
    refuses(&capture, "build_inputs");
}

#[test]
fn components_and_all_relative_reference_paths_refuse_escapes() {
    for component in ["", ".", "..", "docs/child", "C:docs", "docs\\child", "x\0y"] {
        let mut capture = content();
        capture.source_root_components = vec![component.into()];
        refuses(&capture, "source_root_components");
    }
    for bad in [
        "../outside",
        "/absolute",
        "C:/outside",
        "a\\b",
        "a//b",
        "x\0y",
    ] {
        let mut capture = content();
        capture.source_record = bad.into();
        refuses(&capture, "source_record");
        let mut capture = content();
        capture.files[0].path = bad.into();
        refuses(&capture, "files");
        let mut capture = content();
        capture.build_inputs[0].path = bad.into();
        refuses(&capture, "build_inputs");
        let mut capture = content();
        capture.pages[0].generated_from = vec![bad.into()];
        refuses(&capture, "pages.generated_from");
    }
}

#[test]
fn pages_and_generated_links_resolve_in_their_distinct_namespaces() {
    let mut capture = git();
    let mut template = capture.files[0].clone();
    template.path = "template.xml".into();
    capture.files.push(template);
    capture.pages[0].source_kind = TopicSourceKind::Generated;
    capture.pages[0].generated_from = vec![
        "docs/template.xml".into(),
        "documentation-build.toml".into(),
    ];
    capture.files.reverse();
    let before = capture.clone();
    capture.validate_documentation_capture().unwrap();
    assert_eq!(capture, before);
    capture.pages[0].generated_from = vec!["template.xml".into()];
    refuses(&capture, "pages.generated_from");
    capture.pages[0].generated_from.clear();
    capture.pages[0].path = "missing.xml".into();
    refuses(&capture, "pages");
}

#[test]
fn repeated_inventory_page_source_and_build_role_entries_refuse() {
    let mut capture = git();
    capture.files.push(capture.files[0].clone());
    refuses(&capture, "files");
    let mut capture = git();
    capture.pages.push(capture.pages[0].clone());
    refuses(&capture, "pages");
    let mut capture = git();
    capture.pages[0].generated_from = vec!["docs/guide.xml".into(), "docs/guide.xml".into()];
    refuses(&capture, "pages.generated_from");
    let mut capture = git();
    capture.build_inputs.push(capture.build_inputs[0].clone());
    refuses(&capture, "build_inputs");
}

#[test]
fn exclusions_use_source_relative_file_or_single_directory_prefix_semantics() {
    let mut capture = content();
    capture.exclusions.push(Exclusion {
        path: "guide.xml".into(),
        reason: "excluded".into(),
    });
    refuses(&capture, "pages");
    let mut capture = content();
    capture.files[0].path = "private/guide.xml".into();
    capture.pages[0].path = "private/guide.xml".into();
    capture.exclusions.push(Exclusion {
        path: "private".into(),
        reason: "a file exclusion".into(),
    });
    capture.validate_documentation_capture().unwrap();
    capture.exclusions[0].path = "private/".into();
    refuses(&capture, "pages");
    for bad in ["private//", "../private/", "x\0y/"] {
        let mut capture = content();
        capture.exclusions.push(Exclusion {
            path: bad.into(),
            reason: "excluded".into(),
        });
        refuses(&capture, "exclusions");
    }
    let mut capture = content();
    capture.exclusions.push(Exclusion {
        path: "unused/".into(),
        reason: " ".into(),
    });
    refuses(&capture, "exclusions");
    capture.exclusions[0].reason = "declared".into();
    capture.exclusions.push(capture.exclusions[0].clone());
    refuses(&capture, "exclusions");
}

#[test]
fn cross_file_and_build_roles_merge_partial_identity_and_reject_conflicts() {
    let mut capture = git();
    let mut input = capture.build_inputs[0].clone();
    input.path = "docs/guide.xml".into();
    capture.build_inputs.push(input);
    capture.validate_documentation_capture().unwrap();
    capture.build_inputs[1].sha256 = "a".repeat(64);
    refuses(&capture, "byte_identity");
    let mut capture = content();
    capture.build_inputs[0].path = "guide.xml".into();
    capture.build_inputs[0].git_blob_sha1 = Some("a".repeat(40));
    let mut other = capture.build_inputs[0].clone();
    other.purpose = BuildInputPurpose::Generator;
    capture.build_inputs.push(other);
    let before = capture.clone();
    capture.validate_documentation_capture().unwrap();
    assert_eq!(capture, before);
    assert!(capture.files[0].git_blob_sha1.is_none());
    let mut conflict = capture.build_inputs[0].clone();
    conflict.purpose = BuildInputPurpose::Lock;
    conflict.git_blob_sha1 = Some("b".repeat(40));
    capture.build_inputs.push(conflict);
    refuses(&capture, "byte_identity");
}

#[test]
fn toolchain_declarations_are_unique_nonblank_data_not_measured_runs() {
    let mut capture = content();
    capture.toolchains.push(capture.toolchains[0].clone());
    refuses(&capture, "toolchains");
    let mut capture = content();
    capture.toolchains[0].name.clear();
    refuses(&capture, "toolchains");
    let mut capture = content();
    capture.toolchains[0].version = " ".into();
    refuses(&capture, "toolchains");
}
