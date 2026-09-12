//! Officiality, computed from the edges and from nothing else.

use super::*;

fn subject(name: &str) -> Facts {
    Facts {
        group: "org.example".into(),
        name: name.into(),
        version: "1.0.0".into(),
        kind: "flow".into(),
        title: name.into(),
        lang: "en".into(),
        ..Facts::default()
    }
}

fn documentation(name: &str, documents: &str) -> Facts {
    Facts {
        group: "org.example".into(),
        name: name.into(),
        version: "0.1.0".into(),
        kind: "doc".into(),
        title: name.into(),
        lang: "en".into(),
        documents: vec![documents.into()],
        ..Facts::default()
    }
}

fn ranks(shelves: &Shelves, coordinate: &str) -> Vec<(String, Rank)> {
    shelves
        .documentation_of(coordinate)
        .iter()
        .map(|row| (row.coordinate.clone(), row.rank))
        .collect()
}

/// Both ends agree, so the documentation is official — and the one the
/// subject sent a reader to first is primary.
#[test]
fn documentation_the_subject_named_is_official_and_the_first_is_primary() {
    let mut wal = subject("wal");
    wal.documentation = Some((
        Some("org.example/wal-book".into()),
        vec!["org.example/wal-tutorials".into()],
    ));
    let shelves = fold(&[
        wal,
        documentation("wal-book", "org.example/wal"),
        documentation("wal-tutorials", "org.example/wal"),
        documentation("wal-notes", "org.example/wal"),
    ]);
    assert_eq!(
        ranks(&shelves, "org.example/wal"),
        vec![
            ("org.example/wal-book".to_string(), Rank::Primary),
            ("org.example/wal-tutorials".to_string(), Rank::Official),
            ("org.example/wal-notes".to_string(), Rank::Community),
        ]
    );
}

/// The default convention: a subject that says nothing still has primary
/// documentation, so nobody has to republish a package for the obvious
/// case.
#[test]
fn a_subject_that_declares_nothing_gets_the_naming_convention() {
    let shelves = fold(&[
        subject("wal"),
        documentation("wal-docs", "org.example/wal"),
        documentation("wal-notes", "org.example/wal"),
    ]);
    assert_eq!(
        ranks(&shelves, "org.example/wal"),
        vec![
            ("org.example/wal-docs".to_string(), Rank::Primary),
            ("org.example/wal-notes".to_string(), Rank::Community),
        ]
    );
}

/// A declared `[documentation]` replaces the convention entirely.
#[test]
fn a_declared_pointer_makes_the_conventional_name_community_like_any_other() {
    let mut wal = subject("wal");
    wal.documentation = Some((Some("org.example/wal-book".into()), Vec::new()));
    let shelves = fold(&[
        wal,
        documentation("wal-docs", "org.example/wal"),
        documentation("wal-book", "org.example/wal"),
    ]);
    assert_eq!(
        ranks(&shelves, "org.example/wal"),
        vec![
            ("org.example/wal-book".to_string(), Rank::Primary),
            ("org.example/wal-docs".to_string(), Rank::Community),
        ]
    );
}

/// Only an edge from ABOVE confirms. A package naming a subject the
/// catalog does not carry has said one thing, and one thing is community.
#[test]
fn documentation_of_a_subject_nobody_published_is_community() {
    let shelves = fold(&[documentation("mystery-notes", "org.other/mystery")]);
    assert_eq!(
        ranks(&shelves, "org.other/mystery"),
        vec![("org.example/mystery-notes".to_string(), Rank::Community)]
    );
}

/// A subject in another group cannot be claimed by the convention.
#[test]
fn the_convention_does_not_cross_a_group() {
    let mut foreign = documentation("wal-docs", "org.other/wal");
    foreign.group = "org.example".into();
    let shelves = fold(&[subject("wal"), foreign]);
    assert_eq!(
        ranks(&shelves, "org.other/wal"),
        vec![("org.example/wal-docs".to_string(), Rank::Community)]
    );
}

fn adaptation(name: &str, group: &str, lang: &str, source: &str) -> Facts {
    Facts {
        group: group.into(),
        name: name.into(),
        version: "0.1.0".into(),
        kind: "doc".into(),
        title: name.into(),
        lang: lang.into(),
        translates: Some(source.into()),
        ..Facts::default()
    }
}

/// Same group, conventional name: the author of the documentation named
/// it, which is what the mark on a translation means.
#[test]
fn an_adaptation_by_the_same_group_under_the_convention_is_official() {
    let shelves = fold(&[adaptation(
        "wal-docs-ru",
        "org.example",
        "ru",
        "org.example/wal-docs",
    )]);
    let rows = shelves.translations_of("org.example/wal-docs");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rank, Rank::Official);
    assert_eq!(rows[0].rank.star(), "★");
}

