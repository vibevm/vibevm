//! The correctness proof of the PROP-075 renderer: the real Node template
//! renders the two manifests the retired generator produced, byte for
//! byte, modulo the generator's provenance header and the `size` lines —
//! the one deliberate loss PROP-075 `#TEMPLATE-DIGEST-SOURCE` records,
//! because nodejs.org publishes digests but no sizes.
//!
//! `tests/fixtures/node/README.md` says what every fixture is and where
//! it came from.

use vibe_core::manifest::{Manifest, TemplateSection};
use vibe_template::{DigestTable, Stage, VersionRecord, render, template_section, version_records};

const TEMPLATE: &str = include_str!("fixtures/node/vibe.toml");
const INDEX: &str = include_str!("fixtures/node/index-excerpt.json");

/// The committed manifest with the generator's provenance header and its
/// `size` lines removed, plus the number of `size` lines that were there.
fn golden(committed: &str) -> (String, usize) {
    let mut lines = committed.lines().collect::<Vec<_>>();
    assert!(
        lines[0].starts_with("# Rendered by org.vibevm.node/vibevm-package-generator"),
        "the first fixture line is the generator's provenance header"
    );
    let header = lines
        .iter()
        .take_while(|line| line.starts_with('#'))
        .count();
    assert_eq!(header, 4, "the generator writes a four-line header");
    lines.drain(..header);
    let sizes = lines
        .iter()
        .filter(|line| line.starts_with("size = "))
        .count();
    lines.retain(|line| !line.starts_with("size = "));
    let mut text = lines.join("\n");
    text.push('\n');
    (text, sizes)
}

fn record(section: &TemplateSection, version: &str) -> VersionRecord {
    let (records, skipped) = version_records(section, Some(INDEX)).expect("the index parses");
    assert_eq!(
        skipped.len(),
        1,
        "exactly the record whose version is not semver is skipped: {skipped:?}"
    );
    records
        .into_iter()
        .find(|record| record.version.to_string() == version)
        .unwrap_or_else(|| panic!("the index excerpt carries {version}"))
}

fn digests(version: &str) -> DigestTable {
    let text = match version {
        "24.22.2" => include_str!("fixtures/node/SHASUMS256-24.22.2.txt"),
        "22.23.4" => include_str!("fixtures/node/SHASUMS256-22.23.4.txt"),
        other => panic!("no digest fixture for {other}"),
    };
    DigestTable::parse_sha256sums(text).expect("the digest document parses")
}

/// The whole claim of phase P1, as one assertion per version.
#[test]
fn renders_the_committed_node_manifests() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    for (version, committed) in [
        ("24.22.2", include_str!("fixtures/node/vibe-24.22.2.toml")),
        ("22.23.4", include_str!("fixtures/node/vibe-22.23.4.toml")),
    ] {
        let (expected, sizes) = golden(committed);
        assert_eq!(sizes, 7, "{version}: one size line per installer option");
        let rendered = render(TEMPLATE, &record(&section, version), &digests(version))
            .unwrap_or_else(|error| panic!("{version}: {error}"));
        assert_eq!(rendered.stage, Stage::Full);
        assert!(
            rendered.dropped.is_empty(),
            "{version}: both optional rows have digests: {:?}",
            rendered.dropped
        );
        assert_eq!(
            rendered.toml, expected,
            "{version}: the render is not the committed manifest"
        );
        // And the bytes it produced are an ordinary manifest.
        let manifest = Manifest::parse_str(&rendered.toml)
            .unwrap_or_else(|error| panic!("{version}: the render is not a manifest: {error}"));
        let package = manifest.require_package().expect("a package");
        assert_eq!(package.version.to_string(), version);
        assert!(!package.template, "the render strips the flag");
        assert!(manifest.template.is_none(), "the render strips the table");
        assert_eq!(manifest.installer_options.len(), 7);
        assert!(
            manifest
                .installer_options
                .iter()
                .all(|option| !option.template_optional),
            "the render strips template_optional"
        );
    }
}

