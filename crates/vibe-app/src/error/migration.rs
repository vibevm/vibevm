//! The report of a failed migration side, reexported by the error layer
//! (PROP-075 `#MIGRATION-FAILURE`). It lives beside `error.rs` rather than
//! in it because it is boxed, and a boxed payload is the one error shape
//! with a struct of its own.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE");

use specmark::spec;
use thiserror::Error;

use super::AppError;

/// What a failed migration side reports (PROP-075 `#MIGRATION-FAILURE`):
/// nothing is rolled back, so the report's whole job is naming what was not
/// carried and how to retry exactly that.
#[derive(Debug, Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
#[error(
    "the {side} step `{step}` of migration entry `{entry}` of the slot `{slot}` failed; nothing \
     was rolled back, the new version stays installed and the slot's default stays where you put \
     it; its log is `{log}` and the work directory `{work}` is kept (violates \
     spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE; fix: read the log, then \
     retry that entry alone with `--only {entry} --resume`)"
)]
pub struct MigrationFailure {
    pub slot: String,
    pub entry: String,
    pub side: &'static str,
    pub step: String,
    pub work: String,
    pub log: String,
    #[source]
    pub source: Box<AppError>,
}
