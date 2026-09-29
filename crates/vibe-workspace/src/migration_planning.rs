//! Strict, text-only projections of the pure authored-manifest planner.
//!
//! JSON callers use the presence-aware decoder; a generated Option alone
//! cannot distinguish omitted choices from explicitly supplied null.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use serde::{Serialize, Serializer};
use serde_json::Value;
use vibe_core::manifest::{
    ManifestMigrationEpoch, ManifestMigrationOutcome, ManifestMigrationPlan, MigrationRefusalKind,
    MigrationSelection, manifest_migration_profile_epoch, plan_manifest_migration,
};
use vibe_wire::generated::migration::e1::{
    plan_error::{PlanError, PlanErrorStage},
    plan_report::{
        Epoch, EpochAbsent, EpochEpoch, MigrationPlanningError, MigrationPlanningErrorCode as Code,
        Outcome, OutcomeNeedsInput, OutcomeNoOp, OutcomePlanned, OutcomeRefused, Plan, PlanReport,
        PlanReportStage, Question, QuestionField, TokenEdit,
    },
    plan_request::PlanRequest,
};

/// Rust routing value; serialization delegates to exactly one generated root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationPlanningResponse {
    Report(PlanReport),
    Preflight(PlanError),
}

impl Serialize for MigrationPlanningResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Report(report) => report.serialize(serializer),
            Self::Preflight(error) => error.serialize(serializer),
        }
    }
}

/// Semantic validation fault, not another machine format or parser diagnosis.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("migration report field {field}: {reason}")]
pub struct MigrationReportValidationError {
    pub field: &'static str,
    pub reason: &'static str,
}

type Validated<T> = Result<T, MigrationReportValidationError>;
fn invalid(field: &'static str, reason: &'static str) -> MigrationReportValidationError {
    MigrationReportValidationError { field, reason }
}

fn preflight(code: Code, field: &str, reason: &str) -> PlanError {
    PlanError {
        schema: 1,
        stage: PlanErrorStage::Preflight,
        error: MigrationPlanningError {
            code,
            field: field.to_owned(),
            reason: reason.to_owned(),
        },
    }
}

/// A surface may report a failed stdin read without passing private I/O detail.
pub fn input_read_error() -> PlanError {
    preflight(
        Code::InputRead,
        "request",
        "request input could not be read",
    )
}

/// Parse JSON once, preserving typed preflight without entering the planner.
pub fn plan_migration_json(text: &str) -> MigrationPlanningResponse {
    match serde_json::from_str::<Value>(text) {
        Ok(value) => plan_migration_value(value),
        Err(_) => MigrationPlanningResponse::Preflight(preflight(
            Code::InvalidRequest,
            "request",
            "request is not valid JSON",
        )),
    }
}

pub fn plan_migration_value(value: Value) -> MigrationPlanningResponse {
    match decode_migration_request(value) {
        Ok(request) => plan_migration_request(request),
        Err(error) => MigrationPlanningResponse::Preflight(error),
    }
}

/// Optional choices must be strings when present, including before Option
/// lowering. Generated serde owns all other primitive and unknown-field rules.
pub fn decode_migration_request(value: Value) -> Result<PlanRequest, PlanError> {
    let object = value
        .as_object()
        .ok_or_else(|| preflight(Code::InvalidRequest, "request", "request must be an object"))?;
    for field in ["source_profile", "target_profile", "recipe"] {
        if object.get(field).is_some_and(|value| !value.is_string()) {
            return Err(preflight(
                Code::InvalidRequest,
                field,
                "optional selection must be a string when present",
            ));
        }
    }
    let request: PlanRequest = serde_json::from_value(value).map_err(|_| {
        preflight(
            Code::InvalidRequest,
            "request",
            "request does not match the registered shape",
        )
    })?;
    if request.schema != 1 {
        return Err(preflight(
            Code::UnsupportedSchema,
            "schema",
            "unsupported request schema",
        ));
    }
    Ok(request)
}

/// Typed callers have already lost JSON presence provenance; JSON callers
/// must use decode_migration_request or the JSON/Value entrypoints above.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
pub fn plan_migration_request(request: PlanRequest) -> MigrationPlanningResponse {
    if request.schema != 1 {
        return MigrationPlanningResponse::Preflight(preflight(
            Code::UnsupportedSchema,
            "schema",
            "unsupported request schema",
        ));
    }
    let report = match project_core(&request).and_then(|report| {
        validate_report(&report)?;
        Ok(report)
    }) {
        Ok(report) => report,
        Err(_) => PlanReport {
            schema: 1,
            stage: PlanReportStage::Planner,
            outcome: Outcome::Refused(Box::new(OutcomeRefused {
                error: MigrationPlanningError {
                    code: Code::UnprovableEdit,
                    field: "report".to_owned(),
                    reason: "planner result projection invariants could not be proved".to_owned(),
                },
            })),
        },
    };
    MigrationPlanningResponse::Report(report)
}

