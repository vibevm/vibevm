//! M-01-C.3 executable install proof matrix.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageKind, PackageRef};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_workspace::Workspace;
use vibe_workspace::extension_world::ExtensionWorldEpoch;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::source_query::{
    CapturedSourceFile, PreparedOwnerCapture, PreparedWorldWitness, SourceQueryUnavailable,
    WitnessedSourceWorld,
};
use vibe_workspace::vibedeps::{
    CopyMode, SlotRecord, materialise_with_spec_format, read_slot_record,
    read_validated_descriptor_map, write_slot_record,
};

use vibe_core::ContentHash;
use vibe_core::layout;
use vibe_core::manifest::SpecFormat;
use vibe_facts::Registry;
use vibe_install::{
    PreparedPackageOverlay, VerifiedSourceMembership, verify_original_source_capture,
};
use vibe_workspace::vibedeps::CONVERTER_RECIPE;

fn source_path() -> String {
    vibe_core::machine_json_path(&layout::current_specs_root().join("DOC.md"))
}
fn output_path() -> String {
    vibe_core::machine_json_path(&layout::current_specs_root().join("DOC.xml"))
}
const ROOT: &str =
    "[project]\nname='root'\nversion='1.0.0'\n\n[requires.packages]\n'org.demo/dep'='=1.0.0'\n";
const ORIGINAL: &str = "# Document {#root}\n\n@fact:ITEM Original. @status:spec/planned\n";
const ORIGINAL_XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  <title id=\"root\">Document</title>\n  <p><fact id=\"ITEM\" status=\"spec/plan\">Original.</fact></p>\n</spec>\n";
const TAMPERED: &str = "# Document {#root}\n\n@fact:ITEM Changed. @status:spec/planned\n";
const OVERLAY_A: &str = "schema=1\n[[fact]]\naddress='spec://org.demo/dep/DOC#ITEM'\norigin='package'\npackage='org.demo/dep'\nstatus='impl/done'\n";
const OVERLAY_B: &str = "schema=1\n[[fact]]\naddress='spec://org.demo/dep/DOC#ITEM'\norigin='package'\npackage='org.demo/dep'\nstatus='spec/done'\n";

struct SourceCase {
    source: String,
    output: String,
    media: &'static str,
    authored: &'static str,
    format: SpecFormat,
}

impl SourceCase {
    fn markdown_to_xml() -> Self {
        Self {
            source: source_path(),
            output: output_path(),
            media: "text/markdown",
            authored: ORIGINAL,
            format: SpecFormat::Xml,
        }
    }

    fn xml_to_markdown() -> Self {
        Self {
            source: vibe_core::machine_json_path(&layout::current_specs_root().join("DOC.xml")),
            output: vibe_core::machine_json_path(&layout::current_specs_root().join("DOC.md")),
            media: "application/xml",
            authored: ORIGINAL_XML,
            format: SpecFormat::Markdown,
        }
    }

    fn package(&self) -> String {
        format!(
            "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n\n[[package.sources]]\nfile='{}'\nmedia='{}'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
            self.source, self.media
        )
    }
}

fn snapshot(root: &Path, relative: &str) -> StableFileSnapshot {
    Project::open(root)
        .unwrap()
        .read_file_snapshot_bounded(relative, 1024 * 1024)
        .unwrap()
        .unwrap()
}

fn captured_source(root: &Path, case: &SourceCase) -> Vec<CapturedSourceFile> {
    [case.source.clone(), "vibe.toml".into()]
        .into_iter()
        .map(|path| CapturedSourceFile::new(path.clone(), snapshot(root, &path)).unwrap())
        .collect()
}

fn write_overlay(root: &Path, value: Option<&str>) {
    let file = root
        .join(layout::current_vibefacts_root())
        .join("org.demo.dep.toml");
    if let Some(value) = value {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, value).unwrap();
    } else if file.exists() {
        fs::remove_file(file).unwrap();
    }
}

struct Fixture {
    _source: TempDir,
    _project: TempDir,
    slot: PathBuf,
    world: WitnessedSourceWorld,
    proof: VerifiedSourceMembership,
    overlay: PreparedPackageOverlay,
}

