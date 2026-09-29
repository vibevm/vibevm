//! Explicit one-file operation admission before any normal CLI initialization.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES");
use crate::exit_code::{GENERAL, OK, USAGE};
use std::io::{Read, Write};
use std::path::Path;
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
fn execute(text: &str) -> Result<OperationReport, ReportValidationFailure> {
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
    // This explicit CLI invocation admits exactly the payload-selected authored
    // file. The existing grant and operation owners enforce physical/path policy.
    let grant = match OwnedManifestMigrationFile::new(
        Path::new(&authority.project_root),
        &authority.target_relative,
        Path::new(&authority.recovery_store_root),
    ) {
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
        "vibe: migration operation internal report invariant failure: {}",
        failure.fault
    )?;
    diagnostic.write_all(b"UNVALIDATED original operation evidence: ")?;
    serde_json::to_writer(&mut *diagnostic, &failure.report).map_err(std::io::Error::other)?;
    diagnostic.write_all(b"\n")
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
pub(crate) fn run(request_json: Option<&str>) -> ExitCode {
    let mut input_failed = false;
    let result = match request_json {
        Some(text) => execute(text),
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
                execute(&text)
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
            &ReportValidationFailure { report, fault },
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
            eprintln!(
                "vibe: migration operation response could not be serialized; effects may have occurred"
            );
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
            "vibe: migration operation response could not be written; effects may have occurred"
        );
        return ExitCode::from(GENERAL);
    }
    ExitCode::from(code)
}