fn project_core(request: &PlanRequest) -> Validated<PlanReport> {
    let outcome = plan_manifest_migration(
        &request.source_text,
        MigrationSelection {
            source_profile: request.source_profile.as_deref(),
            target_profile: request.target_profile.as_deref(),
            recipe: request.recipe.as_deref(),
        },
    );
    let outcome = match outcome {
        ManifestMigrationOutcome::Planned(plan) => Outcome::Planned(Box::new(OutcomePlanned {
            plan: project_plan(plan)?,
        })),
        ManifestMigrationOutcome::NoOp(plan) => Outcome::NoOp(Box::new(OutcomeNoOp {
            plan: project_plan(plan)?,
        })),
        ManifestMigrationOutcome::NeedsInput(requirements) => {
            let mut questions = Vec::with_capacity(requirements.len());
            for requirement in requirements {
                let field = match requirement.field {
                    "source_profile" => QuestionField::SourceProfile,
                    "target_profile" => QuestionField::TargetProfile,
                    "recipe" => QuestionField::Recipe,
                    _ => return Err(invalid("questions.field", "unknown core input field")),
                };
                questions.push(Question {
                    field,
                    choices: requirement.choices.into_iter().map(str::to_owned).collect(),
                });
            }
            Outcome::NeedsInput(Box::new(OutcomeNeedsInput { questions }))
        }
        ManifestMigrationOutcome::Refused(refusal) => {
            let code = match refusal.kind {
                MigrationRefusalKind::UnknownSelection => Code::UnknownSelection,
                MigrationRefusalKind::IncompatibleEdge => Code::IncompatibleEdge,
                MigrationRefusalKind::MalformedSyntax => Code::MalformedSyntax,
                MigrationRefusalKind::UnrecognizedShape => Code::UnrecognizedShape,
                MigrationRefusalKind::InvalidCurrentManifest => Code::InvalidCurrentManifest,
                MigrationRefusalKind::UnprovableEdit => Code::UnprovableEdit,
            };
            Outcome::Refused(Box::new(OutcomeRefused {
                error: MigrationPlanningError {
                    code,
                    field: refusal.field,
                    reason: refusal.reason.to_owned(),
                },
            }))
        }
    };
    Ok(PlanReport {
        schema: 1,
        stage: PlanReportStage::Planner,
        outcome,
    })
}

fn epoch(profile: &str) -> Validated<Epoch> {
    match manifest_migration_profile_epoch(profile) {
        Some(ManifestMigrationEpoch::Absent) => Ok(Epoch::Absent(Box::new(EpochAbsent {}))),
        Some(ManifestMigrationEpoch::Epoch1) => Ok(Epoch::Epoch(Box::new(EpochEpoch {}))),
        None => Err(invalid(
            "plan.profile",
            "profile has no recognized epoch metadata",
        )),
    }
}

fn project_plan(plan: ManifestMigrationPlan) -> Validated<Plan> {
    Ok(Plan {
        source_epoch: epoch(plan.source_profile)?,
        target_epoch: epoch(plan.target_profile)?,
        source_profile: plan.source_profile.to_owned(),
        target_profile: plan.target_profile.to_owned(),
        recipe: plan.recipe.to_owned(),
        before: plan.before,
        after: plan.after,
        before_identity: plan.before_identity,
        after_identity: plan.after_identity,
        diff_identity: plan.diff_identity,
        edits: plan
            .edits
            .into_iter()
            .map(|edit| TokenEdit {
                field: edit.field.to_owned(),
                before_start: edit.before_span.start.to_string(),
                before_end: edit.before_span.end.to_string(),
                before_token: edit.before_token,
                after_token: edit.after_token,
            })
            .collect(),
    })
}

fn is_preflight(code: &Code) -> bool {
    matches!(
        code,
        Code::InvalidRequest | Code::UnsupportedSchema | Code::InputRead
    )
}

