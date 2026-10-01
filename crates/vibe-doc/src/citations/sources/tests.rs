//! The source chain: which of the four answers, and in which order.

use std::fs;
use std::path::Path;

use super::*;
use vibe_core::manifest::SpecFormat;

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

fn declared_checkout() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let spec_root = root.path().join(vibe_core::layout::current_specs_root());
    let common = spec_root.join("common");
    fs::create_dir_all(&common).unwrap();
    let selected = common.join("guide.xml");
    let raw_markdown = common.join("guide.md");
    let raw_data = common.join("data.xml");
    fs::write(
        &selected,
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"spec/done\">chosen</RULE></p></spec>\n",
    )
    .unwrap();
    fs::write(&raw_markdown, b"# Ordinary CommonMark\n").unwrap();
    fs::write(&raw_data, b"<data>ordinary XML</data>\n").unwrap();
    let slash = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let spec_root = vibe_core::layout::current_specs_root();
    fs::write(
        root.path().join("vibe.toml"),
        format!(
            "[project]\ngroup='org.acme'\nname='host'\nversion='1.1.0'\n\
             [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
             [[project.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
             [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
            slash(&spec_root.join("common/guide.xml")),
            slash(&spec_root.join("common/guide.md")),
            slash(&spec_root.join("common/data.xml")),
        ),
    )
    .unwrap();
    (root, selected, raw_markdown)
}

#[test]
fn opt_in_checkout_citation_selects_vibe_xml_beside_raw_same_stem() {
    let (root, selected, raw_markdown) = declared_checkout();
    let world = SpecSources::for_checkout(root.path(), Some("org.acme"), "host");
    let found = world
        .locate(&address("spec://org.acme/host/common/guide#RULE"))
        .unwrap();
    assert_eq!(found.source, Source::Checkout);
    assert_eq!(found.path, selected);
    assert_eq!(fs::read(raw_markdown).unwrap(), b"# Ordinary CommonMark\n");
}

#[test]
fn selected_checkout_citation_keeps_captured_bytes_after_source_changes() {
    let (root, selected, _) = declared_checkout();
    let world = SpecSources::for_checkout(root.path(), Some("org.acme"), "host");
    let located = world
        .locate(&address("spec://org.acme/host/common/guide#RULE"))
        .unwrap();
    fs::write(
        &selected,
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"spec/done\">changed later</RULE></p></spec>\n",
    )
    .unwrap();
    let doc = crate::citations::anchor::read_spec_selected(
        &located.path,
        located.captured.as_ref().unwrap(),
    )
    .unwrap();
    assert_eq!(
        crate::citations::anchor::anchor_text(&doc, &["RULE".into()]),
        Some("chosen".into())
    );
}

#[test]
fn opt_in_package_checkout_uses_the_same_selected_source_rule() {
    let (root, selected, _) = declared_checkout();
    let manifest_path = root.path().join("vibe.toml");
    let project_manifest = fs::read_to_string(&manifest_path).unwrap();
    let package_manifest = project_manifest
        .replace(
            "[project]\ngroup='org.acme'\nname='host'\nversion='1.1.0'",
            "[package]\ngroup='org.acme'\nname='host'\nkind='flow'\nversion='1.1.0'",
        )
        .replace("[[project.sources]]", "[[package.sources]]");
    fs::write(&manifest_path, package_manifest).unwrap();
    let world = SpecSources::for_checkout(root.path(), Some("org.acme"), "host");
    assert_eq!(
        world
            .locate(&address("spec://org.acme/host/common/guide#RULE"))
            .unwrap()
            .path,
        selected
    );
}

#[test]
fn opt_in_checkout_citation_refuses_raw_and_changed_source() {
    let (root, selected, _) = declared_checkout();
    let world = SpecSources::for_checkout(root.path(), Some("org.acme"), "host");
    let raw = world
        .locate(&address("spec://org.acme/host/common/data#anything"))
        .unwrap_err();
    assert!(raw.contains("no declared scan-loaded Vibe source"), "{raw}");
    fs::write(&selected, [0xff, 0xfe]).unwrap();
    let changed = world
        .locate(&address("spec://org.acme/host/common/guide#RULE"))
        .unwrap_err();
    assert!(changed.contains("not UTF-8"), "{changed}");
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

fn descriptor_lock_world() -> (tempfile::TempDir, PathBuf, String) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.host'\nname='reader'\nversion='1.1.0'\n",
    )
    .unwrap();
    let source = tempfile::tempdir().unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let guide_xml = specs.join("common/guide.xml");
    let guide_md = specs.join("common/guide.md");
    let data_xml = specs.join("common/data.xml");
    for (relative, bytes) in [
        (&guide_xml, b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"spec/done\">selected XML</RULE></p></spec>\n".as_slice()),
        (&guide_md, b"# Ordinary\n\n@fact:FAKE literal CommonMark.\n".as_slice()),
        (&data_xml, b"<data>raw XML</data>\n".as_slice()),
    ] {
        let path = source.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let slash = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\ngroup='org.acme'\nname='lib'\nkind='flow'\nversion='1.1.0'\nformat='normal'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
        slash(&guide_xml),
        slash(&guide_md),
        slash(&data_xml),
    );
    fs::write(source.path().join("vibe.toml"), manifest).unwrap();
    let group = Group::parse("org.acme").unwrap();
    let version: semver::Version = "1.1.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "lib",
        &version,
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "lib", &version);
    let lock = format!(
        "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-01T00:00:00Z'\nschema_version={}\n\
         [[package]]\nkind='flow'\nname='lib'\ngroup='org.acme'\nversion='1.1.0'\n\
         source_url='file:///fixture'\ncontent_hash='{}'\n",
        vibe_core::manifest::CURRENT_SCHEMA_VERSION,
        hash.as_str(),
    );
    fs::write(root.path().join("vibe.lock"), &lock).unwrap();
    (root, slot, lock)
}

