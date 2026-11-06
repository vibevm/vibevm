//! Source recipe through the existing D1 writer; declared reads, not ordinary boot install.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY");
use super::source_tests::{answered, plan, source};
use crate::migration_application::{
    OwnedManifestMigrationFile, execute_manifest_migration_json, validate_operation_report,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_wire::generated::migration::e1::operation_report::{
    ApplicationErrorCode, OperationReportOutcome,
};

const DOCUMENT: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><chosen fact=\"true\" status=\"spec/done\">café exact</chosen></p></spec>\r\n";
const SOURCE_PATH: &str = "boot/not-xml.md";
struct Fixture {
    _temp: tempfile::TempDir,
    project: PathBuf,
    store: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let project = root.join("project");
        std::fs::create_dir_all(project.join("boot")).unwrap();
        std::fs::write(project.join("vibe.toml"), source(SOURCE_PATH)).unwrap();
        std::fs::write(project.join(SOURCE_PATH), DOCUMENT).unwrap();
        Self {
            _temp: temp,
            project,
            store: root.join("recovery"),
        }
    }
    fn operation(&self) -> Value {
        let approved = answered(SOURCE_PATH, "application/xml");
        let plan = plan(approved.clone());
        json!({"schema":1,"operation":"apply","operation_id":"source-explicit","authority":{
            "project_root":self.project,"target_relative":"vibe.toml","recovery_store_root":self.store,"scope":"owned_authored_file"},
            "approved_request_json":approved.to_string(),"before_content_id":plan.before_identity,"after_content_id":plan.after_identity,"diff_id":plan.diff_identity})
    }
    fn execute(
        &self,
        request: &Value,
    ) -> vibe_wire::generated::migration::e1::operation_report::OperationReport {
        let grant =
            OwnedManifestMigrationFile::new(&self.project, "vibe.toml", &self.store).unwrap();
        let report = execute_manifest_migration_json(&request.to_string(), &grant).unwrap();
        validate_operation_report(&report).unwrap();
        report
    }
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !path.exists() {
            return;
        }
        for row in std::fs::read_dir(path).unwrap() {
            let path = row.unwrap().path();
            if path.is_dir() {
                out.insert(path.strip_prefix(root).unwrap().into(), None);
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    Some(std::fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn source_recipe_apply_replay_and_declared_reader_preserve_original_bytes() {
    let fixture = Fixture::new();
    let operation = fixture.operation();
    let expected = plan(answered(SOURCE_PATH, "application/xml"));
    let report = fixture.execute(&operation);
    assert_eq!(
        report.outcome,
        OperationReportOutcome::Completed,
        "{report:#?}"
    );
    let after = std::fs::read_to_string(fixture.project.join("vibe.toml")).unwrap();
    assert_eq!(after, expected.after);
    assert!(after.starts_with(&source(SOURCE_PATH)));
    assert!(!after.replace("\r\n", "").contains('\n'));
    let manifest = vibe_core::manifest::Manifest::parse_str(&after).unwrap();
    let package = manifest.require_package().unwrap();
    let source_bytes = std::fs::read(fixture.project.join(SOURCE_PATH)).unwrap();
    assert_eq!(source_bytes, DOCUMENT.as_bytes());
    let observed =
        vibe_facts::observe_declared_document(package, SOURCE_PATH, Some(&source_bytes)).unwrap();
    let vibe_facts::DeclaredDocumentObservation::Facts { provenance, facts } = observed else {
        panic!("declared XML reader must consume .md exact source: {observed:?}")
    };
    assert_eq!(provenance.reader, Some("vibe-specdoc:xml/spec"));
    assert_eq!(
        provenance.witness.unwrap(),
        vibe_facts::SourceFileWitness::of(SOURCE_PATH, DOCUMENT.as_bytes())
    );
    assert_eq!(facts.len(), 1);
    assert!(facts[0].address.ends_with("#chosen"));
    let stable = tree(fixture._temp.path());
    assert_eq!(
        fixture.execute(&operation).outcome,
        OperationReportOutcome::NoOp
    );
    assert_eq!(tree(fixture._temp.path()), stable);
    // No ordinary install claim: the supported endpoint is explicit declared reading.
}
#[test]
fn stale_source_plan_refuses_without_authored_write_or_journal() {
    let fixture = Fixture::new();
    let operation = fixture.operation();
    std::fs::write(
        fixture.project.join("vibe.toml"),
        format!("{}\r\n# external edit", source(SOURCE_PATH)),
    )
    .unwrap();
    let before = tree(&fixture.project);
    let result = fixture.execute(&operation);
    assert_eq!(result.outcome, OperationReportOutcome::Refused);
    assert_eq!(
        result.error.unwrap().code,
        ApplicationErrorCode::SourceChanged
    );
    assert_eq!(tree(&fixture.project), before);
    assert!(
        tree(&fixture.store)
            .keys()
            .all(|path| path.file_name().unwrap() != "journal.json")
    );
    assert!(
        result
            .effects
            .iter()
            .all(|effect| serde_json::to_value(effect).unwrap()["kind"] != "target_publication")
    );
}
