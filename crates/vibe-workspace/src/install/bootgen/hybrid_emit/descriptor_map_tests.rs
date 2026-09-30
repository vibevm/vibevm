//! Exact selected-slot proof through the real per-unit emitter.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{LinkType, Manifest, PackageFormat, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_spec::SelfCoordinate;

use super::*;
use crate::boot::hybrid::{UnitEdge, UnitInput};
use crate::extension_world::{ExtensionWorldEpoch, OwnerRuntimeLowering, lower_owner_runtimes};
use crate::install::ResolvedDep;
use crate::{Workspace, layout_paths};

struct Fixture {
    workspace: TempDir,
    resolution: Vec<ResolvedDep>,
    table: HashMap<UnitId, UnitInput>,
    parent_static: PathBuf,
    parent_rule: PathBuf,
    raw_markdown: PathBuf,
    raw_xml: PathBuf,
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn fixture(descriptor: bool) -> Fixture {
    let workspace = TempDir::new().unwrap();
    write(
        &workspace.path().join("vibe.toml"),
        "[project]\ngroup='org.host'\nname='root'\nversion='1.1.0'\n",
    );
    let source = TempDir::new().unwrap();
    let group = Group::parse("org.example").unwrap();
    let version: semver::Version = "1.1.0".parse().unwrap();
    let parent_specs = vibe_core::layout::current_specs_root();
    let rule = parent_specs.join(if descriptor {
        "boot/rule.xml"
    } else {
        "boot/rule.md"
    });
    let guide_md = parent_specs.join("boot/guide.md");
    let guide_xml = parent_specs.join("boot/guide.xml");
    write(
        &source.path().join(&rule),
        if descriptor {
            b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"impl/done\">selected XML</RULE></p></spec>\n".as_slice()
        } else {
            b"# Rule {#root}\n\nlegacy body\n".as_slice()
        },
    );
    write(
        &source.path().join(&guide_md),
        b"# Ordinary\n\n#use stays literal.\n",
    );
    write(
        &source.path().join(&guide_xml),
        b"<data><!-- exact --> raw </data>\n",
    );
    let fwd = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let declarations = if descriptor {
        format!(
            "[[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
             [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n\
             [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
            fwd(&guide_md),
            fwd(&guide_xml),
            fwd(&rule),
        )
    } else {
        String::new()
    };
    let parent_manifest = format!(
        "[package]\ngroup='org.example'\nname='parent'\nkind='flow'\nversion='1.1.0'\nformat='normal'\n{declarations}"
    );
    write(&source.path().join("vibe.toml"), &parent_manifest);
    let parent_hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    vibedeps::materialise_with_spec_format(
        workspace.path(),
        &group,
        "parent",
        &version,
        source.path(),
        vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &parent_hash,
    )
    .unwrap();
    let parent_slot = vibedeps::slot_rel_path(&group, "parent", &version);
    let child_slot = vibedeps::slot_rel_path(&group, "child", &version);
    let child_root = workspace.path().join(&child_slot);
    write(
        &child_root.join("vibe.toml"),
        "[package]\ngroup='org.example'\nname='child'\nkind='flow'\nversion='1.1.0'\nformat='simple'\n",
    );
    let child_rule = vibe_core::layout::current_specs_root().join("boot/leaf.md");
    write(
        &child_root.join(&child_rule),
        "# Leaf {#root}\n\nleaf body\n",
    );
    let parent_id = (group.clone(), "parent".into());
    let child_id = (group.clone(), "child".into());
    let parent_boot = layout_paths::slot_specs(
        &parent_slot,
        if descriptor {
            "boot/rule.xml"
        } else {
            "boot/rule.md"
        },
    );
    let child_boot = layout_paths::slot_specs(&child_slot, "boot/leaf.md");
    let table = HashMap::from([
        (
            parent_id.clone(),
            UnitInput {
                own_boot_path: Some(parent_boot),
                fragments: Vec::new(),
                origin: "org.example/parent".into(),
                when: None,
                edges: vec![UnitEdge {
                    target: child_id.clone(),
                    link: LinkType::Static,
                }],
                format: PackageFormat::Normal,
            },
        ),
        (
            child_id,
            UnitInput {
                own_boot_path: Some(child_boot),
                fragments: Vec::new(),
                origin: "org.example/child".into(),
                when: None,
                edges: Vec::new(),
                format: PackageFormat::Simple,
            },
        ),
    ]);
    let parent_root = workspace.path().join(&parent_slot);
    let parent = ResolvedDep {
        kind: PackageKind::Flow,
        group: group.clone(),
        name: "parent".into(),
        version: version.clone(),
        content_dir: parent_root.clone(),
        source_hash: Some(parent_hash),
        embedded_sources: Vec::new(),
        manifest: Manifest::parse_str(&parent_manifest).unwrap(),
        requires: vec![(group.clone(), "child".into())],
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    };
    let child = ResolvedDep {
        kind: PackageKind::Flow,
        group,
        name: "child".into(),
        version,
        content_dir: child_root.clone(),
        source_hash: Some(ContentHash::parse(&format!("sha256:{}", "2".repeat(64))).unwrap()),
        embedded_sources: Vec::new(),
        manifest: Manifest::read(child_root.join("vibe.toml")).unwrap(),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    };
    Fixture {
        parent_static: parent_root
            .join(vibe_core::layout::current_boot_dir())
            .join(boot_artifacts::static_file(SpecFormat::Mixed)),
        parent_rule: parent_root.join(rule),
        raw_markdown: parent_root.join(guide_md),
        raw_xml: parent_root.join(guide_xml),
        workspace,
        resolution: vec![parent, child],
        table,
    }
}

fn emit(fixture: &Fixture) -> Result<HashSet<UnitId>, WorkspaceError> {
    let workspace = Workspace::load(fixture.workspace.path()).unwrap();
    let world = ExtensionWorldEpoch::from_resolution(fixture.workspace.path(), &fixture.resolution)
        .map_err(|error| WorkspaceError::InlineCompile {
            reason: error.to_string(),
        })?;
    let runtimes = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )?;
    emit_package_units(
        fixture.workspace.path(),
        &SelfCoordinate::new(Some("org.host".into()), "root".into()),
        &fixture.resolution,
        &fixture.table,
        &HashSet::new(),
        &HashMap::new(),
        SpecFormat::Mixed,
        None,
        &runtimes,
    )
}

#[test]
fn emitter_reads_selected_vibe_xml_beside_raw_same_stem_resources() {
    let fixture = fixture(true);
    let markdown = fs::read(&fixture.raw_markdown).unwrap();
    let xml_data = fs::read(&fixture.raw_xml).unwrap();
    let emitted = emit(&fixture).unwrap();
    assert!(emitted.contains(&(Group::parse("org.example").unwrap(), "parent".into())));
    let text = fs::read_to_string(&fixture.parent_static).unwrap();
    assert!(text.contains("selected XML"), "{text}");
    assert_eq!(fs::read(&fixture.raw_markdown).unwrap(), markdown);
    assert_eq!(fs::read(&fixture.raw_xml).unwrap(), xml_data);
}

#[test]
fn bootgen_preflight_and_emitter_refuse_unwitnessed_or_changed_descriptor() {
    let mut fixture = fixture(true);
    // The ordinary bootgen world owner rejects a missing hash before it can
    // call the emitter. A supplied but foreign hash reaches the emitter's
    // independent slot-record comparison below.
    fixture.resolution[0].source_hash = None;
    let missing = emit(&fixture).unwrap_err().to_string();
    assert!(missing.contains("content hash"), "{missing}");
    assert!(!fixture.parent_static.exists());
    fixture.resolution[0].source_hash =
        Some(ContentHash::parse(&format!("sha256:{}", "2".repeat(64))).unwrap());
    let foreign = emit(&fixture).unwrap_err().to_string();
    assert!(
        foreign.contains("source hash or format differs"),
        "{foreign}"
    );
    assert!(!fixture.parent_static.exists());
    fixture.resolution[0].source_hash =
        Some(ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap());
    fs::write(&fixture.parent_rule, b"changed XML output\n").unwrap();
    let changed = emit(&fixture).unwrap_err().to_string();
    assert!(changed.contains("changed bytes"), "{changed}");
    assert!(!fixture.parent_static.exists());
}

#[test]
fn emitter_with_only_legacy_packages_keeps_existing_static_output() {
    let fixture = fixture(false);
    let parent = (Group::parse("org.example").unwrap(), "parent".into());
    let slots = fixture
        .resolution
        .iter()
        .map(|dep| ((dep.group.clone(), dep.name.clone()), slot_rel_path(dep)))
        .collect::<HashMap<_, _>>();
    let effective = zone_to_effective(
        &parent,
        &hybrid::resolve_zone(&parent, &fixture.table),
        &fixture.table,
        &with_static_set(&fixture.table),
        &slots,
        &HashSet::new(),
        SpecFormat::Mixed,
    );
    let expected = boot_artifacts::render_static_observed(
        &effective,
        fixture.workspace.path(),
        &SelfCoordinate::new(Some("org.host".into()), "root".into()),
        SpecFormat::Mixed,
        None,
        vibe_spec::CompilePlans::transforms_only(vibe_spec::TransformPlan::empty()),
    )
    .unwrap()
    .unwrap();
    emit(&fixture).unwrap();
    let text = fs::read_to_string(&fixture.parent_static).unwrap();
    assert_eq!(text, expected);
    assert!(text.contains("legacy body"), "{text}");
    assert!(text.contains("leaf body"), "{text}");
}
