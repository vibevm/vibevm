//! M-01-A.1 exact selected address and compatibility corpus.

use std::collections::BTreeMap;
use std::fs;

use vibe_core::manifest::{Manifest, SourceInterpretation};

use super::*;
use crate::{
    DescriptorOutputRow, FileResolver, ResolveError, SelectedPackage, SelfCoordinate,
    ValidatedDescriptorMap,
};

fn selected_fixture() -> (tempfile::TempDir, FileResolver) {
    let temp = tempfile::tempdir().unwrap();
    let slot = temp.path().join("selected");
    let specs = slot.join("spec");
    fs::create_dir_all(specs.join("common")).unwrap();
    fs::write(
        specs.join("common/PROP-042-example.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Example</title></spec>",
    )
    .unwrap();
    let map = ValidatedDescriptorMap::from_witnessed_rows(
        slot.clone(),
        specs.clone(),
        "sha256:selected-package".to_owned(),
        "a".repeat(64),
        vec![DescriptorOutputRow {
            output: "spec/common/PROP-042-example.xml".to_owned(),
            source: "spec/common/PROP-042-example.xml".to_owned(),
            interpretation: Some(SourceInterpretation::VibeSpec),
            output_sha256: "b".repeat(64),
            document_address: Some("common/PROP-042".to_owned()),
        }],
    )
    .unwrap();
    let selected = BTreeMap::from([(
        ("org.demo".to_owned(), "guide".to_owned()),
        SelectedPackage::new("1.0.0", &slot),
    )]);
    let resolver = FileResolver::with_selected_world(
        temp.path().join("host"),
        temp.path(),
        selected,
        SelfCoordinate::new(Some("org.host".to_owned()), "self".to_owned()),
    )
    .with_descriptor_source_map(specs, Some(map))
    .unwrap();
    (temp, resolver)
}

#[test]
fn selected_vibe_and_unique_abbreviated_stem_canonicalize() {
    let (_temp, resolver) = selected_fixture();
    let address = resolver
        .canonical_vibe_resource("spec://org.demo/guide/common/PROP-042#root")
        .unwrap();
    assert_eq!(address.kind(), ResourceAddressKind::VibeDocument);
    assert_eq!(
        address.as_str(),
        "spec://org.demo/guide@1.0.0/common/PROP-042#root"
    );
    let explicit = resolver
        .canonical_vibe_resource("spec://org.demo/guide@1.0.0/common/PROP-042#root")
        .unwrap();
    assert_eq!(address, explicit);
}

#[test]
fn legacy_conversion_is_explicit_and_selected() {
    let (_temp, resolver) = selected_fixture();
    let legacy = SpecAddress::parse("spec://org.demo/guide@=1.0.0/common/PROP-042#root").unwrap();
    assert_eq!(
        resolver
            .convert_legacy_vibe_address(&legacy)
            .unwrap()
            .as_str(),
        "spec://org.demo/guide@1.0.0/common/PROP-042#root"
    );
    let physical_stem =
        SpecAddress::parse("spec://org.demo/guide/common/PROP-042-example#root").unwrap();
    assert_eq!(
        resolver
            .convert_legacy_vibe_address(&physical_stem)
            .unwrap()
            .as_str(),
        "spec://org.demo/guide@1.0.0/common/PROP-042#root"
    );
    assert!(matches!(
        resolver.canonical_vibe_resource(&physical_stem.raw),
        Err(ResourceAddressError::Lookup(
            ResolveError::DocNotFound { .. }
        ))
    ));
    assert!(matches!(
        resolver.canonical_vibe_resource(&legacy.raw),
        Err(ResourceAddressError::InvalidVersion)
    ));
    let partial = SpecAddress::parse("spec://org.demo/guide@0.2/common/PROP-042#root").unwrap();
    assert!(matches!(
        resolver.convert_legacy_vibe_address(&partial),
        Err(ResourceAddressError::InvalidVersion)
    ));
    let pinned = SpecAddress::parse("spec://org.demo/guide/common/PROP-042#root~r2").unwrap();
    assert!(matches!(
        resolver.convert_legacy_vibe_address(&pinned),
        Err(ResourceAddressError::UnsupportedRevision)
    ));
    let undotted = SpecAddress::parse("spec://host/common/PROP-042#root").unwrap();
    assert!(matches!(
        resolver.convert_legacy_vibe_address(&undotted),
        Err(ResourceAddressError::LegacyHostAuthority)
    ));
    let unselected = FileResolver::new(
        "/unused",
        SelfCoordinate::new(Some("org.host".to_owned()), "self".to_owned()),
    );
    assert!(matches!(
        unselected.convert_legacy_vibe_address(
            &SpecAddress::parse("spec://org.demo/guide/common/PROP-042").unwrap()
        ),
        Err(ResourceAddressError::SelectedWorldRequired)
    ));
}

#[test]
fn mutated_legacy_anchor_cannot_publish_an_invalid_resource() {
    let (_temp, resolver) = selected_fixture();
    let mut legacy = SpecAddress::parse("spec://org.demo/guide/common/PROP-042#root").unwrap();
    for segment in ["bad anchor", "root.child"] {
        legacy.anchor = vec![segment.to_owned()];
        assert!(matches!(
            resolver.convert_legacy_vibe_address(&legacy),
            Err(ResourceAddressError::InvalidCharacter)
        ));
    }
    legacy.anchor = vec![String::new()];
    assert!(matches!(
        resolver.convert_legacy_vibe_address(&legacy),
        Err(ResourceAddressError::EmptyComponent)
    ));
}

#[test]
fn refused_percent_empty_revision_and_wrong_version_are_distinct() {
    let (_temp, resolver) = selected_fixture();
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/guide/common/PROP%2D042#root"),
        Err(ResourceAddressError::PercentEncoding)
    ));
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/guide/common//PROP-042#root"),
        Err(ResourceAddressError::EmptyComponent)
    ));
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/guide@2.0.0/common/PROP-042#root"),
        Err(ResourceAddressError::WrongSelectedVersion { .. })
    ));
    assert!(matches!(
        resolver
            .canonical_vibe_resource("spec://org.demo/guide@not-a-version/common/PROP-042#root"),
        Err(ResourceAddressError::InvalidVersion)
    ));
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/guide/common/PROP-042#root~r2"),
        Err(ResourceAddressError::UnsupportedRevision)
    ));
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/guide/common/PROP-042#root~oops"),
        Err(ResourceAddressError::InvalidCharacter)
    ));
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.demo/guide/common/PROP-042.xml#root"),
        Err(ResourceAddressError::PhysicalSerialization)
    ));
}

