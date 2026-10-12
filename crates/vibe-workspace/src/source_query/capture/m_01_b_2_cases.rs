//! B2 current-world checks over actual prepared SafeFS snapshots.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_spec::{DescriptorOutputRow, ValidatedDescriptorMap};

use crate::extension_world::ExtensionWorldEpoch;
use crate::vibedeps::{
    SLOT_RECORD_SCHEMA, SlotFile, SlotFileDisposition, SlotRecord, write_slot_record,
};
use crate::{Workspace, WorkspaceMember};

use super::{
    PreparedOwnerCapture, PreparedWorldWitness, SourceQueryUnavailable, WitnessedSourceWorld,
};

const ROOT: &str =
    "[project]\nname='root'\nversion='1.0.0'\n\n[workspace]\nmembers=['member','broken']\n";
const MEMBER: &str = "[package]\ngroup='org.demo'\nname='member'\nkind='flow'\nversion='1.0.0'\n\n[requires.packages]\n'org.demo/dep'='=1.0.0'\n";
const BROKEN: &str = "[package]\ngroup='org.demo'\nname='broken'\nkind='flow'\nversion='1.0.0'\n";
const DEP: &str = "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n";
const DOC: &str = "# Selected {#selected}\n\nPREPARED_B2\n";

fn snapshot(root: &std::path::Path, relative: &str) -> StableFileSnapshot {
    Project::open(root)
        .unwrap()
        .read_file_snapshot_bounded(relative, 1024 * 1024)
        .unwrap()
        .unwrap()
}

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    workspace: Workspace,
    epoch: ExtensionWorldEpoch,
    captures: Vec<PreparedOwnerCapture>,
    witnesses: PreparedWorldWitness,
    pub(super) dep_root: std::path::PathBuf,
}

pub(super) fn fixture(descriptor: bool) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let member_root = root.path().join("member");
    let broken_root = root.path().join("broken");
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let dep_root = crate::vibedeps::slot_abs_path(root.path(), &group, "dep", &version);
    fs::write(root.path().join("vibe.toml"), ROOT).unwrap();
    for (path, text) in [
        (&member_root, MEMBER),
        (&broken_root, BROKEN),
        (&dep_root, DEP),
    ] {
        fs::create_dir_all(path.join("spec/common")).unwrap();
        fs::write(path.join("vibe.toml"), text).unwrap();
        fs::write(path.join("spec/common/DOC.md"), DOC).unwrap();
    }
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    let lock = format!(
        "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-12T00:00:00Z'\nschema_version=7\n\n[[package]]\nkind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='file:///selected-dep'\ncontent_hash='{hash}'\n"
    );
    fs::write(root.path().join("vibe.lock"), lock).unwrap();
    let dep_manifest = Manifest::parse_str(DEP).unwrap();
    let epoch = ExtensionWorldEpoch::from_resolution(
        root.path(),
        &[crate::install::ResolvedDep {
            kind: PackageKind::Flow,
            group,
            name: "dep".into(),
            version,
            content_dir: dep_root.clone(),
            source_hash: Some(hash.clone()),
            embedded_sources: vec![],
            manifest: dep_manifest,
            requires: vec![],
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        }],
    )
    .unwrap();
    let dep_manifest_snapshot = snapshot(&dep_root, "vibe.toml");
    let dep_output = snapshot(&dep_root, "spec/common/DOC.md");
    let mut slot_records = BTreeMap::new();
    let descriptor_map = if descriptor {
        let record = SlotRecord {
            schema: SLOT_RECORD_SCHEMA,
            source_hash: hash.clone(),
            spec_format: SpecFormat::Mixed,
            converter_recipe: None,
            derived_hash: None,
            overlay_hash: None,
            files: vec![
                SlotFile {
                    path: "spec/common/DOC.md".into(),
                    sha256: dep_output.sha256.clone(),
                    source: None,
                    disposition: None,
                },
                SlotFile {
                    path: "vibe.toml".into(),
                    sha256: dep_manifest_snapshot.sha256.clone(),
                    source: None,
                    disposition: None,
                },
            ],
        };
        write_slot_record(&dep_root, &record).unwrap();
        slot_records.insert(
            ("org.demo".into(), "dep".into()),
            snapshot(&dep_root, ".vibe-slot.toml"),
        );
        Some(
            ValidatedDescriptorMap::from_witnessed_rows(
                dep_root.clone(),
                dep_root.join("spec"),
                hash.as_str().into(),
                dep_manifest_snapshot.sha256.clone(),
                vec![DescriptorOutputRow {
                    output: "spec/common/DOC.md".into(),
                    source: "spec/common/DOC.md".into(),
                    interpretation: None,
                    output_sha256: dep_output.sha256.clone(),
                    document_address: None,
                }],
            )
            .unwrap(),
        )
    } else {
        None
    };
    let captures = vec![
        PreparedOwnerCapture::self_node(
            member_root.clone(),
            snapshot(&member_root, "vibe.toml"),
            BTreeMap::from([(
                "spec/common/DOC.md".into(),
                snapshot(&member_root, "spec/common/DOC.md"),
            )]),
            None,
        ),
        PreparedOwnerCapture::dependency(
            "org.demo",
            "dep",
            dep_root.clone(),
            dep_manifest_snapshot,
            BTreeMap::from([("spec/common/DOC.md".into(), dep_output)]),
            descriptor_map,
        ),
    ];
    let witnesses = PreparedWorldWitness::new(
        Some(snapshot(root.path(), "vibe.lock")),
        BTreeMap::from([
            (".".into(), snapshot(root.path(), "vibe.toml")),
            ("member".into(), snapshot(&member_root, "vibe.toml")),
        ]),
        slot_records,
    );
    let workspace = Workspace {
        root: root.path().to_path_buf(),
        root_manifest: Manifest::parse_str(ROOT).unwrap(),
        members: vec![
            WorkspaceMember {
                rel_path: "member".into(),
                manifest: Manifest::parse_str(MEMBER).unwrap(),
                depth: 0,
                parent: None,
            },
            WorkspaceMember {
                rel_path: "broken".into(),
                manifest: Manifest::parse_str(BROKEN).unwrap(),
                depth: 0,
                parent: None,
            },
        ],
    };
    Fixture {
        root,
        workspace,
        epoch,
        captures,
        witnesses,
        dep_root,
    }
}

