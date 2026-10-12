//! G.3 generated boot outputs stay outside selected source payloads.

use super::*;

#[test]
fn xml_derived_materialisation_excludes_generated_source_boot_files() {
    let workspace = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let boot = vibe_core::layout::current_boot_dir();
    std::fs::create_dir_all(source.path().join(&boot)).unwrap();
    std::fs::write(
        source.path().join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'flow'\nkind = 'flow'\nversion = '1.0.0'\n\n[boot_snippet]\nsource = 'vibevm/vibespecs/boot/10-flow.md'\n",
    )
    .unwrap();
    std::fs::write(
        source.path().join(&boot).join("10-flow.md"),
        "# Authored\nAUTHORED_G3\n",
    )
    .unwrap();
    std::fs::write(
        source.path().join(&boot).join("STATIC.xml"),
        "<spec/>\n<spec/>\n",
    )
    .unwrap();
    std::fs::write(
        source.path().join(&boot).join("INDEX.md"),
        "generated index\n",
    )
    .unwrap();
    std::fs::write(
        source.path().join(&boot).join(".vibe-boot-artifacts.lock"),
        "generated lock\n",
    )
    .unwrap();
    let group = vibe_core::Group::parse("org.example").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let hash = vibe_core::ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
    materialise_with_spec_format(
        workspace.path(),
        &group,
        "flow",
        &version,
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &hash,
    )
    .unwrap();
    let slot = slot_abs_path(workspace.path(), &group, "flow", &version);
    let record = read_slot_record(&slot).unwrap();
    assert!(
        record
            .files
            .iter()
            .any(|row| row.path.ends_with("/10-flow.xml"))
    );
    for row in &record.files {
        assert!(
            !row.path.ends_with("/STATIC.xml")
                && !row.path.ends_with("/INDEX.md")
                && !row.path.contains(".vibe-boot-"),
            "generated output was selected as source: {row:?}"
        );
    }
}
