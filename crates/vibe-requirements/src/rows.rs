//! Row construction: the A1 adoption join mapped exhaustively into the
//! generated closed vocabularies.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use progress_core::model::{Stage, State};
use vibe_facts::AdoptionObservation;
use vibe_wire::generated::requirements_report::{
    AdoptionObservationPresence, AuthoringObservationPresence, FactStatusStage, FactStatusState,
    RequirementRow, RequirementSource, RequirementSourceKind,
};

use crate::QueryError;

/// Map the A1 join's observation of one source into wire rows.
///
/// Rows come back sorted by full address (the join already sorts per
/// source; the caller merges and re-sorts globally). The row's source
/// coordinate is the scan coordinate — the same halves its address was
/// minted from, so the `row-source-binding` law holds by construction.
pub(crate) fn build(
    kind: &RequirementSourceKind,
    package: &str,
    joined: &[vibe_facts::AdoptionRow],
) -> Vec<RequirementRow> {
    joined
        .iter()
        .map(|row| RequirementRow {
            address: row.address.clone(),
            source: RequirementSource {
                kind: kind.clone(),
                package: package.to_string(),
            },
            authoring: vibe_wire::generated::requirements_report::AuthoringObservation {
                presence: if row.authored_status.is_some() {
                    AuthoringObservationPresence::Marked
                } else {
                    AuthoringObservationPresence::Unmarked
                },
                status: row.authored_status.map(map_status),
            },
            adoption: vibe_wire::generated::requirements_report::AdoptionObservation {
                presence: map_adoption(&row.adoption),
                status: match row.adoption {
                    AdoptionObservation::Recorded(status) => Some(map_status(status)),
                    _ => None,
                },
            },
            relations: Vec::new(),
        })
        .collect()
}

fn map_adoption(adoption: &AdoptionObservation) -> AdoptionObservationPresence {
    match adoption {
        AdoptionObservation::NotApplicable => AdoptionObservationPresence::NotApplicable,
        AdoptionObservation::Absent => AdoptionObservationPresence::Absent,
        AdoptionObservation::Indeterminate => AdoptionObservationPresence::Indeterminate,
        AdoptionObservation::Recorded(_) => AdoptionObservationPresence::Recorded,
    }
}

/// The exhaustive progress-core → wire status mapping. Both vocabularies
/// are the same closed PROP-043 sets; the match is exhaustive so a new
/// value on either side is a compile error here, never a silent drop.
pub fn map_status(
    status: vibe_facts::FactStatus,
) -> vibe_wire::generated::requirements_report::FactStatus {
    vibe_wire::generated::requirements_report::FactStatus {
        stage: map_stage(status.stage()),
        state: map_state(status.state()),
    }
}

fn map_stage(stage: Stage) -> FactStatusStage {
    match stage {
        Stage::Unknown => FactStatusStage::Unknown,
        Stage::Idea => FactStatusStage::Idea,
        Stage::Spec => FactStatusStage::Spec,
        Stage::Impl => FactStatusStage::Impl,
        Stage::Test => FactStatusStage::Test,
        Stage::Doc => FactStatusStage::Doc,
        Stage::Freeze => FactStatusStage::Freeze,
    }
}

fn map_state(state: State) -> FactStatusState {
    match state {
        State::Hold => FactStatusState::Hold,
        State::Plan => FactStatusState::Plan,
        State::Work => FactStatusState::Work,
        State::Done => FactStatusState::Done,
        State::Void => FactStatusState::Void,
    }
}

/// Two rows bearing one full address is an invariant, never something
/// to repair: the sorted merge refuses adjacent duplicates instead of
/// deduplicating them — no report row may disappear to make validation
/// green. (Ordinary construction cannot reach this — the A2a scanner
/// refuses duplicate addresses per source and sources are unique by
/// coordinate — so the law is enforced at the merge, where a future
/// bug would land.)
pub(crate) fn refuse_duplicate_addresses(
    rows: &[vibe_wire::generated::requirements_report::RequirementRow],
) -> Result<(), QueryError> {
    for pair in rows.windows(2) {
        if pair[0].address == pair[1].address {
            return Err(QueryError::Invariant(format!(
                "duplicate full fact address `{}` reached the row merge",
                pair[0].address
            )));
        }
    }
    Ok(())
}

/// The row count must fit the wire's `uint32` row bound — an overflow is
/// a typed invariant error, never a truncated-in-silence answer.
pub(crate) fn checked_row_count(rows: usize) -> Result<u32, QueryError> {
    u32::try_from(rows).map_err(|_| {
        QueryError::Invariant(format!("row count {rows} exceeds the wire's row bound"))
    })
}
