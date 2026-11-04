//! Merge only active eligible projections; unanswered exact entries are not bounds.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

use super::{ChoiceConflict, ChoiceGroup, ChoiceProjection, MergeIntent, ProjectionOrigin};
use std::collections::BTreeSet;

pub(super) fn merge(
    group: &ChoiceGroup,
    projections: &[ChoiceProjection],
    conflicts: &mut Vec<ChoiceConflict>,
) -> Option<Vec<String>> {
    let answered: Vec<_> = projections
        .iter()
        .filter(|p| p.origin != ProjectionOrigin::Unanswered)
        .collect();
    let exact: Vec<_> = answered
        .iter()
        .copied()
        .filter(|p| p.intent == MergeIntent::Exact)
        .collect();
    let additive: Vec<_> = answered
        .iter()
        .copied()
        .filter(|p| p.intent == MergeIntent::Additive)
        .collect();
    let before = conflicts.len();
    if let Some(first) = exact.first() {
        for second in exact.iter().skip(1) {
            if first.eligible != second.eligible {
                conflicts.push(ChoiceConflict::ExactDisagreement {
                    group: group.id.clone(),
                    first_consumer: first.consumer.clone(),
                    first_members: first.eligible.clone(),
                    second_consumer: second.consumer.clone(),
                    second_members: second.eligible.clone(),
                });
            }
        }
    }
    for bound in &exact {
        for addition in &additive {
            let outside: Vec<_> = addition
                .eligible
                .iter()
                .filter(|m| !bound.eligible.contains(m))
                .cloned()
                .collect();
            if !outside.is_empty() {
                conflicts.push(ChoiceConflict::AdditiveOutsideExact {
                    group: group.id.clone(),
                    exact_consumer: bound.consumer.clone(),
                    exact_members: bound.eligible.clone(),
                    additive_consumer: addition.consumer.clone(),
                    outside,
                });
            }
        }
    }
    if conflicts.len() != before {
        return None;
    }
    let selected: BTreeSet<_> = match exact.first() {
        Some(bound) => bound.eligible.iter().cloned().collect(),
        None => additive
            .iter()
            .flat_map(|p| p.eligible.iter().cloned())
            .collect(),
    };
    Some(
        group
            .members
            .iter()
            .filter(|id| selected.contains(*id))
            .cloned()
            .collect(),
    )
}