#[test]
fn lock_citation_selects_vibe_xml_beside_raw_same_stem_resources() {
    let (root, slot, _) = descriptor_lock_world();
    let world = SpecSources::new().with_lock(root.path());
    let found = world
        .locate(&address("spec://org.acme/lib/common/guide#RULE"))
        .unwrap();
    assert_eq!(found.source, Source::Lock);
    assert_eq!(
        found.path,
        slot.join(vibe_core::layout::current_specs_root())
            .join("common/guide.xml")
    );
    assert_eq!(
        fs::read(
            slot.join(vibe_core::layout::current_specs_root())
                .join("common/guide.md")
        )
        .unwrap(),
        b"# Ordinary\n\n@fact:FAKE literal CommonMark.\n"
    );
}

#[test]
fn lock_citation_refuses_foreign_hash_or_changed_selected_output() {
    let (root, slot, lock) = descriptor_lock_world();
    let world = SpecSources::new().with_lock(root.path());
    let cite = address("spec://org.acme/lib/common/guide#RULE");
    fs::write(
        root.path().join("vibe.lock"),
        lock.replace(&"1".repeat(64), &"2".repeat(64)),
    )
    .unwrap();
    let foreign = world.locate(&cite).unwrap_err();
    assert!(foreign.contains("source hash"), "{foreign}");
    fs::write(root.path().join("vibe.lock"), lock).unwrap();
    fs::write(
        slot.join(vibe_core::layout::current_specs_root())
            .join("common/guide.xml"),
        b"changed XML\n",
    )
    .unwrap();
    let changed = world.locate(&cite).unwrap_err();
    assert!(changed.contains("changed bytes"), "{changed}");
}

#[test]
fn in_tree_and_store_declared_sources_refuse_unwitnessed_suffix_lookup() {
    let (root, slot, _) = descriptor_lock_world();
    let source = root.path().join("packs/org.acme/lib/v1.1.0");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::rename(&slot, &source).unwrap();
    let cite = address("spec://org.acme/lib/common/guide#RULE");
    for world in [
        SpecSources::new().with_in_tree(root.path().join("packs")),
        SpecSources::new().with_store(root.path().join("packs")),
    ] {
        let error = world.locate(&cite).unwrap_err();
        assert!(error.contains("descriptor-owned"), "{error}");
    }
}

#[test]
fn lock_citation_cannot_downgrade_after_manifest_rows_or_record_disappear() {
    let (root, slot, _) = descriptor_lock_world();
    let world = SpecSources::new().with_lock(root.path());
    let cite = address("spec://org.acme/lib/common/guide#RULE");
    let manifest = fs::read_to_string(slot.join("vibe.toml")).unwrap();
    let stripped = manifest.split("[[package.sources]]").next().unwrap();
    fs::write(slot.join("vibe.toml"), stripped).unwrap();
    let altered = world.locate(&cite).unwrap_err();
    assert!(altered.contains("manifest bytes differ"), "{altered}");
    fs::remove_file(slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME)).unwrap();
    let missing = world.locate(&cite).unwrap_err();
    assert!(missing.contains("no retained slot record"), "{missing}");
}

#[test]
fn no_descriptor_copied_lock_slot_keeps_legacy_lookup() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='reader'\nversion='1.1.0'\n",
    )
    .unwrap();
    let source = tempfile::tempdir().unwrap();
    package(
        source.path(),
        "org.acme",
        "lib",
        "1.0.1",
        None,
        "legacy rule",
    );
    let group = Group::parse("org.acme").unwrap();
    let version: semver::Version = "1.0.1".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "3".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "lib",
        &version,
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    fs::write(
        root.path().join("vibe.lock"),
        format!(
            "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-01T00:00:00Z'\nschema_version={}\n\
         [[package]]\nkind='flow'\nname='lib'\ngroup='org.acme'\nversion='1.0.1'\n\
         source_url='file:///fixture'\ncontent_hash='{}'\n",
            vibe_core::manifest::CURRENT_SCHEMA_VERSION,
            hash.as_str(),
        ),
    )
    .unwrap();
    let world = SpecSources::new().with_lock(root.path());
    let found = world
        .locate(&address("spec://org.acme/lib/common/PROP-001#A-RULE"))
        .unwrap();
    assert_eq!(found.source, Source::Lock);
    assert!(found.path.ends_with("PROP-001-thing.xml"));
}
