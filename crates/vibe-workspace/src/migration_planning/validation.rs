//! Relational wire checks and replay against the existing pure planner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");
use super::*;
fn is_preflight(code: &Code) -> bool {
    matches!(
        code,
        Code::InvalidRequest | Code::UnsupportedSchema | Code::InputRead
    )
}

/// Check a standalone preflight root; it cannot carry planner refusal codes.
/// ```
/// use vibe_workspace::migration_planning::{input_read_error, validate_preflight_error};
/// validate_preflight_error(&input_read_error()).unwrap();
/// ```
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

pub(super) fn validate_report(report: &PlanReport) -> Validated<()> {
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
                let identity = question.field.clone();
                if fields.contains(&identity) {
                    return Err(invalid(
                        "questions.field",
                        "duplicate input field and resource",
                    ));
                }
                fields.push(identity);
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

pub(super) fn validate_plan(plan: &Plan, no_op: bool) -> Validated<()> {
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
    // Wire spelling is checked here; edit meaning and identities have one
    // owner: regenerate this exact proposal through the accepted core planner.
    for edit in &plan.edits {
        decimal(&edit.before_start)?;
        decimal(&edit.before_end)?;
    }
    let request = PlanRequest {
        schema: 1,
        source_text: plan.before.clone(),
        source_profile: Some(plan.source_profile.clone()),
        target_profile: Some(plan.target_profile.clone()),
        recipe: Some(plan.recipe.clone()),
    };
    let expected = match core_outcome(&request)? {
        ManifestMigrationOutcome::Planned(expected) if !no_op => expected,
        ManifestMigrationOutcome::NoOp(expected) if no_op => expected,
        _ => {
            return Err(invalid(
                "plan",
                "selected core recipe does not produce this outcome",
            ));
        }
    };
    if project_plan(expected)? != *plan {
        return Err(invalid(
            "plan",
            "proposal differs from the exact core recipe result",
        ));
    }
    Ok(())
}

/// Validate an untrusted report against its exact request, replaying only the
/// existing pure core owner. No duplicate grammar, transformation or hash law.
/// ```
/// use vibe_workspace::migration_planning::*;
/// let request = decode_migration_request(serde_json::json!({"schema":1,"source_text":""})).unwrap();
/// let MigrationPlanningResponse::Report(report) = plan_migration_request(request.clone()) else { panic!("planner result") };
/// validate_report_for_request(&request, &report).unwrap();
/// ```
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
