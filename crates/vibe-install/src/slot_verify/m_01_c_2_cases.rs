//! C2: a fresh selected world may be locally consistent with coordinated
//! slot tampering, but cannot authenticate itself as original source.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind, PackageRef};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_workspace::Workspace;
use vibe_workspace::extension_world::ExtensionWorldEpoch;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::source_query::{
    CapturedSourceFile, PreparedOwnerCapture, PreparedWorldWitness, SourceMembershipIndex,
    SourceQueryUnavailable, SourceStrength, WitnessedSourceWorld,
};
use vibe_workspace::vibedeps::{
    SLOT_RECORD_SCHEMA, SlotFile, SlotRecord, read_validated_descriptor_map, write_slot_record,
};

use super::{VerifiedSourceMembership, source_strength, verify_original_source_capture};

const PACKAGE: &str = "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n\n[[package.sources]]\nfile='spec/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n";
const ROOT: &str =
    "[project]\nname='root'\nversion='1.0.0'\n\n[requires.packages]\n'org.demo/dep'='=1.0.0'\n";
const ORIGINAL: &[u8] = b"# Original {#root}\n\nSOURCE_MEMBER_C2\n";
const TAMPERED: &[u8] = b"# Original {#root}\n\nCOORDINATED_TAMPER\n";

fn snapshot(root: &Path, relative: &str) -> StableFileSnapshot {
    Project::open(root)
        .unwrap()
        .read_file_snapshot_bounded(relative, 1024 * 1024)
        .unwrap()
        .unwrap()
}

fn original_capture(root: &Path) -> Vec<CapturedSourceFile> {
    ["spec/DOC.md", "vibe.toml"]
        .into_iter()
        .map(|relative| CapturedSourceFile::new(relative.into(), snapshot(root, relative)).unwrap())
        .collect()
}

struct Fixture {
    _source: TempDir,
    _project: TempDir,
    slot: PathBuf,
    world: WitnessedSourceWorld,
    genuine: VerifiedSourceMembership,
    pin: PackageRef,
    selected_hash: ContentHash,
}

fn fixture(slot_doc: &[u8]) -> Fixture {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("spec")).unwrap();
    fs::write(source.path().join("vibe.toml"), PACKAGE).unwrap();
    fs::write(source.path().join("spec/DOC.md"), ORIGINAL).unwrap();
    let pin = PackageRef::parse("flow:org.demo/dep@=1.0.0").unwrap();
    let selected_hash =
        ContentHash::parse(&vibe_registry::compute_content_hash(source.path()).unwrap()).unwrap();
    let genuine = verify_original_source_capture(
        source.path(),
        pin.clone(),
        selected_hash.clone(),
        original_capture(source.path()),
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("spec")).unwrap();
    fs::write(project.path().join("vibe.toml"), ROOT).unwrap();
    fs::write(project.path().join("spec/self.md"), b"# Self\n").unwrap();
    let workspace = Workspace {
        root: project.path().to_path_buf(),
        root_manifest: Manifest::parse_str(ROOT).unwrap(),
        members: vec![],
    };
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let slot = workspace.vibedeps_slot(&group, "dep", &version);
    fs::create_dir_all(slot.join("spec")).unwrap();
    fs::write(slot.join("vibe.toml"), PACKAGE).unwrap();
    fs::write(slot.join("spec/DOC.md"), slot_doc).unwrap();
    let manifest_snapshot = snapshot(&slot, "vibe.toml");
    let output_snapshot = snapshot(&slot, "spec/DOC.md");
    write_slot_record(
        &slot,
        &SlotRecord {
            schema: SLOT_RECORD_SCHEMA,
            source_hash: selected_hash.clone(),
            spec_format: SpecFormat::Mixed,
            converter_recipe: None,
            derived_hash: None,
            overlay_hash: None,
            files: vec![
                SlotFile {
                    path: "spec/DOC.md".into(),
                    sha256: output_snapshot.sha256.clone(),
                    source: None,
                    disposition: None,
                },
                SlotFile {
                    path: "vibe.toml".into(),
                    sha256: manifest_snapshot.sha256.clone(),
                    source: None,
                    disposition: None,
                },
            ],
        },
    )
    .unwrap();
    let map = read_validated_descriptor_map(
        &slot,
        &group,
        "dep",
        &version,
        &selected_hash,
        SpecFormat::Mixed,
    )
    .unwrap()
    .unwrap();
    let captures = vec![
        PreparedOwnerCapture::self_node(
            project.path().to_path_buf(),
            snapshot(project.path(), "vibe.toml"),
            BTreeMap::from([(
                "spec/self.md".into(),
                snapshot(project.path(), "spec/self.md"),
            )]),
            None,
        ),
        PreparedOwnerCapture::dependency(
            "org.demo",
            "dep",
            slot.clone(),
            manifest_snapshot,
            BTreeMap::from([("spec/DOC.md".into(), output_snapshot)]),
            Some(map),
        ),
    ];
    let resolved = ResolvedDep {
        kind: PackageKind::Flow,
        group,
        name: "dep".into(),
        version,
        content_dir: slot.clone(),
        source_hash: Some(selected_hash.clone()),
        embedded_sources: vec![],
        manifest: Manifest::parse_str(PACKAGE).unwrap(),
        requires: vec![],
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    };
    let epoch = ExtensionWorldEpoch::from_resolution(project.path(), &[resolved]).unwrap();
    let lock = format!(
        "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-12T00:00:00Z'\nschema_version=7\n\n[[package]]\nkind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='file:///original'\ncontent_hash='{selected_hash}'\n"
    );
    fs::write(project.path().join("vibe.lock"), lock).unwrap();
    let witnesses = PreparedWorldWitness::new(
        Some(snapshot(project.path(), "vibe.lock")),
        BTreeMap::from([(".".into(), snapshot(project.path(), "vibe.toml"))]),
        BTreeMap::from([(
            ("org.demo".into(), "dep".into()),
            snapshot(&slot, ".vibe-slot.toml"),
        )]),
    );
    let world = WitnessedSourceWorld::from_prepared_node(
        &workspace,
        ".",
        &epoch,
        &BTreeSet::from(["org.demo/dep".into()]),
        captures,
        witnesses,
    )
    .unwrap();
    Fixture {
        _source: source,
        _project: project,
        slot,
        world,
        genuine,
        pin,
        selected_hash,
    }
}

