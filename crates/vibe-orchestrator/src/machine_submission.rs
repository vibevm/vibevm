//! Pre-effect machine admission over a domain-owned durable journal.
//!
//! This is a neutral service, not a default-lifecycle adapter. Its journal
//! port exists so each real domain can later supply its own preparation,
//! result retention and recovery without inventing a universal executor.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SUBMISSION-RESERVATION");

use std::fmt;
use std::sync::Arc;

use specmark::spec;
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, ReservationDecision, SubmissionError, SubmissionNamespace,
    SubmissionReservations, SubmissionScope,
};

/// Exact domain-journal observation under the same outer mutation lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalState<T> {
    Absent,
    Prepared,
    Complete(T),
    Uncertain(String),
}

/// A domain owner supplies a pure proposed identity and durable journal
/// operations. `initialize` must publish the exact ID before returning;
/// `invoke` must retain its result in that journal before returning success.
/// The service re-inspects both claims before allowing the next transition.
pub trait SubmissionJournal {
    type Outcome: Clone + Eq;

    fn proposed_run_id(&self) -> String;
    fn inspect(
        &mut self,
        lease: &LifecycleLease,
        run_id: &str,
    ) -> Result<JournalState<Self::Outcome>, String>;
    fn initialize(&mut self, lease: Arc<LifecycleLease>, run_id: &str) -> Result<(), String>;
    fn invoke(&mut self, lease: Arc<LifecycleLease>, run_id: &str)
    -> Result<Self::Outcome, String>;
}

/// A replay never dispatches; a pending reservation can only prepare its
/// journal and then report recovery required. `Retired` conveys no effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionOutcome<T> {
    Invoked { run_id: String, outcome: T },
    Replayed { run_id: String, outcome: T },
    RecoveryRequired { run_id: String },
    Retired { run_id: Option<String> },
}

#[derive(Debug)]
pub enum SubmissionFailure {
    Reservation(SubmissionError),
    Journal(String),
    MissingJournal { run_id: String },
    ConflictingJournal { run_id: String },
    UncertainJournal { run_id: String, reason: String },
    UnretainedOutcome { run_id: String },
}

impl fmt::Display for SubmissionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reservation(error) => write!(f, "submission reservation: {error}"),
            Self::Journal(error) => write!(f, "domain journal: {error}"),
            Self::MissingJournal { run_id } => {
                write!(f, "prepared journal for {run_id} disappeared")
            }
            Self::ConflictingJournal { run_id } => {
                write!(f, "journal for {run_id} has a conflicting identity")
            }
            Self::UncertainJournal { run_id, reason } => {
                write!(f, "journal for {run_id} is uncertain: {reason}")
            }
            Self::UnretainedOutcome { run_id } => {
                write!(f, "effect for {run_id} did not retain a matching outcome")
            }
        }
    }
}

impl std::error::Error for SubmissionFailure {}

impl From<SubmissionError> for SubmissionFailure {
    fn from(value: SubmissionError) -> Self {
        Self::Reservation(value)
    }
}

fn inspect<J: SubmissionJournal>(
    journal: &mut J,
    lease: &LifecycleLease,
    run_id: &str,
) -> Result<JournalState<J::Outcome>, SubmissionFailure> {
    journal
        .inspect(lease, run_id)
        .map_err(SubmissionFailure::Journal)
}

