//! Unit tests for [`super`], out-of-line per the file-length budget.
//! The image bytes come from the header reader's own builders, so a
//! fixture here is exactly the header the cell will read.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::image::tests::{jpeg, png, webp_vp8x};
use crate::test_support::opts;
use crate::{CheckId, CheckReport, Severity, check_project};

/// A `doc` package declaring `media`, with `extra` appended to the
/// `[media]` table.
fn write_package(root: &Path, media: &str) {
    fs::write(
        root.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.vibevm.core"
name = "vibevm-docs"
kind = "doc"
version = "0.1.0"
title = "VibeVM Manual"
abstract = "What it covers, for whom, what it assumes known, what it leaves out."

[[documents]]
package = "org.vibevm.core/vibevm"
version = "^1.0"

[media]
{media}"#
        ),
    )
    .unwrap();
    fs::write(root.join("README.md"), "# manual\n").unwrap();
    fs::create_dir_all(root.join("media")).unwrap();
}

fn media_findings(report: &CheckReport) -> Vec<&crate::Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.check == CheckId::DocMedia)
        .collect()
}

#[test]
fn a_card_whose_images_fit_their_roles_passes_clean() {
    let project = tempdir().unwrap();
    write_package(
        project.path(),
        "icon = \"media/icon.png\"\nbanner = \"media/banner.jpg\"\npreview = \"media/preview.webp\"\n",
    );
    fs::write(project.path().join("media/icon.png"), png(512, 512)).unwrap();
    fs::write(project.path().join("media/banner.jpg"), jpeg(1500, 500)).unwrap();
    fs::write(
        project.path().join("media/preview.webp"),
        webp_vp8x(1200, 630),
    )
    .unwrap();
    let report = check_project(project.path(), &opts());
    assert!(
        media_findings(&report).is_empty(),
        "a well-shaped card owes nothing: {:?}",
        report.findings
    );
}

#[test]
fn a_declared_image_that_is_not_there_is_an_error() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.png\"\n");
    let report = check_project(project.path(), &opts());
    let hit = media_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Error)
        .expect("a missing image is an error");
    assert!(hit.message.contains("media/icon.png"), "{}", hit.message);
    assert!(hit.message.contains("CARD-MEDIA-SOURCE"));
}

/// The name is an assertion; the first bytes are the file. An SVG named
/// `.png` is refused for being an SVG, and the message says why SVG in
/// particular — it can carry script.
#[test]
fn the_format_is_read_from_the_bytes_never_from_the_name() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.png\"\n");
    fs::write(
        project.path().join("media/icon.png"),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\"/>",
    )
    .unwrap();
    let report = check_project(project.path(), &opts());
    let hit = media_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Error)
        .expect("an SVG is refused whatever it is named");
    assert!(hit.message.contains("is an SVG"), "{}", hit.message);
    assert!(hit.message.contains("script"), "{}", hit.message);

    let other = tempdir().unwrap();
    write_package(other.path(), "icon = \"media/icon.png\"\n");
    fs::write(other.path().join("media/icon.png"), b"GIF89a....").unwrap();
    let report = check_project(other.path(), &opts());
    assert!(
        media_findings(&report)
            .into_iter()
            .any(|f| f.severity == Severity::Error && f.message.contains("no PNG, JPEG or WebP")),
        "a foreign format is refused by signature: {:?}",
        report.findings
    );
}

#[test]
fn an_icon_must_be_square_and_within_its_side_range() {
    let cases = [
        (png(512, 256), "is shown in the page header"),
        (png(128, 128), "px on a side"),
        (png(2048, 2048), "px on a side"),
    ];
    for (bytes, needle) in cases {
        let project = tempdir().unwrap();
        write_package(project.path(), "icon = \"media/icon.png\"\n");
        fs::write(project.path().join("media/icon.png"), bytes).unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            media_findings(&report)
                .into_iter()
                .any(|f| f.severity == Severity::Error && f.message.contains(needle)),
            "expected `{needle}`: {:?}",
            report.findings
        );
    }
}

/// A banner is 3:1 and a preview is 1.91:1 — different shapes, which is
/// exactly why the preview is its own field rather than a cropped
/// banner. The recommended pixel sizes are checked as ratios, so
/// 1200×630 (1.9048:1) passes while a banner's proportions in a
/// preview's slot do not.
#[test]
fn each_wide_role_is_held_to_its_own_ratio() {
    let project = tempdir().unwrap();
    write_package(project.path(), "preview = \"media/preview.png\"\n");
    fs::write(project.path().join("media/preview.png"), png(1500, 500)).unwrap();
    let report = check_project(project.path(), &opts());
    let hit = media_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Error)
        .expect("a 3:1 image is not a link preview");
    assert!(hit.message.contains("1.91:1"), "{}", hit.message);

    let ok = tempdir().unwrap();
    write_package(ok.path(), "preview = \"media/preview.png\"\n");
    fs::write(ok.path().join("media/preview.png"), png(1200, 630)).unwrap();
    let report = check_project(ok.path(), &opts());
    assert!(
        media_findings(&report).is_empty(),
        "the recommended preview size must pass: {:?}",
        report.findings
    );

    let banner = tempdir().unwrap();
    write_package(banner.path(), "banner = \"media/banner.png\"\n");
    fs::write(banner.path().join("media/banner.png"), png(1200, 630)).unwrap();
    let report = check_project(banner.path(), &opts());
    assert!(
        media_findings(&report)
            .into_iter()
            .any(|f| f.severity == Severity::Error && f.message.contains("3:1")),
        "a preview's proportions are not a banner's: {:?}",
        report.findings
    );
}

/// The ceilings are small on purpose: ordinary packages are materialised
/// and committed in consumers' trees.
#[test]
fn an_image_over_its_byte_ceiling_is_an_error() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.png\"\n");
    let mut bytes = png(512, 512);
    bytes.resize(300 * 1024, 0);
    fs::write(project.path().join("media/icon.png"), bytes).unwrap();
    let report = check_project(project.path(), &opts());
    assert!(
        media_findings(&report)
            .into_iter()
            .any(|f| f.severity == Severity::Error && f.message.contains("262144")),
        "an oversized icon is an error naming the ceiling: {:?}",
        report.findings
    );
}

/// A legal picture whose header this reader cannot follow is a WARNING:
/// the bytes are a picture, only the measurement failed, and refusing
/// there would be the linter overstating what it measured.
#[test]
fn a_measurable_format_with_an_unreadable_header_only_warns() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.png\"\n");
    // The signature, then a chunk that is not IHDR.
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&[0, 0, 0, 13]);
    bytes.extend_from_slice(b"tEXt");
    fs::write(project.path().join("media/icon.png"), bytes).unwrap();
    let report = check_project(project.path(), &opts());
    let hits = media_findings(&report);
    assert!(
        hits.iter()
            .any(|f| f.severity == Severity::Warning && f.message.contains("unverified")),
        "an unmeasurable shape warns: {:?}",
        report.findings
    );
    assert!(
        !hits.iter().any(|f| f.severity == Severity::Error),
        "and does not fail the package: {:?}",
        report.findings
    );
}

/// A package that declares no `[media]` is not this cell's business —
/// placeholders are generated from the coordinate's hash, so absence is
/// a complete answer (`##CARD-PLACEHOLDERS-GENERATED`).
#[test]
fn a_package_without_a_card_is_untouched() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let report = check_project(project.path(), &opts());
    assert!(media_findings(&report).is_empty());
}
