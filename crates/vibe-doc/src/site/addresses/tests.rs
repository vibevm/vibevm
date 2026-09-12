//! What `latest` names, and what it refuses to name.

use super::*;

fn published(coordinate: &str, version: &str, lang: &str, pages: usize) -> Published {
    Published {
        coordinate: coordinate.into(),
        version: version.into(),
        lang: lang.into(),
        pages,
    }
}

fn alias(map: &Addresses, coordinate: &str, lang: &str) -> Option<String> {
    map.latest
        .get(&(coordinate.to_string(), lang.to_string()))
        .cloned()
}

/// The one ordering that puts `1.10.0` after `1.9.0`.
#[test]
fn latest_is_the_newest_by_semantic_version_and_not_by_string() {
    let map = of(&[
        published("org.example/wal", "1.9.0", "en", 1),
        published("org.example/wal", "1.10.0", "en", 1),
        published("org.example/wal", "1.2.0", "en", 1),
    ]);
    assert_eq!(
        alias(&map, "org.example/wal", "en").as_deref(),
        Some("1.10.0")
    );
}

/// An adaptation has its own version numbers and they have nothing to do
/// with the source's, so the alias is per language or the two would race.
#[test]
fn one_alias_per_language_and_never_one_race_between_them() {
    let map = of(&[
        published("org.example/wal-docs", "2.0.0", "en", 4),
        published("org.example/wal-docs-ru", "0.9.0", "ru", 4),
    ]);
    assert_eq!(
        alias(&map, "org.example/wal-docs", "en").as_deref(),
        Some("2.0.0")
    );
    assert_eq!(
        alias(&map, "org.example/wal-docs-ru", "ru").as_deref(),
        Some("0.9.0")
    );
    assert_eq!(map.latest.len(), 2);
}

/// Both spellings of every version, and the package page each of them
/// carries.
#[test]
fn the_count_carries_both_spellings_of_every_version() {
    let map = of(&[published("org.example/wal", "1.0.0", "en", 5)]);
    assert_eq!(map.addresses, 12);
}

/// Every version keeps its numbered address; only one may answer at
/// `latest`, and the build says so rather than picking silently.
#[test]
fn a_coordinate_at_two_versions_is_reported_and_not_resolved_by_picking() {
    let map = of(&[
        published("org.example/wal", "1.0.0", "en", 1),
        published("org.example/wal", "2.0.0", "en", 1),
        published("org.example/quiet", "1.0.0", "en", 1),
    ]);
    assert_eq!(map.contested.len(), 1);
    assert!(
        map.contested[0].contains("org.example/wal"),
        "{:?}",
        map.contested
    );
    assert!(
        map.contested[0].contains("1.0.0, 2.0.0"),
        "{:?}",
        map.contested
    );
    // The alias still names the newest — the collision is about which
    // tree the site writes it from, not about which version is newest.
    assert_eq!(
        alias(&map, "org.example/wal", "en").as_deref(),
        Some("2.0.0")
    );
}

#[test]
fn one_version_of_one_coordinate_contests_nothing() {
    let map = of(&[published("org.example/wal", "1.0.0", "en", 1)]);
    assert!(map.contested.is_empty());
}

/// A version number nobody can parse must not take the site down: the
/// alias is an address, and an address is not worth a panic.
#[test]
fn a_version_that_is_not_a_semantic_version_still_gets_an_alias() {
    let map = of(&[
        published("org.example/odd", "nightly", "en", 1),
        published("org.example/odd", "alpha", "en", 1),
    ]);
    assert_eq!(
        alias(&map, "org.example/odd", "en").as_deref(),
        Some("nightly")
    );
}

#[test]
fn the_languages_of_a_coordinate_are_the_ones_it_is_published_in() {
    let map = of(&[
        published("org.example/wal-docs", "1.0.0", "en", 1),
        published("org.example/wal-docs", "1.1.0", "en", 1),
    ]);
    assert_eq!(
        map.languages.get("org.example/wal-docs"),
        Some(&vec!["en".to_string()])
    );
}

#[test]
fn the_report_names_the_collision_and_the_totals() {
    let report = of(&[
        published("org.example/wal", "1.0.0", "en", 2),
        published("org.example/wal", "2.0.0", "en", 2),
    ])
    .render();
    assert!(report.contains("one coordinate, two versions"), "{report}");
    assert!(
        report.contains("addresses: 1 alias(es), 1 package(s), 1 language(s), 12 page address(es)"),
        "{report}"
    );
}

#[test]
fn a_build_with_nothing_published_has_no_addresses() {
    let map = of(&[]);
    assert!(map.latest.is_empty());
    assert_eq!(map.addresses, 0);
    assert!(map.render().contains("0 page address(es)"));
}
