//! One installed-slot boot read with separate resource and document identities.

use std::fs;

use tempfile::TempDir;
use vibe_core::manifest::{LinkType, PackageFormat, SpecFormat};
use vibe_core::{ContentHash, Group};
use vibe_spec::{CompilePlans, SelfCoordinate, SpecAddress, TransformPlan};

use super::*;
use crate::boot::{BootBand, BootEntry, BootProvenance};
use crate::{layout_paths, vibedeps};

fn fixture() -> (TempDir, SelectedSlotWitness, EffectiveBoot, String) {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let group = Group::parse("org.example").unwrap();
    let name = "coexist";
    let version = semver::Version::parse("1.1.0").unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let markdown = specs.join("common/guide.md");
    let data = specs.join("common/guide.xml");
    let selected = specs.join("common/rule.xml");
    for path in [&markdown, &data, &selected] {
        fs::create_dir_all(source.path().join(path).parent().unwrap()).unwrap();
    }
    fs::write(
        source.path().join(&markdown),
        b"# Ordinary\n\n#use is literal.\n",
    )
    .unwrap();
    fs::write(
        source.path().join(&data),
        b"<data><!-- keep --> raw </data>\n",
    )
    .unwrap();
    fs::write(
        source.path().join(&selected),
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"impl/done\">selected XML</RULE></p></spec>\n",
    )
    .unwrap();
    let wire = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\nname = '{name}'\ngroup = '{group}'\nkind = 'flow'\nversion = '{version}'\nformat = 'normal'\n\
         [[package.sources]]\nfile = '{}'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'documentation'\nloading = 'scan'\n\
         [[package.sources]]\nfile = '{}'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'scan'\n\
         [[package.sources]]\nfile = '{}'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n",
        wire(&markdown),
        wire(&data),
        wire(&selected)
    );
    fs::write(source.path().join("vibe.toml"), manifest).unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    vibedeps::materialise_with_spec_format(
        workspace.path(),
        &group,
        name,
        &version,
        source.path(),
        vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let slot = vibedeps::slot_rel_path(&group, name, &version);
    let witness = SelectedSlotWitness {
        subskills: Vec::new(),
        group: group.clone(),
        name: name.into(),
        version,
        root: workspace.path().join(&slot),
        source_hash: Some(hash),
        spec_format: SpecFormat::Mixed,
    };
    let path = layout_paths::slot_specs(slot, "common/rule.xml");
    let boot = EffectiveBoot {
        entries: vec![BootEntry {
            path: path.clone(),
            band: BootBand::Dependency,
            link: LinkType::Static,
            when: None,
            origin: format!("{group}/{name}"),
            provenance: BootProvenance::Dependency {
                group,
                name: name.into(),
            },
            use_ref: false,
            format: PackageFormat::Normal,
            unit_substituted: false,
            elided: false,
        }],
    };
    (workspace, witness, boot, path)
}

fn host() -> SelfCoordinate {
    SelfCoordinate::new(Some("org.host".into()), "host".into())
}

fn plans() -> CompilePlans {
    CompilePlans::transforms_only(TransformPlan::empty())
}

#[test]
fn installed_raw_pair_coexists_and_selected_vibe_xml_boot_reads() {
    let (workspace, witness, boot, _) = fixture();
    let resolver = selected_resolver(workspace.path(), &host(), &[witness.clone()]).unwrap();
    let raw = SpecAddress::parse("spec://org.example/coexist/common/guide").unwrap();
    assert!(matches!(
        resolver.resolve_file(&raw),
        Err(vibe_spec::ResolveError::DocNotFound { .. })
    ));
    let selected = SpecAddress::parse("spec://org.example/coexist/common/rule").unwrap();
    assert!(
        resolver
            .resolve_file(&selected)
            .unwrap()
            .ends_with("common/rule.xml")
    );
    let rendered = render_static_observed_selected(
        &boot,
        workspace.path(),
        &host(),
        SpecFormat::Mixed,
        None,
        plans(),
        &[witness],
    )
    .unwrap()
    .unwrap();
    assert!(rendered.contains("selected XML"), "{rendered}");
}

#[test]
fn installed_descriptor_boot_refuses_changed_output_and_missing_independent_hash() {
    let (workspace, mut witness, boot, path) = fixture();
    witness.source_hash = None;
    let missing = render_static_observed_selected(
        &boot,
        workspace.path(),
        &host(),
        SpecFormat::Mixed,
        None,
        plans(),
        &[witness.clone()],
    )
    .unwrap_err()
    .to_string();
    assert!(
        missing.contains("independent selected source hash"),
        "{missing}"
    );
    witness.source_hash = Some(ContentHash::parse(&format!("sha256:{}", "2".repeat(64))).unwrap());
    let foreign = render_static_observed_selected(
        &boot,
        workspace.path(),
        &host(),
        SpecFormat::Mixed,
        None,
        plans(),
        &[witness.clone()],
    )
    .unwrap_err()
    .to_string();
    assert!(
        foreign.contains("source hash or format differs"),
        "{foreign}"
    );
    witness.source_hash = Some(ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap());
    fs::write(workspace.path().join(path), b"changed output\n").unwrap();
    let changed = render_static_observed_selected(
        &boot,
        workspace.path(),
        &host(),
        SpecFormat::Mixed,
        None,
        plans(),
        &[witness],
    )
    .unwrap_err()
    .to_string();
    assert!(changed.contains("changed bytes"), "{changed}");
}

#[test]
fn install_reader_without_descriptors_keeps_legacy_pair_collision() {
    let workspace = TempDir::new().unwrap();
    let group = Group::parse("org.example").unwrap();
    let version = semver::Version::parse("1.1.0").unwrap();
    let slot = workspace
        .path()
        .join(vibedeps::slot_rel_path(&group, "legacy", &version));
    let specs = slot
        .join(vibe_core::layout::current_specs_root())
        .join("common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        slot.join("vibe.toml"),
        "[package]\nname='legacy'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\n",
    )
    .unwrap();
    fs::write(specs.join("pair.md"), b"# Pair\n").unwrap();
    fs::write(specs.join("pair.xml"), b"<data/>\n").unwrap();
    let witness = SelectedSlotWitness {
        subskills: Vec::new(),
        group,
        name: "legacy".into(),
        version,
        root: slot,
        source_hash: None,
        spec_format: SpecFormat::Mixed,
    };
    let resolver = selected_resolver(workspace.path(), &host(), &[witness]).unwrap();
    let address = SpecAddress::parse("spec://org.example/legacy/common/pair").unwrap();
    assert!(matches!(
        resolver.resolve_file(&address),
        Err(vibe_spec::ResolveError::PairCollision { .. })
    ));
}
