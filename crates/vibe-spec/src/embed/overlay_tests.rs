//! Exact-path overlay REDs for [`FsSectionSource`].

use super::*;
use std::fs;

fn coordinate() -> crate::SelfCoordinate {
    crate::SelfCoordinate::new(Some("org.demo".into()), "host".into())
}

fn source_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("temporary source root")
}

fn document(root: &Path, name: &str, text: &[u8]) -> PathBuf {
    let directory = crate::resolver::specs_root_under(root).join("common");
    fs::create_dir_all(&directory).expect("spec directory");
    let path = directory.join(name);
    fs::write(&path, text).expect("spec document");
    path
}

fn resolver(root: &Path) -> FileResolver {
    FileResolver::new(root, coordinate())
}

fn address(anchor: &str) -> SpecAddress {
    SpecAddress::parse(&format!("spec://org.demo/host/common/DEP#{anchor}")).expect("spec address")
}

fn overlay(path: PathBuf, bytes: impl Into<Arc<[u8]>>) -> BTreeMap<PathBuf, Arc<[u8]>> {
    BTreeMap::from([(path, bytes.into())])
}

#[test]
fn exact_overlay_path_beats_different_disk_bytes() {
    let root = source_root();
    let path = document(root.path(), "DEP.md", b"# Disk {#wanted}\nDISK\n");
    let source = FsSectionSource::with_overlay(
        resolver(root.path()),
        overlay(
            path,
            Arc::<[u8]>::from(&b"# Overlay {#wanted}\nOVERLAY\n"[..]),
        ),
    );

    let text = source
        .section_text(&address("wanted"))
        .expect("overlay text");

    assert!(text.contains("OVERLAY"), "{text}");
    assert!(!text.contains("DISK"), "{text}");
}

#[test]
fn absent_and_near_miss_overlay_paths_fall_back_exactly() {
    let root = source_root();
    let path = document(root.path(), "DEP.md", b"# Disk {#wanted}\nDISK\n");
    let ordinary = FsSectionSource::new(resolver(root.path()));
    let empty = FsSectionSource::with_overlay(resolver(root.path()), BTreeMap::new());
    let near = FsSectionSource::with_overlay(
        resolver(root.path()),
        overlay(
            path.with_file_name("dep.md"),
            Arc::<[u8]>::from(&b"# Wrong {#wanted}\nWRONG\n"[..]),
        ),
    );

    let expected = ordinary.section_text(&address("wanted"));
    assert_eq!(empty.section_text(&address("wanted")), expected);
    assert_eq!(near.section_text(&address("wanted")), expected);

    let missing = address("missing");
    assert_eq!(
        empty.section_text(&missing),
        ordinary.section_text(&missing),
        "empty overlay preserves the ordinary anchor error byte-for-byte"
    );
    let missing_document =
        SpecAddress::parse("spec://org.demo/host/common/MISSING#wanted").expect("missing address");
    assert_eq!(
        empty.section_text(&missing_document),
        ordinary.section_text(&missing_document),
        "empty overlay preserves the ordinary resolver error byte-for-byte"
    );
}

#[test]
fn xml_overlay_uses_the_canonical_projection_before_anchor_lookup() {
    let root = source_root();
    let path = document(
        root.path(),
        "DEP.xml",
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"disk\">Disk</title></spec>\n",
    );
    let xml = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <title id=\"dep\">Overlay</title>\n",
        "  <section id=\"wanted\" title=\"Wanted\"><p>XML OVERLAY</p></section>\n",
        "</spec>\n"
    );
    let source = FsSectionSource::with_overlay(
        resolver(root.path()),
        overlay(path, Arc::<[u8]>::from(xml.as_bytes())),
    );

    let text = source
        .section_text(&address("wanted"))
        .expect("projected XML");

    assert!(text.contains("## Wanted {#wanted}"), "{text}");
    assert!(text.contains("XML OVERLAY"), "{text}");
}

#[test]
fn overlay_and_filesystem_share_exact_anchor_candidate_errors() {
    let root = source_root();
    let body = b"# Dep {#dep}\n\n## Qualified {#origin--short}\nBODY\n";
    let path = document(root.path(), "DEP.md", body);
    let ordinary = FsSectionSource::new(resolver(root.path()));
    let overlaid = FsSectionSource::with_overlay(
        resolver(root.path()),
        overlay(path, Arc::<[u8]>::from(body.as_slice())),
    );

    let expected = ordinary
        .section_text(&address("short"))
        .expect_err("short anchor remains unresolved");
    let actual = overlaid
        .section_text(&address("short"))
        .expect_err("short anchor remains unresolved");

    assert_eq!(actual, expected);
    assert!(actual.contains("qualified candidates for `short`: origin--short"));
}

#[test]
fn pattern_expansion_is_identical_with_an_overlay() {
    let root = source_root();
    for name in ["plugin-beta", "plugin-alpha"] {
        let slot = crate::resolver::vibedeps_root_under(root.path())
            .join(format!("org.demo.plugins.{name}"))
            .join("1.0.0");
        let directory = crate::resolver::specs_root_under(&slot).join("contract");
        fs::create_dir_all(&directory).expect("plugin spec directory");
        fs::write(directory.join("API.md"), "# API\n").expect("plugin spec");
    }
    let pattern = SpecAddress::parse("spec://org.demo.plugins/plugin-*/contract/API")
        .expect("pattern address");
    let ordinary = FsSectionSource::new(resolver(root.path()));
    let overlaid = FsSectionSource::with_overlay(resolver(root.path()), BTreeMap::new());

    assert_eq!(
        overlaid.expand_pattern(&pattern),
        ordinary.expand_pattern(&pattern)
    );
}

#[test]
fn non_utf8_overlay_refuses_without_exposing_bytes() {
    let root = source_root();
    let path = document(root.path(), "DEP.md", b"# Disk {#wanted}\nDISK\n");
    let address = address("wanted");
    let resolver = resolver(root.path());
    let resolved = resolver
        .resolve_file(&address)
        .expect("resolved overlay path");
    let source = FsSectionSource::with_overlay(
        resolver,
        overlay(path.clone(), Arc::<[u8]>::from([0xff, 0xfe].as_slice())),
    );

    let error = source
        .section_text(&address)
        .expect_err("invalid UTF-8 refuses");

    assert!(error.contains("overlay spec source"), "{error}");
    assert!(error.contains(&resolved.display().to_string()), "{error}");
    assert!(error.contains("not UTF-8"), "{error}");
    assert!(
        !error.contains("255") && !error.contains("ff fe"),
        "{error}"
    );
}
