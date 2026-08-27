//! Transformed-slot materialisation and legacy-projection oracles.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};

use super::*;

fn version(value: &str) -> semver::Version {
    semver::Version::parse(value).unwrap()
}

fn group(value: &str) -> Group {
    Group::parse(value).unwrap()
}

fn source_hash() -> ContentHash {
    ContentHash::parse("sha256:1111111111111111111111111111111111111111111111111111111111111111")
        .unwrap()
}

fn write(root: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn derive_markdown(workspace: &Path, source: &Path, name: &str) -> PathBuf {
    materialise_with_spec_format(
        workspace,
        &group("org.example"),
        name,
        &version("1.0.0"),
        source,
        CopyMode::Copy,
        SpecFormat::Markdown,
        &source_hash(),
    )
    .unwrap();
    slot_abs_path(workspace, &group("org.example"), name, &version("1.0.0"))
}

#[test]
fn xml_format_converts_only_spec_genre_and_records_every_file() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "README.md", "# Package\n\nOverview.\n");
    write(
        source.path(),
        crate::layout_paths::specs_path("guide.md"),
        "# Guide\n\nText.\n",
    );
    write(source.path(), "LICENSE.md", "license bytes\n");
    write(source.path(), "vibe.toml", "[package]\nname = \"sample\"\n");

    let footprint = materialise_with_spec_format(
        workspace.path(),
        &group("org.vibevm"),
        "xml",
        &version("1.0.0"),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &source_hash(),
    )
    .unwrap();

    let slot = slot_abs_path(
        workspace.path(),
        &group("org.vibevm"),
        "xml",
        &version("1.0.0"),
    );
    assert!(slot.join("README.xml").is_file());
    assert!(
        slot.join(crate::layout_paths::specs_path("guide.xml"))
            .is_file()
    );
    assert!(!slot.join("README.md").exists());
    assert_eq!(
        fs::read(slot.join("LICENSE.md")).unwrap(),
        b"license bytes\n"
    );
    assert!(!slot.join(DERIVED_MANIFEST_FILENAME).exists());
    assert!(!footprint.iter().any(|path| {
        path == Path::new(SLOT_RECORD_FILENAME) || path == Path::new(DERIVED_MANIFEST_FILENAME)
    }));

    let record = read_slot_record(&slot).unwrap();
    assert_eq!(record.source_hash, source_hash());
    assert_eq!(record.spec_format, SpecFormat::Xml);
    assert_eq!(record.converter_recipe.as_deref(), Some(CONVERTER_RECIPE));
    assert_eq!(record.files.len(), 4);
    assert_eq!(
        record
            .files
            .iter()
            .filter(|file| file.disposition == Some(SlotFileDisposition::Converted))
            .count(),
        2
    );
    for file in &record.files {
        assert_eq!(file.sha256, sha256_file(&slot.join(&file.path)).unwrap());
    }
    assert_eq!(
        record.derived_hash.as_ref().unwrap(),
        &compute_recorded_payload_hash(&slot, &record.files).unwrap()
    );

    let legacy_view = read_derived_manifest(&slot).unwrap();
    assert_eq!(legacy_view.output_format, SpecFormat::Xml);
    assert_eq!(legacy_view.converter_recipe, CONVERTER_RECIPE);
    assert_eq!(legacy_view.files.len(), record.files.len());
    assert_eq!(
        legacy_view.derived_hash,
        compute_derived_hash(&slot).unwrap()
    );
}

#[test]
fn markdown_format_converts_xml_and_copies_a_rejected_candidate_verbatim() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown("# Valid\n\nBody.\n").unwrap());
    write(
        source.path(),
        crate::layout_paths::specs_path("valid.xml"),
        &xml,
    );
    write(
        source.path(),
        crate::layout_paths::specs_path("rejected.xml"),
        "<not-closed",
    );

    materialise_with_spec_format(
        workspace.path(),
        &group("org.vibevm"),
        "markdown",
        &version("1.0.0"),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Markdown,
        &source_hash(),
    )
    .unwrap();

    let slot = slot_abs_path(
        workspace.path(),
        &group("org.vibevm"),
        "markdown",
        &version("1.0.0"),
    );
    assert!(
        slot.join(crate::layout_paths::specs_path("valid.md"))
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(slot.join(crate::layout_paths::specs_path("rejected.xml"))).unwrap(),
        "<not-closed"
    );
    let manifest = read_derived_manifest(&slot).unwrap();
    let rejected = manifest
        .files
        .iter()
        .find(|file| file.source == crate::layout_paths::specs("rejected.xml"))
        .unwrap();
    assert_eq!(rejected.output, crate::layout_paths::specs("rejected.xml"));
    assert_eq!(rejected.disposition, DerivedFileDisposition::Copied);
}

#[test]
fn legacy_derived_manifest_remains_readable_without_a_slot_record() {
    let slot = TempDir::new().unwrap();
    let legacy = DerivedManifest {
        schema: 1,
        source_hash: source_hash().as_str().to_string(),
        output_format: SpecFormat::Xml,
        converter_recipe: CONVERTER_RECIPE.to_string(),
        overlay_hash: None,
        derived_hash: source_hash().as_str().to_string(),
        files: vec![DerivedFile {
            source: crate::layout_paths::specs("RULE.md"),
            output: crate::layout_paths::specs("RULE.xml"),
            disposition: DerivedFileDisposition::Converted,
        }],
    };
    fs::write(
        slot.path().join(DERIVED_MANIFEST_FILENAME),
        toml::to_string_pretty(&legacy).unwrap(),
    )
    .unwrap();

    assert_eq!(read_derived_manifest(slot.path()).unwrap(), legacy);
    assert!(format_is_current(slot.path(), SpecFormat::Xml));
}