fn fixture(
    published_source: &str,
    prepared_overlay: Option<&str>,
    published_overlay: Option<&str>,
    edit_record: impl FnOnce(&mut SlotRecord),
) -> Fixture {
    fixture_for(
        &SourceCase::markdown_to_xml(),
        published_source,
        prepared_overlay,
        published_overlay,
        edit_record,
    )
}

fn fixture_for(
    case: &SourceCase,
    published_source: &str,
    prepared_overlay: Option<&str>,
    published_overlay: Option<&str>,
    edit_record: impl FnOnce(&mut SlotRecord),
) -> Fixture {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join(layout::current_specs_root())).unwrap();
    fs::write(source.path().join("vibe.toml"), case.package()).unwrap();
    fs::write(source.path().join(&case.source), case.authored).unwrap();
    let pin = PackageRef::parse("flow:org.demo/dep@=1.0.0").unwrap();
    let selected_hash =
        ContentHash::parse(&vibe_registry::compute_content_hash(source.path()).unwrap()).unwrap();
    let proof = verify_original_source_capture(
        source.path(),
        pin,
        selected_hash.clone(),
        captured_source(source.path(), case),
    )
    .unwrap();
    fs::write(source.path().join(&case.source), published_source).unwrap();

    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"), ROOT).unwrap();
    fs::create_dir(project.path().join("spec")).unwrap();
    fs::write(project.path().join("spec/self.md"), "# Self\n").unwrap();
    write_overlay(project.path(), prepared_overlay);
    let registry = Registry::load_with_witnesses(project.path()).unwrap();
    let overlay =
        PreparedPackageOverlay::from_registry_snapshot(&registry, "org.demo", "dep").unwrap();
    write_overlay(project.path(), published_overlay);
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    materialise_with_spec_format(
        project.path(),
        &group,
        "dep",
        &version,
        source.path(),
        CopyMode::Copy,
        case.format,
        &selected_hash,
    )
    .unwrap();
    let workspace = Workspace {
        root: project.path().to_path_buf(),
        root_manifest: Manifest::parse_str(ROOT).unwrap(),
        members: vec![],
    };
    let slot = workspace.vibedeps_slot(&group, "dep", &version);
    let mut record = read_slot_record(&slot).unwrap();
    edit_record(&mut record);
    write_slot_record(&slot, &record).unwrap();
    let map =
        read_validated_descriptor_map(&slot, &group, "dep", &version, &selected_hash, case.format)
            .unwrap()
            .unwrap();
    let output = snapshot(&slot, &case.output);
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
            snapshot(&slot, "vibe.toml"),
            BTreeMap::from([(case.output.clone(), output)]),
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
        manifest: Manifest::parse_str(&case.package()).unwrap(),
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
        proof,
        overlay,
    }
}

fn unchanged(_: &mut SlotRecord) {}

#[test]
fn verified_original_builtin_xml_exact_output_is_derived() {
    let case = fixture(ORIGINAL, Some(OVERLAY_A), Some(OVERLAY_A), unchanged);
    let derived = case
        .proof
        .derive_builtin_output(
            &case.world,
            "org.demo",
            "dep",
            &output_path(),
            &case.overlay,
        )
        .unwrap();
    assert_eq!(derived.source_path(), source_path());
    assert_eq!(derived.output_path(), output_path());
    assert_eq!(derived.converter_recipe(), CONVERTER_RECIPE);
    assert_ne!(
        derived.original_raw_sha256(),
        derived.representation_sha256()
    );
    assert_eq!(
        derived.representation_sha256(),
        format!("sha256:{}", snapshot(&case.slot, &output_path()).sha256)
    );
    assert!(derived.overlay_hash().is_some());
}

