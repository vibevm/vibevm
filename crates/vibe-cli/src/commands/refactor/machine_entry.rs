//! Borrowed early dispatch for the two manifest migration machine entries.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES");
use super::{RefactorCommand, migration_operation, migration_plan};
use crate::cli::Command;
use std::process::ExitCode;

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
pub(crate) fn run(command: &Command) -> Option<ExitCode> {
    let Command::Refactor(args) = command else {
        return None;
    };
    match &args.command {
        RefactorCommand::PlanMigration(args) => {
            Some(migration_plan::run(args.request_json.as_deref()))
        }
        RefactorCommand::MigrationOperation(args) => {
            Some(migration_operation::run(args.request_json.as_deref()))
        }
        _ => None,
    }
}
