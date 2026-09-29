//! Codec boundaries and finite declaration consistency, never runtime coverage.

use super::*;
use crate::generated::capability::e1::coverage::{ExternalAccess, Implementation};
use serde_json::{Value, json};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#COVERAGE-INVENTORY");

fn mapped() -> Coverage {
    serde_json::from_str(include_str!(
        "../../../../../formats/corpora/capability-coverage/e1/valid/mapped.json"
    ))
    .unwrap()
}

#[test]
fn authored_inventory_has_valid_declarations() {
    let inventory: Coverage = serde_json::from_str(include_str!(
        "../../../../../formats/capability-coverage/coverage-e1.json"
    ))
    .unwrap();
    inventory.validate_capability_coverage().unwrap();
    // This checks the authored data's declarations, not source freshness or registration coverage.
}

fn refused(value: &Coverage, field: &str) -> CapabilityCoverageError {
    let error = value.validate_capability_coverage().unwrap_err();
    assert_eq!(error.field, field, "{error:?}");
    assert!(!error.subject.trim().is_empty());
    assert!(!error.reason.trim().is_empty());
    error
}

fn changed(section: &str, field: &str, replacement: Value) -> Coverage {
    let mut value = serde_json::to_value(mapped()).unwrap();
    value[section][0][field] = replacement;
    serde_json::from_value(value).unwrap()
}

#[test]
fn generated_reader_owns_all_twelve_codec_fixtures() {
    let empty: Coverage = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/capability-coverage/e1/valid/empty.json"
    ))
    .unwrap();
    refused(&empty, "sources");
    let mapped = mapped();
    mapped.validate_capability_coverage().unwrap();
    assert_eq!(
        mapped,
        serde_json::from_str(&serde_json::to_string(&mapped).unwrap()).unwrap()
    );
    for raw in [
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/missing-model-schema-ref.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/wrong-schema-type.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/unknown-disposition.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/unknown-implementation.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/wrong-model-effects-type.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/unknown-field.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/unknown-external-access.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/unknown-surface-kind.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/schema-out-of-range.json"
        ),
        include_str!(
            "../../../../../formats/corpora/capability-coverage/e1/invalid/wrong-registered-type.json"
        ),
    ] {
        assert!(serde_json::from_str::<Coverage>(raw).is_err());
    }
}

#[test]
fn explicit_registration_role_distinguishes_native_and_direct_declarations() {
    let mut missing = serde_json::to_value(mapped()).unwrap();
    missing["surfaces"][0]
        .as_object_mut()
        .unwrap()
        .remove("registered");
    assert!(serde_json::from_value::<Coverage>(missing).is_err());

    for kind in [
        SurfaceKind::Cli,
        SurfaceKind::Mcp,
        SurfaceKind::DeveloperTool,
    ] {
        let mut value = mapped();
        for surface in &mut value.surfaces {
            surface.kind = kind.clone();
        }
        value.validate_capability_coverage().unwrap();
        value.surfaces[1].registered = false;
        refused(&value, "surfaces.registered");
    }

    let mut extension = mapped();
    for surface in &mut extension.surfaces {
        surface.kind = SurfaceKind::ExtensionRegistration;
        surface.disposition = Disposition::DynamicRegistration;
        surface.operation_id = None;
        surface.registered = false;
    }
    extension.validate_capability_coverage().unwrap();
    extension.surfaces[0].registered = true;
    refused(&extension, "surfaces.registered");

    let mut tui = mapped();
    for surface in &mut tui.surfaces {
        surface.kind = SurfaceKind::Tui;
    }
    tui.surfaces[1].registered = false;
    let original = tui.clone();
    tui.validate_capability_coverage().unwrap();
    assert_eq!(tui, original); // The flag is a declaration, not native coverage proof.
    let mut collision = tui.surfaces[1].clone();
    collision.surface_id = "registered sibling".into();
    collision.name = "other canonical token".into();
    collision.registered = true;
    tui.surfaces.push(collision);
    refused(&tui, "surfaces.names"); // Role does not create another alias namespace.
}

