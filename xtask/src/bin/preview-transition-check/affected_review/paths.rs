//! Endpoint identities and the already-traversed product touched-path union.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");

use super::{CheckFailure, CheckReport, Context, FindingState, capture, fault, git_entry, pending};
use std::collections::BTreeSet;
use vibe_wire::generated::preview::e1::affected_source_review::{
    AffectedSourceReview, BlobIdentity,
};

pub(super) fn row<'a>(
    raw: &'a [u8],
    subject: &str,
    report: &mut CheckReport,
) -> Result<Option<git_entry::Entry<'a>>, CheckFailure> {
    match git_entry::decode(raw) {
        Ok(entry) => Ok(Some(entry)),
        Err(git_entry::DecodeError::PathEncoding) => {
            pending(
                report,
                "affected-review-git-path-unsupported",
                subject,
                "Captured path cannot be represented by the UTF-8 wire format",
                FindingState::Unsupported,
            );
            Ok(None)
        }
        Err(error) => Err(fault("affected-review-git-row", subject, error.reason())),
    }
}

pub(super) enum Blob {
    Absent,
    PresentUnsupported,
    Regular(String, Vec<u8>),
}

pub(super) fn blob(
    context: &Context<'_>,
    commit: &str,
    path: &str,
    subject: &str,
    report: &mut CheckReport,
) -> Result<Blob, CheckFailure> {
    let raw = capture::git(
        context.root,
        &["ls-tree", "-z", "--full-tree", commit, "--", path],
    )?;
    let mut entries = git_entry::rows(&raw);
    let Some(first) = entries.next() else {
        return Ok(Blob::Absent);
    };
    if entries.next().is_some() {
        return Err(fault(
            "affected-review-git-entry",
            subject,
            "Selected path does not denote exactly one immutable entry",
        ));
    }
    let Some(entry) = row(first, subject, report)? else {
        return Ok(Blob::PresentUnsupported);
    };
    if entry.repository_path != path {
        return Err(fault(
            "affected-review-git-entry",
            subject,
            "Immutable entry differs from selected literal path",
        ));
    }
    if !entry.regular() {
        pending(
            report,
            "affected-review-git-entry-unsupported",
            subject,
            "Symlink/gitlink/nonregular entry is not admitted as a regular blob",
            FindingState::Unsupported,
        );
        return Ok(Blob::PresentUnsupported);
    }
    let bytes = capture::git(context.root, &["cat-file", "blob", entry.oid])?;
    Ok(Blob::Regular(entry.oid.into(), bytes))
}

pub(super) fn required_blob(
    context: &Context<'_>,
    path: &str,
    oid: Option<&str>,
    sha: &str,
    subject: &str,
    report: &mut CheckReport,
) -> Result<Option<Vec<u8>>, CheckFailure> {
    let actual = blob(
        context,
        &context.after.repository_commit_sha1,
        path,
        subject,
        report,
    )?;
    let (observed, bytes) = match actual {
        Blob::Regular(oid, bytes) => (oid, bytes),
        Blob::Absent => {
            return Err(fault(
                "affected-contract-source-missing",
                subject,
                "Selected current contract source is absent from the immutable endpoint",
            ));
        }
        Blob::PresentUnsupported => return Ok(None),
    };
    if oid.is_some_and(|oid| oid != observed) || capture::sha256(&bytes) != sha {
        return Err(fault(
            "affected-contract-source-identity",
            subject,
            "Current contract source blob/digest differs from its binding",
        ));
    }
    Ok(Some(bytes))
}

fn endpoint(
    context: &Context<'_>,
    commit: &str,
    path: &str,
    expected: Option<&BlobIdentity>,
    subject: &str,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let actual = blob(context, commit, path, subject, report)?;
    match (actual, expected) {
        (Blob::Regular(oid, bytes), Some(expected)) => {
            if oid != expected.git_blob_sha1 || capture::sha256(&bytes) != expected.sha256 {
                return Err(fault(
                    "affected-review-endpoint-identity",
                    subject,
                    "Endpoint blob/digest differs from reviewed binding",
                ));
            }
            report
                .checked_claims
                .push(format!("affected-path-endpoint-bytes:{subject}"));
        }
        (Blob::Regular(_, _), None) | (Blob::PresentUnsupported, None) => {
            return Err(fault(
                "affected-review-endpoint-absence",
                subject,
                "Reviewed endpoint absence contradicts an actual immutable entry",
            ));
        }
        (Blob::Absent, Some(_)) => {
            return Err(fault(
                "affected-review-endpoint-missing",
                subject,
                "Reviewed regular endpoint is absent from immutable source",
            ));
        }
        (Blob::Absent, None) => report
            .checked_claims
            .push(format!("affected-path-endpoint-absence:{subject}")),
        _ => {}
    }
    Ok(())
}

pub(super) fn verify(
    review: &AffectedSourceReview,
    context: &Context<'_>,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let declared: BTreeSet<_> = review.paths.iter().map(|path| path.path.clone()).collect();
    if &declared != context.touched {
        return Err(fault(
            "affected-review-path-census",
            &review.review_id,
            "Reviewed product paths differ from actual traversed touched paths, including transient/reverted paths",
        ));
    }
    if !context.touched_complete {
        pending(
            report,
            "affected-review-path-census-unsupported",
            &review.review_id,
            "Unrepresentable touched paths prevent a complete finite path census",
            FindingState::Unsupported,
        );
    } else {
        report.checked_claims.push(format!(
            "affected-review-product-path-census:{}",
            review.review_id
        ));
    }
    for path in &review.paths {
        endpoint(
            context,
            &context.before.repository_commit_sha1,
            &path.path,
            path.before.as_ref(),
            &format!("{}:before:{}", review.review_id, path.path),
            report,
        )?;
        endpoint(
            context,
            &context.after.repository_commit_sha1,
            &path.path,
            path.after.as_ref(),
            &format!("{}:after:{}", review.review_id, path.path),
            report,
        )?;
    }
    Ok(())
}