#[test]
fn duplicate_stem_and_two_forms_refuse_without_first_match() {
    let ws = tempfile::tempdir().unwrap();
    let slot = ws.path().join("slot");
    let dir = slot.join("spec/common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PROP-042-one.md"), "# One\n").unwrap();
    fs::write(dir.join("PROP-042-two.md"), "# Two\n").unwrap();
    let selected = BTreeMap::from([(
        ("org.demo".to_owned(), "guide".to_owned()),
        SelectedPackage::new("1.0.0", &slot),
    )]);
    let resolver = FileResolver::with_selected_world(
        ws.path().join("host"),
        ws.path(),
        selected,
        SelfCoordinate::new(Some("org.host".to_owned()), "self".to_owned()),
    );
    let legacy = SpecAddress::parse("spec://org.demo/guide/common/PROP-042").unwrap();
    assert!(matches!(
        resolver.convert_legacy_vibe_address(&legacy),
        Err(ResourceAddressError::Lookup(
            ResolveError::AmbiguousDoc { .. }
        ))
    ));
    fs::remove_file(dir.join("PROP-042-two.md")).unwrap();
    fs::write(dir.join("PROP-042-one.xml"), "<spec/>").unwrap();
    assert!(matches!(
        resolver.convert_legacy_vibe_address(&legacy),
        Err(ResourceAddressError::Lookup(
            ResolveError::PairCollision { .. }
        ))
    ));
}

