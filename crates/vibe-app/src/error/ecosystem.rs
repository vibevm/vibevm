//! The report of a failed ecosystem-adapter verb, reexported by the error
//! layer (PROP-075 `#ECOSYSTEM-ADAPTER`, `#SLOT-EXTRAS`). It lives beside
//! `error.rs` for the reason [`super::MigrationFailure`] does: a boxed
//! payload is the one error shape with a struct of its own.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");

use specmark::spec;
use thiserror::Error;

use super::AppError;

/// What a failed adapter verb reports. **An extra is an extra**: a failing
/// verb never rolls anything back, never uninstalls, and never moves the
/// slot's default — so the report's whole job is naming the adapter, the
/// items asked for, the step, its log and the exact retry command
/// (PROP-075 `#SLOT-EXTRAS`).
#[derive(Debug, Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
#[error(
    "the `{verb}` verb of the ecosystem adapter `{ecosystem}` of the slot `{slot}` failed on \
     step `{step}`; nothing was rolled back, the version stays installed and the slot's default \
     stays where you put it; its log is `{log}` and the work directory `{work}` is kept \
     (violates spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER; fix: read the \
     log, then run exactly `{retry}`)"
)]
pub struct EcosystemFailure {
    pub slot: String,
    pub ecosystem: String,
    pub verb: &'static str,
    pub step: String,
    pub work: String,
    pub log: String,
    /// The exact command that runs this verb again.
    pub retry: String,
    #[source]
    pub source: Box<AppError>,
}