pub(super) fn seal(fixture: &Fixture) -> WitnessedSourceWorld {
    WitnessedSourceWorld::from_prepared_node(
        &fixture.workspace,
        "member",
        &fixture.epoch,
        &BTreeSet::from(["org.demo/dep".into()]),
        fixture.captures.clone(),
        fixture.witnesses.clone(),
    )
    .unwrap()
}

#[test]
fn unrelated_broken_member_does_not_poison_selected_point() {
    let fixture = fixture(false);
    let world = seal(&fixture);
    fs::write(fixture.root.path().join("broken/vibe.toml"), "broken = [").unwrap();
    let ready = world.revalidate_current().unwrap();
    assert_eq!(ready.owner_count(), 2);
    assert_eq!(
        ready
            .captured_output(Some(("org.demo", "dep")), "spec/common/DOC.md")
            .unwrap()
            .bytes,
        DOC.as_bytes()
    );
}

#[test]
fn selected_lock_change_refuses_old_world_new_source() {
    let fixture = fixture(false);
    let world = seal(&fixture);
    assert!(world.revalidate_current().is_ok());
    let lock = fs::read_to_string(fixture.root.path().join("vibe.lock")).unwrap();
    fs::write(
        fixture.root.path().join("vibe.lock"),
        lock.replace("version='1.0.0'", "version='2.0.0'"),
    )
    .unwrap();
    assert!(matches!(
        world.revalidate_current(),
        Err(SourceQueryUnavailable::StaleWorld("selected lock"))
    ));
}

