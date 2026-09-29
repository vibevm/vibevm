//! Finite captured-source membership; central adequacy and global joins remain open.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");

mod contracts;
mod membership;
mod paths;

use super::{
    CheckFailure, CheckReport, CheckReportVerdict, Finding, FindingState, capture, git_entry,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::{
    affected_source_review::AffectedSourceReview, capture::Capture, release_impact::ReleaseImpact,
    transition::ContributionScope,
};

pub(super) struct Context<'a> {
    pub root: &'a Path,
    pub release: &'a Project,
    pub before: &'a Capture,
    pub after: &'a Capture,
    pub scope: &'a ContributionScope,
    pub records: &'a [ReleaseImpact],
    pub touched: &'a BTreeSet<String>,
    pub touched_complete: bool,
    pub current_paths: &'a BTreeMap<&'a str, BTreeSet<String>>,
    pub current_subjects: &'a BTreeMap<&'a str, BTreeSet<String>>,
}

fn fault(code: &'static str, subject: &str, reason: &str) -> CheckFailure {
    CheckFailure::new(code, subject, reason)
}

fn pending(report: &mut CheckReport, code: &str, subject: &str, reason: &str, state: FindingState) {
    report.verdict = CheckReportVerdict::Pending;
    later(report, code, subject, reason, state);
}

fn later(report: &mut CheckReport, code: &str, subject: &str, reason: &str, state: FindingState) {
    report.findings.push(Finding {
        code: code.into(),
        subject: subject.into(),
        reason: reason.into(),
        state,
    });
}

pub(super) fn check(context: Context<'_>, report: &mut CheckReport) -> Result<(), CheckFailure> {
    let Some(selection) = &context.scope.affected_source_review else {
        later(
            report,
            "affected-source-review-missing",
            "affected-source-review",
            "No exact finite affected-source review is selected. Bounded contribution byte/delta proof does not establish BASE.3 central affected-source review.",
            FindingState::Pending,
        );
        return Ok(());
    };
    let bytes = capture::read_within(
        context.release,
        &selection.source,
        "affected-source-review",
        "affected-review-read",
    )?;
    if capture::sha256(&bytes) != selection.sha256 {
        return Err(fault(
            "affected-review-digest",
            "affected-source-review",
            "Selected review bytes differ from their exact digest",
        ));
    }
    let review: AffectedSourceReview = serde_json::from_slice(&bytes).map_err(|_| {
        fault(
            "affected-review-shape",
            "affected-source-review",
            "Review does not match its registered shape",
        )
    })?;
    review
        .validate_affected_source_review()
        .map_err(|error| fault("affected-review-meaning", error.field, error.reason))?;
    if review.before_capture_id != context.scope.before_capture_id
        || review.after_capture_id != context.scope.after_capture_id
    {
        return Err(fault(
            "affected-review-scope",
            &review.review_id,
            "Review endpoints differ from the selected contribution scope",
        ));
    }
    if capture::digest_within(
        context.release,
        &review.source_record,
        &review.review_id,
        "affected-review-source-read",
    )? != review.source_record_sha256
    {
        return Err(fault(
            "affected-review-source-digest",
            &review.review_id,
            "Retained independent review source bytes differ",
        ));
    }
    report.subject_ids.push(review.review_id.clone());
    report
        .checked_claims
        .push(format!("affected-review-bytes:{}", review.review_id));
    report
        .checked_claims
        .push(format!("affected-review-source-bytes:{}", review.review_id));
    paths::verify(&review, &context, report)?;
    contracts::verify(&review, &context, report)?;
    membership::verify(&review, &context, report)?;
    later(
        report,
        "affected-review-central-adequacy-later",
        &review.review_id,
        "Exact retained bytes and finite declared membership do not authenticate a reviewer or establish semantic adequacy/new API completeness; central actual-delta review remains required.",
        FindingState::Pending,
    );
    later(
        report,
        "affected-review-canonical-native-docs-runtime-later",
        &review.review_id,
        "Finite review subclaims do not establish complete canonical capability/topic census, native/migration joins, actual documentation pages or runtime/release proof.",
        FindingState::Pending,
    );
    Ok(())
}