#[test]
fn public_unreplayed_index_cannot_promote_coordinated_slot_tamper() {
    let case = fixture(TAMPERED);
    let raw =
        SourceMembershipIndex::from_captured(&case.pin, original_capture(&case.slot)).unwrap();
    assert!(raw.member_matches("spec/DOC.md", &snapshot(&case.slot, "spec/DOC.md")));
    // The public raw catalogue has no proof-capable promotion route.
    assert_eq!(
        case.world
            .output_strength(Some(("org.demo", "dep")), "spec/DOC.md")
            .unwrap(),
        SourceStrength::RecordConsistent
    );
    assert_eq!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            Some(&case.genuine),
            false
        )
        .unwrap(),
        SourceStrength::RecordConsistent
    );
    assert!(matches!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            Some(&case.genuine),
            true
        ),
        Err(SourceQueryUnavailable::SourceProofUnavailable(
            "original source membership or derivation"
        ))
    ));
}

#[test]
fn verified_original_member_promotes_only_selected_mixed_output() {
    let case = fixture(ORIGINAL);
    assert!(
        case.genuine
            .original_source_member(&case.world, "org.demo", "dep", "spec/DOC.md")
            .unwrap()
    );
    assert!(
        !case
            .genuine
            .original_source_member(&case.world, "org.demo", "dep", "spec/UNRECORDED.md")
            .unwrap()
    );
    assert_eq!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            Some(&case.genuine),
            true
        )
        .unwrap(),
        SourceStrength::SourceProven
    );
    assert_eq!(
        source_strength(&case.world, None, "spec/self.md", None, false).unwrap(),
        SourceStrength::Observed
    );
    assert!(matches!(
        source_strength(&case.world, None, "spec/self.md", None, true),
        Err(SourceQueryUnavailable::SourceProofUnavailable(
            "development source capture"
        ))
    ));
    assert_eq!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            None,
            false
        )
        .unwrap(),
        SourceStrength::RecordConsistent
    );
    assert!(matches!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            None,
            true
        ),
        Err(SourceQueryUnavailable::SourceProofUnavailable(
            "original source capture"
        ))
    ));
}

#[test]
fn record_only_and_payload_only_tamper_refuse_current_world() {
    let case = fixture(ORIGINAL);
    let mut record = vibe_workspace::vibedeps::read_slot_record(&case.slot).unwrap();
    record.files[0].sha256 = "0".repeat(64);
    write_slot_record(&case.slot, &record).unwrap();
    assert!(matches!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            Some(&case.genuine),
            true
        ),
        Err(SourceQueryUnavailable::StaleWorld(
            "selected descriptor record"
        ))
    ));

    let case = fixture(ORIGINAL);
    fs::write(case.slot.join("spec/DOC.md"), b"# Payload only\n").unwrap();
    assert!(matches!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/DOC.md",
            Some(&case.genuine),
            true
        ),
        Err(SourceQueryUnavailable::StaleWorld("selected output"))
    ));
}

#[test]
fn unrecorded_slot_document_cannot_borrow_original_membership() {
    let case = fixture(ORIGINAL);
    fs::write(case.slot.join("spec/UNRECORDED.md"), b"# extra\n").unwrap();
    assert!(!case.genuine.contains("spec/UNRECORDED.md"));
    assert!(matches!(
        source_strength(
            &case.world,
            Some(("org.demo", "dep")),
            "spec/UNRECORDED.md",
            Some(&case.genuine),
            true
        ),
        Err(SourceQueryUnavailable::MissingPreparedCapture(_))
    ));
}
