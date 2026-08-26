//! Strict ownership receipts, durable staging, capability-safe mutation, and
//! R1-style per-file reconciliation for the automatic project-scope
//! package-skill binding.

#[path = "receipt/containment.rs"]
mod containment;
#[path = "receipt/nofollow.rs"]
mod nofollow;
#[path = "receipt/reconcile.rs"]
mod reconcile;
#[path = "receipt/recover.rs"]
mod recover;
#[path = "receipt/stage.rs"]
mod stage;
#[path = "receipt/state.rs"]
mod state;
#[path = "receipt/transaction.rs"]
mod transaction;

pub(super) use containment::{
    FoldKey, ensure_no_follow_walk, fold_key, paths_overlap, valid_relative_file,
};
pub(crate) use reconcile::{reconcile_binding, reconcile_vanished};
pub(crate) use recover::recover_pending;
pub(crate) use state::{
    probe_binding, probe_recovered, probe_vanished, receipt_exists_project_root,
};

#[cfg(test)]
#[path = "receipt/concurrency_tests.rs"]
mod concurrency_tests;

#[cfg(test)]
#[path = "receipt/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "receipt/transaction_red_tests.rs"]
mod transaction_red_tests;
