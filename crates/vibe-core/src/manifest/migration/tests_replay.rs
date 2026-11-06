//! Independent byte-level cases for the reusable replay boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");
use super::*;

fn edit(span: Range<usize>, before: &str, after: &str) -> MigrationTokenEdit {
    MigrationTokenEdit {
        field: "change.schema",
        before_span: span,
        before_token: before.into(),
        after_token: after.into(),
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
fn exact_replay_combines_replacement_deletion_and_distinct_insertions() {
    let text = "α=1 # keep\r\nβ=2\r\n";
    let beta = text.find('β').unwrap();
    let edits = [
        edit(0..0, "", "# start\r\n"),
        edit(3..4, "1", "10"),
        edit(beta..text.len(), &text[beta..], ""),
        edit(text.len()..text.len(), "", "# end\r\n"),
    ];
    assert_eq!(
        replay_edits(text, &edits).unwrap(),
        "# start\r\nα=10 # keep\r\n# end\r\n"
    );
    assert_eq!(text, "α=1 # keep\r\nβ=2\r\n");
    assert_eq!(replay_edits(text, &[]).unwrap(), text);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
fn invalid_replay_refuses_and_names_the_actual_edit_owner() {
    let text = "α=1 # keep\r\n";
    for edits in [
        vec![edit(3..4, "2", "9")],                      // stale before token
        vec![edit(Range { start: 4, end: 3 }, "", "9")], // reversed range
        vec![edit(0..text.len() + 1, "", "9")],          // beyond input
        vec![edit(text.len() + 1..text.len() + 1, "", "9")],
        vec![edit(1..2, "", "9")], // split UTF-8
        vec![edit(0..1, "", "9")],
        vec![edit(0..0, "", "")], // empty insertion
        vec![edit(0..0, "", "x"), edit(0..0, "", "y")],
        vec![edit(3..4, "1", "9"), edit(2..3, "=", "-")], // out of order
        vec![edit(0..3, "α=", "x"), edit(2..4, "=1", "y")], // overlap
        vec![edit(0..0, "", "x"), edit(0..2, "α", "y")],  // same insertion position
    ] {
        let error = replay_edits(text, &edits).unwrap_err();
        assert_eq!(error.kind, MigrationRefusalKind::UnprovableEdit);
        assert_eq!(error.field, "change.schema");
        assert!(!error.reason.contains(text));
        assert_eq!(text, "α=1 # keep\r\n");
    }
}

#[test]
fn missing_selection_is_not_silently_inferred_even_for_malformed_text() {
    assert!(matches!(
        plan_manifest_migration("[package", MigrationSelection::default()),
        ManifestMigrationOutcome::NeedsInput(_)
    ));
    let selection = MigrationSelection {
        source_profile: Some(LEGACY_ABSENT),
        target_profile: Some(CURRENT_ABSENT),
        recipe: Some(SNAPSHOT_TO_COPY),
        source_decisions: &[],
    };
    let ManifestMigrationOutcome::Refused(error) = plan_manifest_migration("[package", selection)
    else {
        panic!("complete selection must validate source")
    };
    assert_eq!(error.kind, MigrationRefusalKind::MalformedSyntax);
}
