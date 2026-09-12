//! The D-19 table, row by row: who confirms what, and what a package
//! that nobody confirmed is called.

use super::*;
use crate::citations::SpecSources;

/// A subject in a project-local registry, with whatever `[documentation]`
/// the test wants it to declare.
fn subject(tmp: &std::path::Path, group: &str, name: &str, documentation: &str) -> SpecSources {
    let dir = tmp
        .join("vibevm/vibepacks")
        .join(group)
        .join(name)
        .join("v1.0.0");
    std::fs::create_dir_all(&dir).expect("subject dir");
    std::fs::write(
        dir.join(MANIFEST),
        format!(
            "[package]\nname = \"{name}\"\ngroup = \"{group}\"\nkind = \"flow\"\n\
             version = \"1.0.0\"\nepoch = 1\nformat = \"normal\"\n{documentation}"
        ),
    )
    .expect("subject manifest");
    SpecSources::new().with_in_tree(tmp.join("vibevm/vibepacks"))
}

/// The default convention: a subject that declares nothing gets the
/// `<name>-docs` package of its own group as primary, so the obvious case
/// costs the subject no re-release.
#[test]
fn the_naming_convention_makes_name_docs_primary() {
    let tmp = tempfile::tempdir().expect("temp");
    let world = subject(tmp.path(), "org.demo", "lib", "");
    assert_eq!(
        documentation_status("org.demo/lib", "org.demo", "lib-docs", &world),
        DocumentationStatus::Primary
    );
}

/// The convention is about the subject's OWN group. Only the group's
/// owner can publish into it, which is why officiality cannot be forged
/// through a name.
#[test]
fn the_convention_does_not_cross_groups() {
    let tmp = tempfile::tempdir().expect("temp");
    let world = subject(tmp.path(), "org.demo", "lib", "");
    assert_eq!(
        documentation_status("org.demo/lib", "com.example", "lib-docs", &world),
        DocumentationStatus::Community
    );
}

/// A declared `[documentation]` replaces the convention ENTIRELY — which
/// is the point of declaring one. The conventionally named package is
/// community here, because the subject did not name it.
#[test]
fn a_declared_pointer_replaces_the_convention() {
    let tmp = tempfile::tempdir().expect("temp");
    let world = subject(
        tmp.path(),
        "org.demo",
        "lib",
        "[documentation]\nprimary = \"org.demo/handbook\"\nofficial = [\"org.demo/cookbook\"]\n",
    );
    assert_eq!(
        documentation_status("org.demo/lib", "org.demo", "handbook", &world),
        DocumentationStatus::Primary
    );
    assert_eq!(
        documentation_status("org.demo/lib", "org.demo", "cookbook", &world),
        DocumentationStatus::Official
    );
    assert_eq!(
        documentation_status("org.demo/lib", "org.demo", "lib-docs", &world),
        DocumentationStatus::Community
    );
}

/// The convention is a rule about names, so it holds when the subject is
/// out of reach as well — which is what lets a manual warmed into a
/// reader's store carry the same star the site gives it. A subject that
/// cannot be heard and a subject that has not spoken end in one place.
#[test]
fn a_subject_no_source_holds_still_falls_to_the_convention() {
    assert_eq!(
        documentation_status("org.demo/lib", "org.demo", "lib-docs", &SpecSources::new()),
        DocumentationStatus::Primary
    );
    assert_eq!(
        documentation_status(
            "org.demo/lib",
            "com.example",
            "handbook",
            &SpecSources::new()
        ),
        DocumentationStatus::Community
    );
}

/// An official translation is the source's own group publishing
/// `<docname>-<lang>`; the language tag is compared case-insensitively,
/// because BCP-47 is and the name convention lower-cases it.
#[test]
fn an_official_translation_is_the_sources_group_and_the_conventional_name() {
    assert_eq!(
        translation_status("org.demo/lib-docs", "org.demo", "lib-docs-pt-br", "pt-BR"),
        TranslationStatus::Official
    );
    assert_eq!(
        translation_status("org.demo/lib-docs", "org.demo", "lib-docs-brazil", "pt-BR"),
        TranslationStatus::Community
    );
    assert_eq!(
        translation_status("org.demo/lib-docs", "com.example", "lib-docs-ru", "ru"),
        TranslationStatus::Community
    );
}

/// A package documenting several subjects can stand differently for each.
/// The card needs one value, and the strongest is the honest one.
#[test]
fn the_cards_status_is_the_strongest_over_the_subjects() {
    let subjects = vec![
        DocumentedSubject {
            package: "org.demo/a".to_owned(),
            version: "^1".to_owned(),
            status: DocumentationStatus::Community,
        },
        DocumentedSubject {
            package: "org.demo/b".to_owned(),
            version: "^1".to_owned(),
            status: DocumentationStatus::Primary,
        },
    ];
    assert_eq!(strongest(&subjects), DocumentationStatus::Primary);
    assert_eq!(strongest(&[]), DocumentationStatus::Community);
}

/// A coordinate with no `/` in it addresses no package, so no edge can
/// converge on it.
#[test]
fn a_malformed_coordinate_converges_on_nothing() {
    assert_eq!(
        documentation_status(
            "not-a-coordinate",
            "org.demo",
            "lib-docs",
            &SpecSources::new()
        ),
        DocumentationStatus::Community
    );
    assert_eq!(
        translation_status("not-a-coordinate", "org.demo", "lib-docs-ru", "ru"),
        TranslationStatus::Community
    );
}
