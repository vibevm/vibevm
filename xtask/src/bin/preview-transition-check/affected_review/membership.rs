//! Finite reviewed relationships and current contribution support, not a net engine.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");

use super::{CheckFailure, CheckReport, Context, FindingState, fault, later, pending};
use std::collections::{BTreeMap, BTreeSet};
use vibe_wire::generated::preview::e1::affected_source_review::AffectedSourceReview;

pub(super) fn verify(
    review: &AffectedSourceReview,
    context: &Context<'_>,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let interfaces: BTreeMap<_, _> = review
        .interfaces
        .iter()
        .map(|interface| (interface.interface_id.as_str(), interface))
        .collect();
    let mut actual_paths: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut evidence_paths: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut anchors: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for record in context.records {
        let current = context
            .current_subjects
            .get(record.change_id.as_str())
            .is_some_and(|subjects| !subjects.is_empty());
        let paths = context.current_paths.get(record.change_id.as_str());
        let required = context
            .scope
            .record_digests
            .iter()
            .find(|binding| binding.change_id == record.change_id)
            .and_then(|binding| binding.required_claim_ids.as_ref());
        let evidence = required.is_some_and(|claims| {
            !claims.is_empty()
                && claims.iter().all(|claim| {
                    report
                        .checked_claims
                        .contains(&format!("evidence-bytes:{}:{claim}", record.change_id))
                })
        });
        let comparable = record.before_capture_id == review.before_capture_id
            && record.after_capture_id == review.after_capture_id;
        for effect in &record.effects {
            let Some(interface) = interfaces.get(effect.interface_id.as_str()) else {
                return Err(fault(
                    "affected-effect-interface",
                    &record.change_id,
                    "Record effect names no reviewed finite interface",
                ));
            };
            if effect.capability_id != interface.capability_id
                || effect.audience != interface.audience
                || effect.platform != interface.platform
            {
                return Err(fault(
                    "affected-effect-scope",
                    &record.change_id,
                    "Record capability/audience/platform differs from reviewed finite relationship",
                ));
            }
            if effect
                .contract_anchors
                .iter()
                .any(|anchor| !interface.contract_anchors.contains(anchor))
            {
                return Err(fault(
                    "affected-effect-anchor",
                    &record.change_id,
                    "Record effect declares a contract anchor outside its reviewed finite relationship",
                ));
            }
            if comparable {
                if effect.kind != interface.kind
                    || effect.before != interface.before
                    || effect.after != interface.after
                {
                    return Err(fault(
                        "affected-effect-comparable-declaration",
                        &record.change_id,
                        "Comparable captured record scope disagrees with reviewed kind/before/after declarations",
                    ));
                }
                report.checked_claims.push(format!(
                    "affected-effect-comparable-declarations:{}:{}",
                    record.change_id, effect.interface_id
                ));
            } else {
                later(
                    report,
                    "affected-effect-semantic-comparison-pending",
                    &record.change_id,
                    "Record has historical/intermediate capture subjects; its effect descriptions/kind are not global net conclusions. Only finite declared membership is checked.",
                    FindingState::Pending,
                );
            }
            report.checked_claims.push(format!(
                "affected-effect-declared-membership:{}:{}",
                record.change_id, effect.interface_id
            ));
            if current {
                let matching: BTreeSet<_> = interface
                    .source_paths
                    .iter()
                    .map(String::as_str)
                    .filter(|path| paths.is_some_and(|paths| paths.contains(*path)))
                    .collect();
                if matching.is_empty() {
                    return Err(fault(
                        "affected-effect-path-membership",
                        &record.change_id,
                        "Current effect has no actual current product path in its reviewed relationship",
                    ));
                }
                actual_paths
                    .entry(&effect.interface_id)
                    .or_default()
                    .extend(&matching);
                anchors
                    .entry(&effect.interface_id)
                    .or_default()
                    .extend(effect.contract_anchors.iter().map(String::as_str));
                if evidence {
                    evidence_paths
                        .entry(&effect.interface_id)
                        .or_default()
                        .extend(matching);
                }
            }
        }
    }
    for interface in &review.interfaces {
        let actual = actual_paths.get(interface.interface_id.as_str());
        if interface
            .source_paths
            .iter()
            .any(|path| !actual.is_some_and(|paths| paths.contains(path.as_str())))
        {
            return Err(fault(
                "affected-interface-current-coverage",
                &interface.interface_id,
                "Reviewed required interface/source path lacks a selected current contribution effect",
            ));
        }
        let selected = anchors.get(interface.interface_id.as_str());
        if interface
            .contract_anchors
            .iter()
            .any(|anchor| !selected.is_some_and(|anchors| anchors.contains(anchor.as_str())))
        {
            return Err(fault(
                "affected-interface-anchor-coverage",
                &interface.interface_id,
                "Reviewed required contract anchor lacks selected current effect membership",
            ));
        }
        let proven = evidence_paths.get(interface.interface_id.as_str());
        if interface
            .source_paths
            .iter()
            .any(|path| !proven.is_some_and(|paths| paths.contains(path.as_str())))
        {
            pending(
                report,
                "affected-interface-current-evidence-unverified",
                &interface.interface_id,
                "Actual path/effect membership does not yet have all selected current required evidence-byte results",
                FindingState::Pending,
            );
        } else {
            report.checked_claims.push(format!(
                "affected-interface-current-evidence-membership:{}:{}",
                review.review_id, interface.interface_id
            ));
        }
        report.checked_claims.push(format!(
            "affected-interface-path-membership:{}:{}",
            review.review_id, interface.interface_id
        ));
        for topic in &interface.topic_ids {
            report.checked_claims.push(format!(
                "affected-topic-obligation-membership:{}:{}:{topic}",
                review.review_id, interface.interface_id
            ));
        }
    }
    Ok(())
}
