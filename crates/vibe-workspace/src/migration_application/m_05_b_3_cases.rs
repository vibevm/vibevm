use super::*;
use crate::migration_planning::feature::{
    plan_feature_migration_json, validate_report_for_request,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use vibe_core::manifest::*;
use vibe_wire::generated::migration::{
    e1::{application_journal::*, operation_report::*},
    e2::plan_report::{Outcome, Plan},
};

struct Fixture {
    _temp: tempfile::TempDir,
    project: PathBuf,
    store: PathBuf,
    before: String,
    approved: String,
    plan: Plan,
}
impl Fixture {
    fn new(epoch1: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let store = temp.path().join("store");
        fs::create_dir(&project).unwrap();
        let before = format!(
            "# untouched café\n[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n{}[requires.packages]\n'org.example/base'={{version='^1.0',link='static',access='private',friend=false,exclude=['docs/**']}}\n[features]\ndefault=['a'] # default\na=['org.example/base?/fast'] # weak\nb=[]\n[features.exclusive]\nmode=['a','b'] # at most one\n",
            if epoch1 { "epoch = 1 # marker\n" } else { "" }
        );
        fs::write(project.join("vibe.toml"), &before).unwrap();
        let approved = request(&before, epoch1);
        let report = plan_feature_migration_json(&approved).unwrap();
        validate_report_for_request(&approved, &report).unwrap();
        let Outcome::Planned(p) = report.outcome else {
            panic!("plan")
        };
        Self {
            _temp: temp,
            project,
            store,
            before,
            approved,
            plan: p.plan,
        }
    }
    fn grant(&self) -> OwnedManifestMigrationFile {
        OwnedManifestMigrationFile::new(&self.project, "vibe.toml", &self.store).unwrap()
    }
    fn authority(&self) -> Value {
        json!({"project_root":self.project,"recovery_store_root":self.store,"target_relative":"vibe.toml","scope":"owned_authored_file"})
    }
    fn apply(&self) -> String {
        json!({"schema":1,"operation":"apply","operation_id":"b3","authority":self.authority(),"approved_request_json":self.approved,"before_content_id":self.plan.before_identity,"after_content_id":self.plan.after_identity,"diff_id":self.plan.diff_identity}).to_string()
    }
    fn directory(&self) -> PathBuf {
        self.store
            .join("migrations")
            .join(identity::operation_storage_key(
                &self.grant().project().identity_token().unwrap(),
                "b3",
            ))
    }
    fn journal(&self) -> ApplicationJournal {
        serde_json::from_slice(&fs::read(self.directory().join("journal.json")).unwrap()).unwrap()
    }
    fn recover(&self, action: &str) -> String {
        json!({"schema":1,"operation":"recover","operation_id":"b3","authority":self.authority(),"expected_intent_id":self.journal().intent_id,"action":action}).to_string()
    }
    fn call(&self, text: &str) -> OperationReport {
        let r = execute_manifest_migration_json(text, &self.grant()).unwrap();
        validate_operation_report(&r).unwrap();
        r
    }
    fn target(&self) -> Vec<u8> {
        fs::read(self.project.join("vibe.toml")).unwrap()
    }
}
fn request(before: &str, epoch1: bool) -> String {
    json!({"schema":2,"source_text":before,"source_profile":if epoch1 {FEATURES_LEGACY_EPOCH1} else {FEATURES_LEGACY_ABSENT},"target_profile":if epoch1 {FEATURES_TYPED_EPOCH1} else {FEATURES_TYPED_ABSENT},"recipe":if epoch1 {FEATURES_TO_TYPED_EPOCH1} else {FEATURES_TO_TYPED}}).to_string()
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, p: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !p.exists() {
            return;
        }
        for e in fs::read_dir(p).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.insert(p.strip_prefix(root).unwrap().to_owned(), None);
                walk(root, &p, out)
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_owned(),
                    Some(fs::read(&p).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn selected_profile_saved_apply_recovery_and_completed_replay() {
    for epoch1 in [false, true] {
        let f = Fixture::new(epoch1);
        let before = snapshot(f._temp.path());
        let report = plan_feature_migration_json(&f.approved).unwrap();
        validate_report_for_request(&f.approved, &report).unwrap();
        let mut old_request: vibe_wire::generated::migration::e1::plan_request::PlanRequest =
            serde_json::from_str(&f.approved).unwrap();
        old_request.schema = 1;
        let mut forged: vibe_wire::generated::migration::e1::plan_report::PlanReport =
            serde_json::from_value(serde_json::to_value(&report).unwrap()).unwrap();
        forged.schema = 1;
        assert!(
            crate::migration_planning::validate_report_for_request(&old_request, &forged).is_err()
        );
        assert!(matches!(
            crate::migration_planning::plan_migration_request(old_request),
            crate::migration_planning::MigrationPlanningResponse::Preflight(_)
        ));
        assert_eq!(snapshot(f._temp.path()), before, "dry-run wrote state");
        assert_eq!(
            f.call(&f.apply()).outcome,
            OperationReportOutcome::Completed
        );
        assert_eq!(f.target(), f.plan.after.as_bytes());
        let checked = read_feature_manifest(&f.plan.after, &f.plan.target_profile).unwrap();
        assert_eq!(checked.manifest.package.unwrap().epoch, epoch1.then_some(1));
        assert_eq!(checked.selection, feature_constraints::Selection::Omitted);
        assert!(
            checked
                .declarations
                .dependencies
                .values()
                .all(|d| !d.optional)
        );
        assert_eq!(
            fs::read(f.directory().join("before.bin")).unwrap(),
            f.before.as_bytes()
        );
        let stable = snapshot(f._temp.path());
        assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::NoOp);
        assert_eq!(snapshot(f._temp.path()), stable);
        let identity=json!({"schema":2,"source_text":f.plan.after,"source_profile":f.plan.target_profile,"target_profile":f.plan.target_profile,"recipe":IDENTITY}).to_string();
        let Outcome::NoOp(p) = plan_feature_migration_json(&identity).unwrap().outcome else {
            panic!("identity")
        };
        assert_eq!(p.plan.before, p.plan.after);
        // Exercise real persisted-state recovery of a supported prepared prefix.
        let mut j = f.journal();
        j.phase = ApplicationJournalPhase::Prepared;
        fs::write(
            f.directory().join("journal.json"),
            serde_json::to_vec(&j).unwrap(),
        )
        .unwrap();
        fs::write(f.project.join("vibe.toml"), &f.before).unwrap();
        assert_eq!(
            f.call(&f.recover("complete")).outcome,
            OperationReportOutcome::Completed
        );
        assert_eq!(f.target(), f.plan.after.as_bytes());
        let stable = snapshot(f._temp.path());
        assert_eq!(
            f.call(&f.recover("complete")).outcome,
            OperationReportOutcome::NoOp
        );
        assert_eq!(snapshot(f._temp.path()), stable);
        assert_eq!(
            f.call(&f.recover("restore")).outcome,
            OperationReportOutcome::Completed
        );
        assert_eq!(f.target(), f.before.as_bytes());
        let stable = snapshot(f._temp.path());
        assert_eq!(
            f.call(&f.recover("restore")).outcome,
            OperationReportOutcome::NoOp
        );
        assert_eq!(snapshot(f._temp.path()), stable);
    }
}

#[test]
fn needs_input_and_concurrent_authored_edit_make_no_partial_changes() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora");
    let expected: vibe_wire::generated::migration::e2::plan_report::PlanReport =
        serde_json::from_slice(
            &fs::read(corpus.join("migration-plan-report/e2/valid/feature-question.json")).unwrap(),
        )
        .unwrap();
    for name in ["absent", "epoch1"] {
        let request =
            fs::read_to_string(corpus.join(format!("migration-plan-request/e2/valid/{name}.json")))
                .unwrap();
        assert_eq!(plan_feature_migration_json(&request).unwrap(), expected);
    }
    assert!(
        serde_json::from_slice::<vibe_wire::generated::migration::e2::plan_report::PlanReport>(
            &fs::read(corpus.join("migration-plan-report/e2/invalid/unknown-question-field.json"))
                .unwrap()
        )
        .is_err()
    );
    let error =
        serde_json::from_slice::<vibe_wire::generated::migration::e2::plan_error::PlanError>(
            &fs::read(corpus.join("migration-plan-error/e2/valid/invalid-request.json")).unwrap(),
        )
        .unwrap();
    assert_eq!(plan_feature_migration_json("{").unwrap_err(), error);
    for epoch1 in [false, true] {
        let f = Fixture::new(epoch1);
        let before = snapshot(f._temp.path());
        for text in [
            f.before.replace("org.example/base?/fast", "missing?/fast"),
            f.before
                .replace("[features]", "'stack:org.example/base'='^1.0'\n[features]"),
            f.before
                .replace("org.example/base?/fast", "subskill:missing/catalogue"),
            f.before
                .replace("version='^1.0'", "version='^1.0', optional=true"),
        ] {
            let approved = request(&text, epoch1);
            let report = plan_feature_migration_json(&approved).unwrap();
            validate_report_for_request(&approved, &report).unwrap();
            let Outcome::NeedsInput(q) = &report.outcome else {
                panic!("structured ambiguity: {report:?}")
            };
            assert!(
                q.questions
                    .iter()
                    .all(|q| q.authored_reference.is_some() && q.resource_path.is_none())
            );
            assert!(
                serde_json::from_value::<
                    vibe_wire::generated::migration::e1::plan_report::PlanReport,
                >(serde_json::to_value(&report).unwrap())
                .is_err()
            );
            let mut apply: Value = serde_json::from_str(&f.apply()).unwrap();
            apply["approved_request_json"] = json!(approved);
            assert_eq!(
                f.call(&apply.to_string()).outcome,
                OperationReportOutcome::Refused
            );
            assert_eq!(snapshot(f._temp.path()), before);
        }
        for value in [
            json!({"schema":2,"source_text":f.before,"recipe":null}),
            json!({"schema":2,"source_text":f.before,"extra":true}),
        ] {
            assert!(plan_feature_migration_json(&value.to_string()).is_err());
        }
        let bad_epoch = request(&f.before.replace("[package]", "[package]\nepoch=8"), epoch1);
        assert!(matches!(
            plan_feature_migration_json(&bad_epoch).unwrap().outcome,
            Outcome::Refused(_)
        ));
        assert_eq!(snapshot(f._temp.path()), before);
        let mut unknown: Value = serde_json::from_str(&f.approved).unwrap();
        unknown["source_profile"] = json!("unknown");
        assert!(matches!(
            plan_feature_migration_json(&unknown.to_string())
                .unwrap()
                .outcome,
            Outcome::Refused(_)
        ));
        let mut frozen: Value = serde_json::from_str(&f.approved).unwrap();
        frozen["source_text"] = json!(f.before.replace("[package]", "[package]\nfrozen=true"));
        let frozen = frozen.to_string();
        let Outcome::Planned(p) = plan_feature_migration_json(&frozen).unwrap().outcome else {
            panic!("frozen plan")
        };
        let mut apply: Value = serde_json::from_str(&f.apply()).unwrap();
        apply["approved_request_json"] = json!(frozen);
        apply["before_content_id"] = json!(p.plan.before_identity);
        apply["after_content_id"] = json!(p.plan.after_identity);
        apply["diff_id"] = json!(p.plan.diff_identity);
        assert_eq!(
            f.call(&apply.to_string()).error.unwrap().code,
            ApplicationErrorCode::AuthorityRefused
        );
        assert_eq!(snapshot(f._temp.path()), before);
        fs::write(f.project.join("vibe.toml"), b"owner changed source").unwrap();
        assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::Refused);
        assert_eq!(f.target(), b"owner changed source");
        assert!(!f.directory().join("journal.json").exists());
        fs::write(f.project.join("vibe.toml"), &f.before).unwrap();
        assert_eq!(
            f.call(&f.apply()).outcome,
            OperationReportOutcome::Completed
        );
        fs::write(f.project.join("vibe.toml"), b"foreign occupant").unwrap();
        fs::write(f.directory().join("foreign-residue"), b"preserve").unwrap();
        let stable = snapshot(f._temp.path());
        assert_eq!(
            f.call(&f.recover("restore")).outcome,
            OperationReportOutcome::Refused
        );
        assert_eq!(snapshot(f._temp.path()), stable);
        assert_eq!(
            fs::read(f.directory().join("before.bin")).unwrap(),
            f.before.as_bytes()
        );
    }
}
