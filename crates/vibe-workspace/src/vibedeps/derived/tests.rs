//! Focused descriptor-owned derived materialization cases.

use std::fs;
use std::path::Path;

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

fn descriptor_manifest(name: &str, rows: &str) -> String {
    format!(
        "[package]\nname = '{name}'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.1.0'\nformat = 'normal'\n{rows}"
    )
}

fn descriptor_slot(
    workspace: &Path,
    source: &Path,
    name: &str,
    format: SpecFormat,
) -> Result<PathBuf, WorkspaceError> {
    let version = version("1.1.0");
    materialise_with_spec_format(
        workspace,
        &group("org.example"),
        name,
        &version,
        source,
        CopyMode::Copy,
        format,
        &source_hash(),
    )?;
    Ok(slot_abs_path(
        workspace,
        &group("org.example"),
        name,
        &version,
    ))
}

#[test]
fn descriptor_same_stem_commonmark_and_xml_data_coexist_byte_exact() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let md_rel = crate::layout_paths::specs_path("guide.md");
    let xml_rel = crate::layout_paths::specs_path("guide.xml");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n\n\
         [[package.sources]]\nfile = '{}'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'scan'\n",
        path_to_slash(&md_rel),
        path_to_slash(&xml_rel)
    );
    let manifest = descriptor_manifest("coexist", &rows);
    write(source.path(), "vibe.toml", &manifest);
    let ordinary = b"# Guide\n\n@fact:LOOKS-VIBE text. #use remains literal.\n";
    let data = b"<?xml version='1.0'?><!-- keep -->\n<spec xmlns='https://vibevm.org/spec/1'>  raw  </spec>\n";
    fs::create_dir_all(source.path().join(md_rel.parent().unwrap())).unwrap();
    fs::write(source.path().join(&md_rel), ordinary).unwrap();
    fs::write(source.path().join(&xml_rel), data).unwrap();
    let slot =
        descriptor_slot(workspace.path(), source.path(), "coexist", SpecFormat::Xml).unwrap();
    assert_eq!(fs::read(slot.join(&md_rel)).unwrap(), ordinary);
    assert_eq!(fs::read(slot.join(&xml_rel)).unwrap(), data);
    assert_eq!(
        fs::read(slot.join("vibe.toml")).unwrap(),
        manifest.as_bytes()
    );
    let record = read_slot_record(&slot).unwrap();
    assert!(
        record
            .files
            .iter()
            .all(|file| file.disposition == Some(SlotFileDisposition::Copied))
    );
    let map = crate::vibedeps::read_validated_descriptor_map(
        &slot,
        &group("org.example"),
        "coexist",
        &version("1.1.0"),
        &source_hash(),
        SpecFormat::Xml,
    )
    .unwrap()
    .unwrap();
    let specs = map.spec_root().to_owned();
    let resolver = vibe_spec::FileResolver::new(
        &slot,
        vibe_spec::SelfCoordinate::new(Some("org.example".into()), "coexist".into()),
    )
    .with_descriptor_source_map(specs, Some(map))
    .unwrap();
    let address = vibe_spec::SpecAddress::parse("spec://org.example/coexist/guide").unwrap();
    assert!(matches!(
        resolver.resolve_file(&address),
        Err(vibe_spec::ResolveError::DocNotFound { .. })
    ));
    let limited = crate::vibedeps::descriptor_map::read_validated_descriptor_map_with_limits(
        &slot,
        &group("org.example"),
        "coexist",
        &version("1.1.0"),
        Some(&source_hash()),
        SpecFormat::Xml,
        crate::vibedeps::descriptor_map::DescriptorMapLimits {
            max_outputs: 1,
            max_total_bytes: 256 * 1024 * 1024,
        },
    )
    .unwrap_err();
    assert!(
        limited.contains("more than 1 selected outputs"),
        "{limited}"
    );
    let limited = crate::vibedeps::descriptor_map::read_validated_descriptor_map_with_limits(
        &slot,
        &group("org.example"),
        "coexist",
        &version("1.1.0"),
        Some(&source_hash()),
        SpecFormat::Xml,
        crate::vibedeps::descriptor_map::DescriptorMapLimits {
            max_outputs: 4096,
            max_total_bytes: 1,
        },
    )
    .unwrap_err();
    assert!(limited.contains("1-byte cap"), "{limited}");
    let foreign_hash = ContentHash::parse(&format!("sha256:{}", "f".repeat(64))).unwrap();
    assert!(
        crate::vibedeps::read_validated_descriptor_map(
            &slot,
            &group("org.example"),
            "coexist",
            &version("1.1.0"),
            &foreign_hash,
            SpecFormat::Xml,
        )
        .is_err()
    );
    fs::write(slot.join(&xml_rel), b"changed output").unwrap();
    assert!(
        crate::vibedeps::read_validated_descriptor_map(
            &slot,
            &group("org.example"),
            "coexist",
            &version("1.1.0"),
            &source_hash(),
            SpecFormat::Xml,
        )
        .is_err()
    );
}

