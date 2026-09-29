//! The pre-initialization, text-only migration planning entry.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use std::io::{Read, Write};
use std::process::ExitCode;
use vibe_wire::generated::migration::e1::plan_report::{MigrationPlanningErrorCode, Outcome};
use vibe_workspace::migration_planning::{
    MigrationPlanningResponse, input_read_error, plan_migration_json,
};

use crate::exit_code::{GENERAL, OK, USAGE};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
pub(crate) fn run(request_json: Option<&str>) -> ExitCode {
    let response = match request_json {
        Some(text) => plan_migration_json(text),
        None => {
            let mut text = String::new();
            match std::io::stdin().read_to_string(&mut text) {
                Ok(_) => plan_migration_json(&text),
                Err(_) => MigrationPlanningResponse::Preflight(input_read_error()),
            }
        }
    };
    let code = match &response {
        MigrationPlanningResponse::Report(report) => match &report.outcome {
            Outcome::Planned(_) | Outcome::NoOp(_) => OK,
            Outcome::NeedsInput(_) => USAGE,
            Outcome::Refused(_) => GENERAL,
        },
        MigrationPlanningResponse::Preflight(error) => match error.error.code {
            MigrationPlanningErrorCode::InputRead => GENERAL,
            _ => USAGE,
        },
    };
    let mut bytes = match serde_json::to_vec(&response) {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("vibe: migration response could not be serialized");
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
        eprintln!("vibe: migration response could not be written");
        return ExitCode::from(GENERAL);
    }
    ExitCode::from(code)
}
