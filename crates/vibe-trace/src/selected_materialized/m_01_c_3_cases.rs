//! C3 selected-slot trace: the carried MD decoration competes with XML output.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use specmap_core::generated::specmap::Specmap;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_workspace::Workspace;
use vibe_workspace::extension_world::ExtensionWorldEpoch;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::source_query::{
    PreparedOwnerCapture, PreparedWorldWitness, WitnessedSourceWorld,
};
use vibe_workspace::vibedeps::{
    CopyMode, materialise_with_spec_format, read_validated_descriptor_map,
};

use super::explain_selected_materialized;
use crate::Explain;

const ROOT: &str =
    "[project]\nname='root'\nversion='1.0.0'\n\n[requires.packages]\n'org.demo/dep'='=1.0.0'\n";
const MD: &str = "# Document {#root}\n\n## Law {#law}\n\n@fact:ITEM The law. @status:impl/done\n";
const DOC_XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  <title id=\"root\">Document</title>\n  <section id=\"law\" title=\"Law\">\n    <note kind=\"tip\">A doc-only tip.</note>\n  </section>\n</spec>\n";

fn source_path() -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_specs_root().join("DOC.md"))
}
fn output_path() -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_specs_root().join("DOC.xml"))
}
struct TraceCase {
    source: String,
    output: String,
    authored: &'static str,
    carried: &'static str,
    media: &'static str,
    interpretation: &'static str,
    role: &'static str,
    kind: &'static str,
    format: SpecFormat,
}