#[test]
fn selected_descriptor_record_change_refuses_without_mixing_bytes() {
    let fixture = fixture(true);
    let world = seal(&fixture);
    assert!(world.revalidate_current().is_ok());
    let record = fs::read_to_string(fixture.dep_root.join(".vibe-slot.toml")).unwrap();
    fs::write(
        fixture.dep_root.join(".vibe-slot.toml"),
        record.replace(&"a".repeat(64), &"b".repeat(64)),
    )
    .unwrap();
    assert!(matches!(
        world.revalidate_current(),
        Err(SourceQueryUnavailable::StaleWorld(
            "selected descriptor record"
        ))
    ));
}

#[test]
fn changed_membership_or_selected_output_refuses_without_path_leak() {
    let first = fixture(false);
    let world = seal(&first);
    fs::write(first.dep_root.join("spec/common/DOC.md"), "changed").unwrap();
    assert!(matches!(
        world.revalidate_current(),
        Err(SourceQueryUnavailable::StaleWorld("selected output"))
    ));
    let fixture = fixture(false);
    let world = seal(&fixture);
    fs::write(
        fixture.root.path().join("vibe.toml"),
        ROOT.replace("'member','broken'", "'broken'"),
    )
    .unwrap();
    let error = world.revalidate_current().unwrap_err();
    assert!(matches!(
        error,
        SourceQueryUnavailable::StaleWorld("membership manifest")
    ));
    assert!(
        !error
            .to_string()
            .contains(fixture.root.path().to_string_lossy().as_ref())
    );
}

#[test]
fn mutation_between_observation_passes_cannot_publish_a_mixed_world() {
    let fixture = fixture(false);
    let world = seal(&fixture);
    let lock_path = fixture.root.path().join("vibe.lock");
    let lock = fs::read_to_string(&lock_path).unwrap();
    let result = world.revalidate_with(|| {
        fs::write(
            &lock_path,
            lock.replace("version='1.0.0'", "version='2.0.0'"),
        )
        .unwrap();
        fs::write(fixture.dep_root.join("spec/common/DOC.md"), "new source").unwrap();
    });
    assert!(matches!(result, Err(SourceQueryUnavailable::StaleWorld(_))));
}

#[test]
fn disappeared_selected_manifest_is_unavailable_without_path_leak() {
    let fixture = fixture(false);
    let world = seal(&fixture);
    fs::remove_file(fixture.dep_root.join("vibe.toml")).unwrap();
    let error = world.revalidate_current().unwrap_err();
    assert!(matches!(
        error,
        SourceQueryUnavailable::WorldUnavailable("selected manifest")
    ));
    assert!(
        !error
            .to_string()
            .contains(fixture.root.path().to_string_lossy().as_ref())
    );
}

#[test]
fn changed_authored_source_descriptor_refuses_selected_world() {
    let fixture = fixture(true);
    let world = seal(&fixture);
    let edited = format!(
        "{DEP}\n[[package.sources]]\nfile='spec/common/DOC.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n"
    );
    let parsed = Manifest::parse_str(&edited).unwrap();
    assert!(matches!(
        parsed.package.unwrap().select_source("spec/common/DOC.md"),
        Ok(vibe_core::manifest::SelectedSource::Descriptor { .. })
    ));
    fs::write(fixture.dep_root.join("vibe.toml"), edited).unwrap();
    assert!(matches!(
        world.revalidate_current(),
        Err(SourceQueryUnavailable::StaleWorld("selected manifest"))
    ));
}

#[test]
fn changed_lock_materialization_cannot_rebind_an_old_copy_root() {
    let mut fixture = fixture(false);
    let lock_path = fixture.root.path().join("vibe.lock");
    let lock = fs::read_to_string(&lock_path).unwrap();
    fs::write(&lock_path, format!("{lock}materialization='in-place'\n")).unwrap();
    fixture.witnesses.lock = Some(snapshot(fixture.root.path(), "vibe.lock"));
    let result = WitnessedSourceWorld::from_prepared_node(
        &fixture.workspace,
        "member",
        &fixture.epoch,
        &BTreeSet::from(["org.demo/dep".into()]),
        fixture.captures,
        fixture.witnesses,
    );
    assert!(
        matches!(
            result,
            Err(SourceQueryUnavailable::StaleWorld("selected lock row"))
        ),
        "{result:?}"
    );
}

