//! The registry's journal of facts — the truth layer the catalog is
//! projected from (PROP-044 §3, phase Ф3.1 of
//! TZ-CHANGE-NATIVE-FORMATS).
//!
//! The journal is append-only NDJSON sharded by calendar month:
//! [`record`] defines the vocabulary (one fact per line), [`store`]
//! reads and writes it, and [`project`] folds it back into the
//! catalog it describes. The clock never runs in here — every `at`
//! arrives from the caller at the edge (PROP-044 §4.3), which the
//! index clock gate in `tools/self-check.sh` enforces over this
//! directory.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

pub mod project;
pub mod record;
pub mod store;

pub use project::project;
pub use record::{Event, JournalRecord};
pub use store::{append, default_dir, replay};

#[cfg(test)]
#[path = "burial_tests.rs"]
mod burial_tests;

#[cfg(test)]
#[path = "project_tests.rs"]
mod project_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
