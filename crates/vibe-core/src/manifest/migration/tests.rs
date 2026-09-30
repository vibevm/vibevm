//! Synthetic package integration cases, plus one exactly retained scalar.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");

use super::*;

fn selection(
    source: &'static str,
    target: &'static str,
    recipe: &'static str,
) -> MigrationSelection<'static> {
    MigrationSelection {
        source_profile: Some(source),
        target_profile: Some(target),
        recipe: Some(recipe),
    }
}

fn legacy() -> MigrationSelection<'static> {
    selection(LEGACY_ABSENT, CURRENT_ABSENT, SNAPSHOT_TO_COPY)
}
fn current() -> MigrationSelection<'static> {
    selection(CURRENT_ABSENT, CURRENT_ABSENT, IDENTITY)
}

// Deliberately synthetic: a package-role integration of the documented rename,
// not a retained historical manifest or a real-file migration corpus.
fn package(token: &str) -> String {
    format!(
        "# UTF-8 café\n[package]\nname = 'demo'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\nmaterialization = {token} # selected\nbridge = true\nfrozen = false\ndescription = 'unrelated snapshot text'\n\n[requires.packages]\n'flow:org.example/base' = {{ version = '^1.0', link = 'static-hard' }} # selector\n# footer\n"
    )
}

fn planned(text: &str) -> ManifestMigrationPlan {
    match plan_manifest_migration(text, legacy()) {
        ManifestMigrationOutcome::Planned(plan) => plan,
        other => panic!("expected planned, got {other:?}"),
    }
}

fn noop(text: &str, choice: MigrationSelection<'_>) -> ManifestMigrationPlan {
    match plan_manifest_migration(text, choice) {
        ManifestMigrationOutcome::NoOp(plan) => plan,
        other => panic!("expected no-op, got {other:?}"),
    }
}

