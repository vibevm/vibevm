//! Finite boot-source migration cases; no suffix selects semantics.

use super::*;

fn source_text(path: &str) -> String {
    format!(
        "# café retained\n[package]\nname = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\nformat = 'normal' # explicit native contract\ndescription = 'unchanged value'\n\n[boot_snippet]\nsource = '{path}' # exact old declaration\n# final comment\n"
    )
}

fn source_choice(path: &str, media: &str) -> super::super::SourceDescriptor {
    serde_json::from_value(serde_json::json!({
        "file": path,
        "media": media,
        "interpretation": "vibe-spec",
        "role": "spec",
        "loading": "explicit"
    }))
    .unwrap()
}

fn source_migration_choice(decisions: &[super::super::SourceDescriptor]) -> MigrationSelection<'_> {
    MigrationSelection {
        source_profile: Some(SOURCE_BOOT_LEGACY),
        target_profile: Some(SOURCE_BOOT_CURRENT),
        recipe: Some(BOOT_SOURCE_INSERT),
        source_decisions: decisions,
    }
}

#[test]
fn source_boot_media_requires_path_specific_input_without_suffix_guess() {
    for path in ["boot/10-flow.xml", "boot/10-flow.md", "boot/10-flow.bin"] {
        let text = source_text(path);
        let choice = MigrationSelection {
            source_profile: Some(SOURCE_BOOT_LEGACY),
            target_profile: Some(SOURCE_BOOT_CURRENT),
            recipe: Some(BOOT_SOURCE_INSERT),
            source_decisions: &[],
        };
        let ManifestMigrationOutcome::NeedsInput(questions) =
            plan_manifest_migration(&text, choice)
        else {
            panic!("old boot source must ask for media")
        };
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].field, "source_media");
        assert_eq!(questions[0].resource_path.as_deref(), Some(path));
        assert_eq!(questions[0].choices, ["application/xml", "text/markdown"]);
    }
}

#[test]
fn supplied_source_decision_without_profiles_asks_only_source_lane_choices() {
    let decision = source_choice("boot/10-flow.xml", "application/xml");
    let choice = MigrationSelection {
        source_decisions: &[decision],
        ..MigrationSelection::default()
    };
    let ManifestMigrationOutcome::NeedsInput(questions) =
        plan_manifest_migration(&source_text("boot/10-flow.xml"), choice)
    else {
        panic!("explicit source decision needs its own profile choices")
    };
    assert_eq!(
        questions[0].choices,
        [SOURCE_BOOT_LEGACY, SOURCE_BOOT_CURRENT]
    );
    assert_eq!(questions[2].choices, [BOOT_SOURCE_INSERT, IDENTITY]);
}

#[test]
fn explicit_boot_source_decision_inserts_one_row_without_rewriting_original_bytes() {
    let path = "boot/10-flow.xml";
    let text = source_text(path);
    let decision = source_choice(path, "application/xml");
    let choice = MigrationSelection {
        source_profile: Some(SOURCE_BOOT_LEGACY),
        target_profile: Some(SOURCE_BOOT_CURRENT),
        recipe: Some(BOOT_SOURCE_INSERT),
        source_decisions: &[decision],
    };
    let ManifestMigrationOutcome::Planned(plan) = plan_manifest_migration(&text, choice) else {
        panic!("explicit boot source must plan")
    };
    assert_eq!(plan.edits.len(), 1);
    assert_eq!(plan.edits[0].before_span, text.len()..text.len());
    assert!(plan.edits[0].before_token.is_empty());
    assert!(plan.after.starts_with(&text));
    assert!(
        plan.after
            .contains("[[package.sources]]\nfile = \"boot/10-flow.xml\"")
    );
    assert!(plan.after.contains("media = \"application/xml\""));
    assert!(plan.after.contains("description = 'unchanged value'"));
    assert_eq!(plan.checked_manifest.package.unwrap().sources.len(), 1);
    let identity = MigrationSelection {
        source_profile: Some(SOURCE_BOOT_CURRENT),
        target_profile: Some(SOURCE_BOOT_CURRENT),
        recipe: Some(IDENTITY),
        source_decisions: &[],
    };
    assert!(matches!(
        plan_manifest_migration(&plan.after, identity),
        ManifestMigrationOutcome::NoOp(_)
    ));
}

#[test]
fn inserted_source_path_is_toml_quoted_without_losing_unicode_or_apostrophe() {
    let path = "boot/été'spec.xml";
    let text = source_text("boot/plain.xml").replace(
        "source = 'boot/plain.xml'",
        "source = \"boot/été'spec.xml\"",
    );
    let decision = source_choice(path, "application/xml");
    let ManifestMigrationOutcome::Planned(plan) =
        plan_manifest_migration(&text, source_migration_choice(&[decision]))
    else {
        panic!("quoted authored source must plan")
    };
    assert!(plan.after.starts_with(&text));
    assert_eq!(
        plan.checked_manifest.package.unwrap().sources[0]
            .selector()
            .path(),
        path
    );
}

#[test]
fn source_profile_refuses_simple_extra_boot_fields_and_unmatched_or_incompatible_decisions() {
    let path = "boot/10-flow.xml";
    let simple = source_text(path).replace("format = 'normal'", "format = 'simple'");
    assert!(matches!(
        plan_manifest_migration(&simple, source_migration_choice(&[])),
        ManifestMigrationOutcome::Refused(_)
    ));
    let extra = source_text(path) + "[[boot_snippet.fragment]]\nsource = 'other.xml'\n";
    assert!(matches!(
        plan_manifest_migration(&extra, source_migration_choice(&[])),
        ManifestMigrationOutcome::Refused(_)
    ));
    let unmatched = source_choice("boot/other.xml", "application/xml");
    assert!(matches!(
        plan_manifest_migration(&source_text(path), source_migration_choice(&[unmatched])),
        ManifestMigrationOutcome::Refused(_)
    ));
    let incompatible: super::super::SourceDescriptor = serde_json::from_value(serde_json::json!({
        "file":path,"media":"application/octet-stream","interpretation":"opaque","role":"resource","loading":"explicit"
    })).unwrap();
    assert!(matches!(
        plan_manifest_migration(&source_text(path), source_migration_choice(&[incompatible])),
        ManifestMigrationOutcome::Refused(_)
    ));
}