#[test]
fn descriptor_opaque_invalid_xml_stays_raw_in_markdown_slot() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let rel = crate::layout_paths::specs_path("opaque.xml");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n",
        path_to_slash(&rel)
    );
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("opaque", &rows),
    );
    fs::create_dir_all(source.path().join(rel.parent().unwrap())).unwrap();
    let bytes = b"\xff\0<not-closed";
    fs::write(source.path().join(&rel), bytes).unwrap();
    let slot = descriptor_slot(
        workspace.path(),
        source.path(),
        "opaque",
        SpecFormat::Markdown,
    )
    .unwrap();
    assert_eq!(fs::read(slot.join(&rel)).unwrap(), bytes);
}

#[test]
fn descriptor_vibe_spec_converts_by_declared_media() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let rel = crate::layout_paths::specs_path("rule.md");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
        path_to_slash(&rel)
    );
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("convert", &rows),
    );
    write(
        source.path(),
        &rel,
        "# Rule\n\n@fact:ONE A real fact. @status:impl/done\n",
    );
    let slot =
        descriptor_slot(workspace.path(), source.path(), "convert", SpecFormat::Xml).unwrap();
    let output = rel.with_extension("xml");
    assert!(vibe_specdoc::from_xml(&fs::read_to_string(slot.join(&output)).unwrap()).is_ok());
    assert!(!slot.join(&rel).exists());
    let record = read_slot_record(&slot).unwrap();
    assert!(record.files.iter().any(|file| file.source.as_deref()
        == Some(path_to_slash(&rel).as_str())
        && file.path == path_to_slash(&output)
        && file.disposition == Some(SlotFileDisposition::Converted)));
    let map = crate::vibedeps::read_validated_descriptor_map(
        &slot,
        &group("org.example"),
        "convert",
        &version("1.1.0"),
        &source_hash(),
        SpecFormat::Xml,
    )
    .unwrap()
    .unwrap();
    assert_eq!(map.source_witness(), source_hash().as_str());
    let specs = map.spec_root().to_owned();
    let resolver = vibe_spec::FileResolver::new(
        &slot,
        vibe_spec::SelfCoordinate::new(Some("org.example".into()), "convert".into()),
    )
    .with_descriptor_source_map(specs, Some(map))
    .unwrap();
    let address = vibe_spec::SpecAddress::parse("spec://org.example/convert/rule").unwrap();
    assert_eq!(resolver.resolve_file(&address).unwrap(), slot.join(output));
}

#[test]
fn declared_media_overrides_a_misleading_source_suffix() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let rel = crate::layout_paths::specs_path("misnamed.md");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
        path_to_slash(&rel)
    );
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("misnamed", &rows),
    );
    let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown("# Rule\n\nBody.\n").unwrap());
    write(source.path(), &rel, &xml);
    let slot = descriptor_slot(
        workspace.path(),
        source.path(),
        "misnamed",
        SpecFormat::Markdown,
    )
    .unwrap();
    let output = fs::read_to_string(slot.join(&rel)).unwrap();
    assert_ne!(output, xml);
    assert!(vibe_specdoc::from_markdown(&output).is_ok());
    let record = read_slot_record(&slot).unwrap();
    assert!(
        record
            .files
            .iter()
            .any(|file| file.path == path_to_slash(&rel)
                && file.disposition == Some(SlotFileDisposition::Converted))
    );
}

