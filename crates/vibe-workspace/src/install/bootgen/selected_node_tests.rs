//! Selected descriptor bytes through the actual bound node writer and analyzer.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};

use super::native_managed::regenerate_boot_from_bound_native;
use super::{ResolvedDep, analyze_node_lane_bound_native};
use crate::boot_artifacts::native_managed_tests::FakeProvider;
use crate::extension_world::{
    ExtensionWorldEpoch, OwnerRuntimeEpoch, OwnerRuntimeLowering, OwnerRuntimeRunFacts,
    lower_owner_runtimes,
};
use crate::{Workspace, vibedeps};

const VIBE_XML: &[u8] = b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"impl/done\">selected XML</RULE></p></spec>\n";
const RAW_MD: &[u8] = b"# Ordinary\n\n#use is literal.\n";
const RAW_XML: &[u8] = b"<data><!-- keep --> raw </data>\n";

struct World {
    root: TempDir,
    resolution: Vec<ResolvedDep>,
    slot: PathBuf,
    rule: PathBuf,
    raw_md: PathBuf,
    raw_xml: PathBuf,
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn world() -> World {
    let root = tempfile::tempdir().unwrap();
    write(
        &root.path().join("vibe.toml"),
        "[project]\ngroup='org.host'\nname='root'\nversion='1.1.0'\n\n[requires.packages]\n'flow:org.example/selected'='=1.1.0'\n",
    );
    let source = tempfile::tempdir().unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let rule = specs.join("boot/guide.xml");
    let raw_md = specs.join("boot/guide.md");
    let raw_xml = specs.join("boot/data.xml");
    write(&source.path().join(&rule), VIBE_XML);
    write(&source.path().join(&raw_md), RAW_MD);
    write(&source.path().join(&raw_xml), RAW_XML);
    let slash = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\ngroup='org.example'\nname='selected'\nkind='flow'\nversion='1.1.0'\nformat='normal'\n\
         [boot_snippet]\nsource='{}'\ncategory='flow'\nlink='static'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
        slash(&rule),
        slash(&rule),
        slash(&raw_md),
        slash(&raw_xml),
    );
    write(&source.path().join("vibe.toml"), &manifest);
    let group = Group::parse("org.example").unwrap();
    let version: semver::Version = "1.1.0".parse().unwrap();
    let source_hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "selected",
        &version,
        source.path(),
        vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &source_hash,
    )
    .unwrap();
    let slot = vibedeps::slot_abs_path(root.path(), &group, "selected", &version);
    let resolution = vec![ResolvedDep {
        kind: PackageKind::Flow,
        group,
        name: "selected".into(),
        version,
        content_dir: slot.clone(),
        source_hash: Some(source_hash),
        embedded_sources: Vec::new(),
        manifest: Manifest::parse_str(&manifest).unwrap(),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    }];
    World {
        rule: slot.join(rule),
        raw_md: slot.join(raw_md),
        raw_xml: slot.join(raw_xml),
        root,
        resolution,
        slot,
    }
}

fn selected_epoch(world: &World) -> (Workspace, OwnerRuntimeEpoch) {
    let workspace = Workspace::load(world.root.path()).unwrap();
    let extension =
        ExtensionWorldEpoch::from_resolution(world.root.path(), &world.resolution).unwrap();
    let lowered = lower_owner_runtimes(
        &workspace,
        &extension,
        OwnerRuntimeLowering::new(".", BTreeMap::new()),
    )
    .unwrap();
    let epoch = lowered.bind_run(OwnerRuntimeRunFacts {
        run_id: "8123456789abcdef0123456789abcdef".into(),
        state_root: world.root.path().join(".vibe"),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        offline: true,
        created_at: "2026-10-01T00:00:00Z".into(),
    });
    (workspace, epoch)
}

#[test]
fn bound_node_writer_and_analyzer_use_one_selected_vibe_xml_source() {
    let world = world();
    let (workspace, epoch) = selected_epoch(&world);
    let raw_md = fs::read(&world.raw_md).unwrap();
    let raw_xml = fs::read(&world.raw_xml).unwrap();
    let written = regenerate_boot_from_bound_native(
        &workspace,
        &world.resolution,
        SpecFormat::Mixed,
        None,
        &epoch,
        None::<&mut FakeProvider>,
    )
    .unwrap();
    assert!(written.nodes.iter().any(|node| node == "."));
    let static_path = world
        .root
        .path()
        .join(vibe_core::layout::current_boot_dir())
        .join("STATIC.md");
    let static_bytes = fs::read(&static_path).unwrap();
    assert!(String::from_utf8_lossy(&static_bytes).contains("selected XML"));
    let analyzed = analyze_node_lane_bound_native(
        &workspace,
        ".",
        &world.resolution,
        &epoch,
        None::<&mut FakeProvider>,
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(analyzed.artifact.bytes(), static_bytes);
    assert_eq!(fs::read(&world.raw_md).unwrap(), raw_md);
    assert_eq!(fs::read(&world.raw_xml).unwrap(), raw_xml);
}

#[test]
fn bound_node_writer_refuses_changed_selected_output_before_static_publication() {
    let world = world();
    fs::write(&world.rule, b"changed XML\n").unwrap();
    let (workspace, epoch) = selected_epoch(&world);
    let error = match regenerate_boot_from_bound_native(
        &workspace,
        &world.resolution,
        SpecFormat::Mixed,
        None,
        &epoch,
        None::<&mut FakeProvider>,
    ) {
        Ok(_) => panic!("changed selected output must refuse"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("changed bytes"), "{error}");
    let analyzed = match analyze_node_lane_bound_native(
        &workspace,
        ".",
        &world.resolution,
        &epoch,
        None::<&mut FakeProvider>,
        None,
    ) {
        Ok(_) => panic!("analyzer must refuse changed selected output"),
        Err(error) => error.to_string(),
    };
    assert!(analyzed.contains("changed bytes"), "{analyzed}");
    assert!(
        !world
            .root
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("STATIC.md")
            .exists()
    );
    assert_eq!(fs::read(&world.raw_md).unwrap(), RAW_MD);
    assert_eq!(fs::read(&world.raw_xml).unwrap(), RAW_XML);
    assert!(world.slot.exists());
}

#[test]
fn bound_node_writer_refuses_missing_or_foreign_selected_hash() {
    for foreign in [false, true] {
        let mut world = world();
        world.resolution[0].source_hash =
            foreign.then(|| ContentHash::parse(&format!("sha256:{}", "2".repeat(64))).unwrap());
        if !foreign {
            let refused =
                match ExtensionWorldEpoch::from_resolution(world.root.path(), &world.resolution) {
                    Ok(_) => panic!("descriptor without selected hash must refuse at epoch"),
                    Err(error) => error.to_string(),
                };
            assert!(refused.contains("content hash"), "{refused}");
            assert!(
                !world
                    .root
                    .path()
                    .join(vibe_core::layout::current_boot_dir())
                    .join("STATIC.md")
                    .exists()
            );
            continue;
        }
        let (workspace, epoch) = selected_epoch(&world);
        let refused = match regenerate_boot_from_bound_native(
            &workspace,
            &world.resolution,
            SpecFormat::Mixed,
            None,
            &epoch,
            None::<&mut FakeProvider>,
        ) {
            Ok(_) => panic!("unwitnessed selected descriptor must refuse"),
            Err(error) => error.to_string(),
        };
        assert!(
            refused.contains("source hash") || refused.contains("content hash"),
            "{refused}"
        );
        assert!(
            !world
                .root
                .path()
                .join(vibe_core::layout::current_boot_dir())
                .join("STATIC.md")
                .exists()
        );
    }
}