/// Submit only against a trusted scope/namespace already issued by the host.
/// The same lease covers reservation, journal preparation, effect and result.
/// Existing and retired reservations never reach `invoke`.
#[allow(clippy::too_many_arguments)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SUBMISSION-RESERVATION",
    r = 2
)]
pub fn submit<J: SubmissionJournal>(
    lease: Arc<LifecycleLease>,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    canonical_request: &[u8],
    canonical_plan: &[u8],
    journal: &mut J,
) -> Result<SubmissionOutcome<J::Outcome>, SubmissionFailure> {
    let proposed = journal.proposed_run_id();
    let decision = SubmissionReservations::reserve(
        &lease,
        namespace,
        scope,
        key,
        canonical_request,
        canonical_plan,
        &proposed,
    )?;
    let (run_id, created) = match decision {
        ReservationDecision::Created { domain_run_id } => (domain_run_id, true),
        ReservationDecision::Existing { domain_run_id } => (domain_run_id, false),
        ReservationDecision::Retired {
            domain_run_id: Some(run_id),
        } => {
            // Expiry forbids dispatch but does not erase a retained outcome.
            if SubmissionReservations::journal_admission(&lease, namespace, scope, key, &run_id)?
                == JournalAdmission::Ready
            {
                return match inspect(journal, &lease, &run_id)? {
                    JournalState::Complete(outcome) => {
                        Ok(SubmissionOutcome::Replayed { run_id, outcome })
                    }
                    JournalState::Absent => Err(SubmissionFailure::MissingJournal { run_id }),
                    JournalState::Uncertain(reason) => {
                        Err(SubmissionFailure::UncertainJournal { run_id, reason })
                    }
                    JournalState::Prepared => Ok(SubmissionOutcome::RecoveryRequired { run_id }),
                };
            }
            return Ok(SubmissionOutcome::Retired {
                run_id: Some(run_id),
            });
        }
        ReservationDecision::Retired {
            domain_run_id: None,
        } => return Ok(SubmissionOutcome::Retired { run_id: None }),
    };
    let ready = SubmissionReservations::journal_admission(&lease, namespace, scope, key, &run_id)?;
    let observed = inspect(journal, &lease, &run_id)?;
    match (ready, observed) {
        (JournalAdmission::Ready, JournalState::Complete(_)) if created => {
            return Err(SubmissionFailure::ConflictingJournal { run_id });
        }
        (JournalAdmission::Ready, JournalState::Complete(outcome)) => {
            return Ok(SubmissionOutcome::Replayed { run_id, outcome });
        }
        (JournalAdmission::Ready, JournalState::Absent) => {
            return Err(SubmissionFailure::MissingJournal { run_id });
        }
        (JournalAdmission::Ready, JournalState::Uncertain(reason)) => {
            return Err(SubmissionFailure::UncertainJournal { run_id, reason });
        }
        (JournalAdmission::Ready, JournalState::Prepared) => {
            return Ok(SubmissionOutcome::RecoveryRequired { run_id });
        }
        (JournalAdmission::Pending, JournalState::Complete(_)) => {
            return Err(SubmissionFailure::ConflictingJournal { run_id });
        }
        (JournalAdmission::Pending, JournalState::Uncertain(reason)) => {
            return Err(SubmissionFailure::UncertainJournal { run_id, reason });
        }
        (JournalAdmission::Pending, JournalState::Absent) => {
            journal
                .initialize(Arc::clone(&lease), &run_id)
                .map_err(SubmissionFailure::Journal)?;
            match inspect(journal, &lease, &run_id)? {
                JournalState::Prepared => {}
                JournalState::Absent => return Err(SubmissionFailure::MissingJournal { run_id }),
                JournalState::Complete(_) => {
                    return Err(SubmissionFailure::ConflictingJournal { run_id });
                }
                JournalState::Uncertain(reason) => {
                    return Err(SubmissionFailure::UncertainJournal { run_id, reason });
                }
            }
        }
        (JournalAdmission::Pending, JournalState::Prepared) if created => {
            return Err(SubmissionFailure::ConflictingJournal { run_id });
        }
        (JournalAdmission::Pending, JournalState::Prepared) => {}
    }
    SubmissionReservations::mark_journal_ready(&lease, namespace, scope, key, &run_id)?;
    if !created {
        // A predecessor might have crashed after reserving or after journal
        // preparation. Reconcile its identity, but never infer effect safety
        // from a missing result and invoke it a second time.
        return Ok(SubmissionOutcome::RecoveryRequired { run_id });
    }
    let outcome = journal
        .invoke(Arc::clone(&lease), &run_id)
        .map_err(SubmissionFailure::Journal)?;
    match inspect(journal, &lease, &run_id)? {
        JournalState::Complete(retained) if retained == outcome => {
            Ok(SubmissionOutcome::Invoked { run_id, outcome })
        }
        JournalState::Uncertain(reason) => {
            Err(SubmissionFailure::UncertainJournal { run_id, reason })
        }
        _ => Err(SubmissionFailure::UnretainedOutcome { run_id }),
    }
}

#[cfg(test)]
mod tests;