#[test]
fn descriptor_output_collision_refuses_before_slot_publication() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let md = crate::layout_paths::specs_path("same.md");
    let xml = crate::layout_paths::specs_path("same.xml");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n\n\
         [[package.sources]]\nfile = '{}'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'scan'\n",
        path_to_slash(&md),
        path_to_slash(&xml)
    );
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("collision", &rows),
    );
    write(source.path(), &md, "# Spec\n");
    write(source.path(), &xml, "<data/>");
    let error = descriptor_slot(
        workspace.path(),
        source.path(),
        "collision",
        SpecFormat::Xml,
    )
    .unwrap_err();
    assert!(error.to_string().contains("same output path"), "{error}");
    assert!(
        !slot_abs_path(
            workspace.path(),
            &group("org.example"),
            "collision",
            &version("1.1.0")
        )
        .exists()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn descriptor_portable_case_collision_refuses_before_slot_publication() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let upper = crate::layout_paths::specs_path("A.md");
    let lower = crate::layout_paths::specs_path("a.md");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n\n\
         [[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n",
        path_to_slash(&upper),
        path_to_slash(&lower)
    );
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("case", &rows),
    );
    write(source.path(), &upper, "# Upper\n");
    write(source.path(), &lower, "# Lower\n");
    let error =
        descriptor_slot(workspace.path(), source.path(), "case", SpecFormat::Xml).unwrap_err();
    assert!(error.to_string().contains("portable-case"), "{error}");
    assert!(
        !slot_abs_path(
            workspace.path(),
            &group("org.example"),
            "case",
            &version("1.1.0")
        )
        .exists()
    );
}

#[test]
fn descriptor_mixed_keeps_raw_bytes_and_refuses_unrecorded_slot_claim() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let rel = crate::layout_paths::specs_path("raw.md");
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n",
        path_to_slash(&rel)
    );
    let manifest = descriptor_manifest("mixed", &rows);
    write(source.path(), "vibe.toml", &manifest);
    let raw = b"# Raw\n\n@fact:LOOKALIKE stays text.\n";
    fs::create_dir_all(source.path().join(rel.parent().unwrap())).unwrap();
    fs::write(source.path().join(&rel), raw).unwrap();
    let slot =
        descriptor_slot(workspace.path(), source.path(), "mixed", SpecFormat::Mixed).unwrap();
    assert_eq!(fs::read(slot.join(&rel)).unwrap(), raw);
    assert_eq!(
        fs::read(slot.join("vibe.toml")).unwrap(),
        manifest.as_bytes()
    );
    assert_eq!(
        read_slot_record(&slot).unwrap().spec_format,
        SpecFormat::Mixed
    );

    let other_workspace = TempDir::new().unwrap();
    let occupied = slot_abs_path(
        other_workspace.path(),
        &group("org.example"),
        "mixed",
        &version("1.1.0"),
    );
    fs::create_dir_all(&occupied).unwrap();
    fs::write(occupied.join("sentinel"), b"keep").unwrap();
    let error = descriptor_slot(
        other_workspace.path(),
        source.path(),
        "mixed",
        SpecFormat::Mixed,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("unrecorded existing slot"),
        "{error}"
    );
    assert_eq!(fs::read(occupied.join("sentinel")).unwrap(), b"keep");
}

#[test]
fn selected_vibe_outside_supported_layout_refuses_instead_of_raw_copy() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let rel = Path::new("docs/out.xml");
    let rows = "[[package.sources]]\nfile = 'docs/out.xml'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n";
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("outside", rows),
    );
    write(
        source.path(),
        rel,
        "<spec xmlns='https://vibevm.org/spec/1'/>",
    );
    let error = descriptor_slot(
        workspace.path(),
        source.path(),
        "outside",
        SpecFormat::Markdown,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("outside the supported"),
        "{error}"
    );
    assert!(
        !slot_abs_path(
            workspace.path(),
            &group("org.example"),
            "outside",
            &version("1.1.0")
        )
        .exists()
    );
}

#[test]
fn missing_declared_exact_source_refuses_before_slot_publication() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let rows = format!(
        "[[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
        crate::layout_paths::specs("missing.md")
    );
    write(
        source.path(),
        "vibe.toml",
        &descriptor_manifest("missing", &rows),
    );
    let error =
        descriptor_slot(workspace.path(), source.path(), "missing", SpecFormat::Xml).unwrap_err();
    assert!(
        error.to_string().contains("absent or not a regular file"),
        "{error}"
    );
    assert!(
        !slot_abs_path(
            workspace.path(),
            &group("org.example"),
            "missing",
            &version("1.1.0")
        )
        .exists()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn legacy_hardlinked_manifest_keeps_suffix_conversion_route() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let witness = TempDir::new().unwrap();
    write(source.path(), "vibe.toml", "[package]\nname = 'legacy'\n");
    fs::hard_link(
        source.path().join("vibe.toml"),
        witness.path().join("manifest-link"),
    )
    .unwrap();
    let rel = crate::layout_paths::specs_path("legacy.md");
    write(source.path(), &rel, "# Legacy\n");
    let slot = descriptor_slot(workspace.path(), source.path(), "legacy", SpecFormat::Xml).unwrap();
    assert!(slot.join(rel.with_extension("xml")).is_file());
}
