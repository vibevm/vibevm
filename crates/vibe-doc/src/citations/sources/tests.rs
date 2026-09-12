//! The source chain: which of the four answers, and in which order.

use std::fs;
use std::path::Path;

use super::*;

/// A package tree with one spec document and a manifest.
fn package(root: &Path, group: &str, name: &str, version: &str, lang: Option<&str>, body: &str) {
    let specs = root.join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    let i18n = lang
        .map(|l| format!("\n[i18n]\ncanonical = \"{l}\"\n"))
        .unwrap_or_default();
    fs::write(
        root.join("vibe.toml"),
        format!(
            "[package]\nname = \"{name}\"\ngroup = \"{group}\"\nkind = \"flow\"\n\
             version = \"{version}\"\n{i18n}"
        ),
    )
    .unwrap();
    fs::write(
        specs.join("PROP-001-thing.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">PROP-001</title>\n  \
               <p><A-RULE fact=\"true\" status=\"spec/done\">{body}</A-RULE></p>\n\
             </spec>\n"
        ),
    )
    .unwrap();
}

fn address(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).expect("address parses")
}

/// The in-tree registry answers before the store: a manual documenting a
/// package that lives beside it must read the tree, not a published
/// version somebody warmed months ago.
#[test]
fn the_chain_answers_nearest_first() {
    let tmp = tempfile::tempdir().unwrap();
    let packages = tmp.path().join("repo/vibevm/vibepacks");
    let store_root = tmp.path().join("store");
    package(
        &packages.join("org.acme/lib/v1.0.0"),
        "org.acme",
        "lib",
        "1.0.0",
        None,
        "in the tree",
    );
    package(
        &store_root.join("org.acme/lib/v1.0.0"),
        "org.acme",
        "lib",
        "1.0.0",
        None,
        "in the store",
    );

    let world = SpecSources::new()
        .with_in_tree(&packages)
        .with_store(&store_root);
    let found = world
        .locate(&address("spec://org.acme/lib/common/PROP-001#A-RULE"))
        .unwrap();
    assert_eq!(found.source, Source::InTree);

    // With the in-tree half removed the store answers the same address.
    let store_only = SpecSources::new().with_store(&store_root);
    let found = store_only
        .locate(&address("spec://org.acme/lib/common/PROP-001#A-RULE"))
        .unwrap();
    assert_eq!(found.source, Source::Store);
}

/// Several versions in one directory: the freshest is the one a bare
/// address means.
#[test]
fn a_bare_address_takes_the_freshest_version_present() {
    let tmp = tempfile::tempdir().unwrap();
    let packages = tmp.path().join("packs");
    for v in ["1.0.0", "1.2.0", "0.9.0"] {
        package(
            &packages.join(format!("org.acme/lib/v{v}")),
            "org.acme",
            "lib",
            v,
            None,
            "text",
        );
    }
    let world = SpecSources::new().with_in_tree(&packages);
    let instance = world.instance_of("org.acme", "lib").expect("found");
    assert_eq!(instance.version, "1.2.0");
}

/// The checkout answers its OWN coordinate and nothing else — a project
/// is not a registry of other people's packages.
#[test]
fn the_checkout_answers_only_its_own_coordinate() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    package(&repo, "org.acme", "host", "1.0.0", None, "the host rule");
    let world = SpecSources::for_checkout(&repo, Some("org.acme"), "host");
    assert_eq!(
        world
            .locate(&address("spec://org.acme/host/common/PROP-001#A-RULE"))
            .unwrap()
            .source,
        Source::Checkout
    );
    let err = world
        .locate(&address("spec://org.acme/other/common/PROP-001#A-RULE"))
        .expect_err("refused");
    assert!(err.contains("no source holds `org.acme/other`"), "{err}");
    assert!(err.contains("checkout `org.acme/host`"), "{err}");
}

/// A pin on the address is CHECKED against the instance the chain found,
/// never dropped: an author who wrote `@2.0.0` and silently got 1.0.0's
/// bytes has no way to notice.
#[test]
fn a_version_pin_that_disagrees_with_the_chain_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let packages = tmp.path().join("packs");
    package(
        &packages.join("org.acme/lib/v1.0.0"),
        "org.acme",
        "lib",
        "1.0.0",
        None,
        "text",
    );
    let world = SpecSources::new().with_in_tree(&packages);
    world
        .locate(&address("spec://org.acme/lib@1.0.0/common/PROP-001#A-RULE"))
        .expect("the agreeing pin passes");
    let err = world
        .locate(&address("spec://org.acme/lib@2.0.0/common/PROP-001#A-RULE"))
        .expect_err("refused");
    assert!(err.contains("2.0.0") && err.contains("1.0.0"), "{err}");
}

/// The cited spec's language is the CITED package's, not the citing
/// page's — a Russian manual quotes an English rule in English.
#[test]
fn the_language_comes_from_the_cited_packages_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let packages = tmp.path().join("packs");
    package(
        &packages.join("org.acme/lib/v1.0.0"),
        "org.acme",
        "lib",
        "1.0.0",
        Some("de"),
        "Die Regel.",
    );
    let world = SpecSources::new().with_in_tree(&packages);
    let found = world
        .locate(&address("spec://org.acme/lib/common/PROP-001#A-RULE"))
        .unwrap();
    assert_eq!(found.lang, "de");
}

/// An undotted authority names no package and says so, instead of
/// falling through to «not installed».
#[test]
fn an_undotted_authority_is_refused_by_name() {
    let world = SpecSources::new();
    let err = world
        .locate(&address("spec://vibevm/common/PROP-001#A-RULE"))
        .expect_err("refused");
    assert!(err.contains("names no package"), "{err}");
}

/// A directory that does not spell a version is not a candidate: a stray
/// folder beside the versions must not be mistaken for one.
#[test]
fn a_directory_that_is_not_a_version_is_not_a_candidate() {
    let tmp = tempfile::tempdir().unwrap();
    let packages = tmp.path().join("packs");
    fs::create_dir_all(packages.join("org.acme/lib/notes")).unwrap();
    let world = SpecSources::new().with_in_tree(&packages);
    assert!(world.instance_of("org.acme", "lib").is_none());
}