#[test]
fn named_export_uses_separate_scheme_and_selected_authority() {
    let (_temp, resolver) = selected_fixture();
    let manifest = Manifest::parse_str(
        "[package]\ngroup = 'org.demo'\nname = 'guide'\nkind = 'flow'\nversion = '1.0.0'\n\n[[package.source_exports]]\nname = 'intro'\nfile = 'notes/intro.md'\nheadings = ['Introduction']\n",
    )
    .unwrap();
    let package = manifest.require_package().unwrap();
    let address = resolver
        .canonical_export_resource("resource://org.demo/guide/intro", package)
        .unwrap();
    assert_eq!(address.kind(), ResourceAddressKind::SourceExport);
    assert_eq!(address.as_str(), "resource://org.demo/guide@1.0.0/intro");
    assert!(matches!(
        resolver.canonical_export_resource("resource://org.demo/guide/other", package),
        Err(ResourceAddressError::UndeclaredExport)
    ));
    assert!(matches!(
        resolver.canonical_export_resource("resource://org.demo/guide@2.0.0/intro", package),
        Err(ResourceAddressError::WrongSelectedVersion { .. })
    ));
    let foreign = Manifest::parse_str(
        "[package]\ngroup = 'org.other'\nname = 'guide'\nkind = 'flow'\nversion = '1.0.0'\n\n[[package.source_exports]]\nname = 'intro'\nfile = 'notes/intro.md'\nheadings = ['Introduction']\n",
    )
    .unwrap();
    assert!(matches!(
        resolver.canonical_export_resource(
            "resource://org.demo/guide/intro",
            foreign.require_package().unwrap(),
        ),
        Err(ResourceAddressError::ExportPackageMismatch)
    ));
}

#[test]
fn self_coordinate_is_unversioned_and_strict_read_needs_vibe_classification() {
    let ws = tempfile::tempdir().unwrap();
    let specs = ws.path().join("spec");
    fs::create_dir_all(specs.join("common")).unwrap();
    fs::write(specs.join("common/OWN.md"), "# Own {#root}\n").unwrap();
    let self_coord = SelfCoordinate::new(Some("org.host".to_owned()), "self".to_owned());
    let unclassified = FileResolver::new(ws.path(), self_coord.clone());
    assert!(matches!(
        unclassified.canonical_vibe_resource("spec://org.host/self/common/OWN#root"),
        Err(ResourceAddressError::VibeSourceRequired)
    ));
    let map = ValidatedDescriptorMap::from_witnessed_rows(
        ws.path().to_path_buf(),
        specs.clone(),
        "sha256:self".to_owned(),
        "a".repeat(64),
        vec![DescriptorOutputRow {
            output: "spec/common/OWN.md".to_owned(),
            source: "spec/common/OWN.md".to_owned(),
            interpretation: Some(SourceInterpretation::VibeSpec),
            output_sha256: "b".repeat(64),
            document_address: Some("common/OWN".to_owned()),
        }],
    )
    .unwrap();
    let resolver = FileResolver::new(ws.path(), self_coord)
        .with_descriptor_source_map(specs, Some(map))
        .unwrap();
    assert_eq!(
        resolver
            .canonical_vibe_resource("spec://org.host/self/common/OWN#root")
            .unwrap()
            .as_str(),
        "spec://org.host/self/common/OWN#root"
    );
    assert!(matches!(
        resolver.canonical_vibe_resource("spec://org.host/self@1.0.0/common/OWN#root"),
        Err(ResourceAddressError::InvalidVersion)
    ));
}