#[test]
fn an_adaptation_by_another_group_is_community_however_it_is_named() {
    let shelves = fold(&[adaptation(
        "wal-docs-ru",
        "org.volunteers",
        "ru",
        "org.example/wal-docs",
    )]);
    assert_eq!(
        shelves.translations_of("org.example/wal-docs")[0].rank,
        Rank::Community
    );
}

#[test]
fn an_adaptation_named_off_the_convention_is_community() {
    let shelves = fold(&[adaptation(
        "wal-russian",
        "org.example",
        "ru",
        "org.example/wal-docs",
    )]);
    assert_eq!(
        shelves.translations_of("org.example/wal-docs")[0].rank,
        Rank::Community
    );
}

/// The source keeps no list of its translations: the languages are the
/// `translates` edges, read at every render — and they arrive in the
/// shelf's own order, which puts the confirmed ones first. A selector
/// that listed a volunteer's adaptation above the author's own would
/// contradict the mark beside it.
#[test]
fn the_languages_of_a_documentation_are_its_adaptations_edges() {
    let shelves = fold(&[
        adaptation(
            "wal-docs-de",
            "org.volunteers",
            "de",
            "org.example/wal-docs",
        ),
        adaptation("wal-docs-ru", "org.example", "ru", "org.example/wal-docs"),
    ]);
    assert_eq!(
        shelves.languages_of("org.example/wal-docs", "en"),
        vec!["en".to_string(), "ru".to_string(), "de".to_string()]
    );
}

#[test]
fn a_dependant_is_the_other_end_of_a_requirement() {
    let mut consumer = subject("app");
    consumer.requires = vec!["org.example/wal".into()];
    let shelves = fold(&[subject("wal"), consumer]);
    let rows = shelves.dependants_of("org.example/wal");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].coordinate, "org.example/app");
}

/// A requirement carries a kind in front and a constraint behind, and a
/// shelf is keyed by neither.
///
/// The kind is the half a live run found missing: every requirement in
/// the published catalog carries one, and keeping it matched no
/// coordinate at all — an empty shelf rather than a wrong one, which is
/// the failure a unit test written from the norm alone would not have
/// caught.
#[test]
fn a_requirement_is_read_past_its_kind_and_its_constraint() {
    for spelled in [
        "lang:org.example/wal@=1.0.0",
        "flow:org.example/wal@^1.0.0",
        "org.example/wal@^1.0",
        "org.example/wal ^1.0",
        "org.example/wal",
    ] {
        assert_eq!(coordinate_of(spelled), "org.example/wal", "for `{spelled}`");
    }
}

/// The kind's separator is only a kind's separator before the slash.
#[test]
fn a_colon_after_the_slash_is_part_of_the_name() {
    assert_eq!(coordinate_of("org.example/wal:2"), "org.example/wal:2");
}

/// The whole point of the strip: an edge from a real catalog record has
/// to land on the shelf its coordinate names.
#[test]
fn a_requirement_as_the_published_catalog_spells_it_reaches_the_shelf() {
    let mut consumer = subject("go-ai-native");
    consumer.requires = vec![coordinate_of("lang:org.example/go-ai-native-lang@=1.0.0")];
    let shelves = fold(&[subject("go-ai-native-lang"), consumer]);
    let rows = shelves.dependants_of("org.example/go-ai-native-lang");
    assert_eq!(rows.len(), 1, "the kind prefix swallowed the edge");
    assert_eq!(rows[0].coordinate, "org.example/go-ai-native");
}

/// Three signals, and they have to agree: the mark, the word and the
/// order.
#[test]
fn the_mark_and_the_word_agree_with_the_order() {
    assert_eq!(Rank::Primary.star(), "★");
    assert_eq!(Rank::Official.star(), "★");
    assert_eq!(Rank::Community.star(), "");
    assert_eq!(Rank::Primary.word(), "primary");
    assert!(Rank::Primary < Rank::Official);
    assert!(Rank::Official < Rank::Community);
}

#[test]
fn a_coordinate_nobody_documents_has_an_empty_shelf() {
    let shelves = fold(&[subject("wal")]);
    assert!(shelves.documentation_of("org.example/wal").is_empty());
    assert!(shelves.translations_of("org.example/wal").is_empty());
    assert!(shelves.dependants_of("org.example/wal").is_empty());
    assert_eq!(
        shelves.languages_of("org.example/wal", "en"),
        vec!["en".to_string()]
    );
}
