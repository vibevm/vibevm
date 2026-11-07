//! Explicit one-file operation admission before any normal CLI initialization.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES");
use super::MigrationOperationArgs;
use crate::exit_code::{GENERAL, OK, USAGE};
use crate::output::language::Refusal;
use std::io::{Read, Write};
use std::process::ExitCode;
use vibe_wire::generated::migration::e1::{
    operation_report::*,
    operation_request::{Authority, OperationRequest},
};
use vibe_workspace::migration_application::{
    ApplicationValidationError, OwnedManifestMigrationFile, ReportBuilder, ReportValidationFailure,
    execute_manifest_migration_json, validate_operation_report,
};

fn preflight(
    code: ApplicationErrorCode,
    field: &'static str,
    reason: &'static str,
    identity: Option<(OperationReportOperation, String)>,
) -> OperationReport {
    let (operation, operation_id) = match identity {
        Some((op, id)) => (Some(op), Some(id)),
        None => (None, None),
    };
    ReportBuilder::preflight(&ApplicationValidationError {
        code,
        field,
        reason,
        operation,
        operation_id,
    })
}

/// Strict generated shape and schema admission precede any project open.
fn execute(
    text: &str,
    args: &MigrationOperationArgs,
) -> Result<OperationReport, ReportValidationFailure> {
    let request: OperationRequest = match serde_json::from_str(text) {
        Ok(request) => request,
        Err(_) => {
            return Ok(preflight(
                ApplicationErrorCode::InvalidRequest,
                "request",
                "invalid operation request shape",
                None,
            ));
        }
    };
    let (schema, authority, operation, id): (u32, &Authority, _, _) = match &request {
        OperationRequest::Apply(r) => (
            r.schema,
            &r.authority,
            OperationReportOperation::Apply,
            &r.operation_id,
        ),
        OperationRequest::Inspect(r) => (
            r.schema,
            &r.authority,
            OperationReportOperation::Inspect,
            &r.operation_id,
        ),
        OperationRequest::Recover(r) => (
            r.schema,
            &r.authority,
            OperationReportOperation::Recover,
            &r.operation_id,
        ),
    };
    let identity = (!id.trim().is_empty()).then(|| (operation.clone(), id.clone()));
    if schema != 1 {
        return Ok(preflight(
            ApplicationErrorCode::UnsupportedSchema,
            "schema",
            "unsupported operation request schema",
            identity,
        ));
    }
    if let OperationRequest::Apply(apply) = &request
        && matches!(
            vibe_workspace::migration_planning::plan_migration_json(&apply.approved_request_json),
            vibe_workspace::migration_planning::MigrationPlanningResponse::Preflight(_)
        )
    {
        return Ok(preflight(
            ApplicationErrorCode::InvalidRequest,
            "approved_request_json",
            "embedded planning request failed strict preflight",
            identity,
        ));
    }
    // The saved request supplies no authority: compare independent CLI grants
    // before constructing any capability or opening a project.
    let (Some(project_root), Some(target_relative), Some(recovery_store)) = (
        args.project_root.as_deref(),
        args.target_relative.as_deref(),
        args.recovery_store.as_deref(),
    ) else {
        return Ok(preflight(
            ApplicationErrorCode::AuthorityRefused,
            "authority",
            "explicit project-root, target-relative and recovery-store grants are required",
            identity,
        ));
    };
    if project_root.to_str() != Some(authority.project_root.as_str())
        || target_relative != authority.target_relative
        || recovery_store.to_str() != Some(authority.recovery_store_root.as_str())
    {
        return Ok(preflight(
            ApplicationErrorCode::AuthorityRefused,
            "authority",
            "explicit project-root, target-relative and recovery-store grants must exactly match the request",
            identity,
        ));
    }
    let grant = match OwnedManifestMigrationFile::new(project_root, target_relative, recovery_store)
        .and_then(|grant| {
            let cache = vibe_core::settings::settings_dir()
                .map(|root| root.join("cache"))
                .ok_or(ApplicationValidationError {
                    code: ApplicationErrorCode::AuthorityRefused,
                    field: "authority",
                    reason: "protected machine cache root is unavailable",
                    operation: None,
                    operation_id: None,
                })?;
            grant.with_protected_machine_cache_root(&cache)
        }) {
        Ok(grant) => grant,
        Err(mut error) => {
            if let Some((operation, id)) = identity {
                error.operation = Some(operation);
                error.operation_id = Some(id);
            }
            return Ok(ReportBuilder::preflight(&error));
        }
    };
    execute_manifest_migration_json(text, &grant)
}

