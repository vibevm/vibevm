//! C4: selected parse failure remains an explicit applied slot outcome.

use std::fs;

use crate::Workspace;
use crate::install::{ResolvedDep, apply_resolution_with_spec_format};
use tempfile::TempDir;
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;
use vibe_core::manifest::SpecFormat;
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{ContentHash, Group};

use super::*;

const BAD_MD: &str = "# Rule {#root}\n\n@fact:GOOD Good. @status:impl/done\n\nPlain prose.\n\n@status:impl/done\n\nFollowing prose.\n";

fn source_path() -> PathBuf {
    layout::current_specs_root().join("DOC.md")
}

fn fixture() -> (TempDir, TempDir) {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let source_file = source.path().join(source_path());
    fs::create_dir_all(source_file.parent().unwrap()).unwrap();
    fs::write(&source_file, BAD_MD).unwrap();
    fs::write(source.path().join("README.md"), "# Ordinary README\n").unwrap();
    fs::write(
        source.path().join("vibe.toml"),
        format!(
            "[package]\nname='dep'\ngroup='org.demo'\nkind='flow'\nversion='1.0.0'\n\n[[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
            path_to_slash(&source_path())
        ),
    ).unwrap();
    (workspace, source)
}

#[test]
fn default_selected_failed_conversion_copies_with_reason_ordinary_readme_stays_copied() {
    let (workspace, source) = fixture();
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let hash = ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
    materialise_with_spec_format(
        workspace.path(),
        &group,
        "dep",
        &version,
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &hash,
    )
    .unwrap();
    let slot = slot_abs_path(workspace.path(), &group, "dep", &version);
    assert_eq!(
        fs::read(slot.join(source_path())).unwrap(),
        BAD_MD.as_bytes()
    );
    assert!(!slot.join(source_path().with_extension("xml")).exists());
    let record = read_slot_record(&slot).unwrap();
    let failed = record
        .files
        .iter()
        .find(|file| file.path == path_to_slash(&source_path()))
        .unwrap();
    let Some(SlotFileDisposition::CopiedUnconverted(reason)) = &failed.disposition else {
        panic!(
            "selected failed conversion must be typed, got {:?}",
            failed.disposition
        )
    };
    assert!(
        reason
            .issues
            .iter()
            .any(|issue| issue.class == crate::vibedeps::ConversionFailureClass::Stranded)
    );
    assert!(
        reason
            .issues
            .iter()
            .all(|issue| issue.line > 0 && !issue.message.is_empty())
    );
    let ordinary = record
        .files
        .iter()
        .find(|file| file.path == "README.md")
        .unwrap();
    assert_eq!(ordinary.disposition, Some(SlotFileDisposition::Copied));
}

#[test]
fn legacy_requested_markdown_failure_is_recorded_ordinary_other_path_is_copied() {
    let (workspace, source) = fixture();
    fs::write(
        source.path().join("vibe.toml"),
        "[package]\nname='dep'\ngroup='org.demo'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::create_dir_all(source.path().join("notes")).unwrap();
    fs::write(source.path().join("notes/ordinary.md"), BAD_MD).unwrap();
    let group = Group::parse("org.demo").unwrap();
    let version = "1.0.0".parse().unwrap();
    let hash = ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
    materialise_with_spec_format(
        workspace.path(),
        &group,
        "dep",
        &version,
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &hash,
    )
    .unwrap();
    let slot = slot_abs_path(workspace.path(), &group, "dep", &version);
    let record = read_slot_record(&slot).unwrap();
    let failed = record
        .files
        .iter()
        .find(|file| file.path == path_to_slash(&source_path()))
        .unwrap();
    assert!(
        matches!(&failed.disposition, Some(SlotFileDisposition::CopiedUnconverted(reason))
        if reason.issues[0].class == crate::vibedeps::ConversionFailureClass::Stranded)
    );
    let ordinary = record
        .files
        .iter()
        .find(|file| file.path == "notes/ordinary.md")
        .unwrap();
    assert_eq!(ordinary.disposition, Some(SlotFileDisposition::Copied));
}

#[test]
fn applied_slot_counts_each_conversion_once_and_skipped_repeat_counts_zero() {
    let (workspace_dir, source) = fixture();
    let good = layout::current_specs_root().join("GOOD.md");
    fs::write(
        source.path().join(&good),
        "# Good {#root}\n\n@fact:GOOD Good. @status:impl/done\n",
    )
    .unwrap();
    let mut package_text = fs::read_to_string(source.path().join("vibe.toml")).unwrap();
    package_text.push_str(&format!(
        "\n[[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
        path_to_slash(&good)
    ));
    fs::write(source.path().join("vibe.toml"), &package_text).unwrap();
    let root_text =
        "[project]\nname='root'\nversion='1.0.0'\n\n[requires.packages]\n'org.demo/dep'='=1.0.0'\n";
    fs::write(workspace_dir.path().join("vibe.toml"), root_text).unwrap();
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let hash = ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
    let workspace = Workspace {
        root: workspace_dir.path().to_path_buf(),
        root_manifest: Manifest::parse_str(root_text).unwrap(),
        members: vec![],
    };
    let dep = ResolvedDep {
        kind: PackageKind::Flow,
        group,
        name: "dep".into(),
        version,
        content_dir: source.path().to_path_buf(),
        source_hash: Some(hash),
        embedded_sources: vec![],
        manifest: Manifest::parse_str(&package_text).unwrap(),
        requires: vec![],
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    };
    let first = apply_resolution_with_spec_format(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        None,
        None,
    )
    .unwrap();
    assert_eq!(first.conversion_counts.converted, 1);
    assert_eq!(first.conversion_counts.copied_unconverted, 1);
    assert_eq!(first.conversion_failures.len(), 1);
    assert!(first.conversion_failures[0].source.ends_with("DOC.md"));
    let second = apply_resolution_with_spec_format(
        &workspace,
        &[dep],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        None,
        None,
    )
    .unwrap();
    assert!(second.materialised.is_empty());
    assert_eq!(second.conversion_counts.converted, 0);
    assert_eq!(second.conversion_counts.copied_unconverted, 0);
}