#[test]
fn redbook_materialises_fully_xml_with_md_readmes_converted() {
    let workspace = TempDir::new().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(vibe_core::layout::current_packages_root())
        .join("org.vibevm.world/redbook/v1.0.0");
    materialise_with_spec_format(
        workspace.path(),
        &group("org.vibevm.world"),
        "redbook",
        &version("1.0.0"),
        &source,
        CopyMode::Copy,
        SpecFormat::Xml,
        &source_hash(),
    )
    .unwrap();

    let slot = slot_abs_path(
        workspace.path(),
        &group("org.vibevm.world"),
        "redbook",
        &version("1.0.0"),
    );
    let manifest = read_derived_manifest(&slot).unwrap();
    let converted = manifest
        .files
        .iter()
        .filter(|file| file.disposition == DerivedFileDisposition::Converted)
        .count();
    let copied = manifest
        .files
        .iter()
        .filter(|file| file.disposition == DerivedFileDisposition::Copied)
        .count();
    assert_eq!(converted, 2);
    assert_eq!(copied, 8);
    assert!(slot.join("README.xml").is_file());
    assert!(
        slot.join(crate::layout_paths::boot_path("03-flow-redbook.xml"))
            .is_file()
    );
    assert!(slot.join("LICENSE.md").is_file());
    assert!(slot.join("vibe.toml").is_file());
    assert_eq!(manifest.derived_hash, compute_derived_hash(&slot).unwrap());
}

#[test]
fn transformed_payload_hash_consumes_flattened_slash_order_not_component_order() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    // Output set: `vibespecs/guide.xml` beside `vibespecs/guide/child.xml` —
    // a directory (`guide`) whose name prefixes a sibling file (`guide.xml`).
    // Host `Path` order compares component-wise and puts `guide/child.xml`
    // first (`guide` < `guide.xml`); the canonical flattened forward-slash
    // order puts `guide.xml` first (`.` sorts before `/`). The payload hash,
    // the persisted rows, and verification must all consume the flattened
    // order — hashing in component order desyncs `derived_hash` from
    // `compute_recorded_payload_hash` the moment the record is written.
    write(
        source.path(),
        crate::layout_paths::specs_path("guide.md"),
        "# Guide\n\nText.\n",
    );
    write(
        source.path(),
        crate::layout_paths::specs_path("guide/child.md"),
        "# Child\n\nText.\n",
    );

    materialise_with_spec_format(
        workspace.path(),
        &group("org.example"),
        "order",
        &version("1.0.0"),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &source_hash(),
    )
    .unwrap();

    let slot = slot_abs_path(
        workspace.path(),
        &group("org.example"),
        "order",
        &version("1.0.0"),
    );
    let record = read_slot_record(&slot).unwrap();
    assert_eq!(
        record
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        [
            crate::layout_paths::specs("guide.xml"),
            crate::layout_paths::specs("guide/child.xml"),
        ],
        "rows are pinned in ascending flattened forward-slash order"
    );
    assert_eq!(
        record.derived_hash.as_ref().unwrap(),
        &compute_recorded_payload_hash(&slot, &record.files).unwrap(),
        "derived_hash must be recomputable from the persisted row order"
    );
}

#[test]
fn unrecorded_outputs_stay_outside_record_identity_and_verification() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(
        source.path(),
        crate::layout_paths::specs_path("a.md"),
        "# A\n",
    );
    materialise_with_spec_format(
        workspace.path(),
        &group("org.vibevm"),
        "identity",
        &version("1.0.0"),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &source_hash(),
    )
    .unwrap();
    let slot = slot_abs_path(
        workspace.path(),
        &group("org.vibevm"),
        "identity",
        &version("1.0.0"),
    );
    let record = read_slot_record(&slot).unwrap();
    let before = compute_recorded_payload_hash(&slot, &record.files).unwrap();
    write(&slot, "target/SENTINEL", "build output");
    write(
        &slot,
        vibe_core::layout::current_boot_static_xml(),
        "<generated/>",
    );
    assert!(verify_recorded_files(&slot, &record).is_ok());
    assert_eq!(
        compute_recorded_payload_hash(&slot, &record.files).unwrap(),
        before
    );
}

#[test]
fn generated_boot_artifacts_stay_outside_the_legacy_derived_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let slot = dir.path();
    fs::create_dir_all(slot.join(vibe_core::layout::current_boot_dir())).expect("mkdir");
    fs::write(
        slot.join(crate::layout_paths::specs_path("a.xml")),
        "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
    )
    .unwrap();
    let before = compute_derived_hash(slot).expect("hash");
    fs::write(
        slot.join(vibe_core::layout::current_boot_static_md()),
        "# generated\n",
    )
    .expect("write");
    fs::write(
        slot.join(vibe_core::layout::current_boot_static_xml()),
        "generated XML\n",
    )
    .unwrap();
    fs::write(
        slot.join(vibe_core::layout::current_boot_index()),
        "schema = 1\n",
    )
    .unwrap();
    let after = compute_derived_hash(slot).expect("hash");
    assert_eq!(before, after, "generated artifacts must not move the hash");
    assert!(derived::is_generated_boot_artifact(
        slot,
        &slot.join(vibe_core::layout::current_boot_static_md())
    ));
    assert!(!derived::is_generated_boot_artifact(
        slot,
        &slot.join(crate::layout_paths::boot_path("03-flow.md"))
    ));
}

#[path = "tests/overlay.rs"]
mod overlay;
