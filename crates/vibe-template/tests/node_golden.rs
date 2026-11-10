//! The correctness proof of the PROP-075 renderer: the real Node template
//! renders the two manifests the retired generator produced, byte for
//! byte, modulo the generator's provenance header and the `size` lines —
//! the one deliberate loss PROP-075 `#TEMPLATE-DIGEST-SOURCE` records,
//! because nodejs.org publishes digests but no sizes.
//!
//! Since phase P3 the inputs are upstream's own frozen documents rather
//! than P1's hand-derived excerpts, and the proof is therefore stronger
//! than byte-identity of two renders: each digest document hashes to
//! exactly the input the committed manifest's provenance header names, so
//! the template and the retired generator are shown to render the same
//! manifest *from the same bytes*.
//!
//! `tests/fixtures/node/README.md` says what every fixture is and where
//! it came from.

use vibe_core::manifest::{Manifest, TemplateSection};
use vibe_template::{DigestTable, Stage, VersionRecord, render, template_section, version_records};

const TEMPLATE: &str = include_str!("fixtures/node/vibe.toml");
const INDEX: &str = include_str!("fixtures/node/index-excerpt.json");

/// The two versions `/fast/git/vibe-pkg-node` publishes concretely, with
/// the manifest each of its tags carries — the golden bytes.
const COMMITTED: [(&str, &str); 2] = [
    ("24.22.2", include_str!("fixtures/node/vibe-24.22.2.toml")),
    ("22.23.4", include_str!("fixtures/node/vibe-22.23.4.toml")),
];

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
    assert!(
        skipped.is_empty(),
        "upstream's own index carries no record vibe cannot read: {skipped:?}"
    );
    records
        .into_iter()
        .find(|record| record.version.to_string() == version)
        .unwrap_or_else(|| panic!("the index excerpt carries {version}"))
}

/// The frozen upstream digest document of one version.
fn digest_document(version: &str) -> &'static str {
    match version {
        "24.22.2" => include_str!("fixtures/node/SHASUMS256-24.22.2.txt"),
        "22.23.4" => include_str!("fixtures/node/SHASUMS256-22.23.4.txt"),
        "26.13.0" => include_str!("fixtures/node/SHASUMS256-26.13.0.txt"),
        "12.0.0" => include_str!("fixtures/node/SHASUMS256-12.0.0.txt"),
        other => panic!("no digest fixture for {other}"),
    }
}

fn digests(version: &str) -> DigestTable {
    DigestTable::parse_sha256sums(digest_document(version)).expect("the digest document parses")
}

/// The whole claim of phase P1, as one assertion per version.
#[test]
fn renders_the_committed_node_manifests() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    for (version, committed) in COMMITTED {
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

/// The frozen digest documents are the retired generator's **own
/// inputs**: each hashes to exactly the digest the committed manifest's
/// provenance header records. Without this the golden comparison would
/// only say that two renderers agree about seven digests somebody typed;
/// with it, both are shown to render the same manifest from the same
/// upstream bytes (campaign `AV-FMT-006`).
#[test]
fn the_frozen_digest_documents_are_the_generators_own_inputs() {
    for (version, committed) in COMMITTED {
        let declared = committed
            .lines()
            .find_map(|line| line.strip_prefix("# input SHASUMS256.txt sha256 = "))
            .unwrap_or_else(|| panic!("{version}: the header records its digest-document input"));
        let frozen = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(digest_document(version).as_bytes());
            format!("{:x}", hasher.finalize())
        };
        assert_eq!(
            frozen, declared,
            "{version}: the frozen SHASUMS256.txt is not the document the generator read"
        );
    }
}

/// The frozen index subset is the template's **whole** version domain, so
/// every channel answer the suites check is upstream's real answer: the
/// document is sorted newest-first and the subset is a leading slice of
/// it (`tests/fixtures/node/README.md` states the selection rule).
#[test]
fn the_frozen_index_subset_is_the_whole_version_domain() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    let (records, skipped) = version_records(&section, Some(INDEX)).expect("the index parses");
    assert!(skipped.is_empty(), "{skipped:?}");
    assert_eq!(records.len(), 406, "the produced versions of the subset");
    assert_eq!(records.first().unwrap().version.to_string(), "26.13.0");
    assert_eq!(records.last().unwrap().version.to_string(), "12.0.0");
    // The two filters the subset exists to exercise: the one prerelease
    // and the three records below the declared `min` are all gone.
    assert!(
        records.iter().all(|record| record.version.pre.is_empty()),
        "the `prerelease` filter dropped v27.0.0-alpha.0.0.0"
    );
    assert!(
        records
            .iter()
            .all(|record| record.version >= semver::Version::new(12, 0, 0)),
        "the `min` filter dropped v11.15.0, v11.14.0 and v11.13.0"
    );
    // And the records still carry the metadata the channel rules read.
    let newest = records.first().unwrap();
    assert_eq!(newest.meta["lts"].to_string(), "Lithium");
    assert_eq!(newest.raw, "v26.13.0");
}

/// The oldest version in the template's `min` window renders — with the
/// three targets upstream did not build for it dropped and reported, and
/// nothing else lost. This is the risk `PLAN.md` names for phase P3: a
/// `template_optional` set that is wrong makes an old version render an
/// archive that does not exist, and the golden comparison (two recent
/// versions) cannot see it.
#[test]
fn the_oldest_version_in_the_min_window_drops_the_three_absent_targets() {
    let section = template_section(TEMPLATE).expect("[template] reads");
    let rendered = render(TEMPLATE, &record(&section, "12.0.0"), &digests("12.0.0"))
        .expect("the three absent targets are dropped, not refused");
    let dropped: Vec<&str> = rendered.dropped.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(
        dropped,
        vec!["linux-musl-x64", "macos-arm64", "windows-arm64"]
    );
    let manifest = Manifest::parse_str(&rendered.toml).expect("an ordinary manifest");
    let ids: Vec<&str> = manifest
        .installer_options
        .iter()
        .map(|option| option.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["linux-x64", "linux-arm64", "macos-x64", "windows-x64"]
    );
    // The rows that survived carry upstream's real digests, not zeros.
    assert_eq!(rendered.stage, Stage::Full);
    assert!(
        rendered
            .toml
            .contains("3268466af05464a7aa698df7cc8bbeec0b8728d8779e12130ef441bb730a8059"),
        "the linux-x64 digest of v12.0.0 is the frozen document's"
    );
}