/// Rendering is deterministic — the generator's own stated invariant, and
/// the reason a `rendered_sha256` can bind a lock (PROP-075
/// `#TEMPLATE-LOCK-RECORD`).
#[test]
fn rendering_the_same_inputs_is_byte_identical() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    for version in ["24.22.2", "22.23.4"] {
        let record = record(&section, version);
        let digests = digests(version);
        let first = render(TEMPLATE, &record, &digests).expect("renders");
        for _ in 0..8 {
            let again = render(TEMPLATE, &record, &digests).expect("renders");
            assert_eq!(first.toml, again.toml, "{version}");
            assert_eq!(first.sha256, again.sha256, "{version}");
        }
        // Two different versions never share a rendering.
        assert_ne!(first.sha256.len(), 0);
    }
    let twenty_four =
        render(TEMPLATE, &record(&section, "24.22.2"), &digests("24.22.2")).expect("renders");
    let twenty_two =
        render(TEMPLATE, &record(&section, "22.23.4"), &digests("22.23.4")).expect("renders");
    assert_ne!(twenty_four.sha256, twenty_two.sha256);
}

/// A version whose upstream `files[]` lacks the musl and Windows-ARM
/// archives renders without those two rows and reports the drop, which is
/// what `template_optional` is for (PROP-075 `#TEMPLATE-OPTION-GATING`).
#[test]
fn an_optional_row_without_a_digest_is_dropped_and_reported() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    let full = digests("24.22.2");
    let mut narrowed = std::collections::BTreeMap::new();
    for option in [
        "linux-x64",
        "linux-arm64",
        "darwin-x64",
        "darwin-arm64",
        "win-x64",
    ] {
        let file = format!(
            "node-v24.22.2-{option}.{}",
            if option.starts_with("win") {
                "zip"
            } else {
                "tar.gz"
            }
        );
        let entry = full.get(&file).expect("the fixture names it").clone();
        narrowed.insert(file, entry);
    }
    let rendered = render(
        TEMPLATE,
        &record(&section, "24.22.2"),
        &DigestTable::from_entries(narrowed),
    )
    .expect("the optional rows are dropped, not refused");
    let dropped: Vec<(&str, &str)> = rendered
        .dropped
        .iter()
        .map(|row| (row.id.as_str(), row.missing.as_str()))
        .collect();
    assert_eq!(
        dropped,
        vec![
            ("linux-musl-x64", "node-v24.22.2-linux-x64-musl.tar.gz"),
            ("windows-arm64", "node-v24.22.2-win-arm64.zip"),
        ]
    );
    let manifest = Manifest::parse_str(&rendered.toml).expect("still a manifest");
    assert_eq!(manifest.installer_options.len(), 5);
    assert!(
        manifest
            .installer_options
            .iter()
            .all(|option| option.id != "linux-musl-x64" && option.id != "windows-arm64")
    );
}

/// A row that is **not** optional and has no digest refuses the render,
/// because a silently wrong manifest is worse than a refused one.
#[test]
fn a_required_row_without_a_digest_refuses() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    let error = render(
        TEMPLATE,
        &record(&section, "24.22.2"),
        &DigestTable::default(),
    )
    .expect_err("refuses");
    assert_eq!(
        error.kind,
        vibe_template::RenderErrorKind::MissingDigest,
        "{error}"
    );
    assert_eq!(
        error.subject.as_deref(),
        Some("node-v24.22.2-linux-x64.tar.gz")
    );
    assert_eq!(error.at, "installer_options[0].steps[0].sha256");
}

/// The skeleton render costs no digest document and is still an ordinary
/// manifest, which is what makes resolution one request (PROP-075
/// `#TEMPLATE-TWO-STAGE`).
#[test]
fn the_skeleton_render_of_the_node_template_validates() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    for version in ["24.22.2", "22.23.4"] {
        let rendered =
            vibe_template::skeleton(TEMPLATE, &record(&section, version)).expect("renders");
        assert_eq!(rendered.stage, Stage::Skeleton);
        let manifest = Manifest::parse_str(&rendered.toml).expect("an ordinary manifest");
        assert_eq!(manifest.installer_options.len(), 7, "nothing is dropped");
        assert_eq!(
            manifest.require_package().unwrap().version.to_string(),
            version
        );
        assert!(rendered.toml.contains(vibe_template::SKELETON_DIGEST));
    }
}