impl TraceCase {
    fn converted_xml() -> Self {
        Self {
            source: source_path(),
            output: output_path(),
            authored: MD,
            carried: MD,
            media: "text/markdown",
            interpretation: "vibe-spec",
            role: "spec",
            kind: "flow",
            format: SpecFormat::Xml,
        }
    }
    fn mixed_markdown() -> Self {
        Self {
            source: source_path(),
            output: source_path(),
            authored: MD,
            carried: MD,
            media: "text/markdown",
            interpretation: "vibe-spec",
            role: "spec",
            kind: "flow",
            format: SpecFormat::Mixed,
        }
    }
    fn documentation_xml() -> Self {
        Self {
            source: output_path(),
            output: output_path(),
            authored: DOC_XML,
            carried: "# Document {#root}\n\n## Law {#law}\n",
            media: "application/xml",
            interpretation: "vibe-doc",
            role: "documentation",
            kind: "doc",
            format: SpecFormat::Xml,
        }
    }
    fn package(&self) -> String {
        let title = if self.kind == "doc" {
            "title='Documentation'\nabstract='Covers one Vibe rule for package readers. Assumes basic Vibe addressing. Leaves native execution and deployment out.'\n"
        } else {
            ""
        };
        let documents = if self.kind == "doc" {
            "\n[[documents]]\npackage='org.demo/subject'\nversion='=1.0.0'\n"
        } else {
            ""
        };
        format!(
            "[package]\ngroup='org.demo'\nname='dep'\nkind='{}'\nversion='1.0.0'\n{title}\n[[package.sources]]\nfile='{}'\nmedia='{}'\ninterpretation='{}'\nrole='{}'\nloading='explicit'\n{documents}",
            self.kind, self.source, self.media, self.interpretation, self.role
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

struct Fixture {
    _source: tempfile::TempDir,
    _project: tempfile::TempDir,
    world: WitnessedSourceWorld,
    map: Specmap,
    target: String,
    output: String,
}

fn fixture() -> Fixture {
    fixture_for(TraceCase::converted_xml())
}

fn fixture_for(case: TraceCase) -> Fixture {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join(vibe_core::layout::current_specs_root())).unwrap();
    fs::write(source.path().join("vibe.toml"), case.package()).unwrap();
    fs::write(source.path().join(&case.source), case.authored).unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"), ROOT).unwrap();
    fs::create_dir(project.path().join("spec")).unwrap();
    fs::write(project.path().join("spec/self.md"), "# Self\n").unwrap();
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let selected_hash = ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
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
    let map_owner =
        read_validated_descriptor_map(&slot, &group, "dep", &version, &selected_hash, case.format)
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
            snapshot(&slot, "vibe.toml"),
            BTreeMap::from([(case.output.clone(), snapshot(&slot, &case.output))]),
            Some(map_owner),
        ),
    ];
    let resolved = ResolvedDep {
        kind: if case.kind == "doc" {
            PackageKind::Doc
        } else {
            PackageKind::Flow
        },
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
        "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-12T00:00:00Z'\nschema_version=7\n\n[[package]]\nkind='{}'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='file:///original'\ncontent_hash='{selected_hash}'\n",
        case.kind
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
    let (units, warnings) =
        specmap_core::mdspec::parse_units(&case.source, case.carried, "org.demo/dep");
    assert!(warnings.is_empty());
    let target = "spec://org.demo/dep/DOC#law".to_owned();
    let map = Specmap {
        schema: 3,
        specUnits: units,
        codeItems: vec![],
        edges: vec![],
        suspects: vec![],
        warnings: vec![],
    };
    Fixture {
        _source: source,
        _project: project,
        world,
        map,
        target,
        output: slot.join(&case.output).to_string_lossy().into_owned(),
    }
}

#[test]
fn selected_xml_locator_and_ir_hash_do_not_rebadge_carried_md_decoration() {
    let mut case = fixture();
    let old_wire = serde_json::to_vec(&case.map).unwrap();
    assert!(!String::from_utf8_lossy(&old_wire).contains("materialized_locator"));
    case.map = serde_json::from_slice(&old_wire).unwrap();
    assert_eq!(case.map.schema, 3);
    let old = case
        .map
        .specUnits
        .iter()
        .find(|unit| unit.uri == case.target)
        .unwrap();
    let old_file = old.file.clone();
    let old_line = old.line;
    let old_hash = old.contentHash.clone();
    let Explain::Json(value) = explain_selected_materialized(
        &case.world,
        "org.demo",
        "dep",
        &mut case.map,
        &case.target,
        true,
    )
    .unwrap() else {
        panic!("expected JSON")
    };
    let unit = case
        .map
        .specUnits
        .iter()
        .find(|unit| unit.uri == case.target)
        .unwrap();
    assert_eq!(case.map.schema, 4);
    assert_eq!(unit.file, old_file);
    assert_eq!(unit.line, old_line);
    assert_eq!(unit.contentHash, old_hash);
    let locator = unit.materializedLocator.as_ref().unwrap();
    assert!(locator.path.ends_with(&output_path()));
    assert!(locator.path.ends_with(".xml"));
    assert!(locator.startLine <= locator.endLine);
    let xml = fs::read_to_string(&case.output).unwrap();
    let line = xml.lines().nth((locator.startLine - 1) as usize).unwrap();
    assert!(line.contains("Law"));
    assert_eq!(
        unit.canonicalNode.as_ref().unwrap().recipe,
        "sha256-specdoc-node/1"
    );
    assert_ne!(unit.canonicalNode.as_ref().unwrap().hash, old_hash);
    assert_eq!(value["source"]["from"], "prepared-selected-materialization");
    assert!(value["units"][0]["materialized_locator"].is_object());
}

#[test]
fn changed_selected_output_refuses_source_decorations_as_a_locator() {
    let mut case = fixture();
    fs::write(&case.output, "<spec xmlns=\"https://vibevm.org/spec/1\"/>").unwrap();
    assert!(
        explain_selected_materialized(
            &case.world,
            "org.demo",
            "dep",
            &mut case.map,
            &case.target,
            true,
        )
        .is_err()
    );
    assert!(
        case.map
            .specUnits
            .iter()
            .all(|unit| unit.materializedLocator.is_none())
    );
}

#[test]
fn mixed_slot_declared_markdown_resolves_actual_markdown_node() {
    let mut case = fixture_for(TraceCase::mixed_markdown());
    let Explain::Json(value) = explain_selected_materialized(
        &case.world,
        "org.demo",
        "dep",
        &mut case.map,
        &case.target,
        true,
    )
    .unwrap() else {
        panic!("expected JSON")
    };
    let locator = &value["units"][0]["materialized_locator"];
    assert!(locator["path"].as_str().unwrap().ends_with("DOC.md"));
    assert_eq!(locator["start_line"], 3);
    assert!(value["units"][0]["canonical_node"].is_object());
}

#[test]
fn declared_vibe_doc_note_keeps_xml_section_native_range() {
    assert!(specmap_core::xmlspec::native_anchor_range(DOC_XML, "law").is_err());
    let mut case = fixture_for(TraceCase::documentation_xml());
    let Explain::Json(value) = explain_selected_materialized(
        &case.world,
        "org.demo",
        "dep",
        &mut case.map,
        &case.target,
        true,
    )
    .unwrap() else {
        panic!("expected JSON")
    };
    let locator = &value["units"][0]["materialized_locator"];
    assert!(locator["path"].as_str().unwrap().ends_with("DOC.xml"));
    assert_eq!(locator["start_line"], 3);
    assert_eq!(locator["end_line"], 5);
    assert!(value["units"][0]["canonical_node"].is_object());
}