/// Check a standalone preflight root; it cannot carry planner refusal codes.
pub fn validate_preflight_error(error: &PlanError) -> Validated<()> {
    if error.schema != 1 {
        return Err(invalid("schema", "unsupported error schema"));
    }
    match error.stage {
        PlanErrorStage::Preflight => {}
    }
    if !is_preflight(&error.error.code) {
        return Err(invalid(
            "error.code",
            "planner refusal code in preflight root",
        ));
    }
    Ok(())
}

fn validate_report(report: &PlanReport) -> Validated<()> {
    if report.schema != 1 {
        return Err(invalid("schema", "unsupported report schema"));
    }
    match report.stage {
        PlanReportStage::Planner => {}
    }
    match &report.outcome {
        Outcome::Planned(outcome) => validate_plan(&outcome.plan, false),
        Outcome::NoOp(outcome) => validate_plan(&outcome.plan, true),
        Outcome::Refused(outcome) => {
            if is_preflight(&outcome.error.code) {
                return Err(invalid(
                    "outcome.error.code",
                    "preflight code in planner refusal",
                ));
            }
            Ok(())
        }
        Outcome::NeedsInput(outcome) => {
            if outcome.questions.is_empty() {
                return Err(invalid(
                    "questions",
                    "input-required report has no questions",
                ));
            }
            let mut fields = Vec::new();
            for question in &outcome.questions {
                if fields.contains(&question.field) {
                    return Err(invalid("questions.field", "duplicate input field"));
                }
                fields.push(question.field.clone());
                if question.choices.is_empty()
                    || question.choices.iter().any(|choice| choice.is_empty())
                {
                    return Err(invalid("questions.choices", "input choices are empty"));
                }
                let mut choices = Vec::new();
                for choice in &question.choices {
                    if choices.contains(choice) {
                        return Err(invalid("questions.choices", "duplicate choice"));
                    }
                    choices.push(choice.clone());
                }
            }
            Ok(())
        }
    }
}

fn decimal(text: &str) -> Validated<usize> {
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(invalid(
            "edits.offset",
            "offset is not canonical unsigned decimal",
        ));
    }
    text.parse().map_err(|_| {
        invalid(
            "edits.offset",
            "offset exceeds source addressable byte bounds",
        )
    })
}

fn validate_plan(plan: &Plan, no_op: bool) -> Validated<()> {
    if epoch(&plan.source_profile)? != plan.source_epoch
        || epoch(&plan.target_profile)? != plan.target_epoch
    {
        return Err(invalid(
            "plan.epoch",
            "epoch does not match core profile metadata",
        ));
    }
    if no_op {
        if !plan.edits.is_empty()
            || plan.before != plan.after
            || plan.before_identity != plan.after_identity
        {
            return Err(invalid(
                "plan",
                "no-op must preserve exact text, content identity and empty edits",
            ));
        }
    } else if plan.edits.is_empty() || plan.before == plan.after {
        return Err(invalid("edits", "planned output must contain a real edit"));
    }
    let mut cursor = 0;
    let mut after = String::new();
    for edit in &plan.edits {
        let start = decimal(&edit.before_start)?;
        let end = decimal(&edit.before_end)?;
        if start < cursor || end <= start {
            return Err(invalid(
                "edits",
                "edits are empty, overlapping or out of source order",
            ));
        }
        let prefix = plan
            .before
            .get(cursor..start)
            .ok_or_else(|| invalid("edits.offset", "offset is outside UTF-8 source boundaries"))?;
        let token = plan
            .before
            .get(start..end)
            .ok_or_else(|| invalid("edits.offset", "offset is outside UTF-8 source boundaries"))?;
        if token != edit.before_token {
            return Err(invalid(
                "edits.before_token",
                "token does not match original source span",
            ));
        }
        after.push_str(prefix);
        after.push_str(&edit.after_token);
        cursor = end;
    }
    after.push_str(
        plan.before
            .get(cursor..)
            .ok_or_else(|| invalid("edits.offset", "offset is outside source"))?,
    );
    if after != plan.after {
        return Err(invalid(
            "plan.after",
            "declared edits do not reconstruct output",
        ));
    }
    Ok(())
}

/// Validate an untrusted report against its exact request, replaying only the
/// existing pure core owner. No duplicate grammar, transformation or hash law.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
pub fn validate_report_for_request(request: &PlanRequest, report: &PlanReport) -> Validated<()> {
    if request.schema != 1 {
        return Err(invalid("request.schema", "unsupported request schema"));
    }
    validate_report(report)?;
    let expected = project_core(request)?;
    validate_report(&expected)?;
    if &expected != report {
        return Err(invalid(
            "report",
            "report differs from exact request's pure core result",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "migration_planning/tests.rs"]
mod tests;
