//! The generic row law, judged by counterexample.

use super::{MAX_ARTIFACTS, validate_shape};
use vibe_wire::generated::lifecycle::e1::context::Artifact;
use vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact;

/// An absolute root in the platform's own spelling: on Windows a leading
/// slash is root-relative, not absolute, and artifact rows are absolute.
fn root() -> &'static str {
    if cfg!(windows) {
        "C:/abs/project"
    } else {
        "/abs/project"
    }
}

fn row(id: &str, relative: &str) -> ReplyArtifact {
    ReplyArtifact {
        id: id.into(),
        kind: "file".into(),
        path: format!("{}/{relative}", root()),
    }
}

fn prior(id: &str, relative: &str) -> Artifact {
    Artifact {
        id: id.into(),
        kind: "file".into(),
        path: format!("{}/{relative}", root()),
        phase: "build".into(),
    }
}

#[test]
fn a_well_formed_set_passes() {
    let rows = vec![row("docs/a.md", "docs/a.md"), row("docs/b.md", "docs/b.md")];
    assert_eq!(validate_shape(&rows, &[], root()), Ok(()));
}

#[test]
fn one_row_past_the_cap_refuses() {
    let rows: Vec<_> = (0..=MAX_ARTIFACTS)
        .map(|index| row(&format!("docs/{index}.md"), &format!("docs/{index}.md")))
        .collect();
    assert_eq!(rows.len(), MAX_ARTIFACTS + 1);
    let error = validate_shape(&rows, &[], root()).unwrap_err();
    assert!(error.contains("exceed the epoch-1 maximum"), "{error}");
    // And the cap itself is accepted, so the refusal is a boundary, not a mood.
    assert_eq!(validate_shape(&rows[..MAX_ARTIFACTS], &[], root()), Ok(()));
}

#[test]
fn an_over_long_id_refuses_and_the_diagnostic_is_bounded() {
    let long = "x".repeat(257);
    let rows = vec![row(&long, "docs/a.md")];
    let error = validate_shape(&rows, &[], root()).unwrap_err();
    assert!(error.contains("257 bytes"), "{error}");
    assert!(error.contains("… (truncated)"), "{error}");
    assert!(error.len() < 512, "diagnostic must stay bounded: {error}");
}

#[test]
fn a_collision_with_a_prior_phase_artifact_refuses() {
    let rows = vec![row("docs/a.md", "docs/a.md")];
    let by_id = validate_shape(&rows, &[prior("docs/a.md", "other.md")], root()).unwrap_err();
    assert!(
        by_id.contains("already produced in phase `build`"),
        "{by_id}"
    );
    let by_path = validate_shape(&rows, &[prior("other", "docs/a.md")], root()).unwrap_err();
    assert!(
        by_path.contains("already produced in phase `build`"),
        "{by_path}"
    );
}

#[test]
fn duplicate_ids_and_paths_within_one_set_refuse() {
    let same_id = vec![row("dup", "docs/a.md"), row("dup", "docs/b.md")];
    assert!(
        validate_shape(&same_id, &[], root())
            .unwrap_err()
            .contains("declared more than once")
    );
    let same_path = vec![row("a", "docs/a.md"), row("b", "docs/a.md")];
    assert!(
        validate_shape(&same_path, &[], root())
            .unwrap_err()
            .contains("already declares")
    );
}

/// Path collisions are physical identity, not string equality: on the volumes
/// this ships to these pairs are one file, and a raw comparison would let the
/// second row silently destroy the first — after the provider was paid.
#[test]
fn paths_that_are_one_physical_file_refuse_however_they_are_spelled() {
    for (label, left, right) in [
        ("case fold", "docs/a.md", "Docs/A.md"),
        ("nfc vs nfd", "docs/\u{e9}.md", "docs/e\u{301}.md"),
        ("sharp s", "docs/Ma\u{df}e.md", "docs/MASSE.MD"),
        ("final sigma", "docs/\u{3c2}.md", "docs/\u{3a3}.md"),
    ] {
        let planned = vec![row("first", left), row("second", right)];
        assert!(
            validate_shape(&planned, &[], root())
                .unwrap_err()
                .contains("already declares"),
            "`{label}` must refuse as one file",
        );
        // And against a prior phase's artifact, where only the shared law can
        // see the collision at all.
        let single = vec![row("first", left)];
        assert!(
            validate_shape(&single, &[prior("earlier", right)], root())
                .unwrap_err()
                .contains("already produced in phase `build`"),
            "`{label}` must refuse against a prior artifact",
        );
    }
}

/// The stored spelling is untouched: identity is a comparison key, never a
/// rewrite of what the artifact row records.
#[test]
fn identity_comparison_does_not_rewrite_the_stored_spelling() {
    let planned = vec![row("Docs/A.md", "Docs/A.md")];
    assert_eq!(validate_shape(&planned, &[], root()), Ok(()));
    assert_eq!(planned[0].id, "Docs/A.md");
    assert_eq!(planned[0].path, format!("{}/Docs/A.md", root()));
}

#[test]
fn a_path_outside_the_project_or_of_the_wrong_shape_refuses() {
    for (label, path) in [
        ("relative", "docs/a.md".to_string()),
        ("backslash", format!("{}/docs\\a.md", root())),
        (
            "outside",
            format!("{}/elsewhere/a.md", if cfg!(windows) { "D:" } else { "" }),
        ),
        ("root itself", root().to_string()),
        ("prefix lookalike", format!("{}-evil/a.md", root())),
    ] {
        let rows = vec![ReplyArtifact {
            id: "a".into(),
            kind: "file".into(),
            path,
        }];
        assert!(
            validate_shape(&rows, &[], root()).is_err(),
            "`{label}` must refuse"
        );
    }
}

#[test]
fn an_empty_or_over_long_kind_refuses() {
    for kind in [String::new(), "k".repeat(129)] {
        let rows = vec![ReplyArtifact {
            id: "a".into(),
            kind,
            path: format!("{}/docs/a.md", root()),
        }];
        assert!(validate_shape(&rows, &[], root()).is_err());
    }
}