fn refused(
    text: &str,
    choice: MigrationSelection<'_>,
    kind: MigrationRefusalKind,
) -> MigrationRefusal {
    match plan_manifest_migration(text, choice) {
        ManifestMigrationOutcome::Refused(reason) => {
            assert_eq!(reason.kind, kind);
            reason
        }
        other => panic!("expected refusal, got {other:?}"),
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
fn transforms_one_token_preserving_choices_and_unrelated_bytes() {
    let text = package("'snapshot'");
    let plan = planned(&text);
    assert_eq!(
        plan.after,
        text.replacen("'snapshot' # selected", "'copy' # selected", 1)
    );
    assert_eq!(plan.edits.len(), 1);
    let edit = &plan.edits[0];
    assert_eq!(text.get(edit.before_span.clone()), Some("'snapshot'"));
    assert_eq!(edit.field, "package.materialization");
    assert_eq!(edit.after_token, "'copy'");
    let package = plan.checked_manifest.package.as_ref().unwrap();
    assert_eq!(package.epoch, None);
    assert_eq!(package.version.to_string(), "1.1.0");
    assert!(package.bridge);
    assert!(!package.frozen);
    assert_eq!(
        package.description.as_deref(),
        Some("unrelated snapshot text")
    );
    assert_eq!(package.materialization, super::super::Materialization::Copy);
}

#[test]
fn deterministic_plan_and_identity_depend_on_exact_input_and_selection() {
    let text = package("\"snapshot\"");
    let first = planned(&text);
    assert_eq!(first, planned(&text));
    assert_eq!(first.before_identity.len(), 64);
    assert_ne!(first.before_identity, first.after_identity);
    let altered = planned(&(text.clone() + "# comment changes identity\n"));
    assert_ne!(first.before_identity, altered.before_identity);
    assert_ne!(first.after_identity, altered.after_identity);
    assert_ne!(first.diff_identity, altered.diff_identity);
    let identity_plan = noop(&first.after, current());
    assert_eq!(first.after_identity, identity_plan.before_identity);
    assert_eq!(identity_plan.before_identity, identity_plan.after_identity);
    assert_ne!(first.diff_identity, identity_plan.diff_identity);
    refused(&text, current(), MigrationRefusalKind::UnrecognizedShape);
}

#[test]
fn planned_output_is_byte_identical_current_fixpoint() {
    let output = planned(&package("'snapshot'")).after;
    let second = noop(&output, current());
    assert_eq!(second.before, output);
    assert_eq!(second.after, output);
    assert!(second.edits.is_empty());
    assert_eq!(second, noop(&second.after, current()));
}

#[test]
fn current_modes_and_absent_default_are_exact_identity() {
    for token in ["'copy'", "\"hardlink\"", "'in-place'"] {
        let text = package(token);
        let plan = noop(&text, current());
        assert_eq!(plan.after, text);
        assert!(plan.edits.is_empty());
        assert_eq!(plan.checked_manifest.package.unwrap().epoch, None);
    }
    let text = package("'copy'").replace("materialization = 'copy' # selected\n", "");
    let plan = noop(&text, current());
    assert_eq!(plan.after, text);
    assert_eq!(plan.checked_manifest.package.unwrap().epoch, None);
}

#[test]
fn explicit_epoch_one_has_separate_identity_without_epoch_insertion() {
    let text = package("'copy'").replace("[package]\n", "[package]\nepoch = 1\n");
    let choice = selection(CURRENT_EPOCH1, CURRENT_EPOCH1, IDENTITY);
    let plan = noop(&text, choice);
    assert_eq!(plan.after, text);
    assert_eq!(plan.checked_manifest.package.unwrap().epoch, Some(1));
    refused(&text, current(), MigrationRefusalKind::UnrecognizedShape);
    refused(
        &package("'copy'"),
        choice,
        MigrationRefusalKind::UnrecognizedShape,
    );
    refused(
        &text.replace("epoch = 1", "epoch = 0"),
        choice,
        MigrationRefusalKind::UnrecognizedShape,
    );
    refused(
        &text.replace("epoch = 1", "epoch = '1'"),
        choice,
        MigrationRefusalKind::UnrecognizedShape,
    );
    refused(
        &text,
        selection(CURRENT_EPOCH1, CURRENT_ABSENT, IDENTITY),
        MigrationRefusalKind::IncompatibleEdge,
    );
}

#[test]
fn inline_and_normal_tables_keep_original_authored_syntax() {
    let text = "# inline\npackage = { name='demo', group='org.example', kind='flow', version='0.1.0', materialization='snapshot', bridge=false, frozen=true } # keep\nrequires = { packages = { 'flow:org.example/base' = { version='=1.0', link='static-transitive' }, 'org.example/other'='^0.2' } }\n";
    let plan = planned(text);
    assert_eq!(
        plan.after,
        text.replacen("materialization='snapshot'", "materialization='copy'", 1)
    );
    let normal = package("'snapshot'").replace(
        "'flow:org.example/base' = { version = '^1.0', link = 'static-hard' } # selector",
        "[requires.packages.'flow:org.example/base']\nversion = '^1.0'\nlink = 'dynamic'",
    );
    assert_eq!(
        planned(&normal).after,
        normal.replacen("'snapshot' # selected", "'copy' # selected", 1)
    );
}

#[test]
fn quote_families_escaped_decoding_and_initial_newlines_survive() {
    for (before, after) in [
        ("'snapshot'", "'copy'"),
        ("\"snapshot\"", "\"copy\""),
        ("\"\\u0073napshot\"", "\"copy\""),
        ("'''snapshot'''", "'''copy'''"),
        ("\"\"\"snapshot\"\"\"", "\"\"\"copy\"\"\""),
        ("'''\nsnapshot'''", "'''\ncopy'''"),
        ("\"\"\"\r\nsnapshot\"\"\"", "\"\"\"\r\ncopy\"\"\""),
        ("\"\"\"\\u0073napshot\"\"\"", "\"\"\"copy\"\"\""),
    ] {
        let text = package(before);
        let plan = planned(&text);
        assert_eq!(plan.edits[0].before_token, before);
        assert_eq!(plan.edits[0].after_token, after);
        assert_eq!(
            plan.after,
            text.replacen(
                &format!("{before} # selected"),
                &format!("{after} # selected"),
                1
            )
        );
    }
    refused(
        &package("'''snapshot\n'''"),
        legacy(),
        MigrationRefusalKind::UnrecognizedShape,
    );
}

#[test]
fn crlf_and_utf8_outside_token_are_untouched() {
    let text = package("'snapshot'").replace('\n', "\r\n");
    let plan = planned(&text);
    assert_eq!(
        plan.after,
        text.replacen("'snapshot' # selected", "'copy' # selected", 1)
    );
    assert!(plan.after.starts_with("# UTF-8 café\r\n"));
    assert!(plan.after.ends_with("# footer\r\n"));
}

#[test]
fn unknown_shapes_types_and_vocabularies_refuse() {
    let base = package("'snapshot'");
    for text in [
        base.replace("name = 'demo'", "name = 3"),
        base.replace("kind = 'flow'", "kind = 'doc'"),
        base.replace("bridge = true", "bridge = 'true'"),
        base.replace("frozen = false", "frozen = 0"),
        base.replace("description =", "unknown ="),
        base.replace("'snapshot' # selected", "3 # selected"),
        base.replace("'snapshot' # selected", "'unknown' # selected"),
        base.replace("[package]\n", "[package]\nauthors = [3]\n"),
        base.replace("[package]\n", "[package]\nepoch = 0\n"),
        base.replace("link = 'static-hard'", "link = 'inline'"),
        base.replace("version = '^1.0'", "version = { var = 'version' }"),
        base.replace("version = '^1.0'", "path = 'local', version = '^1.0'"),
        base.clone() + "\n[workspace]\nmembers = []\n",
        base.replace(
            "[requires.packages]",
            "[requires]\ncapabilities = []\n[requires.packages]",
        ),
    ] {
        refused(&text, legacy(), MigrationRefusalKind::UnrecognizedShape);
    }
}

#[test]
fn malformed_and_current_semantically_invalid_output_refuse() {
    refused("[package", legacy(), MigrationRefusalKind::MalformedSyntax);
    refused(
        &package("'snapshot'").replace("version = '1.1.0'", "version = 'broken'"),
        legacy(),
        MigrationRefusalKind::InvalidCurrentManifest,
    );
    refused(
        &package("'snapshot'").replace("'flow:org.example/base'", "'flow:bad group/base'"),
        legacy(),
        MigrationRefusalKind::InvalidCurrentManifest,
    );
}

#[test]
fn unknown_ids_edges_and_each_missing_choice_are_typed() {
    let text = package("'snapshot'");
    for index in 0..3 {
        let mut choice = legacy();
        match index {
            0 => choice.source_profile = Some("unknown-r2"),
            1 => choice.target_profile = Some("unknown-r2"),
            _ => choice.recipe = Some("unknown-r2"),
        }
        refused(&text, choice, MigrationRefusalKind::UnknownSelection);
        let mut choice = legacy();
        let field = match index {
            0 => {
                choice.source_profile = None;
                "source_profile"
            }
            1 => {
                choice.target_profile = None;
                "target_profile"
            }
            _ => {
                choice.recipe = None;
                "recipe"
            }
        };
        match plan_manifest_migration(&text, choice) {
            ManifestMigrationOutcome::NeedsInput(need) => {
                assert_eq!(need.len(), 1);
                assert_eq!(need[0].field, field);
                assert_eq!(
                    need[0].choices,
                    vec![[LEGACY_ABSENT, CURRENT_ABSENT, SNAPSHOT_TO_COPY][index]]
                );
            }
            other => panic!("expected input need, got {other:?}"),
        }
    }
    match plan_manifest_migration(&text, MigrationSelection::default()) {
        ManifestMigrationOutcome::NeedsInput(need) => {
            assert_eq!(need.len(), 3);
            assert_eq!(
                need[0].choices,
                vec![LEGACY_ABSENT, CURRENT_ABSENT, CURRENT_EPOCH1]
            );
            assert_eq!(need[2].choices, vec![SNAPSHOT_TO_COPY, IDENTITY]);
        }
        other => panic!("expected named choices, got {other:?}"),
    }
    refused(
        &text,
        selection(LEGACY_ABSENT, CURRENT_EPOCH1, SNAPSHOT_TO_COPY),
        MigrationRefusalKind::IncompatibleEdge,
    );
    refused(
        &text,
        selection(LEGACY_ABSENT, LEGACY_ABSENT, IDENTITY),
        MigrationRefusalKind::IncompatibleEdge,
    );
}

#[test]
fn retained_historical_scalar_is_not_relabelled_as_package_manifest() {
    // Exact 14 decoded bytes in immutable commit
    // d4d6c4754e5a07f60419e35f8d19d6adbd777352,
    // crates/vibe-core/src/manifest/package/tests.rs,
    // materialization_refuses_the_legacy_snapshot_name_with_the_recipe.
    // Source blob 2ff001d7c01d174191bc6c696879888e0d852d2f.
    let scalar = "v = \"snapshot\"";
    assert_eq!(scalar.len(), 14);
    assert_eq!(
        format!("{:x}", Sha256::digest(scalar.as_bytes())),
        "e312f6bc9eb19eba4d8938aed9cdbade86a7e1d38285d2bb9ccd0a9b6e6d71fc"
    );
    let reason = refused(scalar, legacy(), MigrationRefusalKind::UnrecognizedShape);
    assert_eq!(reason.field, "document.v");
}

#[test]
fn frozen_dependency_qualification_refuses_structural_unknowns() {
    for key in [
        "unqualified",
        "unknown:org.example/base",
        "doc:org.example/base",
        "flow:org.example/base/extra",
        "org.example//base",
        "/base",
        "org.example/",
        "org.example/base@1.0",
        "flow:tool:org.example/base",
        "flow:/base",
        "flow:org.example/",
        "org.example:base/other",
    ] {
        let text = package("'snapshot'").replace("'flow:org.example/base'", &format!("'{key}'"));
        let reason = refused(&text, legacy(), MigrationRefusalKind::UnrecognizedShape);
        assert_eq!(reason.field, format!("requires.packages[{key}]"));
    }
    for key in [
        "org.example/base",
        "flow:org.example/base",
        "feat:org.example/base",
        "stack:org.example/base",
        "tool:org.example/base",
        "mcp:org.example/base",
        "lang:org.example/base",
    ] {
        let text = package("'snapshot'").replace("'flow:org.example/base'", &format!("'{key}'"));
        let output = planned(&text);
        assert!(output.after.contains(&format!("'{key}'")));
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
fn independently_fixed_r1_identity_and_token_vector() {
    // Independently computed with Python hashlib, explicit bytes and the
    // source-defined u64be framing; no production hashing helper supplied IDs.
    let before = "[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"snapshot\"\n";
    let after = "[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"copy\"\n";
    let plan = planned(before);
    assert_eq!(plan.before, before);
    assert_eq!(plan.after, after);
    assert_eq!(
        plan.edits,
        vec![MigrationTokenEdit {
            field: "package.materialization",
            before_span: 86..96,
            before_token: "\"snapshot\"".to_owned(),
            after_token: "\"copy\"".to_owned(),
        }]
    );
    assert_eq!(
        plan.before_identity,
        "905cf0b4eb7376819f47c9f7da02fecb8920d91dd9b76b4cb3454da77fa81887"
    );
    assert_eq!(
        plan.after_identity,
        "6798d562a0ff899a5afd793054821fc6a114f8cd55fc53118d7772e9167a44df"
    );
    assert_eq!(
        plan.diff_identity,
        "42faf5376728e4649c80657e6b0f93e9910a107261413a5d0c7f019bf5ab27e0"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
fn frozen_profile_epoch_metadata_is_explicit_and_unknown_is_unrecognized() {
    assert_ne!(
        ManifestMigrationEpoch::Absent,
        ManifestMigrationEpoch::Epoch1
    );
    for profile in [LEGACY_ABSENT, CURRENT_ABSENT] {
        assert_eq!(
            manifest_migration_profile_epoch(profile),
            Some(ManifestMigrationEpoch::Absent)
        );
    }
    assert_eq!(
        manifest_migration_profile_epoch(CURRENT_EPOCH1),
        Some(ManifestMigrationEpoch::Epoch1)
    );
    for profile in [
        "",
        "unknown",
        "1.1.0",
        "epoch0",
        "materialization-current-absent-r2",
        "materialization-current-epoch1-r2",
    ] {
        assert_eq!(manifest_migration_profile_epoch(profile), None);
    }
}

#[test]
fn ordered_insertions_keep_untouched_utf8_comments_and_crlf_bytes() {
    let before = "α=1 # keep\r\nβ=2\r\n";
    let edits = [
        MigrationTokenEdit {
            field: "package.sources",
            before_span: 0..0,
            before_token: String::new(),
            after_token: "# source\r\n".into(),
        },
        MigrationTokenEdit {
            field: "package.sources",
            before_span: before.len()..before.len(),
            before_token: String::new(),
            after_token: "# end\r\n".into(),
        },
    ];
    assert_eq!(
        replay_edits(before, &edits).unwrap(),
        format!("# source\r\n{before}# end\r\n")
    );
    let duplicated = [edits[0].clone(), edits[0].clone()];
    assert!(replay_edits(before, &duplicated).is_err());
    let split_utf8 = [MigrationTokenEdit {
        field: "package.sources",
        before_span: 1..1,
        before_token: String::new(),
        after_token: "x".into(),
    }];
    assert!(replay_edits(before, &split_utf8).is_err());
}
