//! Temp-only real restart probes; helpers are not product admission evidence.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY");
use super::*;
use crate::migration_planning::{MigrationPlanningResponse, plan_migration_json};

#[path = "retained_sync_tests.rs"]
mod retained_sync_tests;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use vibe_core::manifest::{CURRENT_ABSENT, IDENTITY, LEGACY_ABSENT, SNAPSHOT_TO_COPY};

const BEFORE: &str = "# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.1.0'\r\nmaterialization='snapshot' # keep\r\n";
struct Fixture {
    _temp: tempfile::TempDir,
    project: PathBuf,
    store: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let store = temp.path().join("store");
        std::fs::create_dir(&project).unwrap();
        std::fs::write(project.join("vibe.toml"), BEFORE).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                project.join("vibe.toml"),
                std::fs::Permissions::from_mode(0o640),
            )
            .unwrap();
        }
        Self {
            _temp: temp,
            project,
            store,
        }
    }
    fn grant(&self) -> OwnedManifestMigrationFile {
        OwnedManifestMigrationFile::new(&self.project, "vibe.toml", &self.store).unwrap()
    }
    fn authority(&self) -> Value {
        json!({"project_root":self.project,"recovery_store_root":self.store,"target_relative":"vibe.toml","scope":"owned_authored_file"})
    }
    fn apply(&self) -> String {
        let approved = json!({"schema":1,"source_text":BEFORE,"source_profile":LEGACY_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":SNAPSHOT_TO_COPY}).to_string();
        let MigrationPlanningResponse::Report(report) = plan_migration_json(&approved) else {
            panic!("plan")
        };
        let vibe_wire::generated::migration::e1::plan_report::Outcome::Planned(p) = report.outcome
        else {
            panic!("planned")
        };
        json!({"schema":1,"operation":"apply","operation_id":"operation-one","authority":self.authority(),"approved_request_json":approved,"before_content_id":p.plan.before_identity,"after_content_id":p.plan.after_identity,"diff_id":p.plan.diff_identity}).to_string()
    }
    fn inspect(&self) -> String {
        json!({"schema":1,"operation":"inspect","operation_id":"operation-one","authority":self.authority()}).to_string()
    }
    fn directory(&self) -> PathBuf {
        let token = self.grant().project().identity_token().unwrap();
        self.store
            .join("migrations")
            .join(identity::operation_storage_key(&token, "operation-one"))
    }
    fn journal(&self) -> Option<ApplicationJournal> {
        std::fs::read(self.directory().join("journal.json"))
            .ok()
            .map(|b| serde_json::from_slice(&b).unwrap())
    }
    fn recover(&self, id: &str, action: &str) -> String {
        json!({"schema":1,"operation":"recover","operation_id":"operation-one","authority":self.authority(),"expected_intent_id":id,"action":action}).to_string()
    }
    fn call(&self, text: &str) -> OperationReport {
        let r = execute_manifest_migration_json(text, &self.grant()).unwrap();
        super::super::validate_operation_report(&r).unwrap();
        r
    }
    fn target(&self) -> Vec<u8> {
        std::fs::read(self.project.join("vibe.toml")).unwrap()
    }
}

fn tree(root: &Path) -> std::collections::BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, p: &Path, out: &mut std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !p.exists() {
            return;
        }
        for e in std::fs::read_dir(p).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.insert(p.strip_prefix(root).unwrap().to_owned(), None);
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_owned(),
                    Some(std::fs::read(&p).unwrap()),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn malformed_forged_grant_and_frozen_preflight_create_no_state() {
    let f = Fixture::new();
    let before = tree(f._temp.path());
    assert_eq!(f.call("{}").outcome, OperationReportOutcome::Refused);
    let mut value: Value = serde_json::from_str(&f.apply()).unwrap();
    value["authority"]["target_relative"] = json!("other.toml");
    assert_eq!(
        f.call(&value.to_string()).error.unwrap().code,
        ApplicationErrorCode::AuthorityRefused
    );
    let mut value: Value = serde_json::from_str(&f.apply()).unwrap();
    let approved = json!({"schema":1,"source_text":format!("{BEFORE}frozen=true\r\n"),"source_profile":LEGACY_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":SNAPSHOT_TO_COPY}).to_string();
    let MigrationPlanningResponse::Report(p) = plan_migration_json(&approved) else {
        panic!("plan")
    };
    let vibe_wire::generated::migration::e1::plan_report::Outcome::Planned(p) = p.outcome else {
        panic!("planned")
    };
    value["approved_request_json"] = json!(approved);
    value["before_content_id"] = json!(p.plan.before_identity);
    value["after_content_id"] = json!(p.plan.after_identity);
    value["diff_id"] = json!(p.plan.diff_identity);
    assert_eq!(
        f.call(&value.to_string()).error.unwrap().code,
        ApplicationErrorCode::AuthorityRefused
    );
    assert_eq!(tree(f._temp.path()), before);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn apply_reader_mode_restore_and_terminal_replay_preserve_exact_bytes() {
    let f = Fixture::new();
    assert!(vibe_core::manifest::Manifest::parse_str(BEFORE).is_err());
    let r = f.call(&f.apply());
    assert_eq!(r.outcome, OperationReportOutcome::Completed, "{r:#?}");
    let after = BEFORE.replace("'snapshot'", "'copy'");
    assert_eq!(f.target(), after.as_bytes());
    assert!(vibe_core::manifest::Manifest::parse_str(&after).is_ok());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(f.project.join("vibe.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o640
        );
    }
    let stable = tree(f._temp.path());
    assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::NoOp);
    assert_eq!(tree(f._temp.path()), stable);
    let id = f.journal().unwrap().intent_id;
    assert_eq!(
        f.call(&f.recover(&id, "restore")).outcome,
        OperationReportOutcome::Completed
    );
    assert_eq!(f.target(), BEFORE.as_bytes());
    let stable = tree(f._temp.path());
    assert_eq!(
        f.call(&f.recover(&id, "restore")).outcome,
        OperationReportOutcome::NoOp
    );
    assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::NoOp);
    assert_eq!(
        tree(f._temp.path()),
        stable,
        "restored operation was implicitly reapplied"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn inspection_existing_only_and_orphan_or_foreign_store_preservation() {
    let f = Fixture::new();
    let stable = tree(f._temp.path());
    assert_eq!(
        f.call(&f.inspect()).outcome,
        OperationReportOutcome::Refused
    );
    assert_eq!(tree(f._temp.path()), stable);
    let store =
        vibe_safefs::ExternalStore::open_or_create_disjoint(&f.store, f.grant().project()).unwrap();
    let token = f.grant().project().identity_token().unwrap();
    let _lock = store.open_and_lock_project(&token).unwrap();
    std::fs::create_dir_all(f.directory()).unwrap();
    std::fs::write(f.directory().join("before.bin"), b"foreign bytes").unwrap();
    drop(_lock);
    let stable = tree(f._temp.path());
    let r = f.call(&f.inspect());
    assert_eq!(r.outcome, OperationReportOutcome::Refused);
    assert!(!r.residue.is_empty());
    assert_eq!(tree(f._temp.path()), stable);
    assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::Refused);
    assert_eq!(f.target(), BEFORE.as_bytes());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn changed_absent_third_state_and_forged_intent_do_not_overwrite() {
    let f = Fixture::new();
    std::fs::write(f.project.join("vibe.toml"), b"third state").unwrap();
    let r = f.call(&f.apply());
    assert_eq!(r.target_state, OperationReportTargetState::ThirdState);
    assert_eq!(f.target(), b"third state");
    let f = Fixture::new();
    f.call(&f.apply());
    let journal = f.journal().unwrap();
    assert_eq!(
        f.call(&f.recover(&"0".repeat(64), "restore"))
            .error
            .unwrap()
            .code,
        ApplicationErrorCode::IntentMismatch
    );
    std::fs::write(f.project.join("vibe.toml"), b"external edit").unwrap();
    assert_eq!(
        f.call(&f.recover(&journal.intent_id, "restore")).outcome,
        OperationReportOutcome::Refused
    );
    assert_eq!(f.target(), b"external edit");
    std::fs::remove_file(f.project.join("vibe.toml")).unwrap();
    assert_eq!(
        f.call(&f.recover(&journal.intent_id, "complete"))
            .target_state,
        OperationReportTargetState::Absent
    );
    assert!(!f.project.join("vibe.toml").exists());
    assert!(f.directory().join("before.bin").exists());
}

fn crash(f: &Fixture, boundary: &str, action: &str) {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("MIGRATION_TEST_PROJECT", &f.project)
        .env("MIGRATION_TEST_STORE", &f.store)
        .env("MIGRATION_TEST_BOUNDARY", boundary)
        .env("MIGRATION_TEST_ACTION", action)
        .args([
            "--exact",
            "migration_application::runtime::runtime_tests::restart_probe",
            "--ignored",
        ])
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(73),
        "actual abrupt boundary probe: {boundary}"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn process_restart_apply_boundaries_preserve_supported_prefixes() {
    for point in [
        "before_store_bootstrap",
        "after_store_bootstrap",
        "before_barrier_create",
        "after_barrier_create",
        "before_before_image",
        "after_before_image",
        "before_prepared_journal",
        "after_prepared_journal",
        "before_publishing_journal",
        "after_publishing_journal",
        "before_retained_image_sync",
        "after_retained_image_sync",
        "before_target_publish",
        "after_target_publish",
        "before_published_journal",
        "after_published_journal",
    ] {
        let f = Fixture::new();
        crash(&f, point, "apply");
        let before_inspect = tree(f._temp.path());
        let _ = f.call(&f.inspect());
        assert_eq!(
            tree(f._temp.path()),
            before_inspect,
            "inspect wrote state at {point}"
        );
        if let Some(j) = f.journal() {
            assert!(matches!(
                f.call(&f.recover(&j.intent_id, "complete")).outcome,
                OperationReportOutcome::Completed | OperationReportOutcome::NoOp
            ));
        } else {
            // No valid intent authorizes automatic recovery. A separately
            // explicit approved apply may prepare the exact retained image.
            assert_eq!(
                f.call(&f.apply()).outcome,
                OperationReportOutcome::Completed
            );
        }
        assert_eq!(
            f.target(),
            BEFORE.replace("'snapshot'", "'copy'").as_bytes()
        );
        let stable = tree(f._temp.path());
        assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::NoOp);
        assert_eq!(tree(f._temp.path()), stable);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn process_restart_restore_boundaries_are_idempotent() {
    for point in [
        "before_restoring_journal",
        "after_restoring_journal",
        "before_retained_image_sync",
        "after_retained_image_sync",
        "before_target_restore",
        "after_target_restore",
        "before_restored_journal",
        "after_restored_journal",
    ] {
        let f = Fixture::new();
        f.call(&f.apply());
        let id = f.journal().unwrap().intent_id;
        crash(&f, point, "restore");
        let r = f.call(&f.recover(&id, "restore"));
        assert!(matches!(
            r.outcome,
            OperationReportOutcome::Completed | OperationReportOutcome::NoOp
        ));
        assert_eq!(f.target(), BEFORE.as_bytes());
        let stable = tree(f._temp.path());
        assert_eq!(
            f.call(&f.recover(&id, "restore")).outcome,
            OperationReportOutcome::NoOp
        );
        assert_eq!(tree(f._temp.path()), stable);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn impossible_prefixes_and_foreign_stages_preserve_evidence() {
    let f = Fixture::new();
    f.call(&f.apply());
    let mut j = f.journal().unwrap();
    j.phase = ApplicationJournalPhase::Prepared;
    std::fs::write(
        f.directory().join("journal.json"),
        serde_json::to_vec(&j).unwrap(),
    )
    .unwrap();
    assert_eq!(
        f.call(&f.recover(&j.intent_id, "complete")).outcome,
        OperationReportOutcome::Refused
    );
    let f = Fixture::new();
    crash(&f, "after_prepared_journal", "apply");
    let j = f.journal().unwrap();
    let stage = identity::transaction_stage_name(&j.intent_id, identity::StageAction::Apply);
    std::fs::write(f.project.join(&stage), b"foreign stage").unwrap();
    let r = f.call(&f.recover(&j.intent_id, "complete"));
    assert_eq!(r.outcome, OperationReportOutcome::Refused);
    assert_eq!(f.target(), BEFORE.as_bytes());
    assert_eq!(
        std::fs::read(f.project.join(stage)).unwrap(),
        b"foreign stage"
    );
    assert!(!r.residue.is_empty());
    assert!(f.directory().join("before.bin").exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn no_op_no_images_and_failed_journal_keep_prior_effects() {
    let f = Fixture::new();
    let after = BEFORE.replace("'snapshot'", "'copy'");
    std::fs::write(f.project.join("vibe.toml"), &after).unwrap();
    let approved = json!({"schema":1,"source_text":after,"source_profile":CURRENT_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":IDENTITY}).to_string();
    let MigrationPlanningResponse::Report(p) = plan_migration_json(&approved) else {
        panic!("plan")
    };
    let vibe_wire::generated::migration::e1::plan_report::Outcome::NoOp(p) = p.outcome else {
        panic!("noop")
    };
    let request = json!({"schema":1,"operation":"apply","operation_id":"operation-one","authority":f.authority(),"approved_request_json":approved,"before_content_id":p.plan.before_identity,"after_content_id":p.plan.after_identity,"diff_id":p.plan.diff_identity}).to_string();
    assert_eq!(f.call(&request).outcome, OperationReportOutcome::NoOp);
    assert!(!f.directory().join("before.bin").exists());
    assert!(!f.directory().join("journal.json").exists());
    let f = Fixture::new();
    crash(&f, "after_before_image", "apply");
    std::fs::create_dir(f.directory().join("journal.json")).unwrap();
    let r = f.call(&f.apply());
    assert_eq!(r.outcome, OperationReportOutcome::Refused);
    assert!(
        r.effects
            .iter()
            .any(|e| e.kind == EffectKind::Barrier && e.state == EffectState::Completed)
    );
    assert_eq!(f.target(), BEFORE.as_bytes());
    assert!(f.directory().join("before.bin").exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn prior_checkpoint_and_unknown_durability_are_not_erased_by_later_attempts() {
    let mut report = ReportBuilder::new(
        Some(OperationReportOperation::Apply),
        Some("probe".to_owned()),
    );
    set_effect(&mut report, EffectKind::Journal, EffectState::Completed);
    let error = PublishError {
        stage: PublishStage::BeforePublication,
        created_directories: vec![],
        source: std::io::Error::other("test-only before-publication failure").into(),
    };
    publication_failure(
        &mut report,
        EffectKind::Journal,
        EffectState::Completed,
        &error,
        ResidueLocation::Store,
        Path::new("/"),
    );
    assert_eq!(
        effect_state(&mut report, &EffectKind::Journal),
        EffectState::Completed
    );
    set_effect(&mut report, EffectKind::Journal, EffectState::Uncertain);
    set_effect(&mut report, EffectKind::Journal, EffectState::Completed);
    assert_eq!(
        effect_state(&mut report, &EffectKind::Journal),
        EffectState::Uncertain
    );
    unconfirmed(&mut report);
    observe_durability(&mut report, DirectoryDurability::Synced).unwrap();
    assert_eq!(
        report.evidence_mut().guarantees.durability,
        GuaranteesDurability::Unconfirmed
    );
    let f = Fixture::new();
    let store =
        vibe_safefs::ExternalStore::open_or_create_disjoint(&f.store, f.grant().project()).unwrap();
    let token = f.grant().project().identity_token().unwrap();
    let lock = store.open_and_lock_project(&token).unwrap();
    std::fs::create_dir_all(f.directory()).unwrap();
    std::fs::write(f.directory().join("before.bin"), BEFORE).unwrap();
    drop(lock);
    let r = f.call(&f.apply());
    assert_eq!(r.outcome, OperationReportOutcome::Completed);
    assert_eq!(
        r.guarantees.durability,
        GuaranteesDurability::Unconfirmed,
        "later journal sync cannot authenticate an earlier orphan image sync"
    );
    assert_eq!(
        std::fs::read(f.directory().join("before.bin")).unwrap(),
        BEFORE.as_bytes()
    );
}

#[test]
#[ignore = "child-only abrupt restart probe, selected exactly by parent cases"]
fn restart_probe() {
    let project = PathBuf::from(std::env::var_os("MIGRATION_TEST_PROJECT").unwrap());
    let store = PathBuf::from(std::env::var_os("MIGRATION_TEST_STORE").unwrap());
    let point = std::env::var("MIGRATION_TEST_BOUNDARY").unwrap();
    let action = std::env::var("MIGRATION_TEST_ACTION").unwrap();
    let grant = OwnedManifestMigrationFile::new(&project, "vibe.toml", &store).unwrap();
    let authority = json!({"project_root":project,"recovery_store_root":store,"target_relative":"vibe.toml","scope":"owned_authored_file"});
    let text = if action == "apply" {
        let approved = json!({"schema":1,"source_text":BEFORE,"source_profile":LEGACY_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":SNAPSHOT_TO_COPY}).to_string();
        let MigrationPlanningResponse::Report(p) = plan_migration_json(&approved) else {
            panic!("plan")
        };
        let vibe_wire::generated::migration::e1::plan_report::Outcome::Planned(p) = p.outcome
        else {
            panic!("plan")
        };
        json!({"schema":1,"operation":"apply","operation_id":"operation-one","authority":authority,"approved_request_json":approved,"before_content_id":p.plan.before_identity,"after_content_id":p.plan.after_identity,"diff_id":p.plan.diff_identity}).to_string()
    } else {
        let key = identity::operation_storage_key(
            &grant.project().identity_token().unwrap(),
            "operation-one",
        );
        let j: ApplicationJournal = serde_json::from_slice(
            &std::fs::read(store.join("migrations").join(key).join("journal.json")).unwrap(),
        )
        .unwrap();
        json!({"schema":1,"operation":"recover","operation_id":"operation-one","authority":authority,"expected_intent_id":j.intent_id,"action":"restore"}).to_string()
    };
    stop_at(point);
    let _ = execute_manifest_migration_json(&text, &grant);
    panic!("requested boundary was not reached");
}