#[test]
fn opaque_identities_survive_and_declarations_do_not_infer_proof() {
    let mut value = mapped();
    let opaque = " ../opaque:/α\n identity ";
    value.inventory_id = opaque.into();
    value.baseline_capture_id = opaque.into();
    value.observation_capture_id = opaque.into();
    value.sources[0].source_id = opaque.into();
    value.sources[0].path = "not-observed/source.rs".into();
    value.sources[0].sha256 = "0".repeat(64);
    value.surfaces[0].surface_id = opaque.into();
    value.surfaces[1].parent_surface_id = Some(opaque.into());
    value.surfaces[1].operation_id = Some(opaque.into());
    value.models[0].model_id = opaque.into();
    value.planned_families[0].family_id = opaque.into();
    for surface in &mut value.surfaces {
        surface.source_ids = vec![opaque.into()];
        surface.effects.clear();
        surface.conformance_cases.clear();
        surface.implementation = Implementation::SourceObserved;
        surface.external_access = ExternalAccess::Structured;
        surface.authority = "declared authorized and fully verified label".into();
    }
    value.models[0].source_ids = vec![opaque.into()];
    value.planned_families[0].source_ids = vec![opaque.into()];
    let original = value.clone();
    value.validate_capability_coverage().unwrap();
    assert_eq!(value, original);
    assert_eq!(value.inventory_id, opaque);
    // No lookup of the invented source or interpretation of producer labels ran.
}

#[test]
fn epoch_identity_and_required_sections_refuse_without_source_reads() {
    let mut value = mapped();
    value.schema = 2;
    refused(&value, "schema");
    for field in [
        "inventory_id",
        "baseline_capture_id",
        "observation_capture_id",
    ] {
        let mut json = serde_json::to_value(mapped()).unwrap();
        json[field] = json!(" \n\t");
        refused(&serde_json::from_value(json).unwrap(), field);
    }
    for section in ["sources", "surfaces", "models"] {
        let mut json = serde_json::to_value(mapped()).unwrap();
        json[section] = json!([]);
        refused(&serde_json::from_value(json).unwrap(), section);
    }
    let mut value = mapped();
    value.planned_families.clear();
    value.validate_capability_coverage().unwrap();
}

#[test]
fn source_paths_hashes_and_duplicate_identity_or_path_refuse() {
    for path in [
        "",
        " ",
        "/absolute",
        "a//b",
        "a/./b",
        "../b",
        "a/../b",
        "a/",
        "a\\b",
        "a\0b",
        "C:/file",
        "a/C:file",
        "a/file:stream",
    ] {
        refused(&changed("sources", "path", json!(path)), "sources.path");
    }
    for digest in [
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
    ] {
        refused(
            &changed("sources", "sha256", json!(digest)),
            "sources.sha256",
        );
    }
    let mut value = mapped();
    value.sources.push(value.sources[0].clone());
    refused(&value, "sources.source_id");
    value.sources[1].source_id = "distinct ID".into();
    refused(&value, "sources.path");
    let mut value = mapped();
    value.sources[0].path = "src/α file.rs".into();
    value.validate_capability_coverage().unwrap();
}

#[test]
fn all_source_reference_families_require_unique_known_nonempty_bindings() {
    for section in ["surfaces", "models", "planned_families"] {
        for references in [
            json!([]),
            json!(["unknown"]),
            json!([" "]),
            json!(["source:shape-fixture", "source:shape-fixture"]),
        ] {
            refused(
                &changed(section, "source_ids", references),
                &format!("{section}.source_ids"),
            );
        }
    }
    // The same retained source can legitimately support several independent rows.
    mapped().validate_capability_coverage().unwrap();
}

#[test]
fn mandatory_description_fields_and_optional_identities_are_nonblank() {
    for (section, fields) in [
        ("sources", vec!["source_id"]),
        (
            "surfaces",
            vec![
                "surface_id",
                "name",
                "owner",
                "target",
                "authority",
                "carrier",
                "schema_ref",
                "completeness",
                "retention",
                "reason",
            ],
        ),
        (
            "models",
            vec![
                "model_id",
                "name",
                "owner",
                "target",
                "authority",
                "carrier",
                "schema_ref",
                "completeness",
                "retention",
            ],
        ),
        (
            "planned_families",
            vec!["family_id", "name", "owner", "reason"],
        ),
    ] {
        for field in fields {
            refused(
                &changed(section, field, json!(" \n")),
                &format!("{section}.{field}"),
            );
        }
    }
    for field in ["parent_surface_id", "operation_id"] {
        refused(
            &changed("surfaces", field, json!(" \n")),
            &format!("surfaces.{field}"),
        );
    }
}