#[test]
fn unchanged_output_hash_cannot_join_different_record_source() {
    let mut fixture = fixture(false);
    let source_a = "spec/common/A.md";
    let source_b = "spec/common/B.md";
    let manifest_text = format!(
        "{DEP}\n[[package.sources]]\nfile='{source_a}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n\n[[package.sources]]\nfile='{source_b}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n"
    );
    let manifest = Manifest::parse_str(&manifest_text).unwrap();
    fs::write(fixture.dep_root.join("vibe.toml"), &manifest_text).unwrap();
    for path in [source_a, source_b] {
        fs::write(fixture.dep_root.join(path), DOC).unwrap();
    }
    let group = Group::parse("org.demo").unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    fixture.epoch = ExtensionWorldEpoch::from_resolution(
        fixture.root.path(),
        &[crate::install::ResolvedDep {
            kind: PackageKind::Flow,
            group,
            name: "dep".into(),
            version: semver::Version::parse("1.0.0").unwrap(),
            content_dir: fixture.dep_root.clone(),
            source_hash: Some(hash.clone()),
            embedded_sources: vec![],
            manifest,
            requires: vec![],
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        }],
    )
    .unwrap();
    let captured_manifest = snapshot(&fixture.dep_root, "vibe.toml");
    let output = snapshot(&fixture.dep_root, "spec/common/DOC.md");
    let record = SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: hash.clone(),
        spec_format: SpecFormat::Markdown,
        converter_recipe: Some("test-recipe".into()),
        derived_hash: Some(hash.clone()),
        overlay_hash: None,
        files: vec![
            SlotFile {
                path: "spec/common/DOC.md".into(),
                sha256: output.sha256.clone(),
                source: Some(source_b.into()),
                disposition: Some(SlotFileDisposition::Converted),
            },
            SlotFile {
                path: "vibe.toml".into(),
                sha256: captured_manifest.sha256.clone(),
                source: Some("vibe.toml".into()),
                disposition: Some(SlotFileDisposition::Copied),
            },
        ],
    };
    write_slot_record(&fixture.dep_root, &record).unwrap();
    fixture.witnesses.slot_records.insert(
        ("org.demo".into(), "dep".into()),
        snapshot(&fixture.dep_root, ".vibe-slot.toml"),
    );
    fixture.captures[1].manifest = captured_manifest.clone();
    fixture.captures[1].descriptor = Some(
        ValidatedDescriptorMap::from_witnessed_rows(
            fixture.dep_root.clone(),
            fixture.dep_root.join("spec"),
            hash.as_str().into(),
            captured_manifest.sha256,
            vec![DescriptorOutputRow {
                output: "spec/common/DOC.md".into(),
                source: source_a.into(),
                interpretation: Some(vibe_core::manifest::SourceInterpretation::Commonmark),
                output_sha256: output.sha256,
                document_address: None,
            }],
        )
        .unwrap(),
    );
    let mut matching = fixture.captures.clone();
    matching[1].descriptor = Some(
        ValidatedDescriptorMap::from_witnessed_rows(
            fixture.dep_root.clone(),
            fixture.dep_root.join("spec"),
            hash.as_str().into(),
            matching[1].manifest.sha256.clone(),
            vec![DescriptorOutputRow {
                output: "spec/common/DOC.md".into(),
                source: source_b.into(),
                interpretation: Some(vibe_core::manifest::SourceInterpretation::Commonmark),
                output_sha256: matching[1].outputs["spec/common/DOC.md"].sha256.clone(),
                document_address: None,
            }],
        )
        .unwrap(),
    );
    assert!(
        WitnessedSourceWorld::from_prepared_node(
            &fixture.workspace,
            "member",
            &fixture.epoch,
            &BTreeSet::from(["org.demo/dep".into()]),
            matching,
            fixture.witnesses.clone(),
        )
        .is_ok()
    );
    let result = WitnessedSourceWorld::from_prepared_node(
        &fixture.workspace,
        "member",
        &fixture.epoch,
        &BTreeSet::from(["org.demo/dep".into()]),
        fixture.captures,
        fixture.witnesses,
    );
    assert!(
        matches!(
            result,
            Err(SourceQueryUnavailable::StaleWorld(
                "selected descriptor record"
            ))
        ),
        "{result:?}"
    );
}