#[test]
fn verified_original_vibe_xml_to_markdown_has_exact_builtin_derivation() {
    let input = SourceCase::xml_to_markdown();
    let case = fixture_for(
        &input,
        ORIGINAL_XML,
        Some(OVERLAY_A),
        Some(OVERLAY_A),
        unchanged,
    );
    assert!(
        case.proof
            .original_source_member(&case.world, "org.demo", "dep", &input.source,)
            .unwrap()
    );
    let record = read_slot_record(&case.slot).unwrap();
    assert_eq!(record.spec_format, SpecFormat::Markdown);
    assert_eq!(record.converter_recipe.as_deref(), Some(CONVERTER_RECIPE));
    let row = record
        .files
        .iter()
        .find(|row| row.path == input.output)
        .unwrap();
    assert_eq!(row.source.as_deref(), Some(input.source.as_str()));
    assert_eq!(
        row.disposition,
        Some(vibe_workspace::vibedeps::SlotFileDisposition::Converted)
    );
    let derived = case
        .proof
        .derive_builtin_output(&case.world, "org.demo", "dep", &input.output, &case.overlay)
        .unwrap();
    assert_eq!(derived.source_path(), input.source);
    assert_eq!(derived.output_path(), input.output);
    assert_eq!(derived.converter_recipe(), CONVERTER_RECIPE);
    assert_eq!(
        derived.original_raw_sha256(),
        format!(
            "sha256:{}",
            snapshot(case._source.path(), &input.source).sha256
        )
    );
    assert_eq!(
        derived.representation_sha256(),
        format!("sha256:{}", snapshot(&case.slot, &input.output).sha256)
    );
    assert_ne!(
        derived.original_raw_sha256(),
        derived.representation_sha256()
    );
    let (_, selected_package_hash) = case.world.selected_source_pin("org.demo", "dep").unwrap();
    assert_ne!(
        selected_package_hash.as_str(),
        derived.original_raw_sha256()
    );
    let output = fs::read_to_string(case.slot.join(&input.output)).unwrap();
    assert!(output.contains("@fact:ITEM"));
    assert!(output.contains("@status:impl/done"));
}

#[test]
fn coherent_tampered_output_and_record_cannot_borrow_original_capture() {
    let case = fixture(TAMPERED, None, None, unchanged);
    assert!(matches!(
        case.proof.derive_builtin_output(
            &case.world,
            "org.demo",
            "dep",
            &output_path(),
            &case.overlay,
        ),
        Err(SourceQueryUnavailable::DerivationUnavailable(
            "materialized output differs from builtin replay"
        ))
    ));
}

#[test]
fn retained_overlay_a_refuses_coherent_overlay_b_output() {
    let case = fixture(ORIGINAL, Some(OVERLAY_A), Some(OVERLAY_B), unchanged);
    assert!(matches!(
        case.proof.derive_builtin_output(
            &case.world,
            "org.demo",
            "dep",
            &output_path(),
            &case.overlay,
        ),
        Err(SourceQueryUnavailable::DerivationUnavailable(
            "authorized overlay identity"
        ))
    ));
}

#[test]
fn native_recipe_never_promotes() {
    let native = fixture(ORIGINAL, None, None, |record| {
        record.converter_recipe = Some("native/1".into());
    });
    assert!(matches!(
        native.proof.derive_builtin_output(
            &native.world,
            "org.demo",
            "dep",
            &output_path(),
            &native.overlay,
        ),
        Err(SourceQueryUnavailable::DerivationUnavailable(
            "builtin converter recipe or disposition"
        ))
    ));
}

#[test]
fn explicit_no_overlay_is_a_valid_selected_input() {
    let case = fixture(ORIGINAL, None, None, unchanged);
    let derived = case
        .proof
        .derive_builtin_output(
            &case.world,
            "org.demo",
            "dep",
            &output_path(),
            &case.overlay,
        )
        .unwrap();
    assert!(derived.overlay_hash().is_none());
}

#[test]
fn selected_source_output_mapping_change_refuses_prepared_world() {
    let case = fixture(ORIGINAL, None, None, unchanged);
    let mut record = read_slot_record(&case.slot).unwrap();
    let row = record
        .files
        .iter_mut()
        .find(|row| row.path == output_path())
        .unwrap();
    row.source = Some("unselected/OTHER.md".into());
    write_slot_record(&case.slot, &record).unwrap();
    assert!(matches!(
        case.proof.derive_builtin_output(
            &case.world,
            "org.demo",
            "dep",
            &output_path(),
            &case.overlay,
        ),
        Err(SourceQueryUnavailable::StaleWorld(
            "selected descriptor record"
        ))
    ));
}