#[test]
fn row_identities_are_unique_but_shared_operation_identity_is_legal() {
    for (section, expected) in [
        ("surfaces", "surfaces.surface_id"),
        ("models", "models.model_id"),
        ("planned_families", "planned_families.family_id"),
    ] {
        let mut json = serde_json::to_value(mapped()).unwrap();
        let duplicate = json[section][0].clone();
        json[section].as_array_mut().unwrap().push(duplicate);
        refused(&serde_json::from_value(json).unwrap(), expected);
    }
    let mut value = mapped();
    let mut other = value.surfaces[1].clone();
    other.surface_id = "other channel".into();
    other.kind = SurfaceKind::Mcp;
    other.parent_surface_id = None;
    value.surfaces.push(other);
    value.validate_capability_coverage().unwrap();
    assert_eq!(
        value.surfaces[1].operation_id,
        value.surfaces[2].operation_id
    );
    value.surfaces[1].operation_id = None;
    refused(&value, "surfaces.operation_id");
}

#[test]
fn parent_dangling_cross_kind_self_and_cycles_refuse() {
    for parent in ["unknown", "surface:fixture-read"] {
        let mut value = mapped();
        value.surfaces[1].parent_surface_id = Some(parent.into());
        refused(&value, "surfaces.parent_surface_id");
    }
    let mut value = mapped();
    value.surfaces[0].kind = SurfaceKind::DeveloperTool;
    refused(&value, "surfaces.parent_surface_id");
    let mut value = mapped();
    value.surfaces[0].parent_surface_id = Some(value.surfaces[1].surface_id.clone());
    let error = refused(&value, "surfaces.parent_surface_id");
    assert_eq!(error.reason, "parent cycle");
    let mut third = value.surfaces[0].clone();
    third.surface_id = "third".into();
    third.name = "third".into();
    value.surfaces[1].parent_surface_id = Some("third".into());
    third.parent_surface_id = Some(value.surfaces[0].surface_id.clone());
    value.surfaces.push(third);
    assert_eq!(
        refused(&value, "surfaces.parent_surface_id").reason,
        "parent cycle"
    );
}

#[test]
fn canonical_alias_and_sibling_collisions_refuse_but_parent_scopes_are_distinct() {
    for aliases in [json!([" "]), json!(["same", "same"])] {
        refused(&changed("surfaces", "aliases", aliases), "surfaces.aliases");
    }
    let mut value = mapped();
    let canonical = value.surfaces[1].name.clone();
    value.surfaces[1].aliases.push(canonical);
    refused(&value, "surfaces.names");
    let mut value = mapped();
    let mut sibling = value.surfaces[1].clone();
    sibling.surface_id = "sibling".into();
    sibling.name = "distinct canonical name".into();
    value.surfaces.push(sibling);
    refused(&value, "surfaces.names");
    value.surfaces[2].aliases.clear();
    value.surfaces[2].name = value.surfaces[1].aliases[0].clone();
    refused(&value, "surfaces.names");
    let mut group = value.surfaces[0].clone();
    group.surface_id = "other parent".into();
    group.name = "other parent".into();
    value.surfaces[2] = value.surfaces[1].clone();
    value.surfaces[2].surface_id = "sibling".into();
    value.surfaces[2].parent_surface_id = Some(group.surface_id.clone());
    value.surfaces.push(group);
    value.validate_capability_coverage().unwrap();
}

#[test]
fn effects_and_cases_validate_declared_entries_without_guessing_semantics() {
    for section in ["surfaces", "models"] {
        for field in ["effects", "conformance_cases"] {
            for entries in [json!([" "]), json!(["same", "same"])] {
                refused(
                    &changed(section, field, entries),
                    &format!("{section}.{field}"),
                );
            }
            changed(section, field, json!([]))
                .validate_capability_coverage()
                .unwrap();
        }
    }
    for disposition in [
        Disposition::Presentation,
        Disposition::Group,
        Disposition::DynamicRegistration,
    ] {
        let mut value = mapped();
        value.surfaces[1].disposition = disposition;
        value.surfaces[1].operation_id = None;
        value.surfaces[1].effects = vec!["arbitrary declared external write label".into()];
        value.validate_capability_coverage().unwrap();
        value.surfaces[1].reason = " ".into();
        refused(&value, "surfaces.reason");
    }
}