#[test]
fn changed_selected_dependency_edge_pin_cannot_join_old_epoch() {
    let mut fixture = fixture(false);
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let leaf_root = crate::vibedeps::slot_abs_path(fixture.root.path(), &group, "leaf", &version);
    let leaf_text = DEP.replace("name='dep'", "name='leaf'");
    let dep_text = format!("{DEP}\n[requires.packages]\n'org.demo/leaf'='=1.0.0'\n");
    fs::create_dir_all(leaf_root.join("spec/common")).unwrap();
    fs::write(leaf_root.join("vibe.toml"), &leaf_text).unwrap();
    fs::write(leaf_root.join("spec/common/DOC.md"), DOC).unwrap();
    fs::write(fixture.dep_root.join("vibe.toml"), &dep_text).unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    let resolved =
        |name: &str, text: &str, requires: Vec<(Group, String)>| crate::install::ResolvedDep {
            kind: PackageKind::Flow,
            group: group.clone(),
            name: name.into(),
            version: version.clone(),
            content_dir: crate::vibedeps::slot_abs_path(
                fixture.root.path(),
                &group,
                name,
                &version,
            ),
            source_hash: Some(hash.clone()),
            embedded_sources: vec![],
            manifest: Manifest::parse_str(text).unwrap(),
            requires,
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        };
    fixture.epoch = ExtensionWorldEpoch::from_resolution(
        fixture.root.path(),
        &[
            resolved("dep", &dep_text, vec![(group.clone(), "leaf".into())]),
            resolved("leaf", &leaf_text, vec![]),
        ],
    )
    .unwrap();
    fixture.captures[1].manifest = snapshot(&fixture.dep_root, "vibe.toml");
    fixture.captures.push(PreparedOwnerCapture::dependency(
        "org.demo",
        "leaf",
        leaf_root.clone(),
        snapshot(&leaf_root, "vibe.toml"),
        BTreeMap::from([(
            "spec/common/DOC.md".into(),
            snapshot(&leaf_root, "spec/common/DOC.md"),
        )]),
        None,
    ));
    let mut lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(fixture.root.path().join("vibe.lock")).unwrap())
            .unwrap();
    let mut leaf = lock.packages[0].clone();
    leaf.name = vibe_core::PackageName::parse("leaf").unwrap();
    lock.packages.push(leaf.clone());
    lock.packages[0]
        .dependencies
        .push(leaf.as_package_ref().unwrap());
    fs::write(
        fixture.root.path().join("vibe.lock"),
        toml::to_string(&lock).unwrap(),
    )
    .unwrap();
    fixture.witnesses.lock = Some(snapshot(fixture.root.path(), "vibe.lock"));
    assert!(
        WitnessedSourceWorld::from_prepared_node(
            &fixture.workspace,
            "member",
            &fixture.epoch,
            &BTreeSet::from(["org.demo/dep".into()]),
            fixture.captures.clone(),
            fixture.witnesses.clone(),
        )
        .is_ok()
    );
    let mut wrong_pin = leaf;
    wrong_pin.version = semver::Version::parse("2.0.0").unwrap();
    lock.packages[0].dependencies[0] = wrong_pin.as_package_ref().unwrap();
    fs::write(
        fixture.root.path().join("vibe.lock"),
        toml::to_string(&lock).unwrap(),
    )
    .unwrap();
    fixture.witnesses.lock = Some(snapshot(fixture.root.path(), "vibe.lock"));
    let result = WitnessedSourceWorld::from_prepared_node(
        &fixture.workspace,
        "member",
        &fixture.epoch,
        &BTreeSet::from(["org.demo/dep".into()]),
        fixture.captures,
        fixture.witnesses,
    );
    assert!(
        matches!(
            result,
            Err(SourceQueryUnavailable::StaleWorld("selected lock row"))
        ),
        "{result:?}"
    );
}