/// Failure diagnostics preserve every original field without asserting validity.
pub(crate) fn render_failure(
    failure: &ReportValidationFailure,
    diagnostic: &mut impl Write,
) -> std::io::Result<()> {
    writeln!(
        diagnostic,
        "{}",
        Refusal {
            headline: format!(
                "migration operation internal report invariant failure: {}",
                failure.fault
            ),
            ..Default::default()
        }
        .block()
        .render()
    )?;
    diagnostic.write_all(b"UNVALIDATED original operation evidence: ")?;
    serde_json::to_writer(&mut *diagnostic, &failure.report).map_err(std::io::Error::other)?;
    diagnostic.write_all(b"\n")
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
pub(crate) fn run(args: &MigrationOperationArgs) -> ExitCode {
    let mut input_failed = false;
    let result = match args.request_json.as_deref() {
        Some(text) => execute(text, args),
        None => {
            let mut text = String::new();
            if std::io::stdin().read_to_string(&mut text).is_err() {
                input_failed = true;
                Ok(preflight(
                    ApplicationErrorCode::InvalidRequest,
                    "request.input",
                    "operation request input could not be read",
                    None,
                ))
            } else {
                execute(&text, args)
            }
        }
    };
    let report = match result {
        Ok(report) => report,
        Err(failure) => {
            let _ = render_failure(&failure, &mut std::io::stderr().lock());
            return ExitCode::from(GENERAL);
        }
    };
    // Surface-created preflight values have the same finite owner validation.
    if let Err(fault) = validate_operation_report(&report) {
        let _ = render_failure(
            &ReportValidationFailure {
                report: Box::new(report),
                fault,
            },
            &mut std::io::stderr().lock(),
        );
        return ExitCode::from(GENERAL);
    }
    let code = if input_failed {
        GENERAL
    } else {
        match report.outcome {
            OperationReportOutcome::Completed | OperationReportOutcome::NoOp => OK,
            OperationReportOutcome::Pending => USAGE,
            _ if report.error.as_ref().is_some_and(|e| {
                matches!(
                    e.code,
                    ApplicationErrorCode::InvalidRequest | ApplicationErrorCode::UnsupportedSchema
                )
            }) =>
            {
                USAGE
            }
            _ => GENERAL,
        }
    };
    let mut bytes = match serde_json::to_vec(&report) {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("{}", Refusal {
                headline: "migration operation response could not be serialized; effects may have occurred".into(),
                ..Default::default()
            }.block().render());
            return ExitCode::from(GENERAL);
        }
    };
    bytes.push(b'\n');
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(&bytes)
        .and_then(|()| stdout.flush())
        .is_err()
    {
        eprintln!(
            "{}",
            Refusal {
                headline:
                    "migration operation response could not be written; effects may have occurred"
                        .into(),
                ..Default::default()
            }
            .block()
            .render()
        );
        return ExitCode::from(GENERAL);
    }
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
        r = 1
    )]
    fn internal_invariant_diagnostic_retains_unvalidated_original_evidence() {
        let mut builder = ReportBuilder::new(
            Some(OperationReportOperation::Apply),
            Some("inconsistent-observation".into()),
        );
        let r = builder.evidence_mut();
        r.guarantees.uncooperating_writers_excluded = true;
        r.effects.push(Effect {
            kind: EffectKind::TargetPublication,
            state: EffectState::Completed,
        });
        let failure = builder.into_report().unwrap_err();
        let original = serde_json::to_value(&failure.report).unwrap();
        let mut diagnostic = Vec::new();
        render_failure(&failure, &mut diagnostic).unwrap();
        let text = String::from_utf8(diagnostic).unwrap();
        assert!(text.contains("internal report invariant failure"));
        let raw = text
            .split("UNVALIDATED original operation evidence: ")
            .nth(1)
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(raw.trim()).unwrap(), original);
        assert!(text.contains("target_publication"));
        assert!(text.contains("completed"));
    }
}
