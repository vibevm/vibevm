//! Opt-in epoch-2 projection of the same core planner, including ambiguity.
use e2::{
    plan_error::{PlanError, PlanErrorStage},
    plan_report::*,
    plan_request::PlanRequest,
};
use serde_json::Value;
use vibe_core::manifest::{ManifestMigrationOutcome, MigrationRefusalKind};
use vibe_wire::generated::migration::{e1, e2};

fn error(field: &str, reason: &str) -> PlanError {
    PlanError {
        schema: 2,
        stage: PlanErrorStage::Preflight,
        error: MigrationPlanningError {
            code: MigrationPlanningErrorCode::InvalidRequest,
            field: field.into(),
            reason: reason.into(),
        },
    }
}
fn convert<T: serde::de::DeserializeOwned>(v: impl serde::Serialize) -> Result<T, PlanError> {
    serde_json::to_value(v)
        .and_then(serde_json::from_value)
        .map_err(|_| error("projection", "generated representation mismatch"))
}

pub fn plan_feature_migration_json(text: &str) -> Result<PlanReport, PlanError> {
    let value: Value = serde_json::from_str(text).map_err(|_| error("request", "invalid JSON"))?;
    let object = value
        .as_object()
        .ok_or_else(|| error("request", "expected object"))?;
    for key in ["source_profile", "target_profile", "recipe"] {
        if object.get(key).is_some_and(|v| !v.is_string()) {
            return Err(error(key, "optional choice must be a string when present"));
        }
    }
    let request: PlanRequest = serde_json::from_value(value)
        .map_err(|_| error("request", "invalid epoch-2 request shape"))?;
    if request.schema != 2 {
        return Err(error("schema", "expected schema 2"));
    }
    // Request fields are unchanged across epochs; only the report's closed
    // question vocabulary expands. Generated types own both representations.
    let mut old: e1::plan_request::PlanRequest = convert(request)?;
    old.schema = 1;
    super::validate_source_decisions(&old)
        .map_err(|_| error("source_decisions", "invalid source decision"))?;
    let outcome =
        super::core_outcome(&old).map_err(|_| error("request", "core admission failed"))?;
    let outcome = match outcome {
        ManifestMigrationOutcome::Planned(p) => Outcome::Planned(Box::new(OutcomePlanned {
            plan: convert(super::project_plan(p).map_err(|_| error("plan", "invalid plan"))?)?,
        })),
        ManifestMigrationOutcome::NoOp(p) => Outcome::NoOp(Box::new(OutcomeNoOp {
            plan: convert(super::project_plan(p).map_err(|_| error("plan", "invalid plan"))?)?,
        })),
        ManifestMigrationOutcome::NeedsInput(requirements) => {
            let mut questions = Vec::new();
            for r in requirements {
                let field = match r.field {
                    "source_profile" => QuestionField::SourceProfile,
                    "target_profile" => QuestionField::TargetProfile,
                    "recipe" => QuestionField::Recipe,
                    "source_media" => QuestionField::SourceMedia,
                    "source_interpretation" => QuestionField::SourceInterpretation,
                    "source_role" => QuestionField::SourceRole,
                    "source_loading" => QuestionField::SourceLoading,
                    "feature_reference" => QuestionField::FeatureReference,
                    _ => return Err(error("questions.field", "unknown core question")),
                };
                if r.choices.is_empty() || r.choices.iter().any(String::is_empty) {
                    return Err(error("questions.choices", "empty core choices"));
                }
                if (r.field == "feature_reference") != r.authored_reference.is_some()
                    || (r.authored_reference.is_some() && r.resource_path.is_some())
                {
                    return Err(error("questions", "invalid reference identity"));
                }
                questions.push(Question {
                    field,
                    choices: r.choices,
                    resource_path: r.resource_path,
                    authored_reference: r.authored_reference,
                });
            }
            Outcome::NeedsInput(Box::new(OutcomeNeedsInput { questions }))
        }
        ManifestMigrationOutcome::Refused(r) => {
            let code = match r.kind {
                MigrationRefusalKind::UnknownSelection => {
                    MigrationPlanningErrorCode::UnknownSelection
                }
                MigrationRefusalKind::IncompatibleEdge => {
                    MigrationPlanningErrorCode::IncompatibleEdge
                }
                MigrationRefusalKind::MalformedSyntax => {
                    MigrationPlanningErrorCode::MalformedSyntax
                }
                MigrationRefusalKind::UnrecognizedShape => {
                    MigrationPlanningErrorCode::UnrecognizedShape
                }
                MigrationRefusalKind::InvalidCurrentManifest => {
                    MigrationPlanningErrorCode::InvalidCurrentManifest
                }
                MigrationRefusalKind::UnprovableEdit => MigrationPlanningErrorCode::UnprovableEdit,
            };
            Outcome::Refused(Box::new(OutcomeRefused {
                error: MigrationPlanningError {
                    code,
                    field: r.field,
                    reason: r.reason.into(),
                },
            }))
        }
    };
    Ok(PlanReport {
        schema: 2,
        stage: PlanReportStage::Planner,
        outcome,
    })
}

/// Replay is the validator: no untrusted after bytes bypass the core owner.
pub fn validate_report_for_request(text: &str, report: &PlanReport) -> Result<(), PlanError> {
    if &plan_feature_migration_json(text)? != report {
        return Err(error("report", "report differs from exact request replay"));
    }
    Ok(())
}

pub(crate) fn application_plan(text: &str) -> Result<(e1::plan_report::Plan, bool), PlanError> {
    match plan_feature_migration_json(text)?.outcome {
        Outcome::Planned(p) => Ok((convert(p.plan)?, false)),
        Outcome::NoOp(p) => Ok((convert(p.plan)?, true)),
        _ => Err(error(
            "approved_request_json",
            "input does not authorize application",
        )),
    }
}
