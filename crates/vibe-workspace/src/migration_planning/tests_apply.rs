//! Source descriptor migration through the incumbent M-13 application owner.

use std::fs;

use serde_json::json;
use vibe_wire::generated::migration::e1::{
    operation_report::OperationReportOutcome, plan_report::Outcome, plan_request::PlanRequest,
};

use super::{fixture, plan_migration_request, validate_report_for_request};
use crate::migration_application::{
    OwnedManifestMigrationFile, execute_manifest_migration_json, validate_operation_report,
};
use crate::migration_planning::MigrationPlanningResponse;

fn planned_request() -> (
    PlanRequest,
    String,
    vibe_wire::generated::migration::e1::plan_report::Plan,
) {
    let value = fixture("migration-plan-request", "valid", "source-decision");
    let approved = value.to_string();
    let request: PlanRequest = serde_json::from_value(value).unwrap();
    let MigrationPlanningResponse::Report(report) = plan_migration_request(request.clone()) else {
        panic!("source planner report")
    };
    validate_report_for_request(&request, &report).unwrap();
    let Outcome::Planned(planned) = report.outcome else {
        panic!("source edit plan")
    };
    (request, approved, planned.plan)
}

fn operation(
    project: &std::path::Path,
    store: &std::path::Path,
    approved: &str,
    plan: &vibe_wire::generated::migration::e1::plan_report::Plan,
) -> String {
    json!({
        "schema": 1,
        "operation": "apply",
        "operation_id": "source-operation-one",
        "authority": {
            "project_root": project,
            "recovery_store_root": store,
            "target_relative": "vibe.toml",
            "scope": "owned_authored_file"
        },
        "approved_request_json": approved,
        "before_content_id": plan.before_identity,
        "after_content_id": plan.after_identity,
        "diff_id": plan.diff_identity
    })
    .to_string()
}

#[test]
fn explicit_source_question_plan_and_application_preserve_original_manifest_bytes() {
    let (request, approved, plan) = planned_request();
    let mut undecided = request.clone();
    undecided.source_decisions.clear();
    let MigrationPlanningResponse::Report(pending) = plan_migration_request(undecided.clone())
    else {
        panic!("path-specific question")
    };
    validate_report_for_request(&undecided, &pending).unwrap();
    let Outcome::NeedsInput(questions) = pending.outcome else {
        panic!("media question")
    };
    assert_eq!(questions.questions.len(), 1);
    assert_eq!(
        questions.questions[0].resource_path.as_deref(),
        Some("vibevm/vibespecs/boot/10-flow.xml")
    );

    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    let store = temp.path().join("store");
    fs::create_dir(&project).unwrap();
    let target = project.join("vibe.toml");
    fs::write(&target, &request.source_text).unwrap();
    let grant = OwnedManifestMigrationFile::new(&project, "vibe.toml", &store).unwrap();
    let apply = operation(&project, &store, &approved, &plan);
    let completed = execute_manifest_migration_json(&apply, &grant).unwrap();
    validate_operation_report(&completed).unwrap();
    assert_eq!(
        completed.outcome,
        OperationReportOutcome::Completed,
        "{completed:#?}"
    );
    assert_eq!(fs::read(&target).unwrap(), plan.after.as_bytes());
    assert!(plan.after.starts_with(&request.source_text));
    assert!(plan.after.contains("[[package.sources]]"));
    assert_eq!(
        vibe_core::manifest::Manifest::parse_str(&plan.after)
            .unwrap()
            .package
            .unwrap()
            .sources
            .len(),
        1
    );
    let retained = fs::read(&target).unwrap();
    let repeated = execute_manifest_migration_json(&apply, &grant).unwrap();
    validate_operation_report(&repeated).unwrap();
    assert_eq!(repeated.outcome, OperationReportOutcome::NoOp);
    assert_eq!(fs::read(&target).unwrap(), retained);
}

#[test]
fn changed_authored_source_refuses_without_target_write_or_journal() {
    let (request, approved, plan) = planned_request();
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    let store = temp.path().join("store");
    fs::create_dir(&project).unwrap();
    let target = project.join("vibe.toml");
    let changed = request.source_text.replace(
        "description = 'unchanged value'",
        "description = 'owner changed value'",
    );
    fs::write(&target, &changed).unwrap();
    let grant = OwnedManifestMigrationFile::new(&project, "vibe.toml", &store).unwrap();
    let refused =
        execute_manifest_migration_json(&operation(&project, &store, &approved, &plan), &grant)
            .unwrap();
    validate_operation_report(&refused).unwrap();
    assert_eq!(refused.outcome, OperationReportOutcome::Refused);
    assert_eq!(fs::read(&target).unwrap(), changed.as_bytes());
    // The incumbent owner may bootstrap its disjoint recovery store before
    // observing the target. It must not publish an operation journal or write
    // the changed authored file after the source mismatch.
    let key = crate::migration_application::identity::operation_storage_key(
        &grant.project().identity_token().unwrap(),
        "source-operation-one",
    );
    assert!(
        !store
            .join("migrations")
            .join(key)
            .join("journal.json")
            .exists()
    );
}
