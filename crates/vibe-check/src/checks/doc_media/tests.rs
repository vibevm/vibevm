//! Unit tests for [`super`].
//!
//! The card's RULES are tested where they live — `vibe_doc::media` owns
//! the formats, the proportions and the ceilings, and proves them there
//! on bytes it builds itself. What is tested here is the only thing this
//! cell decides: that a finding reaches a `vibe check` report as the
//! right severity against the right path, and that a project with no
//! card is left alone.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::test_support::opts;
use crate::{CheckId, CheckReport, Severity, check_project};

/// A PNG that is nothing but its signature and IHDR — everything the
/// pipeline's header reader looks at.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes
}

/// A `doc` package whose `[media]` table holds `media`.
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

/// The green half: a card the pipeline accepts produces no line of
/// linter output.
#[test]
fn a_card_the_pipeline_accepts_produces_no_finding() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.png\"\n");
    fs::write(project.path().join("media/icon.png"), png(512, 512)).unwrap();

    let report = check_project(project.path(), &opts());
    assert!(media_findings(&report).is_empty(), "{report:#?}");
}

/// The red half, and the only thing this cell does with it: the
/// pipeline's refusal arrives as an error against the image's own path,
/// with the message the pipeline wrote.
#[test]
fn a_refusal_arrives_as_an_error_against_the_image_path() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.png\"\n");
    // Square is the icon's rule; 512×256 is not.
    fs::write(project.path().join("media/icon.png"), png(512, 256)).unwrap();

    let report = check_project(project.path(), &opts());
    let found = media_findings(&report);
    assert_eq!(found.len(), 1, "{report:#?}");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].path.as_deref(),
        Some(Path::new("media/icon.png")),
        "the finding names the image, not the manifest"
    );
    assert!(found[0].message.contains("square"), "{}", found[0].message);
    assert!(
        found[0].message.contains("CARD-MEDIA-SOURCE"),
        "{}",
        found[0].message
    );
}

/// A measurement the pipeline could not make arrives as a warning, not
/// an error: the bytes are a picture and only the shape is unknown.
#[test]
fn an_unverifiable_shape_arrives_as_a_warning() {
    let project = tempdir().unwrap();
    write_package(project.path(), "icon = \"media/icon.webp\"\n");
    // A WebP container whose chunk the header reader does not follow.
    let mut odd = b"RIFF".to_vec();
    odd.extend_from_slice(&0u32.to_le_bytes());
    odd.extend_from_slice(b"WEBPXXXX");
    fs::write(project.path().join("media/icon.webp"), odd).unwrap();

    let report = check_project(project.path(), &opts());
    let found = media_findings(&report);
    assert_eq!(found.len(), 1, "{report:#?}");
    assert_eq!(found[0].severity, Severity::Warning);
}

/// A package with no `[media]` is not a package with an empty one: the
/// placeholders are generated, and the cell has nothing to say.
#[test]
fn a_package_without_a_card_is_untouched() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"thing\"\nkind = \"tool\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let report = check_project(project.path(), &opts());
    assert!(media_findings(&report).is_empty(), "{report:#?}");
}
